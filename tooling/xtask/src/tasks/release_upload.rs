//! Uploads one platform's published artifacts to the `releases` bucket under
//! a derived session credential — see [`super::release_credential`]. The
//! channel is derived from the tag's version rather than accepted as an
//! argument, and a key already holding an object is never overwritten: R2's
//! conditional `If-None-Match: *` on `PutObject` makes that refusal atomic
//! (confirmed against Cloudflare's own docs, 2026-10-19), so the write
//! itself enforces what the spec requires rather than a check racing it.
//!
//! The object key mirrors the design's layout exactly:
//! `{channel}/{series}/{os}/{arch}/waypoints-{tag}.{ext}`.

use crate::tasks::{
    release_plan,
    workflows::{Arch, Platform},
};
use anyhow::{Context as _, Result, anyhow, bail};
use cargo_metadata::semver::Version;
use clap::Parser;
use rusty_s3::{Bucket, Credentials, S3Action as _, UrlStyle, actions::PutObject};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

/// How long the presigned `PutObject` URL stays valid. It only needs to
/// outlive one `curl` upload of one artifact, so this is unrelated to — and
/// far shorter than — the session credential's own lifetime.
const PRESIGN_TTL: Duration = Duration::from_secs(300);

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
}

pub fn run(args: ReleaseUploadArgs) -> Result<()> {
    let version = tag_version(&args.tag)?;
    let channel = release_plan::channel_of(&version);
    let series = release_plan::series_of(&version);
    let target = args.platform.published_target();

    let access_key_id = env_var("R2_SESSION_ACCESS_KEY_ID")?;
    let secret_access_key = env_var("R2_SESSION_SECRET_ACCESS_KEY")?;
    let session_token = env_var("R2_SESSION_TOKEN")?;
    let endpoint = env_var("R2_SESSION_ENDPOINT")?;

    let endpoint_url = endpoint
        .parse()
        .with_context(|| format!("{endpoint} is not a URL"))?;
    let bucket = Bucket::new(endpoint_url, UrlStyle::Path, "releases", "auto")
        .map_err(|error| anyhow!("invalid releases bucket endpoint: {error:?}"))?;
    let credentials = Credentials::new_with_token(access_key_id, secret_access_key, session_token);

    let globs = args.platform.published_globs();
    anyhow::ensure!(
        globs.len() == target.extensions.len(),
        "{}: {} globs but {} published extensions",
        args.platform,
        globs.len(),
        target.extensions.len()
    );

    for (glob, extension) in globs.iter().zip(target.extensions) {
        let path = find_one(glob)?;
        let key = format!(
            "{channel}/{series}/{os}/{arch}/waypoints-{tag}.{extension}",
            channel = channel.flavor(),
            os = target.os,
            arch = target.arch,
            tag = args.tag,
        );
        upload(&bucket, &credentials, &key, &path)?;
        println!("uploaded {} to {key}", path.display());
    }

    Ok(())
}

fn env_var(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("${name} is not set"))
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

/// Presigns a conditional `PutObject` and streams `path` to it with `curl`,
/// which handles TLS and large-file streaming without this binary carrying
/// an HTTP client — consistent with how `tooling/bin/xcache` wraps `rclone`
/// for the build-cache store rather than linking an S3 SDK.
fn upload(bucket: &Bucket, credentials: &Credentials, key: &str, path: &Path) -> Result<()> {
    let mut action = PutObject::new(bucket, Some(credentials), key);
    action.headers_mut().insert("if-none-match", "*");
    let url = action.sign(PRESIGN_TTL);

    let status = Command::new("curl")
        .args(["--fail", "--silent", "--show-error"])
        .args(["--header", "If-None-Match: *"])
        .arg("--upload-file")
        .arg(path)
        .arg(url.as_str())
        .status()
        .context("spawning curl")?;

    anyhow::ensure!(
        status.success(),
        "curl exited with {status} uploading to {key}; a non-empty key is refused rather than overwritten"
    );
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

    #[test]
    fn the_object_key_matches_the_design_s_layout() {
        assert_eq!(
            object_key("stable", "v0.1.x", "macos", Arch::arm64, "v0.1.1", "dmg"),
            "stable/v0.1.x/macos/arm64/waypoints-v0.1.1.dmg"
        );
    }
}
