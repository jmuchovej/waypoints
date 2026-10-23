//! Where a build's artifacts go between the bundler and the store.
//!
//! Every build renames its published and updater artifacts out of the
//! bundler's nested output into the flat [`STAGE_DIR`], each as
//! `waypoints-{tag}.{ext}` — the name it is stored under — so the build
//! uploads one flat artifact and the publishing job only checks and writes
//! the merged directory. Every platform's extensions are its own, so the
//! extension alone files a staged name under its `{os}/{arch}`, and the
//! object key mirrors the design's layout exactly:
//! `{channel}/{series}/{os}/{arch}/waypoints-{tag}.{ext}`.
//!
//! The same key set serves the uploader (what a staged file may be), the
//! withdrawal (what a shortfall run could have written), and the stage (what
//! a build must have produced). One derivation, three readers.
//!
//! iOS also stages the [`APP_STORE_INFO`] its export wrote, beside the ipa:
//! TestFlight needs it there to upload off macOS, but it is upload metadata,
//! never a store object.

use crate::{Channel, Platform, PublishedTarget, releases::version::Version};
use anyhow::{Context as _, Result, bail};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// The flat directory `release-stage` fills and `release-upload --from`
/// reads.
pub const STAGE_DIR: &str = "release";

/// Upload metadata the iOS build stages beside its ipa: TestFlight reads it
/// off macOS, and it never becomes a store object.
pub const APP_STORE_INFO: &str = "AppStoreInfo.plist";

/// Where `xcodebuild -exportArchive` writes it: beside the ipa.
const APP_STORE_INFO_GLOB: &str = "src-tauri/gen/apple/build/**/AppStoreInfo.plist";

/// What a published artifact is named, staged and stored alike.
pub fn file_name(tag: &str, extension: &str) -> String {
    format!("waypoints-{tag}.{extension}")
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
        "{channel}/{series}/{os}/{arch}/{name}",
        channel = channel.flavor(),
        os = target.os,
        arch = target.arch,
        name = file_name(tag, extension),
    )
}

