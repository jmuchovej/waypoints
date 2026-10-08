//! Every job the workflows are assembled from. A workflow module names jobs
//! and wires their `needs:`.

use super::{
    Channel, GITHUB_TOKEN, MiseEnv, Platform, PrepareJob, TestSuite, WorkflowContext,
    helpers::{ANDROID_HOME, Profile},
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

/// Names the mise config environments a job installs. Additive: each one adds
/// its `[tools]` to the base, so a job declares what it does and gets exactly
/// that. Replaced a deny-list of tool specs, which named what to skip and so
/// had to be re-derived every time the tool set moved — and silently no-opped
/// when a spec it named had been renamed out from under it.
fn mise_env(job: Job, envs: &[MiseEnv]) -> Job {
    job.add_env(MiseEnv::select(envs))
}

/// Resolves the nightly tag: derive it from the workspace version, skip the run
/// when nothing landed since the last nightly, and create the tag.
pub fn prepare(context: &WorkflowContext) -> PrepareJob {
    let planner = steps::cargo_xtask("nightly-plan", &[(BUILD_ONLY_ON_DISPATCH, None)])
        .id("nightly-plan")
        .add_env(("GH_TOKEN", GITHUB_TOKEN));

    let p_tag = StepOutput::new_trusted(&planner, "tag");
    let p_build = StepOutput::new_trusted(&planner, "should_build");
    let p_version = StepOutput::new_trusted(&planner, "version_code");
    let p_bundle = StepOutput::new_trusted(&planner, "bundle_version");

    let job = NamedJob::new(
        "prepare",
        // The planner is a `cargo xtask` run and nothing more.
        mise_env(Job::default(), &[MiseEnv::Xtask])
            .name("Plan the nightly")
            // Write: the planner creates the nightly tag ref.
            .permissions(Permissions::default().contents(Level::Write))
            .runs_on(runners::LINUX)
            // Tags without history: the planner resolves them to commits but
            // never walks between them.
            .add_step(steps::checkout().add_with(("fetch-tags", true)))
            .add_step(steps::setup_mise(false.into(), &context.mise_version))
            // Compiling the planner is the whole job: 5m25s of a 6m10s run,
            // against 39s to install its tools. Cached because nothing here is
            // published — see `rust_cache` for why writing only from `main` is
            // what keeps that safe.
            .add_step(steps::rust_cache())
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
pub fn test(suite: TestSuite, platform: Platform, context: &WorkflowContext) -> NamedJob {
    // The command publishes its own step output, so no shell redirection and
    // nothing spelled differently for PowerShell on Windows.
    let plan = steps::cargo_xtask("test-plan", &[(suite.flag(), None)]).id("test-plan");
    let filterset = StepOutput::new_trusted(&plan, "filterset");

    // Both suites run the same two commands, so both want the same environment
    // — unlike the deny-list this replaced, where the changed-suite job was
    // left with the full tool set because narrowing it meant naming tools.
    let job = mise_env(Job::default(), &[MiseEnv::Test])
        .name(format!("Test {platform}"))
        .permissions(Permissions::default().contents(Level::Read))
        .runs_on(platform.runner())
        // The change-scoped plan diffs against the merge base, which a shallow
        // clone does not contain.
        .add_step(steps::checkout().add_with(("fetch-depth", 0)))
        // Before mise and before the build: the workspace's debug test build
        // exhausts a stock Linux image on its own, and reclaiming afterwards
        // would be too late. Safe to drop the SDK here — this job tests, it
        // never builds for Android.
        .when(platform.is_linux(), |job| {
            job.add_step(steps::free_disk_space(true))
        })
        // mise's own cache stays off for the release suite, which runs cold
        // alongside the build jobs that publish.
        .add_step(steps::setup_mise(
            matches!(suite, TestSuite::Changed).into(),
            &context.mise_version,
        ))
        // The compiled-dependency cache, however, applies to both suites: a
        // test job publishes nothing, and compiling the workspace is what makes
        // these runs 20–33 minutes. After mise, because the cache key
        // fingerprints the resolved rustc version.
        .add_step(steps::rust_cache())
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
pub fn create_release(
    channel: Channel,
    prepare: Option<&PrepareJob>,
    context: &WorkflowContext,
) -> NamedJob {
    let tag = prepare.map(|prepare| &prepare.tag);
    let publishing = || Channel::tag_condition(tag);

    // `git-cliff` is the only tool this job runs through mise, and it lives in
    // `xtask` — the lane the release-automation jobs share.
    let job = mise_env(Job::default(), &[MiseEnv::Xtask])
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
        .add_step(steps::setup_mise(false.into(), &context.mise_version).if_condition(publishing()))
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

    // The mobile lanes need `fastlane`, `xcodegen` and the nushell the `_build`
    // recipes are written in; the desktop ones do not.
    let envs: &[MiseEnv] = if platform.is_mobile() {
        &[MiseEnv::BuildMobile]
    } else {
        &[MiseEnv::BuildDesktop]
    };

    let job = mise_env(Job::default(), envs)
        .permissions(Permissions::default().contents(Level::Write))
        .name(format!("Build {platform}"))
        // The build and, on mobile, the publish resolve the release profile,
        // so the vault credential comes from the release environment.
        .environment(Profile::Release.environment())
        .runs_on(platform.runner())
        .cond(channel.guard(platform, prepare.map(|prepare| &prepare.should_build)))
        .add_step(steps::checkout())
        // Both of these build on an Ubuntu runner, and both are large enough to
        // be worth the reclaim. It runs before `setup-android` so nothing is
        // removed after being installed — and Android keeps its runtime, since
        // `setup-android` configures the image's SDK rather than fetching one.
        .when(platform.is_linux() || platform.is_android(), |job| {
            job.add_step(steps::free_disk_space(!platform.is_android()))
        })
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
        .add_step(steps::setup_mise(false.into(), &context.mise_version))
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
        let prepare = prepare(&WorkflowContext::fixture());

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
            "tooling/bin/xtauri",
        ] {
            let source = fs::read_to_string(root.join(consumer))
                .unwrap_or_else(|error| panic!("{consumer}: {error}"));
            assert!(source.contains(BUILD_CODE_ENV), "{consumer}");
        }
    }
}
