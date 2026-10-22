//! Every job the workflows are assembled from. A workflow module names jobs
//! and wires their `needs:`.

use super::{
    Channel, MiseEnv, Platform, PrepareJob, TestSuite, WorkflowContext, WorkflowInput,
    helpers::{ANDROID_HOME, Environment},
    runners, steps,
    steps::Tools,
};
use gh_workflow::{
    Environment as GhEnvironment, Expression, FluentBuilder, Job, JobOutput, Level, NamedJob,
    Permissions, Step, StepOutput,
};

/// The environment variable a build reads its store build number from. One
/// name; [`build_code`] decides which value fills it.
pub const BUILD_CODE_ENV: &str = "WAYPOINTS_BUILD_CODE";

/// The environment variable a mobile build reads the app version from. Tauri
/// takes it as its config `version`, which the manifest leaves unset, so
/// without this the stores see Gradle's and Xcode's fallbacks (`1.0`,
/// `1.0.0`). iOS shows the release and the date as `0.1.0 (2026.10.10)`;
/// the Gradle project renders Android's `versionName` the same way.
pub const VERSION_ENV: &str = "WAYPOINTS_VERSION";

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

/// The planner output carrying the app version, for the platforms that stamp
/// it: the mobile ones, whose stores reject Tauri's fallbacks. The desktop
/// bundles take their version from the crate.
fn app_version(prepare: &PrepareJob, platform: Platform) -> Option<&JobOutput> {
    platform.is_mobile().then_some(&prepare.version)
}

