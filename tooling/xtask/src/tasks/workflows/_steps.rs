//! Step constructors shared across workflows. Each returns a labelled step
//! ready to drop into a job.

use super::{
    Artifact, Channel, Platform, TestSuite,
    helpers::{
        GITHUB_TOKEN, SECRETSPEC_PROVIDER, SOPS_AGE_KEY_ENV, WP_BOTSWAIN_APP_ID,
        WP_BOTSWAIN_PRIVATE_KEY,
    },
};
use gh_workflow::{FluentBuilder, JobOutput, Run, Step, StepOutput, Use};
use indoc::indoc;

/// A `uses:` reference — `{owner}/{repo}@{sha}`. Pinning to the hash rather
/// than a tag means a retagged (compromised) release cannot reach CI on its
/// own. Each call site carries a `// {version}` comment beside the sha naming
/// the release it was resolved from; the custom manager in
/// `.github/renovate.json5` matches the pair and moves them together.
pub struct Action {
    owner: &'static str,
    repo: &'static str,
    sha: &'static str,
}

impl Action {
    pub const fn new(owner: &'static str, repo: &'static str, sha: &'static str) -> Self {
        Self { owner, repo, sha }
    }

    /// A `Step<Use>` invoking this action, labelled `name`.
    pub fn step(&self, name: impl ToString) -> Step<Use> {
        Step::new(name).uses(self.owner, self.repo, self.sha)
    }
}

impl From<Action> for Step<Use> {
    fn from(a: Action) -> Self {
        Step::new(a.repo).uses(a.owner, a.repo, a.sha)
    }
}

pub fn checkout() -> Step<Use> {
    Action::new(
        "actions",
        "checkout",
        "d23441a48e516b6c34aea4fa41551a30e30af803", // v6
    )
    .step("Checkout")
    .with(("persist-credentials", false))
    .id("checkout")
}

pub fn install_nix() -> Step<Use> {
    Action::new(
        "cachix",
        "install-nix-action",
        "13d8dd58da0234aa297dedd986986ccb8e7f3e24", // v31
    )
    .step("Install Nix")
    .id("setup-nix")
}

/// Reads (and, given a token, writes) the named Cachix binary cache.
pub fn cachix(cache: &str) -> Step<Use> {
    Action::new(
        "cachix",
        "cachix-action",
        "5f2d7c5294214f71b873db4b969586b980625e71", // v17
    )
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
        .shell("bash")
        .id("setup-nix-profile")
}

/// Runs a `just` recipe — the same command that runs locally.
pub fn just(recipe: &str) -> Step<Run> {
    Step::new(format!("just {recipe}"))
        .run(format!("just {recipe}"))
        .shell("bash")
        .id(recipe)
}

/// Runs an `xtask` subcommand via the `cargo xtask` alias in
/// `.cargo/config.toml`.
///
/// Values are quoted: several carry a `${{ }}` expression, which is substituted
/// before the shell sees the line.
pub fn cargo_xtask(subcommand: &str, args: &[(&str, Option<&str>)]) -> Step<Run> {
    let command = args.iter().fold(
        format!("cargo xtask {subcommand}"),
        |command, (flag, value)| {
            if let Some(arg) = value {
                format!("{command} {flag} \"{arg}\"")
            } else {
                format!("{command} {flag}")
            }
        },
    );

    Step::new(format!("cargo xtask {subcommand}"))
        .run(command)
        .shell("bash")
}

/// Installs the whole toolchain — node, pnpm, just, sops, age, yq, nu, and
/// rust with every cross-compilation target, read from `rust-toolchain.toml`.
pub fn setup_mise(with_cache: Option<bool>) -> Step<Use> {
    Action::new(
        "jdx",
        "mise-action",
        "c37c93293d6b742fc901e1406b8f764f6fb19dac", // v2
    )
    .step("Setup mise")
    .add_with(("cache", with_cache.unwrap_or(true)))
}

/// Restores compiled registry dependencies between runs.
///
/// Written only from `main`: caches are branch-scoped with fallback to the
/// default branch, so one warm entry there serves every branch.
pub(crate) fn rust_cache() -> Step<Use> {
    Action::new(
        "Swatinem",
        "rust-cache",
        "6323deb102c322ba6fcbdcafc7e3dddab59af2b6", // v2
    )
    .step("Cache Rust dependencies")
    .add_with(("save-if", "${{ github.ref == 'refs/heads/main' }}"))
}

