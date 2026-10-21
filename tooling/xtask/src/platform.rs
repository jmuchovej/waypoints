//! The build matrix: the platforms a release is built for, the host
//! architecture each build targets, and where each files its artifacts.

use derive_more::{Display, IsVariant, Unwrap};
use strum::EnumIter;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Unwrap, Display, IsVariant, EnumIter, clap::ValueEnum,
)]
pub enum Platform {
    #[allow(non_camel_case_types)]
    #[value(name = "macos")]
    macOS,
    Windows,
    Linux,
    #[allow(non_camel_case_types)]
    #[value(name = "ios")]
    iOS,
    Android,
}

impl Platform {
    /// Job id and artifact suffix.
    pub fn slug(self) -> String {
        self.to_string().to_lowercase()
    }

    /// The label used by the `platforms` dispatch input. Cased for the
    /// dropdown, so it does not match [`Platform::slug`].
    pub fn dispatch_label(self) -> String {
        self.to_string()
    }

    pub fn is_desktop(self) -> bool {
        use Platform::{Linux, Windows, macOS};
        matches!(self, Linux | macOS | Windows)
    }

    pub fn is_mobile(self) -> bool {
        use Platform::{Android, iOS};
        matches!(self, Android | iOS)
    }

    pub fn is_ios(self) -> bool {
        matches!(self, Platform::iOS)
    }

    #[allow(dead_code, reason = "completes the platform predicates")]
    pub fn is_macos(self) -> bool {
        matches!(self, Platform::macOS)
    }

    /// The host architecture this platform's single build job targets. The
    /// toolchain declares more (arm64/x86_64 desktop, musl) than any job
    /// builds — see the design's "Targets are the host architecture of each
    /// platform" decision — so this names only what is actually produced.
    pub fn arch(self) -> Arch {
        use Platform::*;
        match self {
            macOS | Android | iOS => Arch::arm64,
            Linux | Windows => Arch::amd64,
        }
    }

    /// Where this platform's artifacts are filed in the release store: the
    /// `{os}/{arch}` key segments and the installer extensions uploaded
    /// there. One per platform — a target may still carry more than one
    /// artifact (Android's `.apk` and `.aab` share a target), which is why
    /// `extensions` is a list while the target itself is singular.
    pub fn published_target(self) -> PublishedTarget {
        use Platform::*;
        let extensions: &'static [&'static str] = match self {
            macOS => &["dmg"],
            // The AppImage first: it is the one the download link serves. The
            // deb and rpm are for package managers, and each updates itself
            // from its own format.
            Linux => &["AppImage", "deb", "rpm"],
            Windows => &["exe"],
            Android => &["apk", "aab"],
            iOS => &["ipa"],
        };
        PublishedTarget {
            os: self.slug(),
            arch: self.arch(),
            extensions,
        }
    }

    /// Globs matching this platform's installers on disk — what a person
    /// downloads — in the same order as [`PublishedTarget::extensions`].
    /// `release-stage` renames each match, and requires exactly one, so a
    /// change in what the bundler exports fails the build.
    pub fn published_globs(self) -> &'static [&'static str] {
        use Platform::*;
        match self {
            macOS => &["target/release/bundle/dmg/*.dmg"],
            Linux => &[
                "target/release/bundle/appimage/*.AppImage",
                "target/release/bundle/deb/*.deb",
                "target/release/bundle/rpm/*.rpm",
            ],
            Windows => &["target/release/bundle/nsis/*.exe"],
            // Gradle splits flavor from build type for an APK
            // (`apk/universal/release`) and joins them for a bundle
            // (`bundle/universalRelease`).
            Android => &[
                "src-tauri/gen/android/app/build/outputs/apk/*/release/*.apk",
                "src-tauri/gen/android/app/build/outputs/bundle/*Release/*.aab",
            ],
            iOS => &["src-tauri/gen/apple/build/**/*.ipa"],
        }
    }

    /// This platform's updater artifacts beyond its installer: the payload
    /// Tauri's updater fetches, and its signature. Empty for the two mobile
    /// platforms, which update through their stores rather than Tauri's own.
    ///
    /// Linux and Windows sign the installer in place — `createUpdaterArtifacts`
    /// wraps nothing; the `.nsis.zip` and `.AppImage.tar.gz` of Tauri v1 exist
    /// only under `"v1Compatible"` — so the updater fetches the same file
    /// [`published_target`] already uploads, and only the signature is new
    /// here. macOS's updater cannot consume a `.dmg`, so its payload is a
    /// separate `.app.tar.gz`. Linux signs all three installers: the updater
    /// in an app installed from a deb or rpm asks for its own format's entry
    /// and refuses any other payload. Paired one-to-one with
    /// [`updater_globs`].
    pub fn updater_extensions(self) -> &'static [&'static str] {
        use Platform::*;
        match self {
            macOS => &["app.tar.gz", "app.tar.gz.sig"],
            Linux => &["AppImage.sig", "deb.sig", "rpm.sig"],
            Windows => &["exe.sig"],
            Android | iOS => &[],
        }
    }

    /// Globs matching this platform's updater artifacts on disk, in the same
    /// order as [`updater_extensions`]: what the updater needs beyond the
    /// installer itself.
    pub fn updater_globs(self) -> &'static [&'static str] {
        use Platform::*;
        match self {
            macOS => &[
                "target/release/bundle/macos/*.tar.gz",
                "target/release/bundle/macos/*.tar.gz.sig",
            ],
            Linux => &[
                "target/release/bundle/appimage/*.AppImage.sig",
                "target/release/bundle/deb/*.deb.sig",
                "target/release/bundle/rpm/*.rpm.sig",
            ],
            Windows => &["target/release/bundle/nsis/*.exe.sig"],
            Android | iOS => &[],
        }
    }
}

