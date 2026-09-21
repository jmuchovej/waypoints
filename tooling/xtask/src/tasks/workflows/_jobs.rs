//! Every job the workflows are assembled from. A workflow module names jobs
//! and wires their `needs:`.

use super::{
    Channel, GITHUB_TOKEN, Platform, PrepareJob, TestSuite, WorkflowContext, helpers::ANDROID_HOME,
    runners, steps,
};
use gh_workflow::{
    Expression, FluentBuilder, Job, JobOutput, Level, NamedJob, Permissions, StepOutput,
};

/// The environment variable a build reads its store build number from. One
/// name; [`build_code`] decides which value fills it.
pub const BUILD_CODE_ENV: &str = "WAYPOINTS_BUILD_CODE";

/// The planner output carrying `platform`'s build number, or `None` where the
/// platform's store has no such concept. Android's `versionCode` is a single
/// integer; iOS's `CFBundleVersion` is dotted.
fn build_code(prepare: &PrepareJob, platform: Platform) -> Option<&JobOutput> {
    match platform {
        Platform::Android => Some(&prepare.version_code),
        Platform::iOS => Some(&prepare.bundle_version),
        _ => None,
    }
}

/// Passed to the planner only on a manual dispatch, which builds without
/// publishing.
///
/// A nightly's store build number is its date, so a second publish on one day
/// could not produce a distinct one. A dispatch exercises the pipeline and
/// leaves publishing to the next scheduled run.
const BUILD_ONLY_ON_DISPATCH: &str =
    "${{ github.event_name == 'workflow_dispatch' && '--build-only' || '' }}";

/// Release jobs never format, so mise skips the formatter tool set devenv
/// projects into `[vars].WAYPOINTS_FORMATTERS` — one config file, and one
/// fewer install per formatter on every release runner.
fn without_formatters(job: Job, formatters: &str) -> Job {
    job.add_env(("MISE_DISABLE_TOOLS", formatters.to_owned()))
}

/// Resolves the nightly tag: derive it from the workspace version, skip the run
/// when nothing landed since the last nightly, and create the tag.
pub fn prepare(formatters: &str) -> PrepareJob {
    let planner = steps::cargo_xtask("nightly-plan", &[(BUILD_ONLY_ON_DISPATCH, None)])
        .id("nightly-plan")
        .add_env(("GH_TOKEN", GITHUB_TOKEN));

    let p_tag = StepOutput::new_trusted(&planner, "tag");
    let p_build = StepOutput::new_trusted(&planner, "should_build");
    let p_version = StepOutput::new_trusted(&planner, "version_code");
    let p_bundle = StepOutput::new_trusted(&planner, "bundle_version");

    let job = NamedJob::new(
        "prepare",
        without_formatters(Job::default(), formatters)
            .name("Plan the nightly")
            // Write: the planner creates the nightly tag ref.
            .permissions(Permissions::default().contents(Level::Write))
            .runs_on(runners::LINUX)
            // Tags without history: the planner resolves them to commits but
            // never walks between them.
            .add_step(steps::checkout().add_with(("fetch-tags", true)))
            .add_step(steps::setup_mise(false.into()))
            .add_step(planner)
            .outputs([
                p_tag.clone().as_output(),
                p_build.clone().as_output(),
                p_version.clone().as_output(),
                p_bundle.clone().as_output(),
            ]),
    );

    PrepareJob::new(
        p_tag.as_job_output(&job),
        p_build.as_job_output(&job),
        p_version.as_job_output(&job),
        p_bundle.as_job_output(&job),
        job,
    )
}

/// Runs the suite on one platform, as two steps: the plan resolves the
/// filterset, and the run is skipped when it comes back empty.
pub fn test(suite: TestSuite, platform: Platform, formatters: &str) -> NamedJob {
    // The command publishes its own step output, so no shell redirection and
    // nothing spelled differently for PowerShell on Windows.
    let plan = steps::cargo_xtask("test-plan", &[(suite.flag(), None)]).id("test-plan");
    let filterset = StepOutput::new_trusted(&plan, "filterset");

    let job = Job::default()
        // The changed-suite job is the one formatter-adjacent CI run, so it
        // keeps the full tool set.
        .when(matches!(suite, TestSuite::Release), |job| {
            without_formatters(job, formatters)
        })
        .name(format!("Test {platform}"))
        .permissions(Permissions::default().contents(Level::Read))
        .runs_on(platform.runner())
        // The change-scoped plan diffs against the merge base, which a shallow
        // clone does not contain.
        .add_step(steps::checkout().add_with(("fetch-depth", 0)))
        // Release workflows run cold: their caching steps would count against
        // the artifacts the sibling build jobs publish.
        .add_step(steps::setup_mise(
            matches!(suite, TestSuite::Changed).into(),
        ))
        // After mise: the cache key fingerprints the resolved rustc version.
        .when(matches!(suite, TestSuite::Changed), |job| {
            job.add_step(steps::rust_cache())
        })
        // Tauri links WebKitGTK on Linux, so even a library test build needs it.
        .when(platform.is_linux(), |job| {
            job.add_step(steps::install_linux_deps())
        })
        .add_step(plan)
        .add_step(steps::cargo_nextest(suite).with_filter_expr(&filterset.to_string()));

    NamedJob::new(format!("test-{}", platform.slug()), job)
}

