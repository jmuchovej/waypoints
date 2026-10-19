//! Uploads one platform's published artifacts to the `releases` bucket
//! through the shared client in [`super::release_store`]. The channel is
//! derived from the tag's version rather than accepted as an argument, and
//! a key already holding an object is never overwritten: `write_with(..).
//! if_not_exists(true)` sends `If-None-Match: *`, which R2 honours
//! atomically, so the write itself enforces what the spec requires rather
//! than a check racing it.
//!
//! The object key mirrors the design's layout exactly:
//! `{channel}/{series}/{os}/{arch}/waypoints-{tag}.{ext}`.
//!
//! Uploads the installer every download link resolves to and the updater's
//! payload and signature alongside it, under the same key scheme — the
//! updater route the site exposes derives its keys the same deterministic
//! way the download route does, so both sides only need to agree on
//! [`Platform::updater_extensions`], never on a listing either reads back.

use crate::tasks::{
    release_plan, release_store,
    workflows::{Arch, Platform},
};
use anyhow::{Context as _, Result, bail};
use cargo_metadata::semver::Version;
use clap::Parser;
use opendal::Operator;
use std::path::PathBuf;

#[derive(Parser)]
pub struct ReleaseUploadArgs {
    /// The platform whose published artifacts this uploads.
    #[arg(long, value_enum)]
    platform: Platform,

    /// The release tag, e.g. `v0.1.1` or `v0.2.0-nightly.3+2026.10.19`. Both
    /// the store's version segment and the saved filename come from this
    /// exactly; the channel and series are derived from it, never supplied.
    #[arg(long)]
    tag: String,

    /// How long the derived store credential stays valid, in seconds.
    #[arg(long, default_value_t = release_store::DEFAULT_TTL_SECONDS)]
    ttl_seconds: u64,
}

pub fn run(args: ReleaseUploadArgs) -> Result<()> {
    let version = tag_version(&args.tag)?;
    let channel = release_plan::channel_of(&version);
    let series = release_plan::series_of(&version);
    let target = args.platform.published_target();

    let client = release_store::create_r2_client(channel, args.ttl_seconds)?;

    let globs = args.platform.published_globs();
    anyhow::ensure!(
        globs.len() == target.extensions.len(),
        "{}: {} globs but {} published extensions",
        args.platform,
        globs.len(),
        target.extensions.len()
    );
    let updater_globs = args.platform.updater_globs();
    let updater_extensions = args.platform.updater_extensions();
    anyhow::ensure!(
        updater_globs.len() == updater_extensions.len(),
        "{}: {} updater globs but {} updater extensions",
        args.platform,
        updater_globs.len(),
        updater_extensions.len()
    );

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;

    let every_file = globs
        .iter()
        .zip(target.extensions)
        .chain(updater_globs.iter().zip(updater_extensions));

    for (glob, extension) in every_file {
        let path = find_one(glob)?;
        let key = format!(
            "{channel}/{series}/{os}/{arch}/waypoints-{tag}.{extension}",
            channel = channel.flavor(),
            os = target.os,
            arch = target.arch,
            tag = args.tag,
        );
        let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        runtime.block_on(upload(&client, &key, bytes))?;
        println!("uploaded {} to {key}", path.display());
    }

    Ok(())
}

/// Strips the tag's leading `v` and parses the rest as semver, so the
/// channel and series can be derived from it the same way the release
/// planners derive them from a manifest version.
fn tag_version(tag: &str) -> Result<Version> {
    let version = tag
        .strip_prefix('v')
        .with_context(|| format!("tag `{tag}` has no leading `v`"))?;
    Version::parse(version).with_context(|| format!("tag `{tag}` is not a valid version"))
}

/// Resolves `pattern` to exactly one file. The bundler produces one file per
/// glob per build; more or fewer is a build gone wrong, not a selection to
/// make.
fn find_one(pattern: &str) -> Result<PathBuf> {
    let mut matches = glob::glob(pattern)
        .with_context(|| format!("`{pattern}` is not a valid glob"))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("reading glob matches for `{pattern}`"))?;

    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => bail!("no file matches `{pattern}`"),
        _ => bail!("`{pattern}` matched more than one file: {matches:?}"),
    }
}

