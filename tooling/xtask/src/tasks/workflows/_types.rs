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

    /// Globs matching what this platform's build produces: the installers a
    /// person downloads, and the payloads and signatures the updater fetches.
    ///
    /// `createUpdaterArtifacts` signs each installer in place rather than
    /// wrapping it — the `.nsis.zip` and `.AppImage.tar.gz` these globs used
    /// to name are Tauri v1's shape, kept only under `"v1Compatible"` — so on
    /// Linux and Windows one file serves both roles. macOS is the exception:
    /// the updater cannot consume a `.dmg`, so its payload is a separate
    /// `.app.tar.gz`, which is also why that platform lists two downloads.
    ///
    /// Linux signs the deb as well, which nothing fetches: the updater takes
    /// the AppImage, and a deb is installed by a package manager.
    pub fn artifacts(self) -> &'static str {
        use Platform::*;
        match self {
            macOS => indoc! {"
                target/release/bundle/dmg/*.dmg
                target/release/bundle/macos/*.tar.gz
                target/release/bundle/macos/*.tar.gz.sig"},
            Linux => indoc! {"
                target/release/bundle/appimage/*.AppImage
                target/release/bundle/appimage/*.AppImage.sig
                target/release/bundle/deb/*.deb"},
            Windows => indoc! {"
                target/release/bundle/nsis/*.exe
                target/release/bundle/nsis/*.exe.sig"},
            // Gradle splits flavor from build type for an APK
            // (`apk/universal/release`) and joins them for a bundle
            // (`bundle/universalRelease`).
            Android => indoc! {"
                src-tauri/gen/android/app/build/outputs/apk/*/release/*.apk
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
            Linux => &["AppImage"],
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

    /// Globs matching this platform's published artifacts on disk: the
    /// subset of [`Platform::artifacts`] that reaches the release store, in
    /// the same order as [`PublishedTarget::extensions`] — every other glob
    /// there (signatures, the updater's `.tar.gz` payload, the `.deb`) stays
    /// a build output without becoming a published target.
    pub fn published_globs(self) -> &'static [&'static str] {
        use Platform::*;
        match self {
            macOS => &["target/release/bundle/dmg/*.dmg"],
            Linux => &["target/release/bundle/appimage/*.AppImage"],
            Windows => &["target/release/bundle/nsis/*.exe"],
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
    /// wraps nothing, so the updater fetches the same file [`published_target`]
    /// already uploads, and only the signature is new here. macOS's updater
    /// cannot consume a `.dmg`, so its payload is the separate `.app.tar.gz`
    /// [`artifacts`] already builds — see that doc for the full story. Paired
    /// one-to-one with [`updater_globs`].
    pub fn updater_extensions(self) -> &'static [&'static str] {
        use Platform::*;
        match self {
            macOS => &["app.tar.gz", "app.tar.gz.sig"],
            Linux => &["AppImage.sig"],
            Windows => &["exe.sig"],
            Android | iOS => &[],
        }
    }

    /// Globs matching this platform's updater artifacts on disk, in the same
    /// order as [`updater_extensions`] — the subset of [`Platform::artifacts`]
    /// the updater needs beyond the installer itself.
    pub fn updater_globs(self) -> &'static [&'static str] {
        use Platform::*;
        match self {
            macOS => &[
                "target/release/bundle/macos/*.tar.gz",
                "target/release/bundle/macos/*.tar.gz.sig",
            ],
            Linux => &["target/release/bundle/appimage/*.AppImage.sig"],
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
    /// suite. Belongs on a job that runs a formatting or lint pass; the
    /// workflow audit is the one that does through mise, since
    /// `check-generated` reaches its tooling through nix.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Unwrap, Display, EnumIter)]
pub enum Arch {
    #[allow(non_camel_case_types)]
    amd64,
    #[allow(non_camel_case_types)]
    arm64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Display, IsVariant, clap::ValueEnum)]
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

    /// Platforms that must each upload successfully before this channel's
    /// manifest is written. All three channels require today's full build
    /// matrix — see the design's "Targets are the host architecture of each
    /// platform" decision; a channel narrowing its own required set is a
    /// future change to this, not to the publish gate that reads it.
    pub fn required_targets(self) -> &'static [Platform] {
        use Platform::*;
        &[macOS, Windows, Linux, iOS, Android]
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
    /// Whether this run itself created `tag`'s ref. True only on the
    /// publish path — the withdraw job reads it back to decide whether it
    /// may delete the tag.
    pub tag_minted: JobOutput,
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
        tag_minted: JobOutput,
        should_build: JobOutput,
        version: JobOutput,
        version_code: JobOutput,
        bundle_version: JobOutput,
        job: NamedJob,
    ) -> Self {
        Self {
            job,
            tag,
            tag_minted,
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
        assert_eq!(linux.extensions, &["AppImage"]);

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

    /// Today's required set is every platform, for every channel — a channel
    /// narrowing it is a change to this list, not to the gate that counts
    /// against it.
    #[test]
    fn every_channel_requires_the_full_target_set() {
        let all: Vec<Platform> = Platform::iter().collect();
        for channel in [Channel::Stable, Channel::Preview, Channel::Nightly] {
            assert_eq!(channel.required_targets(), all.as_slice(), "{channel}");
        }
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
        assert_eq!(Platform::Linux.updater_extensions(), &["AppImage.sig"]);
        assert_eq!(Platform::Windows.updater_extensions(), &["exe.sig"]);
        for platform in [Platform::Android, Platform::iOS] {
            assert!(platform.updater_extensions().is_empty(), "{platform}");
        }
    }
}