/// Passed to the planner only on a manual dispatch, which builds without
/// publishing.
///
/// A nightly's store build number is its date, so a second publish on one day
/// could not produce a distinct one. A dispatch exercises the pipeline and
/// leaves publishing to the next scheduled run; its artifacts still carry the
/// numbers that run would mint, so they can be uploaded by hand.
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
///
/// The tag is minted with the wp-botswain App token rather than
/// `GITHUB_TOKEN`: a ref created with the latter raises no `push` event, so
/// nothing downstream of the tag — the release index — would ever run.
pub fn prepare(context: &WorkflowContext) -> PrepareJob {
    let app_token = steps::app_token(&[("contents", "write")]);
    let token = StepOutput::new_trusted(&app_token, "token");

    let planner = steps::cargo_xtask("nightly-plan", &[(BUILD_ONLY_ON_DISPATCH, None)])
        .id("nightly-plan")
        .add_env(("GH_TOKEN", token.to_string()));

    let p_tag = StepOutput::new_trusted(&planner, "tag");
    let p_tag_minted = StepOutput::new_trusted(&planner, "tag_minted");
    let p_build = StepOutput::new_trusted(&planner, "should_build");
    let p_app_version = StepOutput::new_trusted(&planner, "version");
    let p_version = StepOutput::new_trusted(&planner, "version_code");
    let p_bundle = StepOutput::new_trusted(&planner, "bundle_version");

    let registry = steps::cargo_registry(steps::CARGO_HOME);

    let job = NamedJob::new(
        "prepare",
        // The planner is a `cargo xtask` run and nothing more.
        mise_env(Job::default(), &[MiseEnv::Xtask])
            .name("Plan the nightly")
            // Only what `checkout` needs: the tag ref is created with the App
            // token, which carries its own write scope.
            .permissions(Permissions::default().contents(Level::Read))
            // Authenticates as the App and compiles through the CI cache, so
            // the vault credential comes from the automation environment.
            .environment(GhEnvironment::from(Environment::Ci))
            .runs_on(runners::LINUX)
            // Tags and the history between them: the planner walks from the
            // last nightly to HEAD with a pathspec that drops the index
            // directory. In a shallow clone the tip is a root commit diffed
            // against the empty tree, so every path looks changed and the
            // exclusion cannot bite — nightly.5 was minted from an index-only
            // commit exactly so.
            .add_step(
                steps::checkout()
                    .add_with(("fetch-depth", 0))
                    .add_with(("fetch-tags", true)),
            )
            .add_step(steps::setup_mise(false.into(), &context.mise_version))
            // Compiling the planner is the whole job: 5m25s of a 6m10s run,
            // against 39s to install its tools. The registry snapshot and the
            // compiler cache are the ones the build jobs fill, under the same
            // keys, so the compile is mostly hits.
            .add_step(steps::cache_restore(&registry, Tools::Mise))
            .add_step(steps::sccache_start(Tools::Mise))
            .add_step(steps::botswain_credentials())
            .add_step(app_token)
            .add_step(steps::with_sccache(planner))
            .add_step(steps::sccache_stats(Tools::Mise))
            .add_step(steps::cache_save(&registry, Tools::Mise))
            .outputs([
                p_tag.clone().as_output(),
                p_tag_minted.clone().as_output(),
                p_build.clone().as_output(),
                p_app_version.clone().as_output(),
                p_version.clone().as_output(),
                p_bundle.clone().as_output(),
            ]),
    );

    PrepareJob::new(
        p_tag.as_job_output(&job),
        p_tag_minted.as_job_output(&job),
        p_build.as_job_output(&job),
        p_app_version.as_job_output(&job),
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

/// Audits the generated workflows with zizmor, so a suppression is the only
/// way past a finding.
pub fn zizmor(context: &WorkflowContext) -> NamedJob {
    let job = mise_env(Job::default(), &[MiseEnv::Devtools])
        .name("Audit the workflows")
        .permissions(Permissions::default().contents(Level::Read))
        .runs_on(runners::LINUX)
        .add_step(steps::checkout())
        .add_step(steps::setup_mise(false.into(), &context.mise_version))
        .add_step(steps::zizmor());

    NamedJob::new("zizmor", job)
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
        .permissions(
            Permissions::default()
                .contents(Level::Write)
                .pull_requests(Level::Read),
        )
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

/// The job id a platform's build runs under. Shared between [`build`], which
/// names the job, and [`publish_release`], which reads its recorded-target
/// output back — so the convention is stated once.
fn build_job_name(platform: Platform) -> String {
    format!("build-{}", platform.slug())
}

/// The `needs.<job>.outputs.target == '<slug>'` clauses, ANDead together,
/// that assert every required target has recorded in. Shared between the
/// publish gate and the withdraw gate, which assert the fact and its
/// negation respectively.
fn completeness_expr(channel: Channel) -> String {
    channel
        .required_targets()
        .iter()
        .map(|platform| {
            format!(
                "needs.{job}.outputs.target == '{slug}'",
                job = build_job_name(*platform),
                slug = platform.slug(),
            )
        })
        .collect::<Vec<_>>()
        .join(" && ")
}

/// The publishing job's id, which the withdraw gate reads the point of no
/// return back through.
pub const PUBLISH_JOB: &str = "publish";

/// Enters the devenv shell: the tree's pinned `cargo`, and the typst, its
/// fonts, and ImageMagick `xfastlane prepare` renders with, exist nowhere
/// else. Entering realises the whole environment, which a stock image
/// cannot hold.
fn with_devenv(job: Job) -> Job {
    job.add_step(steps::free_disk_space(true))
        .add_step(steps::install_nix())
        .add_step(steps::cachix("devenv"))
        .add_step(steps::nix_profile_install(&["devenv"]))
}

/// Publishes a complete run, in order of retractability: the store upload,
/// the public record, the point of no return, both store submissions, and
/// the manifest last. One job, so the order is a step order.
///
/// Completeness is counted, not inferred from the absence of failure: a
/// target skipped by a `platforms` subset dispatch blocks the gate exactly
/// as one that failed, because neither writes the output this reads — see
/// the design's "The counted gate stays" decision. The tag guard skips
/// build-only runs and nightlies that built nothing.
pub fn publish(channel: Channel, prepare: Option<&PrepareJob>) -> NamedJob {
    let tag = prepare.map(|prepare| &prepare.tag);
    let completeness = completeness_expr(channel);
    let expr = format!(
        "({completeness}) && ({tag})",
        tag = Channel::tag_condition(tag).0
    );

    let mark = steps::mark_submitted();
    let submitted = StepOutput::new_trusted(&mark, "submitted");

    // The devenv shell leaves `CARGO_HOME` unset, so cargo keeps its registry
    // at its default; the snapshot has to land there.
    let registry = steps::cargo_registry("${HOME}/.cargo");
    let compile = Step::new("Build xtask")
        .run(Tools::Devenv.command("cargo build --package xtask"))
        .shell("bash")
        .add_env((
            "SECRETSPEC_REASON",
            "compiles xtask; no secret values are read",
        ))
        .id("build-xtask");

    let job = Job::default()
        .name("Publish the release")
        .permissions(Permissions::default().contents(Level::Write))
        // Every step that resolves anything reads the vault credential from
        // the release environment.
        .environment(GhEnvironment::from(Environment::AppRelease))
        .runs_on(runners::LINUX)
        .cond(Expression::new(expr))
        // `prepare` renders the release notes with git-cliff, which walks
        // from the tag back to the one before it.
        .add_step(
            steps::checkout()
                .add_with(("fetch-depth", 0))
                .add_with(("fetch-tags", true)),
        );
    let job = with_devenv(job)
        // Compiled up front, through the build jobs' cache, so its server is
        // stopped before anything publishes; the wrapper stays set on every
        // later `cargo xtask`, so none of them recompiles.
        .add_step(steps::cache_restore(&registry, Tools::Devenv))
        .add_step(steps::sccache_start(Tools::Devenv))
        .add_step(steps::with_sccache(compile))
        .add_step(steps::sccache_stats(Tools::Devenv))
        .add_step(steps::cache_save(&registry, Tools::Devenv))
        .add_step(steps::download_staged())
        .add_step(steps::download_app_store_info())
        .add_step(steps::prepare_listing(channel))
        .add_step(steps::with_sccache(steps::upload_to_store(tag)))
        .add_step(steps::publish_release(tag))
        .add_step(mark)
        .add_step(steps::submit(Platform::Android, channel))
        .add_step(steps::submit(Platform::iOS, channel))
        // Last: the manifest is what the updater and the download links
        // read, and it is idempotent, so a failure here is repaired by
        // running it again.
        .add_step(steps::with_sccache(steps::write_manifest(tag)))
        .outputs([submitted.as_output()])
        // Reads the planner's tag, so it waits on the planner.
        .when_some(prepare, |job, prepare| {
            job.add_needs(prepare.name().to_owned())
        });

    NamedJob::new(PUBLISH_JOB, job)
}

/// Cleans up a run that failed or fell short before reaching a store:
/// discards the drafted record always, best-effort deletes the objects this
/// run uploaded, and deletes the tag only when this run minted it — see the
/// design's "Withdrawal reads the point of no return" decision.
///
/// Gated on the negation of the completeness count, ORed with `failure()`:
/// a skipped build already leaves its target unrecorded, which the negated
/// count alone catches, but `failure()` is kept explicit so a failure
/// elsewhere in the job's dependency chain — `create-release`, or
/// [`publish`] itself — still triggers cleanup. Never past the point of no
/// return: once [`publish`] has marked `submitted`, a store already has the
/// build. An explicit `if:` is required either way: the job's own `needs`
/// may have failed, and GitHub's default implicit gate would skip this job
/// exactly when it is needed.
pub fn withdraw_release(channel: Channel, prepare: Option<&PrepareJob>) -> NamedJob {
    let tag = prepare.map(|prepare| &prepare.tag);
    let completeness = completeness_expr(channel);
    let expr = format!(
        "({tag}) && (failure() || !({completeness})) && \
         needs.{PUBLISH_JOB}.outputs.submitted != 'true'",
        tag = Channel::tag_condition(tag).0,
    );

    let job = Job::default()
        .name("Withdraw the release")
        .permissions(Permissions::default().contents(Level::Write))
        // The object delete below resolves the store credential, so the
        // vault credential comes from the same environment every other
        // store-touching job declares.
        .environment(GhEnvironment::from(Environment::AppRelease))
        .runs_on(runners::LINUX)
        .cond(Expression::new(expr))
        // `release-withdraw` is a `cargo xtask`, so it needs the tree.
        .add_step(steps::checkout());
    let job = with_devenv(job)
        .add_step(steps::discard_release(tag))
        .add_step(steps::withdraw_from_store(tag))
        .when_some(prepare, |job, prepare| {
            job.add_step(steps::delete_minted_tag(&prepare.tag, &prepare.tag_minted))
                .add_needs(prepare.name().to_owned())
        });

    NamedJob::new("withdraw-release", job)
}

/// Fails fast when a dispatch pairs a non-empty `tag` with a `platforms`
/// subset. The combination can never satisfy a channel's required-target
/// count — [`publish_release`]'s gate — so letting it proceed would burn a
/// full build matrix on a run that looks green but quietly never publishes.
/// Only wired for the tag-driven channels: nightly takes no `tag` dispatch
/// input, since it mints its own.
pub fn guard_dispatch(tag: &WorkflowInput, platforms: &WorkflowInput) -> NamedJob {
    let expr = format!(
        "github.event_name == 'workflow_dispatch' && {tag} != '' && {platforms} != 'All'",
        tag = tag.expr(),
        platforms = platforms.expr(),
    );

    let job = Job::default()
        .name("Guard the dispatch inputs")
        .permissions(Permissions::default())
        .runs_on(runners::LINUX)
        .cond(Expression::new(expr))
        .add_step(steps::reject_tag_with_platforms_subset());

    NamedJob::new("guard-dispatch", job)
}

pub fn build(
    channel: Channel,
    context: &WorkflowContext,
    platform: Platform,
    prepare: Option<&PrepareJob>,
) -> NamedJob {
    // Absent off nightly, where the tag's own semver suffices.
    let build_code = prepare.and_then(|prepare| build_code(prepare, platform));
    let version = prepare.and_then(|prepare| app_version(prepare, platform));
    let tag = prepare.map(|prepare| &prepare.tag);

    // The mobile lanes need `fastlane`, `xcodegen` and the nushell the `_build`
    // recipes are written in; the desktop ones do not.
    let envs: &[MiseEnv] = if platform.is_mobile() {
        &[MiseEnv::BuildMobile]
    } else {
        &[MiseEnv::BuildDesktop]
    };

    // Everything the build would otherwise fetch: restored before its consumer,
    // saved after it, keyed by the manifest that pins it.
    let caches = steps::cached_dirs(platform, &context.android_ndk);
    let cache = |name: &str| {
        caches
            .iter()
            .find(|dir| dir.name == name)
            .unwrap_or_else(|| panic!("no cached directory named {name} for {platform}"))
    };

    // Built before the job it joins, so its output can be declared on the
    // job once the step itself has moved in.
    let record_target = steps::record_target(platform);
    let target = StepOutput::new_trusted(&record_target, "target");

    let job = mise_env(Job::default(), envs)
        .permissions(Permissions::default().contents(Level::Write))
        .name(format!("Build {platform}"))
        // Builds, signs, and publishes, so the vault credential comes from the
        // release environment — for the signing scope the build resolves and
        // the cache scope the store steps resolve alike.
        .environment(GhEnvironment::from(Environment::AppRelease))
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
            job.add_step(steps::setup_java(&context.android_jdk))
                .add_step(steps::setup_android_sdk())
        })
        .when(platform.is_linux(), |job| {
            // Tauri's Linux backend is WebKitGTK, absent from the runner image.
            job.add_step(steps::install_linux_deps())
        })
        // mise's own install cache is the one third-party cache in this job:
        // the credential resolver is itself a mise tool, so nothing from the
        // bucket can be restored before mise has run. GitHub's default write
        // rules keep low-trust triggers out of it, and `mise.lock` pins what
        // it holds, so it is sound on the same terms as the snapshots below.
        .add_step(steps::setup_mise(true.into(), &context.mise_version))
        .when(platform.is_android(), |job| {
            // `setup-android` installs the SDK but not an NDK. The snapshot
            // restores the version devenv pins; `sdkmanager` then finds it in
            // place and downloads nothing.
            job.add_step(steps::cache_restore(cache("gradle"), Tools::Mise))
                .add_step(steps::cache_restore(cache("android-ndk"), Tools::Mise))
                .add_step(steps::install_ndk(&context.android_ndk))
        })
        // Every pnpm in the job, the build's own included, must agree on the
        // store before its snapshot is restored — see `steps::pin_pnpm_store`.
        .add_step(steps::pin_pnpm_store())
        .add_step(steps::cache_restore(cache("pnpm-store"), Tools::Mise))
        .add_step(steps::pnpm_install())
        .add_step(steps::cache_save(cache("pnpm-store"), Tools::Mise))
        .add_step(steps::cache_restore(cache("cargo-registry"), Tools::Mise))
        // The server holds the store credential for the job; the build that
        // follows runs with none and reaches the store only through it.
        .add_step(steps::sccache_start(Tools::Mise))
        .add_step(
            steps::with_sccache(steps::build(channel, platform))
                .when_some(build_code, |step, code| {
                    step.add_env((BUILD_CODE_ENV, code.to_string()))
                })
                .when_some(version, |step, version| {
                    step.add_env((VERSION_ENV, version.to_string()))
                })
                .when(platform.is_android(), |step| {
                    step.add_env((
                        "NDK_HOME",
                        format!("{ANDROID_HOME}/ndk/{ndk}", ndk = context.android_ndk),
                    ))
                }),
        )
        .add_step(steps::sccache_stats(Tools::Mise))
        .add_step(steps::cache_save(cache("cargo-registry"), Tools::Mise))
        .when(platform.is_android(), |job| {
            job.add_step(steps::cache_save(cache("gradle"), Tools::Mise))
                .add_step(steps::cache_save(cache("android-ndk"), Tools::Mise))
        })
        .add_step(steps::upload_artifact(channel, platform))
        // Staged, not published: the publishing job uploads and submits once
        // every build has recorded in.
        .add_step(steps::stage_release(platform, tag))
        .add_step(steps::upload_staged(platform, tag))
        .when(platform.is_ios(), |job| {
            job.add_step(steps::upload_app_store_info(tag))
        })
        .add_step(record_target)
        .outputs([target.clone().as_output()])
        // Reads the planner's outputs, so it waits on the planner.
        .when_some(prepare, |job, prepare| {
            job.add_needs(prepare.name().to_owned())
        });

    NamedJob::new(build_job_name(platform), job)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};
    use strum::IntoEnumIterator as _;

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

    /// The names injected here have to be the names the build scripts read;
    /// every consumer falls back silently when a variable is absent.
    #[test]
    fn the_build_scripts_read_the_variables_we_inject() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let read = |consumer: &str| {
            fs::read_to_string(root.join(consumer))
                .unwrap_or_else(|error| panic!("{consumer}: {error}"))
        };
        for consumer in [
            "src-tauri/gen/android/app/build.gradle.kts",
            "tooling/bin/xtauri",
        ] {
            let source = read(consumer);
            assert!(source.contains(BUILD_CODE_ENV), "{consumer}");
            assert!(source.contains(VERSION_ENV), "{consumer}");
        }
    }

    /// Only mobile builds stamp the version: the desktop bundles take theirs
    /// from the crate, and the stores are what reject the fallbacks.
    #[test]
    fn only_mobile_builds_receive_the_version() {
        let prepare = prepare(&WorkflowContext::fixture());

        for platform in [Platform::Android, Platform::iOS] {
            assert_eq!(
                app_version(&prepare, platform).map(JobOutput::expr),
                Some(prepare.version.expr()),
                "{platform}"
            );
        }
        for platform in [Platform::macOS, Platform::Linux, Platform::Windows] {
            assert!(app_version(&prepare, platform).is_none(), "{platform}");
        }
    }

    /// Each build records its own slug; the build job's `outputs:` has to
    /// expose it for `publish_release` to read back through `needs`.
    #[test]
    fn a_build_records_its_target_as_a_job_output() {
        let named = build(
            Channel::Stable,
            &WorkflowContext::fixture(),
            Platform::macOS,
            None,
        );
        let value = serde_json::to_value(&named.job).expect("job serializes");
        assert_eq!(
            value["outputs"]["target"],
            "${{ steps.record-target.outputs.target }}"
        );
        let steps = value["steps"].as_array().expect("steps");
        let record = steps
            .iter()
            .find(|step| step["id"] == "record-target")
            .expect("record-target step");
        assert_eq!(record["run"], r#"echo "target=macos" >> "$GITHUB_OUTPUT""#);
    }

    /// Completeness is counted against every required target, by name, and
    /// never falls back to inferring it from `failure()`/`cancelled()` — see
    /// the design's "Completeness is counted, not inferred" decision.
    #[test]
    fn publish_counts_every_required_target() {
        let named = publish(Channel::Stable, None);
        let value = serde_json::to_value(&named.job).expect("job serializes");
        let cond = value["if"].as_str().expect("if");

        for platform in Channel::Stable.required_targets() {
            assert!(
                cond.contains(&format!(
                    "needs.build-{slug}.outputs.target == '{slug}'",
                    slug = platform.slug()
                )),
                "{platform}: {cond}"
            );
        }
        assert!(!cond.contains("failure()"), "{cond}");
        assert!(!cond.contains("cancelled()"), "{cond}");
    }

    fn publish_steps() -> Vec<serde_json::Value> {
        let named = publish(Channel::Stable, None);
        let value = serde_json::to_value(&named.job).expect("job serializes");
        assert_eq!(value["environment"]["name"], "App Release");
        value["steps"].as_array().expect("steps").clone()
    }

    fn step<'a>(steps: &'a [serde_json::Value], id: &str) -> &'a serde_json::Value {
        steps
            .iter()
            .find(|step| step["id"] == id)
            .unwrap_or_else(|| panic!("no {id} step"))
    }

    /// Ordered by retractability: everything that only touches the runner,
    /// then the store, the record, the mark, the submissions, and the
    /// manifest last — nothing between them.
    #[test]
    fn the_publish_steps_run_in_order_of_retractability() {
        let order = [
            "download-staged",
            "download-app-store-info",
            "prepare-listing",
            "upload-to-store",
            "publish-release",
            "mark-submitted",
            "submit-android",
            "submit-ios",
            "write-manifest",
        ];
        let steps = publish_steps();
        let ids = steps
            .iter()
            .filter_map(|step| step["id"].as_str())
            .skip_while(|id| *id != order[0])
            .collect::<Vec<_>>();
        assert_eq!(ids, order);
    }

    /// The upload reads the merged staged tree, and the submissions take
    /// their packages from it — the iOS one with its plist.
    #[test]
    fn the_publish_job_works_from_the_staged_tree() {
        let steps = publish_steps();
        assert!(
            step(&steps, "upload-to-store")["run"]
                .as_str()
                .unwrap()
                .contains("cargo xtask release-upload --from release --tag")
        );
        assert_eq!(step(&steps, "download-staged")["with"]["path"], "release");
        assert_eq!(
            step(&steps, "download-staged")["with"]["merge-multiple"],
            "true"
        );
        let ios = step(&steps, "submit-ios")["run"].as_str().unwrap();
        assert!(
            ios.contains(
                "xfastlane publish ios stable --from release --app-store-info app-store/AppStoreInfo.plist"
            ),
            "{ios}"
        );
        let android = step(&steps, "submit-android")["run"].as_str().unwrap();
        assert!(
            android.contains("xfastlane publish android stable --from release"),
            "{android}"
        );
    }

    /// Each step resolves only its own credential: the store on the upload
    /// and the manifest (derived in-process), each platform's release scope
    /// on its own submission, `GITHUB_TOKEN` on the record, and nothing on
    /// the preparation or the mark.
    #[test]
    fn every_publish_step_resolves_only_its_own_scope() {
        let steps = publish_steps();
        let vault = |id: &str| {
            step(&steps, id)["env"]
                .get("OP_SERVICE_ACCOUNT_TOKEN")
                .is_some()
        };
        let github = |id: &str| step(&steps, id)["env"].get("GH_TOKEN").is_some();
        let run = |id: &str| step(&steps, id)["run"].as_str().unwrap().to_owned();

        for id in ["prepare-listing", "mark-submitted"] {
            assert!(!vault(id) && !github(id), "{id} resolves nothing");
        }
        for id in ["upload-to-store", "write-manifest"] {
            assert!(vault(id) && !github(id), "{id}");
            assert!(
                !run(id).contains("--scope"),
                "{id}: the store scope is in-process"
            );
        }
        assert!(github("publish-release") && !vault("publish-release"));
        for (id, scope, other) in [
            ("submit-android", "android", "ios"),
            ("submit-ios", "ios", "android"),
        ] {
            assert!(vault(id) && !github(id), "{id}");
            assert!(run(id).contains("--profile release"), "{id}");
            assert!(run(id).contains(&format!("--scope {scope} ")), "{id}");
            assert!(!run(id).contains(&format!("--scope {other}")), "{id}");
        }
    }

    /// fastlane's own variables — Transporter's and the staged package's —
    /// are set inside `xfastlane`, never by the workflow.
    #[test]
    fn no_publish_step_names_a_fastlane_variable() {
        let rendered = serde_json::to_string(&publish_steps()).unwrap();
        for name in ["FASTLANE_", "WAYPOINTS_STORE_PACKAGE"] {
            assert!(!rendered.contains(name), "{name}");
        }
    }

    /// `prepare` reads the release notes from git-cliff, which needs the
    /// history back to the previous tag.
    #[test]
    fn the_publish_job_checks_out_full_history_and_tags() {
        let steps = publish_steps();
        let checkout = step(&steps, "checkout");
        assert_eq!(checkout["with"]["fetch-depth"], "0");
        assert_eq!(checkout["with"]["fetch-tags"], "true");
    }

    /// The point of no return is a job output, which the withdraw gate can
    /// read across jobs.
    #[test]
    fn the_publish_job_exposes_its_point_of_no_return() {
        let named = publish(Channel::Stable, None);
        let value = serde_json::to_value(&named.job).expect("job serializes");
        assert_eq!(
            value["outputs"]["submitted"],
            "${{ steps.mark-submitted.outputs.submitted }}"
        );
    }

    /// Builds stage and hand off; they never reach the store or a store's
    /// submission themselves.
    #[test]
    fn no_build_job_publishes() {
        for platform in Platform::iter() {
            let named = build(Channel::Stable, &WorkflowContext::fixture(), platform, None);
            let value = serde_json::to_value(&named.job).expect("job serializes");
            let steps = value["steps"].as_array().expect("steps");
            for step in steps {
                let run = step["run"].as_str().unwrap_or("");
                for publishing in ["release-upload", "release-manifest", "xfastlane"] {
                    assert!(!run.contains(publishing), "{platform}: {run}");
                }
            }

            let stage = steps
                .iter()
                .find(|step| step["id"] == "stage-release")
                .unwrap_or_else(|| panic!("{platform}: no stage-release step"));
            assert!(stage["env"].get("OP_SERVICE_ACCOUNT_TOKEN").is_none());
            assert!(stage["if"].as_str().unwrap().contains("!= ''"));
            let upload = steps
                .iter()
                .find(|step| step["id"] == "upload-staged")
                .unwrap_or_else(|| panic!("{platform}: no upload-staged step"));
            assert_eq!(
                upload["with"]["name"],
                format!("staged-{}", platform.slug())
            );
            assert_eq!(
                steps
                    .iter()
                    .any(|step| step["id"] == "upload-app-store-info"),
                platform.is_ios(),
                "{platform}"
            );
        }
    }

    /// The record carries no bytes: a release build's artifacts reach only
    /// the store, never a GitHub release asset.
    #[test]
    fn the_build_job_has_no_github_release_asset_upload() {
        let named = build(
            Channel::Stable,
            &WorkflowContext::fixture(),
            Platform::macOS,
            None,
        );
        let value = serde_json::to_value(&named.job).expect("job serializes");
        let steps = value["steps"].as_array().expect("steps");
        assert!(
            steps.iter().all(|step| !step["run"]
                .as_str()
                .unwrap_or("")
                .contains("gh release upload")),
            "{steps:?}"
        );
    }

    /// Only nightly mints its own tag, so only nightly's withdraw can
    /// delete it — preview's and stable's tags predate the run, and
    /// deleting them would move a ref out from under whoever already
    /// fetched it. See the design's "A run deletes only a tag it minted"
    /// decision.
    #[test]
    fn nightlys_withdraw_deletes_the_tag_and_the_others_do_not() {
        let prepare = prepare(&WorkflowContext::fixture());
        let nightly = withdraw_release(Channel::Nightly, Some(&prepare));
        let value = serde_json::to_value(&nightly.job).expect("job serializes");
        let steps = value["steps"].as_array().expect("steps");
        let tag_delete = steps
            .iter()
            .find(|step| step["id"] == "delete-minted-tag")
            .expect("nightly withdraw deletes the minted tag");
        assert_eq!(
            tag_delete["if"].as_str().unwrap(),
            format!("{} == 'true'", prepare.tag_minted.expr())
        );

        for channel in [Channel::Preview, Channel::Stable] {
            let named = withdraw_release(channel, None);
            let value = serde_json::to_value(&named.job).expect("job serializes");
            let steps = value["steps"].as_array().expect("steps");
            assert!(
                steps.iter().all(|step| step["id"] != "delete-minted-tag"),
                "{channel}: should not delete a tag it did not mint"
            );
        }
    }

    /// The withdraw gate is the negation of the publish gate's completeness,
    /// ORed with `failure()` — an explicit `if:`, since the job's own
    /// `needs` may have failed and GitHub's default implicit gate would
    /// skip it exactly when it is needed.
    #[test]
    fn withdraw_gates_on_failure_or_shortfall() {
        let named = withdraw_release(Channel::Stable, None);
        let value = serde_json::to_value(&named.job).expect("job serializes");
        let cond = value["if"].as_str().expect("if");
        assert!(cond.contains("failure()"), "{cond}");
        // A top-level conjunct, so neither a failure nor a shortfall can
        // override it: past the mark, a store already has the build.
        assert!(
            cond.ends_with(") && needs.publish.outputs.submitted != 'true'"),
            "{cond}"
        );
        for platform in Channel::Stable.required_targets() {
            assert!(
                cond.contains(&format!(
                    "needs.build-{slug}.outputs.target == '{slug}'",
                    slug = platform.slug()
                )),
                "{platform}: {cond}"
            );
        }
    }

    /// Every channel's withdraw discards the drafted record and best-effort
    /// cleans up the store, regardless of whether it may also delete a tag.
    #[test]
    fn every_channel_s_withdraw_discards_the_record_and_the_store() {
        for channel in [Channel::Nightly, Channel::Preview, Channel::Stable] {
            let prepare = channel
                .is_nightly()
                .then(|| prepare(&WorkflowContext::fixture()));
            let named = withdraw_release(channel, prepare.as_ref());
            let value = serde_json::to_value(&named.job).expect("job serializes");
            assert_eq!(value["environment"]["name"], "App Release");
            let steps = value["steps"].as_array().expect("steps");
            assert!(
                steps.iter().any(|step| step["id"] == "discard-release"),
                "{channel}: no discard-release step"
            );
            assert!(
                steps.iter().any(|step| step["id"] == "withdraw-from-store"),
                "{channel}: no withdraw-from-store step"
            );
        }
    }
}