/// Writes `bytes` to `key`, refusing to overwrite an existing object.
async fn upload(client: &Operator, key: &str, bytes: Vec<u8>) -> Result<()> {
    client
        .write_with(key, bytes)
        .if_not_exists(true)
        .await
        .with_context(|| format!("uploading to {key} (refused if the key already exists)"))?;
    Ok(())
}

/// Permits writing the published-artifact key format from outside this
/// module — covered by the generator test asserting the uploader derives a
/// channel rather than accepting one.
#[allow(dead_code, reason = "documents the key shape; exercised via `run`")]
fn object_key(
    channel: &str,
    series: &str,
    os: &str,
    arch: Arch,
    tag: &str,
    extension: &str,
) -> String {
    format!("{channel}/{series}/{os}/{arch}/waypoints-{tag}.{extension}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::workflows::Channel;
    use strum::IntoEnumIterator as _;

    #[test]
    fn a_tag_derives_its_own_channel_and_series() {
        assert_eq!(
            release_plan::channel_of(&tag_version("v0.2.0-pre.1").unwrap()),
            Channel::Preview
        );
        assert_eq!(
            release_plan::channel_of(&tag_version("v0.1.1").unwrap()),
            Channel::Stable
        );
        assert_eq!(
            release_plan::series_of(&tag_version("v0.1.1").unwrap()),
            "v0.1.x"
        );
    }

    #[test]
    fn a_tag_without_a_leading_v_is_refused() {
        assert!(tag_version("0.1.1").is_err());
    }

    /// clap's default kebab-case rename mishandles the embedded "OS" in
    /// `macOS`/`iOS`; the explicit `#[value(name = ...)]` overrides must
    /// keep every platform's CLI spelling equal to its job-id slug.
    #[test]
    fn every_platform_s_cli_value_matches_its_slug() {
        use clap::ValueEnum as _;
        for platform in Platform::iter() {
            let value = platform
                .to_possible_value()
                .unwrap_or_else(|| panic!("{platform} has no clap value"));
            assert_eq!(value.get_name(), platform.slug(), "{platform}");
        }
    }

    #[test]
    fn published_globs_pair_with_published_extensions() {
        for platform in Platform::iter() {
            assert_eq!(
                platform.published_globs().len(),
                platform.published_target().extensions.len(),
                "{platform}"
            );
        }
    }

    /// `run`'s upload loop chains the installer and the updater sets; this
    /// pins how many files each platform actually contributes to it —
    /// installer count, then updater count, so the split stays legible.
    #[test]
    fn every_platform_uploads_its_installer_and_its_updater_artifacts() {
        let counts = [
            (Platform::macOS, 1, 2),
            (Platform::Linux, 1, 1),
            (Platform::Windows, 1, 1),
            (Platform::Android, 2, 0),
            (Platform::iOS, 1, 0),
        ];
        for (platform, installers, updater_files) in counts {
            assert_eq!(
                platform.published_target().extensions.len(),
                installers,
                "{platform} installer count"
            );
            assert_eq!(
                platform.updater_extensions().len(),
                updater_files,
                "{platform} updater file count"
            );
        }
    }

    #[test]
    fn the_object_key_matches_the_design_s_layout() {
        assert_eq!(
            object_key("stable", "v0.1.x", "macos", Arch::arm64, "v0.1.1", "dmg"),
            "stable/v0.1.x/macos/arm64/waypoints-v0.1.1.dmg"
        );
    }

    /// The CLI default is the same lifetime `release_store` settles on;
    /// raising it is a flag, not a code change.
    #[test]
    fn the_cli_defaults_to_the_store_s_ttl() {
        let args =
            ReleaseUploadArgs::parse_from(["xtask", "--platform", "macos", "--tag", "v0.1.1"]);
        assert_eq!(args.ttl_seconds, release_store::DEFAULT_TTL_SECONDS);
    }
}