/// One platform's filing location in the release store, from
/// [`Platform::published_target`].
#[derive(Debug, PartialEq, Eq)]
pub struct PublishedTarget {
    pub os: String,
    pub arch: Arch,
    pub extensions: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Unwrap, Display, EnumIter)]
pub enum Arch {
    #[allow(non_camel_case_types)]
    amd64,
    #[allow(non_camel_case_types)]
    arm64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use strum::IntoEnumIterator as _;

    /// `published_target` returns a single `PublishedTarget`, never a
    /// collection, so every platform names exactly one target by
    /// construction — this pins the values to the design's target set.
    #[test]
    fn each_platform_reports_exactly_one_published_target() {
        let targets: Vec<PublishedTarget> =
            Platform::iter().map(Platform::published_target).collect();
        assert_eq!(targets.len(), Platform::iter().count());

        let macos = Platform::macOS.published_target();
        assert_eq!(macos.os, "macos");
        assert_eq!(macos.arch, Arch::arm64);
        assert_eq!(macos.extensions, &["dmg"]);

        let linux = Platform::Linux.published_target();
        assert_eq!(linux.os, "linux");
        assert_eq!(linux.arch, Arch::amd64);
        assert_eq!(linux.extensions, &["AppImage", "deb", "rpm"]);

        let windows = Platform::Windows.published_target();
        assert_eq!(windows.os, "windows");
        assert_eq!(windows.arch, Arch::amd64);
        assert_eq!(windows.extensions, &["exe"]);

        let android = Platform::Android.published_target();
        assert_eq!(android.os, "android");
        assert_eq!(android.arch, Arch::arm64);
        assert_eq!(android.extensions, &["apk", "aab"]);

        let ios = Platform::iOS.published_target();
        assert_eq!(ios.os, "ios");
        assert_eq!(ios.arch, Arch::arm64);
        assert_eq!(ios.extensions, &["ipa"]);
    }

    /// Every updater glob names an updater extension, one for one, the same
    /// invariant `published_globs_pair_with_published_extensions` holds for
    /// the installer set — and the two mobile platforms have neither, since
    /// they update through their stores.
    #[test]
    fn updater_globs_pair_with_updater_extensions() {
        for platform in Platform::iter() {
            assert_eq!(
                platform.updater_globs().len(),
                platform.updater_extensions().len(),
                "{platform}"
            );
        }
        assert_eq!(
            Platform::macOS.updater_extensions(),
            &["app.tar.gz", "app.tar.gz.sig"]
        );
        assert_eq!(
            Platform::Linux.updater_extensions(),
            &["AppImage.sig", "deb.sig", "rpm.sig"]
        );
        assert_eq!(Platform::Windows.updater_extensions(), &["exe.sig"]);
        for platform in [Platform::Android, Platform::iOS] {
            assert!(platform.updater_extensions().is_empty(), "{platform}");
        }
    }
}
