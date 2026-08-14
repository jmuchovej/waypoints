//! Step constructors shared across workflows.
//!
//! Each returns a fully-labelled step, so a workflow reads as a list of
//! intentions rather than a pile of `uses:`/`run:` noise.

use gh_workflow::{Expression, Run, Step, StepValue, Use};
use indoc::indoc;

use super::{Artifact, Channel, Platform, actions, vars};

pub fn checkout() -> Step<Use> {
    actions::CHECKOUT.step("Checkout").id("checkout")
}

pub fn install_nix() -> Step<Use> {
    actions::INSTALL_NIX.step("Install Nix").id("setup-nix")
}

/// Reads (and, given a token, writes) the named Cachix binary cache.
pub fn cachix(cache: &str) -> Step<Use> {
    actions::CACHIX
        .step(format!("Use the {cache} binary cache"))
        .id(format!("setup-{cache}-cachix"))
        .add_with(("name", cache))
}

/// Installs flake outputs into the ambient profile, for tools needed *before*
/// the devenv shell can be entered.
pub fn nix_profile_install(packages: &[&str]) -> Step<Run> {
    let refs = packages
        .iter()
        .map(|package| format!("nixpkgs#{package}"))
        .collect::<Vec<_>>()
        .join(" ");

    Step::new(format!("Install {}", packages.join(" and ")))
        .run(format!("nix profile install {refs}"))
        .id("setup-nix-profile")
}

/// Runs a `just` recipe. The single entry point for real work — CI should ask
/// the justfile for things, never inline its own copy of a command.
pub fn just(recipe: &str) -> Step<Run> {
    Step::new(format!("just {recipe}"))
        .run(format!("just {recipe}"))
        .id(recipe)
}

/// Runs an `xtask` subcommand. The `cargo xtask` alias lives in
/// `.cargo/config.toml`, so this is the same invocation used locally.
///
/// Argument values are quoted — several carry a `${{ }}` expression, whose
/// substitution happens before the shell sees the line.
pub fn cargo_xtask(subcommand: &str, args: &[(&str, &str)]) -> Step<Run> {
    let command = args.iter().fold(
        format!("cargo xtask {subcommand}"),
        |command, (flag, value)| format!("{command} {flag} \"{value}\""),
    );

    Step::new(format!("cargo xtask {subcommand}")).run(command)
}

/// A step invoking an action checked into this repository.
///
/// `Step::uses` only builds `owner/repo@version`, so a `./path` reference has
/// to be assembled by hand.
pub fn local_action(name: &str, path: &str) -> Step<Use> {
    Step {
        value: StepValue {
            name: Some(name.to_owned()),
            uses: Some(path.to_owned()),
            ..Default::default()
        },
        marker: Use,
    }
}

/// mise provides the entire toolchain: node, pnpm, just, sops, age, yq — and
/// rust, which it reads from `rust-toolchain.toml` (see
/// `idiomatic_version_file_enable_tools` in `mise.toml`). That file already
/// declares every cross-compilation target, so nothing else needs to install
/// a toolchain or add targets.
pub fn setup_mise() -> Step<Use> {
    actions::MISE.step("Setup mise")
}

pub fn pnpm_install() -> Step<Run> {
    Step::new("Install JS deps").run("pnpm install --frozen-lockfile")
}

/// Tauri's Linux backend is WebKitGTK, which the runner image does not carry.
pub fn install_linux_deps() -> Step<Run> {
    Step::new("Install system deps (Linux)").run(
        indoc! {"
            sudo apt-get update
            sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
        "}
        .trim_end(),
    )
}

pub fn setup_java(major: &str) -> Step<Use> {
    actions::SETUP_JAVA
        .step("Setup Java")
        .add_with(("distribution", "zulu"))
        .add_with(("java-version", major))
}

pub fn setup_android_sdk() -> Step<Use> {
    actions::SETUP_ANDROID.step("Setup Android SDK")
}

pub fn install_ndk(version: &str) -> Step<Run> {
    Step::new("Install Android NDK").run(format!("sdkmanager \"ndk;{version}\""))
}

pub fn generate_icons(channel: Channel) -> Step<Run> {
    Step::new("Generate icons")
        .run(format!("just icons::{}", channel.flavor()))
        .working_directory("waypoints/src-tauri")
}

/// Builds and, when `tag` is non-empty, publishes to a GitHub release.
pub fn tauri_build_desktop(channel: Channel, platform: Platform) -> Step<Use> {
    let args = format!(
        "--features {flavor} --config src-tauri/Tauri.flavor={flavor}.toml",
        flavor = channel.flavor()
    );
    let tag = channel.tag();

    let mut step = actions::TAURI
        .step("Build")
        .add_env(("GITHUB_TOKEN", vars::GITHUB_TOKEN));

    if platform == Platform::macOS {
        step = step
            .add_env(("APPLE_CERTIFICATE", "${{ env.APPLE_CERTIFICATE }}"))
            .add_env((
                "APPLE_CERTIFICATE_PASSWORD",
                "${{ env.APPLE_CERTIFICATE_PASSWORD }}",
            ))
            .add_env(("APPLE_API_ISSUER", "${{ env.APPLE_API_ISSUER }}"))
            .add_env(("APPLE_API_KEY", "${{ env.APPLE_API_KEY }}"))
            .add_env(("APPLE_API_KEY_PATH", "${{ env.APPLE_API_KEY_PATH }}"));
    }
    step.add_env((
        "TAURI_SIGNING_PRIVATE_KEY",
        "${{ env.TAURI_SIGNING_PRIVATE_KEY }}",
    ))
    .add_env((
        "TAURI_SIGNING_PRIVATE_KEY_PASSWORD",
        "${{ env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}",
    ))
    .add_with(("projectPath", "./waypoints"))
    .add_with(("args", args))
    .add_with(("tagName", tag.clone()))
    .add_with(("releaseName", format!("Waypoints {tag}")))
    .add_with(("prerelease", channel.prerelease()))
}

/// Uploads everything the platform's build produced, under a name naming both
/// the platform and the channel.
pub fn upload_artifact(channel: Channel, platform: Platform) -> Step<Use> {
    actions::UPLOAD_ARTIFACT
        .step("Upload artifacts")
        .add_with(("name", Artifact(channel, platform).to_string()))
        .add_with(("path", platform.artifacts()))
        .add_with(("if-no-files-found", "error"))
}

/// Attaches files to the release for the channel's tag. Skipped entirely on a
/// build-only run, where the tag is empty.
pub fn publish_to_release(channel: Channel, platform: Platform) -> Step<Use> {
    actions::GH_RELEASE
        .step("Upload to GitHub Release")
        .if_condition(channel.tag_condition())
        .add_with(("tag_name", channel.tag()))
        .add_with(("prerelease", channel.prerelease()))
        .add_with(("files", platform.artifacts()))
}

pub fn fastlane(platform: Platform, channel: Channel) -> Step<Run> {
    Step::new(match platform {
        Platform::Android => "Publish to Play Store",
        Platform::iOS => "Publish to TestFlight",
        _ => unreachable!(),
    })
    .run(format!(
        "fastlane {lane} {flavor}",
        lane = platform.slug(),
        flavor = channel.flavor()
    ))
}
