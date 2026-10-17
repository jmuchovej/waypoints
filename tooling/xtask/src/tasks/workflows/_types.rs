use derive_more::{Display, IsVariant, Unwrap};
use gh_workflow::{
    Expression, JobOutput, NamedJob, WorkflowCallInput, WorkflowCallSecret, WorkflowDispatchInput,
};
use indoc::indoc;
use strum::EnumIter;

/// Which test suite to run.
#[derive(Clone, Copy)]
pub enum TestSuite {
    /// Everything the shipped artifacts are built from. Gates a release.
    Release,
    /// Only what the change can break.
    Changed,
}

impl TestSuite {
    pub(crate) fn flag(self) -> &'static str {
        match self {
            TestSuite::Release => " --release",
            TestSuite::Changed => "",
        }
    }

    pub(crate) fn describe(self) -> &'static str {
        match self {
            TestSuite::Release => "release",
            TestSuite::Changed => "affected",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Unwrap, Display, IsVariant, EnumIter)]
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
    /// Job id and artifact suffix.
    pub fn slug(self) -> String {
        self.to_string().to_lowercase()
    }

    /// The label used by the `platforms` dispatch input. Cased for the
    /// dropdown, so it does not match [`Platform::slug`].
    pub fn dispatch_label(self) -> String {
        self.to_string()
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
                src-tauri/gen/android/app/build/outputs/apk/*Release/*.apk
                src-tauri/gen/android/app/build/outputs/bundle/*Release/*.aab"},
            iOS => "src-tauri/gen/apple/build/**/*.ipa",
        }
        .trim()
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
}

/// A mise config environment, as declared in `waypoints.mise.environments` and
/// materialised into `.config/mise.<name>.toml`.
///
/// mise does not validate `MISE_ENV`: a misspelled name loads the base config
/// and nothing else, so a job gets base-only tooling and fails on a missing
/// binary far from the typo. Naming environments through this enum is what
/// makes that unwritable; `every_variant_has_a_generated_config` keeps it
/// honest against the nix declaration.
///
/// The variants are the whole set — there is no union environment to exclude,
/// because `MISE_ENV` composes at selection and a job names the lanes it works
/// in. [`MiseEnv::select`] is what builds that list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Display, EnumIter)]
#[display(rename_all = "kebab-case")]
pub enum MiseEnv {
    Xtask,
    Test,
    /// Every formatter and every check — treefmt and the rest of the `hk`
    /// suite. Belongs on a job that runs a formatting or lint pass; no
    /// generated job does one through mise today, since `check-generated`
    /// reaches its tooling through nix.
    Devtools,
    BuildDesktop,
    BuildMobile,
    Website,
}

impl MiseEnv {
    /// The `MISE_ENV` value selecting all of `envs`. mise reads it as a
    /// comma-separated list and composes the named configs additively.
    pub fn select(envs: &[Self]) -> (&'static str, String) {
        (
            "MISE_ENV",
            envs.iter()
                .map(Self::to_string)
                .collect::<Vec<_>>()
                .join(","),
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Unwrap, Display, EnumIter)]
#[allow(dead_code, reason = "no job is arch-scoped yet")]
pub enum Arch {
    #[allow(non_camel_case_types)]
    amd64,
    #[allow(non_camel_case_types)]
    arm64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Display, IsVariant)]
#[display(rename_all = "lowercase")]
pub enum Channel {
    Stable,
    Preview,
    Nightly,
}

impl Channel {
    /// One string naming the workflow, the `just icons::<flavor>` recipe, and
    /// the cargo feature. Keep them identical.
    pub fn flavor(self) -> String {
        self.to_string()
    }

    pub fn prerelease(self) -> bool {
        self != Channel::Stable
    }

    /// This channel's release tag, unwrapped for `if:` — already an expression
    /// context, so `${{ }}` there is an error.
    ///
    /// A planner output means nightly, which mints its tag in the `prepare`
    /// job; without one the tag comes from the pushed ref or the dispatch
    /// input.
    fn tag_expr(tag: Option<&JobOutput>) -> String {
        match tag {
            Some(tag) => tag.expr(),
            None => "github.event_name == 'push' && github.ref_name || inputs.tag".to_owned(),
        }
    }

