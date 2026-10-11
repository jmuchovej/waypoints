//! Runner labels and the platforms we build for.

use gh_workflow::RunsOn;

use super::Platform;

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
/// App Store Connect only accepts iOS uploads built against the iOS 26 SDK,
/// and this is the first image whose default Xcode ships it.
pub const MACOS: Runner = Runner("macos-26");
pub const WINDOWS: Runner = Runner("windows-latest");

impl Platform {
    pub fn runner(self) -> Runner {
        use Platform::*;
        match self {
            iOS | macOS => MACOS,
            Linux | Android => LINUX_BUILD,
            Windows => WINDOWS,
        }
    }
}
