//! Copies one platform's published and updater artifacts out of its build
//! output into [`STAGE_DIR`], each at the path of the store key it will be
//! published under, so the publishing job's `release-upload --from` only
//! checks and writes them. Touches nothing outside the working tree and
//! resolves no credential.
//!
//! iOS also stages the `AppStoreInfo.plist` its export wrote, to
//! [`APP_STORE_INFO`]: TestFlight needs it beside the ipa to upload off
//! macOS, but it is upload metadata, never a store object.

use crate::tasks::{release_upload, workflows::Platform};
use anyhow::{Context as _, Result};
use clap::Parser;
use std::path::{Path, PathBuf};

/// The staged tree, laid out as store keys.
pub const STAGE_DIR: &str = "release";

/// Where the iOS build stages its App Store description.
pub const APP_STORE_INFO: &str = "app-store/AppStoreInfo.plist";

/// Where `xcodebuild -exportArchive` writes it: beside the ipa.
const APP_STORE_INFO_GLOB: &str = "src-tauri/gen/apple/build/**/AppStoreInfo.plist";

#[derive(Parser)]
pub struct ReleaseStageArgs {
    /// The platform whose build output this stages.
    #[arg(long, value_enum)]
    platform: Platform,

    /// The release tag; names the staged files and, through its version,
    /// their channel and series.
    #[arg(long)]
    tag: String,
}

pub fn run(args: ReleaseStageArgs) -> Result<()> {
    for path in stage(args.platform, &args.tag, Path::new("."))? {
        println!("staged {}", path.display());
    }
    Ok(())
}

/// Stages `platform`'s artifacts from the build output under `root` into
/// the staged tree under `root`, returning the paths written.
fn stage(platform: Platform, tag: &str, root: &Path) -> Result<Vec<PathBuf>> {
    let mut copies = release_upload::built_files(platform, tag, root)?
        .into_iter()
        .map(|(source, key)| (source, root.join(STAGE_DIR).join(key)))
        .collect::<Vec<_>>();
    if platform.is_ios() {
        let plist = release_upload::find_one(root, APP_STORE_INFO_GLOB)?;
        copies.push((plist, root.join(APP_STORE_INFO)));
    }

    for (source, destination) in &copies {
        let parent = destination.parent().expect("a staged path has a parent");
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
        std::fs::copy(source, destination).with_context(|| {
            format!("copying {} to {}", source.display(), destination.display())
        })?;
    }

    Ok(copies
        .into_iter()
        .map(|(_, destination)| destination)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator as _;

    /// A scratch working tree, removed on drop.
    struct Tree(PathBuf);

    impl Tree {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("xtask-stage-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
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

        fn staged(&self, paths: Vec<PathBuf>) -> Vec<String> {
            let mut staged = paths
                .iter()
                .map(|p| {
                    p.strip_prefix(&self.0)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect::<Vec<_>>();
            staged.sort();
            staged
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn every_platform_stages_under_its_store_keys() {
        let expected: [(Platform, &[&str]); 5] = [
            (
                Platform::macOS,
                &[
                    "release/stable/v0.1.x/macos/arm64/waypoints-v0.1.1.app.tar.gz",
                    "release/stable/v0.1.x/macos/arm64/waypoints-v0.1.1.app.tar.gz.sig",
                    "release/stable/v0.1.x/macos/arm64/waypoints-v0.1.1.dmg",
                ],
            ),
            (
                Platform::Linux,
                &[
                    "release/stable/v0.1.x/linux/amd64/waypoints-v0.1.1.AppImage",
                    "release/stable/v0.1.x/linux/amd64/waypoints-v0.1.1.AppImage.sig",
                ],
            ),
            (
                Platform::Windows,
                &[
                    "release/stable/v0.1.x/windows/amd64/waypoints-v0.1.1.exe",
                    "release/stable/v0.1.x/windows/amd64/waypoints-v0.1.1.exe.sig",
                ],
            ),
            (
                Platform::Android,
                &[
                    "release/stable/v0.1.x/android/arm64/waypoints-v0.1.1.aab",
                    "release/stable/v0.1.x/android/arm64/waypoints-v0.1.1.apk",
                ],
            ),
            (
                Platform::iOS,
                &[
                    "app-store/AppStoreInfo.plist",
                    "release/stable/v0.1.x/ios/arm64/waypoints-v0.1.1.ipa",
                ],
            ),
        ];
        assert_eq!(expected.len(), Platform::iter().count());

        for (platform, paths) in expected {
            let tree = Tree::new();
            tree.build_platform(platform);
            let staged = stage(platform, "v0.1.1", &tree.0).unwrap();
            assert_eq!(tree.staged(staged), paths, "{platform}");
        }
    }

    /// The bytes are the build's own: macOS's updater payload is the
    /// `.app.tar.gz`, not the signature beside it.
    #[test]
    fn a_staged_file_is_a_copy_of_its_build_output() {
        let tree = Tree::new();
        tree.build_platform(Platform::macOS);
        stage(Platform::macOS, "v0.1.1", &tree.0).unwrap();
        let payload = tree
            .0
            .join("release/stable/v0.1.x/macos/arm64/waypoints-v0.1.1.app.tar.gz");
        assert_eq!(
            std::fs::read_to_string(payload).unwrap(),
            "target/release/bundle/macos/*.tar.gz"
        );
    }

    #[test]
    fn a_missing_artifact_fails_the_stage() {
        let tree = Tree::new();
        tree.build(Platform::Android.published_globs()[0]);
        assert!(stage(Platform::Android, "v0.1.1", &tree.0).is_err());
    }

    #[test]
    fn an_ios_build_without_its_app_store_description_fails_the_stage() {
        let tree = Tree::new();
        tree.build(Platform::iOS.published_globs()[0]);
        let error = stage(Platform::iOS, "v0.1.1", &tree.0).unwrap_err();
        assert!(error.to_string().contains("AppStoreInfo.plist"), "{error}");
    }

    /// What `release-stage` writes under [`STAGE_DIR`] is exactly what
    /// `release-upload --from` accepts.
    #[test]
    fn the_stage_uploads_as_staged() {
        let tree = Tree::new();
        for platform in Platform::iter() {
            tree.build_platform(platform);
            stage(platform, "v0.1.1", &tree.0).unwrap();
        }
        let staged = release_upload::staged_files(
            &tree.0.join(STAGE_DIR),
            crate::tasks::workflows::Channel::Stable,
            "v0.1.1",
        )
        .unwrap();
        let count = Platform::iter()
            .map(|p| p.published_target().extensions.len() + p.updater_extensions().len())
            .sum::<usize>();
        assert_eq!(staged.len(), count);
    }
}