    /// The same tag, wrapped for use as a `with:` value.
    pub fn tag(tag: Option<&JobOutput>) -> String {
        format!("${{{{ {tag} }}}}", tag = Self::tag_expr(tag))
    }

    /// True when this run should publish. A build-only run has an empty tag.
    pub fn tag_condition(tag: Option<&JobOutput>) -> Expression {
        Expression::new(match tag {
            Some(tag) => format!("{tag} != ''", tag = tag.expr()),
            None => format!("({tag}) != ''", tag = Self::tag_expr(None)),
        })
    }

    /// The tag pattern a push must match to trigger this channel.
    /// `None` for nightly, which mints its own tags — a push trigger would
    /// re-enter the workflow on the tag it had just created.
    pub fn tag_pattern(self) -> Option<&'static str> {
        use Channel::*;
        match self {
            Stable => Some("v[0-9]+.[0-9]+.[0-9]+"),
            Preview => Some("v[0-9]+.[0-9]+.[0-9]+-pre.[0-9]+"),
            Nightly => None,
        }
    }

    /// The full set of choices offered by the `platforms` dispatch input.
    pub fn dispatch_platforms(self) -> Vec<&'static str> {
        vec![
            "All", "Desktop", "macOS", "Linux", "Windows", "Mobile", "Android", "iOS",
        ]
    }

    /// Guards a job on the `platforms` dispatch input.
    ///
    /// A job answers to `All`, its group (`Desktop`/`Mobile`), and its own
    /// label. Given a planner verdict it additionally waits on that, which is
    /// how a nightly with nothing new skips its builds.
    pub fn guard(self, platform: Platform, should_build: Option<&JobOutput>) -> Expression {
        let group = match &platform {
            Platform::Android | Platform::iOS => "Mobile",
            Platform::Linux | Platform::macOS | Platform::Windows => "Desktop",
        };
        let labels = ["All", group, &platform.dispatch_label()];
        let choices = labels
            .iter()
            .map(|label| format!("\"{label}\""))
            .collect::<Vec<_>>()
            .join(",");
        let selected = format!("contains(fromJson('[{choices}]'), inputs.platforms)");

        Expression::new(match should_build {
            Some(should_build) => format!(
                "{planner} == 'true' && \
                 (github.event_name != 'workflow_dispatch' || {selected})",
                planner = should_build.expr(),
            ),
            None => format!("github.event_name == 'push' || {selected}"),
        })
    }
}

/// The `upload-artifact` name for one build: `waypoints-{platform}-{channel}`.
///
/// A newtype rather than a plain `(Channel, Platform)`: the orphan rule counts
/// tuples as foreign, so `Display` cannot be implemented on one.
#[derive(Display)]
#[display("waypoints-{}-{}", _1.slug(), _0)]
pub struct Artifact(pub Channel, pub Platform);

/// The nightly planner, together with the outputs its dependents read back.
///
/// Built once and lent to the jobs that depend on it. Each output already
/// carries the job name, so nothing downstream respells either that or the
/// output keys — and a typo becomes a missing field rather than an expression
/// that silently resolves to an empty string.
pub struct PrepareJob {
    job: NamedJob,
    pub tag: JobOutput,
    pub should_build: JobOutput,
    pub version: JobOutput,
    pub version_code: JobOutput,
    pub bundle_version: JobOutput,
}

impl PrepareJob {
    /// Outputs first: each is derived from `job`, so they have to be built
    /// before it is moved in.
    pub fn new(
        tag: JobOutput,
        should_build: JobOutput,
        version: JobOutput,
        version_code: JobOutput,
        bundle_version: JobOutput,
        job: NamedJob,
    ) -> Self {
        Self {
            job,
            tag,
            should_build,
            version,
            version_code,
            bundle_version,
        }
    }

    /// The job id, for wiring `needs:` without consuming the job.
    pub fn name(&self) -> &str {
        &self.job.name
    }
}