/// Drafts the release every build uploads into, with git-cliff notes.
///
/// The tag guards sit on the steps, not the job: the builds `need` this job,
/// and a skipped job would drag them down with it — on a build-only run the
/// job runs and every step no-ops.
pub fn create_release(channel: Channel, prepare: Option<&PrepareJob>) -> NamedJob {
    let tag = prepare.map(|prepare| &prepare.tag);
    let publishing = || Channel::tag_condition(tag);

    let job = Job::default()
        .name("Draft the release")
        .permissions(Permissions::default().contents(Level::Write))
        .runs_on(runners::LINUX)
        // A nightly that builds nothing drafts nothing; the builds skip on
        // the same verdict.
        .when_some(prepare, |job, prepare| {
            let expr = format!("{} == 'true'", prepare.should_build.expr());
            job.add_needs(prepare.name().to_owned())
                .cond(Expression::new(expr))
        })
        // git-cliff walks from the release tag back to the tag before it.
        .add_step(
            steps::checkout()
                .add_with(("fetch-depth", 0))
                .add_with(("fetch-tags", true))
                .if_condition(publishing()),
        )
        .add_step(steps::setup_mise(false.into()).if_condition(publishing()))
        .add_step(steps::release_notes(tag))
        .add_step(steps::create_release(channel, tag));

    NamedJob::new("create-release", job)
}

/// Publishes the draft once every upload has had its say.
///
/// `!failure() && !cancelled()`: a platform-filtered dispatch skips some
/// builds and must still publish, while one failed build leaves the draft
/// unpublished rather than shipping a partial release. The tag guard skips
/// build-only runs and nightlies that built nothing.
pub fn publish_release(prepare: Option<&PrepareJob>) -> NamedJob {
    let tag = prepare.map(|prepare| &prepare.tag);
    let expr = format!(
        "!failure() && !cancelled() && ({tag})",
        tag = Channel::tag_condition(tag).0
    );

    let job = Job::default()
        .name("Publish the release")
        .permissions(Permissions::default().contents(Level::Write))
        .runs_on(runners::LINUX)
        .cond(Expression::new(expr))
        .add_step(steps::publish_release(tag))
        // Reads the planner's tag, so it waits on the planner.
        .when_some(prepare, |job, prepare| {
            job.add_needs(prepare.name().to_owned())
        });

    NamedJob::new("publish-release", job)
}

pub fn build(
    channel: Channel,
    context: &WorkflowContext,
    platform: Platform,
    prepare: Option<&PrepareJob>,
) -> NamedJob {
    // Absent off nightly, where the tag's own semver suffices.
    let build_code = prepare.and_then(|prepare| build_code(prepare, platform));

    let job = without_formatters(Job::default(), &context.formatters)
        .permissions(Permissions::default().contents(Level::Write))
        .name(format!("Build {platform}"))
        .runs_on(platform.runner())
        .cond(channel.guard(platform, prepare.map(|prepare| &prepare.should_build)))
        .add_step(steps::checkout())
        .when(platform.is_android(), |job| {
            // `setup-android` installs the SDK but not an NDK; the version devenv
            // pins has to be requested explicitly.
            job.add_step(steps::setup_java(&context.android_jdk))
                .add_step(steps::setup_android_sdk())
                .add_step(steps::install_ndk(&context.android_ndk))
        })
        .when(platform.is_linux(), |job| {
            // Tauri's Linux backend is WebKitGTK, absent from the runner image.
            job.add_step(steps::install_linux_deps())
        })
        .add_step(steps::setup_mise(false.into()))
        .add_step(steps::pnpm_install())
        .add_step(
            steps::build(channel, platform)
                .when_some(build_code, |step, code| {
                    step.add_env((BUILD_CODE_ENV, code.to_string()))
                })
                .when(platform.is_android(), |step| {
                    step.add_env((
                        "NDK_HOME",
                        format!("{ANDROID_HOME}/ndk/{ndk}", ndk = context.android_ndk),
                    ))
                }),
        )
        .add_step(steps::upload_artifact(channel, platform))
        .add_step(steps::upload_to_release(
            platform,
            prepare.map(|prepare| &prepare.tag),
        ))
        .when(platform.is_mobile(), |job| {
            job.add_step(steps::fastlane(platform, channel))
        })
        // Reads the planner's outputs, so it waits on the planner.
        .when_some(prepare, |job, prepare| {
            job.add_needs(prepare.name().to_owned())
        });

    NamedJob::new(format!("build-{}", platform.slug()), job)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};

    /// Swapping these would hand Play a value it cannot parse.
    #[test]
    fn each_platform_draws_its_own_build_code() {
        let prepare = prepare("aqua:numtide/treefmt");

        assert_eq!(
            build_code(&prepare, Platform::Android).map(JobOutput::expr),
            Some(prepare.version_code.expr())
        );
        assert_eq!(
            build_code(&prepare, Platform::iOS).map(JobOutput::expr),
            Some(prepare.bundle_version.expr())
        );

        for platform in [Platform::macOS, Platform::Linux, Platform::Windows] {
            assert!(build_code(&prepare, platform).is_none(), "{platform}");
        }
    }

    /// The name injected here has to be the name the build scripts read; both
    /// consumers fall back silently when the variable is absent.
    #[test]
    fn the_build_scripts_read_the_variable_we_inject() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for consumer in [
            "src-tauri/gen/android/app/build.gradle.kts",
            "src-tauri/.just/iOS.justfile",
        ] {
            let source = fs::read_to_string(root.join(consumer))
                .unwrap_or_else(|error| panic!("{consumer}: {error}"));
            assert!(source.contains(BUILD_CODE_ENV), "{consumer}");
        }
    }
}