pub fn pnpm_install() -> Step<Run> {
    Step::new("Install JS deps")
        .run("pnpm install --frozen-lockfile")
        .shell("bash")
        .id("pnpm-install")
}

/// Tauri's Linux backend is WebKitGTK, which the runner image does not carry.
pub fn install_linux_deps() -> Step<Run> {
    Step::new("Install system deps (Linux)")
        .run(
            indoc! {"
            sudo apt-get update
            sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
        "}
            .trim_end(),
        )
        .shell("bash")
        .id("linux-deps")
}

pub fn setup_java(major: &str) -> Step<Use> {
    Action::new(
        "actions",
        "setup-java",
        "b6effb05e454b25005698d916606bdc6ffcbf961", // v5
    )
    .step("Setup Java")
    .add_with(("distribution", "zulu"))
    .add_with(("java-version", major))
    .id("setup-java")
}

pub fn setup_android_sdk() -> Step<Use> {
    Action::new(
        "android-actions",
        "setup-android",
        "40fd30fb8d7440372e1316f5d1809ec01dcd3699", // v4
    )
    .step("Setup Android SDK")
    .id("setup-android-sdk")
}

pub fn install_ndk(version: &str) -> Step<Run> {
    Step::new("Install Android NDK")
        .run(format!("sdkmanager \"ndk;{version}\""))
        .id("setup-android-ndk")
}

/// Builds via the same `just` recipe used locally.
///
/// That recipe wraps the build in `secretspec run`, so signing material reaches
/// only the build process and never `$GITHUB_ENV`, which later steps inherit.
/// CI supplies just the key that unlocks the store.
pub fn build(channel: Channel, platform: Platform) -> Step<Run> {
    Step::new(format!("Build {platform}"))
        .run(format!(
            "just tauri::{slug}::{recipe}",
            slug = platform.slug(),
            recipe = channel.build_recipe(),
        ))
        .shell("bash")
        .add_env(SOPS_AGE_KEY_ENV)
        .id("build-recipe")
}

/// Runs a fastlane lane with the platform's secrets injected for its lifetime.
pub fn fastlane(platform: Platform, channel: Channel) -> Step<Run> {
    let spec = SecretSpec::new(format!(
        "fastlane {lane} {flavor}",
        lane = platform.slug(),
        flavor = channel.flavor()
    ))
    .with_profile("release")
    .with_scope(platform.slug());

    Into::<Step<Run>>::into(spec)
        .when(platform.is_android(), |step| {
            step.name("Upload to the Play Store")
        })
        .when(platform.is_ios(), |step| {
            step.name("Upload to the App Store")
        })
        .id("publish-mobile-app")
}

/// Uploads everything the platform's build produced, under a name naming both
/// the platform and the channel.
pub fn upload_artifact(channel: Channel, platform: Platform) -> Step<Use> {
    Action::new(
        "actions",
        "upload-artifact",
        "ea165f8d65b6e75b540449e92b4886f43607fa02", // v4
    )
    .step("Upload artifacts")
    .add_with(("name", Artifact(channel, platform).to_string()))
    .add_with(("path", platform.artifacts()))
    .add_with(("if-no-files-found", "error"))
    .id("upload-artifacts")
}

/// Renders the notes for `tag` with git-cliff: everything since the previous
/// tag, grouped straight from the Conventional Commit history.
pub fn release_notes(tag: Option<&JobOutput>) -> Step<Run> {
    Step::new("Generate release notes")
        .run(r#"git-cliff --latest --output "${RUNNER_TEMP}/notes.md""#)
        .shell("bash")
        .if_condition(Channel::tag_condition(tag))
        .id("gen-release-notes")
}

/// Drafts the release for `tag`, carrying the notes from [`release_notes`].
///
/// A draft: the platform builds upload into it, and it goes public only when
/// [`publish_release`] flips it — so a failed build never leaves a
/// half-populated release visible. The `view` guard makes a re-run reuse the
/// draft it left behind; `gh` resolves drafts by tag name.
pub fn create_release(channel: Channel, tag: Option<&JobOutput>) -> Step<Run> {
    let view = r#"gh release view "${TAG}" >/dev/null 2>&1"#;
    let create = r#"gh release create "${TAG}" --draft --verify-tag --notes-files "${RUNNER_TEMP}/notes.md""#;
    let create = format!(
        "{view} \\\n|| {create}{prerelease}",
        prerelease = if channel.prerelease() {
            " --prerelease"
        } else {
            ""
        },
    );

    Step::new("Create the draft")
        .run(create)
        .shell("bash")
        .if_condition(Channel::tag_condition(tag))
        .add_env(("GH_TOKEN", GITHUB_TOKEN))
        .add_env(("TAG", Channel::tag(tag)))
        .id("draft-release")
}

/// Uploads the platform's artifacts to the release for `tag`. Skipped entirely
/// on a build-only run, where the tag is empty.
///
/// `tag` is the planner's output on nightly, and `None` elsewhere — where the
/// tag comes from the pushed ref or the dispatch input instead.
pub fn upload_to_release(platform: Platform, tag: Option<&JobOutput>) -> Step<Run> {
    let globs = platform
        .artifacts()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    Step::new("Upload to GitHub Release")
        .run(format!(r#"gh release upload "${{TAG}}" {globs} --clobber"#))
        .shell("bash")
        .if_condition(Channel::tag_condition(tag))
        .add_env(("GH_TOKEN", GITHUB_TOKEN))
        .add_env(("TAG", Channel::tag(tag)))
        .id("upload-release")
}

/// Flips the draft public. When this runs is the publish job's decision.
pub fn publish_release(tag: Option<&JobOutput>) -> Step<Run> {
    Step::new("Publish the release")
        .run(r#"gh release edit "${TAG}" --draft=false"#)
        .shell("bash")
        .add_env(("GH_TOKEN", GITHUB_TOKEN))
        .add_env(("TAG", Channel::tag(tag)))
        .id("publish-release")
}

pub(crate) struct Nextest(Step<Run>);

pub(crate) fn cargo_nextest(suite: TestSuite) -> Nextest {
    let step = Step::new(format!("Run {} tests", suite.describe()))
        .run("cargo nextest run --workspace --no-fail-fast")
        .shell("bash")
        .id("nextest");
    Nextest(step)
}

impl Nextest {
    #[allow(dead_code)]
    pub(crate) fn with_filter_expr(mut self, filter_expr: &str) -> Self {
        if let Some(command) = self.0.value.run.as_mut() {
            command.push_str(r#" -E "${FILTER_EXPR}""#);
        }
        self.0 = self.0.add_env(("FILTER_EXPR", filter_expr));
        self
    }
}

impl From<Nextest> for Step<Run> {
    fn from(value: Nextest) -> Self {
        value.0
    }
}

#[derive(Clone, Default)]
pub(crate) struct SecretSpec {
    profile: Option<String>,
    scope: Option<String>,
    cmd: String,
}

impl SecretSpec {
    pub(crate) fn new(cmd: String) -> Self {
        Self {
            cmd,
            ..Default::default()
        }
    }
    #[allow(dead_code)]
    pub(crate) fn with_profile(mut self, profile: impl ToString) -> Self {
        self.profile = Some(profile.to_string());
        self
    }

    #[allow(dead_code)]
    pub(crate) fn with_scope(mut self, scope: impl ToString) -> Self {
        self.scope = Some(scope.to_string());
        self
    }

    #[allow(dead_code)]
    pub(crate) fn with_cmd(mut self, cmd: impl ToString) -> Self {
        self.cmd = cmd.to_string();
        self
    }
}

impl From<SecretSpec> for Step<Run> {
    fn from(spec: SecretSpec) -> Self {
        let mut command = String::from("secretspec run");
        if let Some(profile) = spec.profile {
            command.push_str(&format!(" --profile {profile}"));
        }
        if let Some(scope) = spec.scope {
            command.push_str(&format!(" --scope {scope}"));
        }
        command.push_str(&format!(" -- {cmd}", cmd = spec.cmd));

        Step::new("secretspec")
            .run(command)
            .add_env(("SOPS_AGE_KEY", SOPS_AGE_KEY))
    }
}