impl From<PrepareJob> for NamedJob {
    fn from(prepare: PrepareJob) -> Self {
        prepare.job
    }
}

#[derive(Clone)]
pub struct WorkflowInput {
    pub input_type: &'static str,
    pub name: &'static str,
    pub default: Option<String>,
    pub description: Option<String>,
    pub options: Vec<String>,
}

impl WorkflowInput {
    pub fn string(name: &'static str, default: Option<String>) -> Self {
        Self {
            input_type: "string",
            name,
            default,
            description: None,
            options: vec![],
        }
    }

    pub fn bool(name: &'static str, default: Option<bool>) -> Self {
        Self {
            input_type: "boolean",
            name,
            default: default.as_ref().map(ToString::to_string),
            description: None,
            options: vec![],
        }
    }

    pub fn choices(
        name: &'static str,
        choices: Vec<impl ToString>,
        default: Option<impl ToString>,
    ) -> Self {
        Self {
            input_type: "choice",
            name,
            default: default.as_ref().map(ToString::to_string),
            description: None,
            options: choices.iter().map(ToString::to_string).collect(),
        }
    }

    pub fn description(mut self, description: impl ToString) -> Self {
        self.description = Some(description.to_string());
        self
    }

    pub fn input(&self) -> WorkflowDispatchInput {
        WorkflowDispatchInput {
            description: self
                .description
                .clone()
                .unwrap_or_else(|| self.name.to_owned()),
            required: self.default.is_none(),
            input_type: self.input_type.to_owned(),
            default: self.default.clone(),
            options: self.options.clone(),
        }
    }

    pub fn call_input(&self) -> WorkflowCallInput {
        WorkflowCallInput {
            description: self.name.to_owned(),
            required: self.default.is_none(),
            input_type: self.input_type.to_owned(),
            default: self.default.clone(),
        }
    }

    pub(crate) fn expr(&self) -> String {
        format!("inputs.{}", self.name)
    }
}

impl std::fmt::Display for WorkflowInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${{{{ {} }}}}", self.expr())
    }
}

impl serde::Serialize for WorkflowInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<WorkflowInput> for (String, WorkflowDispatchInput) {
    fn from(value: WorkflowInput) -> Self {
        (value.name.into(), value.input())
    }
}

#[derive(Clone)]
#[allow(dead_code, reason = "for workflows that take `secrets:` inputs")]
pub(crate) struct WorkflowSecret {
    pub name: &'static str,
    description: String,
    required: bool,
}

#[allow(dead_code, reason = "for workflows that take `secrets:` inputs")]
impl WorkflowSecret {
    pub fn new(name: &'static str, description: impl ToString) -> Self {
        Self {
            name,
            description: description.to_string(),
            required: true,
        }
    }

    pub fn secret_configuration(&self) -> WorkflowCallSecret {
        WorkflowCallSecret {
            description: self.description.clone(),
            required: self.required,
        }
    }
}

impl std::fmt::Display for WorkflowSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${{{{ secrets.{} }}}}", self.name)
    }
}

impl serde::Serialize for WorkflowSecret {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use strum::IntoEnumIterator as _;

    /// mise will not validate `MISE_ENV`, so nothing but this test connects a
    /// variant to the config it selects. A renamed or dropped environment in
    /// `devenv.nix` leaves the variant naming a file that is not there, and a
    /// job selecting it silently gets base-only tooling.
    #[test]
    fn every_variant_has_a_generated_config() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for env in MiseEnv::iter() {
            let config = root.join(format!(".config/mise.{env}.toml"));
            assert!(
                config.exists(),
                "MiseEnv::{env:?} selects {}, which does not exist — has an environment been renamed in devenv.nix?",
                config.display(),
            );
        }
    }

    /// The whole point of the enum: a job names environments, and the value it
    /// produces is the comma-separated list mise composes.
    #[test]
    fn selecting_several_environments_joins_them() {
        assert_eq!(
            MiseEnv::select(&[MiseEnv::Xtask, MiseEnv::BuildMobile]),
            ("MISE_ENV", "xtask,build-mobile".to_owned()),
        );
    }
}
