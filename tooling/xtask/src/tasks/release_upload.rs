//! Uploads published artifacts to the `releases` bucket through the shared
//! client in [`super::release_store`]. The channel is derived from the tag's
//! version rather than accepted as an argument, and a key already holding an
//! object is never overwritten: `write_with(..).if_not_exists(true)` sends
//! `If-None-Match: *`, which R2 honours atomically, so the write itself
//! enforces what the spec requires rather than a check racing it.
//!
//! The object key mirrors the design's layout exactly:
//! `{channel}/{series}/{os}/{arch}/waypoints-{tag}.{ext}`.
//!
//! Reads a directory [`super::release_stage`] filled, whose paths already are
//! the keys. Every staged path is checked against the keys the scheme
//! produces for the tag before any credential is derived.
//!
//! Uploads the installer every download link resolves to and the updater's
//! payload and signature alongside it, under the same key scheme — the
//! updater route the site exposes derives its keys the same deterministic
//! way the download route does, so both sides only need to agree on
//! [`Platform::updater_extensions`], never on a listing either reads back.

use crate::tasks::{
    release_plan, release_store,
    workflows::{_types::PublishedTarget, Channel, Platform},
};
use anyhow::{Context as _, Result, bail};
use cargo_metadata::semver::Version;
use clap::Parser;
use opendal::Operator;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(Parser)]
pub struct ReleaseUploadArgs {
    /// A directory `release-stage` filled; every file under it is uploaded
    /// to the key its path names.
    #[arg(long)]
    from: PathBuf,

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

    let files = staged_files(&args.from, channel, &args.tag)?;

    let client = release_store::create_r2_client(channel, args.ttl_seconds)?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;

    for (path, key) in files {
        let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        runtime.block_on(upload(&client, &key, bytes))?;
        println!("uploaded {} to {key}", path.display());
    }

    Ok(())
}

/// Strips the tag's leading `v` and parses the rest as semver, so the
/// channel and series can be derived from it the same way the release
/// planners derive them from a manifest version.
pub fn tag_version(tag: &str) -> Result<Version> {
    let version = tag
        .strip_prefix('v')
        .with_context(|| format!("tag `{tag}` has no leading `v`"))?;
    Version::parse(version).with_context(|| format!("tag `{tag}` is not a valid version"))
}

/// Pairs each of `platform`'s published and updater artifacts under `root`
/// with the key it is published at.
pub fn built_files(platform: Platform, tag: &str, root: &Path) -> Result<Vec<(PathBuf, String)>> {
    let version = tag_version(tag)?;
    let channel = release_plan::channel_of(&version);
    let series = release_plan::series_of(&version);
    let target = platform.published_target();

    let globs = platform.published_globs();
    anyhow::ensure!(
        globs.len() == target.extensions.len(),
        "{platform}: {} globs but {} published extensions",
        globs.len(),
        target.extensions.len()
    );
    let updater_globs = platform.updater_globs();
    let updater_extensions = platform.updater_extensions();
    anyhow::ensure!(
        updater_globs.len() == updater_extensions.len(),
        "{platform}: {} updater globs but {} updater extensions",
        updater_globs.len(),
        updater_extensions.len()
    );

    globs
        .iter()
        .zip(target.extensions)
        .chain(updater_globs.iter().zip(updater_extensions))
        .map(|(glob, extension)| {
            let path = find_one(root, glob)?;
            Ok((path, object_key(channel, &series, &target, tag, extension)))
        })
        .collect()
}

/// Every file under `dir`, paired with the key its relative path names.
/// Refuses a path that is not a key the scheme produces for `tag` in
/// `channel` — another channel's or series', or a file no build publishes —
/// and an empty directory.
pub(crate) fn staged_files(
    dir: &Path,
    channel: Channel,
    tag: &str,
) -> Result<Vec<(PathBuf, String)>> {
    let expected = expected_keys(channel, tag)?;

    let mut files = Vec::new();
    for path in walk(dir)? {
        let key = path
            .strip_prefix(dir)
            .expect("walked from `dir`")
            .components()
            .map(|part| part.as_os_str().to_str())
            .collect::<Option<Vec<_>>>()
            .with_context(|| format!("{} is not UTF-8", path.display()))?
            .join("/");
        if !expected.contains(&key) {
            bail!("{} is not a store key for {tag}", path.display());
        }
        files.push((path, key));
    }

    if files.is_empty() {
        bail!("nothing staged under {}", dir.display());
    }
    Ok(files)
}

/// Every key `channel`'s required targets publish for `tag`: installers and
/// updater artifacts both.
fn expected_keys(channel: Channel, tag: &str) -> Result<BTreeSet<String>> {
    let series = release_plan::series_of(&tag_version(tag)?);
    Ok(channel
        .required_targets()
        .iter()
        .flat_map(|platform| {
            let target = platform.published_target();
            let extensions = target
                .extensions
                .iter()
                .chain(platform.updater_extensions());
            extensions
                .map(|extension| object_key(channel, &series, &target, tag, extension))
                .collect::<Vec<_>>()
        })
        .collect())
}

/// The files under `dir`, recursively, in a stable order.
fn walk(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("reading {}", dir.display()))?;
    entries.sort();

    let mut files = Vec::new();
    for path in entries {
        if path.is_dir() {
            files.extend(walk(&path)?);
        } else {
            files.push(path);
        }
    }
    Ok(files)
}

