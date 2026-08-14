//! Runner labels and the desktop platforms we build for, so a fleet change is a
//! one-line edit rather than a sweep across every workflow.

use derive_more::{Display, Unwrap};
use gh_workflow::RunsOn;
use indoc::indoc;
use strum::EnumIter;
use strum::IntoEnumIterator;

#[derive(Clone, Copy)]
pub struct Runner(&'static str);

impl From<Runner> for RunsOn {
    fn from(runner: Runner) -> Self {
        runner.0.into()
    }
}

/// The default for anything that isn't a platform-specific build.
pub const LINUX: Runner = Runner("ubuntu-latest");

/// Pinned one LTS back: the AppImage we produce links against the runner's
/// glibc, so building on a newer Ubuntu would raise our minimum.
pub const LINUX_BUILD: Runner = Runner("ubuntu-22.04");
pub const MACOS: Runner = Runner("macos-15");
pub const WINDOWS: Runner = Runner("windows-latest");

#[derive(Clone, Copy, PartialEq, Eq, Unwrap, Display, EnumIter)]
pub enum Platform {
    #[allow(non_camel_case_types)]
    macOS,
    Windows,
    Linux,
    #[allow(non_camel_case_types)]
    iOS,
    Android,
}

impl Platform {
    pub fn runner(self) -> Runner {
        use Platform::*;
        match self {
            iOS | macOS => MACOS,
            Linux | Android => LINUX_BUILD,
            Windows => WINDOWS,
        }
    }

    /// Job id and artifact suffix.
    pub fn slug(self) -> String {
        self.to_string().to_lowercase()
    }

    /// The label used by the `platforms` dispatch input. Cased for humans
    /// reading the dropdown, so it does not match [`Desktop::slug`].
    pub fn dispatch_label(self) -> String {
        self.to_string()
    }

    pub fn is_desktop(self) -> bool {
        use Platform::{Linux, Windows, macOS};
        matches!(self, Linux | macOS | Windows)
    }

    /// Globs matching everything this platform's build produces.
    pub fn artifacts(self) -> &'static str {
        use Platform::*;
        match self {
            macOS => indoc! {"
                target/release/bundle/dmg/*.dmg
                target/release/bundle/macos/*.tar.gz
                target/release/bundle/macos/*.tar.gz.sig"},
            Linux => indoc! {"
                target/release/bundle/appimage/*.AppImage
                target/release/bundle/appimage/*.AppImage.tar.gz
                target/release/bundle/appimage/*.AppImage.tar.gz.sig
                target/release/bundle/deb/*.deb"},
            Windows => indoc! {"
                target/release/bundle/nsis/*.exe
                target/release/bundle/nsis/*.nsis.zip
                target/release/bundle/nsis/*.nsis.zip.sig"},
            Android => indoc! {"
                waypoints/src-tauri/gen/android/app/build/outputs/apk/*Release/*.apk
                waypoints/src-tauri/gen/android/app/build/outputs/bundle/*Release/*.aab"},
            iOS => "waypoints/src-tauri/gen/apple/build/**/*.ipa",
        }
    }

    pub fn is_mobile(self) -> bool {
        use Platform::{Android, iOS};
        matches!(self, Android | iOS)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Unwrap, Display, EnumIter)]
pub enum Arch {
    #[allow(non_camel_case_types)]
    amd64,
    #[allow(non_camel_case_types)]
    arm64,
}
