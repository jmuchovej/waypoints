//! Renames one platform's published and updater artifacts out of the
//! bundler's nested output into the flat [`STAGE_DIR`], each as
//! `waypoints-{tag}.{ext}` — the name it is stored under — so the build
//! uploads one flat artifact and the publishing job only checks and writes
//! the merged directory. Every expected output must match exactly one file:
//! a Tauri change to what the bundler exports fails the build here, not the
//! publish. Touches nothing outside the working tree and resolves no
//! credential.
//!
//! iOS also stages the [`APP_STORE_INFO`] its export wrote, beside the ipa:
//! TestFlight needs it there to upload off macOS, but it is upload metadata,
//! never a store object.

use crate::{
    tasks::{
        release_upload::{self, APP_STORE_INFO},
        workflows::Platform,
    },
    workspace,
};
use anyhow::{Context as _, Result};
use clap::Parser;
use std::path::{Path, PathBuf};

/// The flat staged directory every platform's artifact holds.
pub const STAGE_DIR: &str = "release";

/// Where `xcodebuild -exportArchive` writes it: beside the ipa.
const APP_STORE_INFO_GLOB: &str = "src-tauri/gen/apple/build/**/AppStoreInfo.plist";

#[derive(Parser)]
pub struct ReleaseStageArgs {
    /// The platform whose build output this stages.
    #[arg(long, value_enum)]
    platform: Platform,

    /// The release tag the files are named for. A build-only run has none:
    /// it passes the tag it would have published, or omits this to take
    /// `v` and the app's own version.
    #[arg(long)]
    tag: Option<String>,
}

pub fn run(args: ReleaseStageArgs) -> Result<()> {
    let tag = match args.tag {
        Some(tag) => tag,
        None => format!("v{}", workspace::version()?),
    };
    for path in stage(args.platform, &tag, Path::new("."))? {
        println!("staged {}", path.display());
    }
    Ok(())
}

/// Stages `platform`'s artifacts from the build output under `root` into
/// the flat staged directory under `root`, returning the paths written.
fn stage(platform: Platform, tag: &str, root: &Path) -> Result<Vec<PathBuf>> {
    release_upload::tag_version(tag)?;
    let staged = root.join(STAGE_DIR);

    let mut copies = release_upload::built_files(platform, root)?
        .into_iter()
        .map(|(source, extension)| {
            (
                source,
                staged.join(release_upload::file_name(tag, extension)),
            )
        })
        .collect::<Vec<_>>();
    if platform.is_ios() {
        let plist = release_upload::find_one(root, APP_STORE_INFO_GLOB)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::workflows::Channel;
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
        let missing = Tree::new();
        missing.build(Platform::Android.published_globs()[0]);
        assert!(stage(Platform::Android, "v0.1.1", &missing.0).is_err());

        let ambiguous = Tree::new();
        ambiguous.build_platform(Platform::Linux);
        ambiguous.build("target/release/bundle/appimage/second.AppImage");
        let error = stage(Platform::Linux, "v0.1.1", &ambiguous.0).unwrap_err();
        assert!(error.to_string().contains("more than one"), "{error}");
    }

    #[test]
    fn an_ios_build_without_its_app_store_description_fails_the_stage() {
        let tree = Tree::new();
        tree.build(Platform::iOS.published_globs()[0]);
        let error = stage(Platform::iOS, "v0.1.1", &tree.0).unwrap_err();
        assert!(error.to_string().contains("AppStoreInfo.plist"), "{error}");
    }

    /// Every platform's stage merges into one flat directory that is
    /// exactly what `release-upload --from` accepts.
    #[test]
    fn the_merged_stage_uploads_as_staged() {
        let tree = Tree::new();
        for platform in Platform::iter() {
            tree.build_platform(platform);
            stage(platform, "v0.1.1", &tree.0).unwrap();
        }
        let staged =
            release_upload::staged_files(&tree.0.join(STAGE_DIR), Channel::Stable, "v0.1.1")
                .unwrap();
        let count = Platform::iter()
            .map(|p| p.published_target().extensions.len() + p.updater_extensions().len())
            .sum::<usize>();
        assert_eq!(staged.len(), count);
    }
}