/// Resolves `pattern`, relative to `root`, to exactly one file. The bundler
/// produces one file per glob per build; more or fewer is a build gone
/// wrong, not a selection to make.
pub fn find_one(root: &Path, pattern: &str) -> Result<PathBuf> {
    let root = glob::Pattern::escape(&root.to_string_lossy());
    let pattern = format!("{root}/{pattern}");
    let mut matches = glob::glob(&pattern)
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

/// `{channel}/{series}/{os}/{arch}/waypoints-{tag}.{extension}`.
pub fn object_key(
    channel: Channel,
    series: &str,
    target: &PublishedTarget,
    tag: &str,
    extension: &str,
) -> String {
    format!(
        "{channel}/{series}/{os}/{arch}/waypoints-{tag}.{extension}",
        channel = channel.flavor(),
        os = target.os,
        arch = target.arch,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator as _;

    /// A scratch directory, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("xtask-upload-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn touch(&self, relative: &str) {
            let path = self.0.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, relative).unwrap();
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

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

    /// `built_files` chains the installer and the updater sets; this pins
    /// how many files each platform actually contributes to it — installer
    /// count, then updater count, so the split stays legible.
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
            object_key(
                Channel::Stable,
                "v0.1.x",
                &Platform::macOS.published_target(),
                "v0.1.1",
                "dmg"
            ),
            "stable/v0.1.x/macos/arm64/waypoints-v0.1.1.dmg"
        );
    }

    /// The CLI default is the same lifetime `release_store` settles on;
    /// raising it is a flag, not a code change.
    #[test]
    fn the_cli_defaults_to_the_store_s_ttl() {
        let args = ReleaseUploadArgs::parse_from(["xtask", "--from", "release", "--tag", "v0.1.1"]);
        assert_eq!(args.ttl_seconds, release_store::DEFAULT_TTL_SECONDS);
    }

    /// A build's output is never uploaded directly; only a staged tree is.
    #[test]
    fn the_cli_uploads_only_a_staged_tree() {
        let tag = ["--tag", "v0.1.1"];
        assert!(ReleaseUploadArgs::try_parse_from(["xtask"].iter().chain(&tag)).is_err());
        assert!(
            ReleaseUploadArgs::try_parse_from(["xtask", "--platform", "ios"].iter().chain(&tag))
                .is_err()
        );
    }

    #[test]
    fn a_staged_tree_uploads_to_the_keys_its_paths_name() {
        let staged = Scratch::new();
        staged.touch("stable/v0.1.x/macos/arm64/waypoints-v0.1.1.dmg");
        staged.touch("stable/v0.1.x/macos/arm64/waypoints-v0.1.1.app.tar.gz.sig");
        staged.touch("stable/v0.1.x/android/arm64/waypoints-v0.1.1.aab");

        let keys = staged_files(&staged.0, Channel::Stable, "v0.1.1")
            .unwrap()
            .into_iter()
            .map(|(_, key)| key)
            .collect::<Vec<_>>();
        assert_eq!(
            keys,
            [
                "stable/v0.1.x/android/arm64/waypoints-v0.1.1.aab",
                "stable/v0.1.x/macos/arm64/waypoints-v0.1.1.app.tar.gz.sig",
                "stable/v0.1.x/macos/arm64/waypoints-v0.1.1.dmg",
            ]
        );
    }

    #[test]
    fn a_path_the_scheme_would_not_produce_is_refused() {
        for foreign in [
            // another channel
            "preview/v0.1.x/macos/arm64/waypoints-v0.1.1.dmg",
            // another series
            "stable/v0.2.x/macos/arm64/waypoints-v0.1.1.dmg",
            // another tag
            "stable/v0.1.x/macos/arm64/waypoints-v0.1.0.dmg",
            // a file no build publishes
            "stable/v0.1.x/linux/amd64/waypoints-v0.1.1.deb",
            "AppStoreInfo.plist",
        ] {
            let staged = Scratch::new();
            staged.touch("stable/v0.1.x/macos/arm64/waypoints-v0.1.1.dmg");
            staged.touch(foreign);
            let error = staged_files(&staged.0, Channel::Stable, "v0.1.1").unwrap_err();
            assert!(
                error.to_string().contains("is not a store key"),
                "{foreign}: {error}"
            );
        }
    }

    #[test]
    fn an_empty_staging_directory_is_refused() {
        let staged = Scratch::new();
        assert!(staged_files(&staged.0, Channel::Stable, "v0.1.1").is_err());
    }

    /// Every key a required target publishes is accepted — installers and
    /// updater artifacts alike — so a full release is never refused.
    #[test]
    fn every_published_key_is_expected() {
        let keys = expected_keys(Channel::Nightly, "v0.2.0-nightly.3+2026.10.19").unwrap();
        let count = Channel::Nightly
            .required_targets()
            .iter()
            .map(|p| p.published_target().extensions.len() + p.updater_extensions().len())
            .sum::<usize>();
        assert_eq!(keys.len(), count);
        assert!(keys.contains(
            "nightly/v0.2.x/macos/arm64/waypoints-v0.2.0-nightly.3+2026.10.19.app.tar.gz"
        ));
    }
}