/// Every file name `channel`'s required targets publish for `tag` —
/// installers and updater artifacts both — mapped to its store key.
pub fn published_keys(channel: Channel, tag: &str) -> Result<BTreeMap<String, String>> {
    let series = tag.parse::<Version>()?.series();
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
                .map(|extension| {
                    (
                        file_name(tag, extension),
                        object_key(channel, &series, &target, tag, extension),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect())
}

/// Pairs each of `platform`'s published and updater artifacts under `root`
/// with its extension.
pub fn built_files(platform: Platform, root: &Path) -> Result<Vec<(PathBuf, &'static str)>> {
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
        .map(|(glob, extension)| Ok((find_one(root, glob)?, *extension)))
        .collect()
}

/// Stages `platform`'s artifacts from the build output under `root` into
/// the flat staged directory under `root`, returning the paths written.
/// Every expected output must match exactly one file: a Tauri change to what
/// the bundler exports fails the build here, not the publish.
pub fn stage(platform: Platform, tag: &str, root: &Path) -> Result<Vec<PathBuf>> {
    tag.parse::<Version>()?;
    let staged = root.join(STAGE_DIR);

    let mut copies = built_files(platform, root)?
        .into_iter()
        .map(|(source, extension)| (source, staged.join(file_name(tag, extension))))
        .collect::<Vec<_>>();
    if platform.is_ios() {
        let plist = find_one(root, APP_STORE_INFO_GLOB)?;
        copies.push((plist, staged.join(APP_STORE_INFO)));
    }

    std::fs::create_dir_all(&staged).with_context(|| format!("creating {}", staged.display()))?;
    for (source, destination) in &copies {
        std::fs::copy(source, destination).with_context(|| {
            format!("copying {} to {}", source.display(), destination.display())
        })?;
    }

    Ok(copies
        .into_iter()
        .map(|(_, destination)| destination)
        .collect())
}

/// Every file in the flat directory `dir`, paired with the key it is stored
/// at; [`APP_STORE_INFO`] is skipped. Refuses a subdirectory, a file not
/// named for `tag` with an extension one of `channel`'s required targets
/// publishes, and an empty directory.
pub fn staged_files(dir: &Path, channel: Channel, tag: &str) -> Result<Vec<(PathBuf, String)>> {
    let keys = published_keys(channel, tag)?;

    let mut entries = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("reading {}", dir.display()))?;
    entries.sort();

    let mut files = Vec::new();
    for path in entries {
        let name = path.file_name().and_then(|name| name.to_str());
        if path.is_dir() || name.is_none() {
            bail!("{} is not a staged file", path.display());
        }
        let name = name.expect("checked above");
        if name == APP_STORE_INFO {
            continue;
        }
        let key = keys
            .get(name)
            .with_context(|| format!("{} is not a published artifact of {tag}", path.display()))?;
        files.push((path, key.clone()));
    }

    if files.is_empty() {
        bail!("nothing staged in {}", dir.display());
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

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator as _;

    /// A scratch directory, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let dir =
                std::env::temp_dir().join(format!("xtask-artifacts-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn touch(&self, relative: &str) {
            let path = self.0.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, relative).unwrap();
        }

        /// Writes a file matching `glob`, its contents naming the glob.
        fn build(&self, glob: &str) {
            let path = self
                .0
                .join(glob.replace("**", "arm64").replace('*', "universal"));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, glob).unwrap();
        }

        /// What `platform`'s build leaves behind.
        fn build_platform(&self, platform: Platform) {
            for glob in platform
                .published_globs()
                .iter()
                .chain(platform.updater_globs())
            {
                self.build(glob);
            }
            if platform.is_ios() {
                self.build(APP_STORE_INFO_GLOB);
            }
        }

        fn relative(&self, paths: Vec<PathBuf>) -> Vec<String> {
            let mut relative = paths
                .iter()
                .map(|p| {
                    p.strip_prefix(&self.0)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect::<Vec<_>>();
            relative.sort();
            relative
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
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
            (Platform::Linux, 3, 3),
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

    /// Every key a required target publishes is expected — installers and
    /// updater artifacts alike — so a full release is never refused, and a
    /// shortfall run withdraws every key it could have written.
    #[test]
    fn published_keys_cover_every_required_target_and_extension() {
        let keys = published_keys(Channel::Nightly, "v0.2.0-nightly.3+2026.10.19").unwrap();
        let count = Channel::Nightly
            .required_targets()
            .iter()
            .map(|p| p.published_target().extensions.len() + p.updater_extensions().len())
            .sum::<usize>();
        assert_eq!(keys.len(), count);
        assert_eq!(
            keys["waypoints-v0.2.0-nightly.3+2026.10.19.app.tar.gz"],
            "nightly/v0.2.x/macos/arm64/waypoints-v0.2.0-nightly.3+2026.10.19.app.tar.gz"
        );

        let stable = published_keys(Channel::Stable, "v0.1.1").unwrap();
        for key in [
            "stable/v0.1.x/macos/arm64/waypoints-v0.1.1.dmg",
            "stable/v0.1.x/android/arm64/waypoints-v0.1.1.apk",
            "stable/v0.1.x/android/arm64/waypoints-v0.1.1.aab",
            "stable/v0.1.x/macos/arm64/waypoints-v0.1.1.app.tar.gz",
            "stable/v0.1.x/linux/amd64/waypoints-v0.1.1.rpm.sig",
        ] {
            assert!(stable.values().any(|k| k == key), "{key}");
        }
    }

    #[test]
    fn published_keys_stay_under_the_channel_s_own_prefix() {
        for key in published_keys(Channel::Nightly, "v0.2.0-nightly.3+2026.10.19")
            .unwrap()
            .into_values()
        {
            assert!(key.starts_with("nightly/"), "{key}");
        }
    }

    #[test]
    fn every_platform_stages_flat_under_its_published_names() {
        let expected: [(Platform, &[&str]); 5] = [
            (
                Platform::macOS,
                &[
                    "release/waypoints-v0.1.1.app.tar.gz",
                    "release/waypoints-v0.1.1.app.tar.gz.sig",
                    "release/waypoints-v0.1.1.dmg",
                ],
            ),
            (
                Platform::Linux,
                &[
                    "release/waypoints-v0.1.1.AppImage",
                    "release/waypoints-v0.1.1.AppImage.sig",
                    "release/waypoints-v0.1.1.deb",
                    "release/waypoints-v0.1.1.deb.sig",
                    "release/waypoints-v0.1.1.rpm",
                    "release/waypoints-v0.1.1.rpm.sig",
                ],
            ),
            (
                Platform::Windows,
                &[
                    "release/waypoints-v0.1.1.exe",
                    "release/waypoints-v0.1.1.exe.sig",
                ],
            ),
            (
                Platform::Android,
                &[
                    "release/waypoints-v0.1.1.aab",
                    "release/waypoints-v0.1.1.apk",
                ],
            ),
            (
                Platform::iOS,
                &["release/AppStoreInfo.plist", "release/waypoints-v0.1.1.ipa"],
            ),
        ];
        assert_eq!(expected.len(), Platform::iter().count());

        for (platform, paths) in expected {
            let tree = Scratch::new();
            tree.build_platform(platform);
            let staged = stage(platform, "v0.1.1", &tree.0).unwrap();
            assert_eq!(tree.relative(staged), paths, "{platform}");
        }
    }

    /// The bytes are the build's own: macOS's updater payload is the
    /// `.app.tar.gz`, not the signature beside it.
    #[test]
    fn a_staged_file_is_a_copy_of_its_build_output() {
        let tree = Scratch::new();
        tree.build_platform(Platform::macOS);
        stage(Platform::macOS, "v0.1.1", &tree.0).unwrap();
        let payload = tree.0.join("release/waypoints-v0.1.1.app.tar.gz");
        assert_eq!(
            std::fs::read_to_string(payload).unwrap(),
            "target/release/bundle/macos/*.tar.gz"
        );
    }

    /// A Tauri change to what the bundler exports — a file gone, or a
    /// second where one was expected — fails the build at the rename.
    #[test]
    fn an_unexpected_export_fails_the_stage() {
        let missing = Scratch::new();
        missing.build(Platform::Android.published_globs()[0]);
        assert!(stage(Platform::Android, "v0.1.1", &missing.0).is_err());

        let ambiguous = Scratch::new();
        ambiguous.build_platform(Platform::Linux);
        ambiguous.build("target/release/bundle/appimage/second.AppImage");
        let error = stage(Platform::Linux, "v0.1.1", &ambiguous.0).unwrap_err();
        assert!(error.to_string().contains("more than one"), "{error}");
    }

    #[test]
    fn an_ios_build_without_its_app_store_description_fails_the_stage() {
        let tree = Scratch::new();
        tree.build(Platform::iOS.published_globs()[0]);
        let error = stage(Platform::iOS, "v0.1.1", &tree.0).unwrap_err();
        assert!(error.to_string().contains("AppStoreInfo.plist"), "{error}");
    }

    /// Every platform's stage merges into one flat directory that is
    /// exactly what `release-upload --from` accepts.
    #[test]
    fn the_merged_stage_uploads_as_staged() {
        let tree = Scratch::new();
        for platform in Platform::iter() {
            tree.build_platform(platform);
            stage(platform, "v0.1.1", &tree.0).unwrap();
        }
        let staged = staged_files(&tree.0.join(STAGE_DIR), Channel::Stable, "v0.1.1").unwrap();
        let count = Platform::iter()
            .map(|p| p.published_target().extensions.len() + p.updater_extensions().len())
            .sum::<usize>();
        assert_eq!(staged.len(), count);
    }

    /// Each extension belongs to one platform, so a flat name files itself;
    /// the App Store description beside the ipa is skipped.
    #[test]
    fn a_staged_file_is_filed_by_its_extension() {
        let staged = Scratch::new();
        staged.touch("waypoints-v0.1.1.dmg");
        staged.touch("waypoints-v0.1.1.app.tar.gz.sig");
        staged.touch("waypoints-v0.1.1.aab");
        staged.touch(APP_STORE_INFO);

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
    fn a_file_the_scheme_would_not_produce_is_refused() {
        for foreign in [
            // another tag
            "waypoints-v0.1.0.dmg",
            // a file no build publishes: Tauri v1's updater payload shape
            "waypoints-v0.1.1.AppImage.tar.gz",
            // an unrenamed bundler output
            "Waypoints (Nightly)_0.1.0-nightly_aarch64.dmg",
        ] {
            let staged = Scratch::new();
            staged.touch("waypoints-v0.1.1.dmg");
            staged.touch(foreign);
            let error = staged_files(&staged.0, Channel::Stable, "v0.1.1").unwrap_err();
            assert!(
                error.to_string().contains("is not a published artifact"),
                "{foreign}: {error}"
            );
        }
    }

    /// The staged tree is flat; a nested directory is a build that skipped
    /// its rename.
    #[test]
    fn a_nested_directory_is_refused() {
        let staged = Scratch::new();
        staged.touch("waypoints-v0.1.1.dmg");
        staged.touch("dmg/waypoints-v0.1.1.dmg");
        let error = staged_files(&staged.0, Channel::Stable, "v0.1.1").unwrap_err();
        assert!(error.to_string().contains("not a staged file"), "{error}");
    }

    #[test]
    fn an_empty_staging_directory_is_refused() {
        let staged = Scratch::new();
        assert!(staged_files(&staged.0, Channel::Stable, "v0.1.1").is_err());
    }
}
