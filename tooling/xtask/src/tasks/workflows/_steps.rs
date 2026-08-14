//! Step constructors shared across workflows. Each returns a labelled step
//! ready to drop into a job.

use super::{Artifact, Channel, FluentBuilder, JobOutput, Platform, SOPS_AGE_KEY, TestSuite};
use gh_workflow::{Run, Step, Use};
use indoc::indoc;

/// A `uses:` reference — `{owner}/{repo}@{version}`.
pub struct Action {
    owner: &'static str,
    repo: &'static str,
    version: &'static str,
}

impl Action {
    pub const fn new(owner: &'static str, repo: &'static str, version: &'static str) -> Self {
        Self {
            owner,
            repo,
            version,
        }
    }

    /// A `Step<Use>` invoking this action, labelled `name`.
    pub fn step(&self, name: impl ToString) -> Step<Use> {
        Step::new(name).uses(self.owner, self.repo, self.version)
    }
}

impl From<Action> for Step<Use> {
    fn from(a: Action) -> Self {
        Step::new(a.repo).uses(a.owner, a.repo, a.version)
    }
}

pub fn checkout() -> Step<Use> {
    Action::new("actions", "checkout", "v6")
        .step("Checkout")
        .with(("persist-credentials", false))
        .id("checkout")
}

pub fn install_nix() -> Step<Use> {
    Action::new("cachix", "install-nix-action", "v31")
        .step("Install Nix")
        .id("setup-nix")
}

/// Reads (and, given a token, writes) the named Cachix binary cache.
pub fn cachix(cache: &str) -> Step<Use> {
    Action::new("cachix", "cachix-action", "v17")
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

/// Runs a `just` recipe — the same command that runs locally.
pub fn just(recipe: &str) -> Step<Run> {
    Step::new(format!("just {recipe}"))
        .run(format!("just {recipe}"))
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

    Step::new(format!("cargo xtask {subcommand}")).run(command)
}

/// Installs the whole toolchain — node, pnpm, just, sops, age, yq, nu, and
/// rust with every cross-compilation target, read from `rust-toolchain.toml`.
pub fn setup_mise() -> Step<Use> {
    Action::new("jdx", "mise-action", "v2").step("Setup mise")
}

/// Restores compiled registry dependencies between runs.
///
/// Written only from `main`: caches are branch-scoped with fallback to the
/// default branch, so one warm entry there serves every branch.
pub(crate) fn rust_cache() -> Step<Use> {
    Action::new("Swatinem", "rust-cache", "v2")
        .step("Cache Rust dependencies")
        .add_with(("save-if", "${{ github.ref == 'refs/heads/main' }}"))
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
    Action::new("actions", "setup-java", "v5")
        .step("Setup Java")
        .add_with(("distribution", "zulu"))
        .add_with(("java-version", major))
}

pub fn setup_android_sdk() -> Step<Use> {
    Action::new("android-actions", "setup-android", "v4").step("Setup Android SDK")
}

pub fn install_ndk(version: &str) -> Step<Run> {
    Step::new("Install Android NDK").run(format!("sdkmanager \"ndk;{version}\""))
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
        .add_env(("SOPS_AGE_KEY", SOPS_AGE_KEY))
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
}

/// Uploads everything the platform's build produced, under a name naming both
/// the platform and the channel.
pub fn upload_artifact(channel: Channel, platform: Platform) -> Step<Use> {
    Action::new("actions", "upload-artifact", "v4")
        .step("Upload artifacts")
        .add_with(("name", Artifact(channel, platform).to_string()))
        .add_with(("path", platform.artifacts()))
        .add_with(("if-no-files-found", "error"))
}

/// Attaches files to the release for the channel's tag. Skipped entirely on a
/// build-only run, where the tag is empty.
/// Attaches the platform's artifacts to the release for `tag`.
///
/// `tag` is the planner's output on nightly, and `None` elsewhere — where the
/// tag comes from the pushed ref or the dispatch input instead.
pub fn publish_to_release(
    channel: Channel,
    platform: Platform,
    tag: Option<&JobOutput>,
) -> Step<Use> {
    Action::new("softprops", "action-gh-release", "v3")
        .step("Upload to GitHub Release")
        .if_condition(Channel::tag_condition(tag))
        .add_with(("tag_name", Channel::tag(tag)))
        .add_with(("prerelease", channel.prerelease()))
        .add_with(("files", platform.artifacts()))
}

pub(crate) struct Nextest(Step<Run>);

pub(crate) fn cargo_nextest(suite: TestSuite) -> Nextest {
    let step = Step::new(format!("Run {} tests", suite.describe()))
        .run("cargo nextest run --workspace --no-fail-fast");
    Nextest(step)
}

impl Nextest {
    #[allow(dead_code)]
    pub(crate) fn with_filter_expr(mut self, filter_expr: &str) -> Self {
        if let Some(command) = self.0.value.run.as_mut() {
            command.push_str(&format!(r#" -E "${{FILTER_EXPR}}""#));
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
