//! The build jobs, shared across every channel.
//!
//! Mobile builds invoke `tauri`, not `cargo tauri`. mise installs
//! `npm:@tauri-apps/cli`, which ships only a `tauri` binary; the `cargo tauri`
//! form the local justfiles use comes from `pkgs.cargo-tauri`, which devenv
//! substitutes for the same mise entry and which ships only `cargo-tauri`.
//! Same tool and same pinned version, two binary names — see `devenv.nix`.
//!
//! In the hand-written YAML these lived in `_build-{desktop,android,ios}.yaml`
//! as reusable workflows, reached through six `workflow_call` inputs. Sharing
//! happens at the Rust level instead, so the inputs collapse into [`Channel`]
//! and the callers gain a flat job graph.

use gh_workflow::{Job, Level, Permissions};
use indoc::indoc;

use super::utils::{Channel, Platform, secrets, steps};

/// A job plus the id it is registered under, so callers can wire `needs:`
/// without restating the string.
pub struct NamedJob {
    pub id: String,
    pub job: Job,
}

/// Publishing a release needs write access; the source workflows inherited it
/// from the repository default, which is easy to tighten by accident.
fn release_job() -> Job {
    Job::default().permissions(Permissions::default().contents(Level::Write))
}

/// One job per desktop OS.
///
/// The source workflow generated a matrix from a bash `case` over `os-filter`
/// and then re-tested `matrix.platform` on individual steps. Emitting the jobs
/// separately makes both disappear: a step that only applies to one OS is
/// simply absent from the other two.
pub fn desktop(channel: Channel, platform: Platform) -> NamedJob {
    let mut job = release_job()
        .name(format!("Build {} desktop", platform.dispatch_label()))
        .runs_on(platform.runner())
        .cond(channel.desktop_guard(platform))
        .add_step(steps::checkout());

    if platform == Platform::Linux {
        job = job.add_step(steps::install_linux_deps());
    }

    job = job
        .add_step(steps::setup_mise())
        .add_step(steps::pnpm_install());

    if platform == Platform::macOS {
        job = job.add_step(secrets::apple_desktop());
    }

    NamedJob {
        id: platform.slug(),
        job: job
            .add_step(secrets::updater())
            .add_step(steps::generate_icons(channel))
            .add_step(steps::tauri_build_desktop(channel, platform))
            .add_step(steps::upload_artifact(channel, platform)),
    }
}

pub fn android(channel: Channel, jdk: &str, ndk: &str, abis: &[String]) -> NamedJob {
    const PLATFORM: Platform = Platform::Android;

    let build = gh_workflow::Step::new("Build Android")
        .run(format!(
            "tauri android build --features {flavor} --target {abis} --apk --aab",
            flavor = channel.flavor(),
            abis = abis.join(" ")
        ))
        .working_directory("waypoints")
        .add_env(("WAYPOINTS_FLAVOR", channel.flavor()))
        .add_env(("WAYPOINTS_BUILD", "1"))
        .add_env(("NDK_HOME", format!("${{{{ env.ANDROID_HOME }}}}/ndk/{ndk}")));

    // Nightly builds share a base semver, so their versionCode cannot be
    // derived from it and rides on the run number instead.
    let build = match channel {
        Channel::Nightly => build.add_env(("WAYPOINTS_VERSION_CODE", "${{ github.run_number }}")),
        _ => build,
    };

    NamedJob {
        id: PLATFORM.slug(),
        job: release_job()
            .name(format!("Build {PLATFORM}"))
            .runs_on(PLATFORM.runner())
            .cond(channel.android_guard())
            .add_step(steps::checkout())
            .add_step(steps::setup_mise())
            .add_step(steps::setup_java(jdk))
            .add_step(steps::setup_android_sdk())
            .add_step(steps::install_ndk(ndk))
            .add_step(steps::pnpm_install())
            .add_step(secrets::android_signing())
            .add_step(steps::generate_icons(channel))
            .add_step(build)
            .add_step(steps::upload_artifact(channel, PLATFORM))
            .add_step(steps::publish_to_release(channel, PLATFORM))
            .add_step(secrets::play_store())
            .add_step(steps::fastlane(PLATFORM, channel)),
    }
}

pub fn ios(channel: Channel) -> NamedJob {
    const PLATFORM: Platform = Platform::iOS;

    let xcodegen = gh_workflow::Step::new("Regenerate Xcode project")
        .run(
            indoc! {"
                mkdir -p Externals/arm64 assets
                xcodegen generate
            "}
            .trim_end(),
        )
        .working_directory("waypoints/src-tauri/gen/apple");

    let build = gh_workflow::Step::new("Build iOS")
        .run(format!(
            "tauri ios build --features {} --export-method app-store-connect",
            channel.flavor()
        ))
        .working_directory("waypoints");

    NamedJob {
        id: PLATFORM.slug(),
        job: release_job()
            .name(format!("Build {PLATFORM}"))
            .runs_on(PLATFORM.runner())
            .cond(channel.ios_guard())
            // Selects the iOS tool set — xcodegen in particular — which the
            // other jobs have no use for.
            .add_env(("MISE_ENV", "ios"))
            .add_step(steps::checkout())
            .add_step(steps::setup_mise())
            .add_step(steps::pnpm_install())
            .add_step(secrets::apple_ios())
            .add_step(steps::generate_icons(channel))
            .add_step(xcodegen)
            .add_step(build)
            .add_step(steps::upload_artifact(channel, PLATFORM))
            .add_step(steps::publish_to_release(channel, PLATFORM))
            .add_step(steps::fastlane(PLATFORM, channel)),
    }
}
