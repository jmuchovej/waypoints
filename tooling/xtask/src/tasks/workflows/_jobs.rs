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
    Permissions, StepOutput,
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

/// Publishes the draft once every required target has recorded its success.
///
/// Completeness is counted, not inferred from the absence of failure: a
/// target skipped by a `platforms` subset dispatch blocks the gate exactly
/// as one that failed, because neither writes the output this reads — see
/// the design's "Completeness is counted, not inferred" decision. The tag
/// guard skips build-only runs and nightlies that built nothing.
pub fn publish_release(channel: Channel, prepare: Option<&PrepareJob>) -> NamedJob {
    let tag = prepare.map(|prepare| &prepare.tag);
    let completeness = completeness_expr(channel);
    let expr = format!(
        "({completeness}) && ({tag})",
        tag = Channel::tag_condition(tag).0
    );

    let job = Job::default()
        .name("Publish the release")
        .permissions(Permissions::default().contents(Level::Write))
        // The manifest write below resolves the store credential, so the
        // vault credential comes from the same environment every other
        // store-touching job declares.
        .environment(GhEnvironment::from(Environment::AppRelease))
        .runs_on(runners::LINUX)
        .cond(Expression::new(expr))
        .add_step(steps::publish_release(tag))
        // Last: the manifest is the run's true final write, ordered after
        // the record itself goes public so the reverse — a published
        // record naming artifacts the manifest has not confirmed — cannot
        // happen.
        .add_step(steps::write_manifest(tag))
        // Reads the planner's tag, so it waits on the planner.
        .when_some(prepare, |job, prepare| {
            job.add_needs(prepare.name().to_owned())
        });

    NamedJob::new("publish-release", job)
}

/// Cleans up a run that failed or fell short of [`publish_release`]'s
/// completeness: discards the drafted record always, best-effort deletes
/// the objects this run uploaded, and deletes the tag only when this run
/// minted it — see the design's "A withdrawal deletes the keys it wrote,
/// best-effort" and "A run deletes only a tag it minted" decisions.
///
/// Gated on the negation of the completeness count, ORed with `failure()`:
/// a skipped build already leaves its target unrecorded, which the negated
/// count alone catches, but `failure()` is kept explicit so a failure
/// earlier in the job's own dependency chain — `create-release`, say, which
/// a shortfall in target recording would not by itself prove — still
/// triggers cleanup. An explicit `if:` is required either way: the job's
/// own `needs` may have failed, and GitHub's default implicit gate would
/// skip this job exactly when it is needed.
pub fn withdraw_release(channel: Channel, prepare: Option<&PrepareJob>) -> NamedJob {
    let tag = prepare.map(|prepare| &prepare.tag);
    let completeness = completeness_expr(channel);
    let expr = format!(
        "({tag}) && (failure() || !({completeness}))",
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
        .add_step(steps::upload_to_store(platform, tag))
        .when(platform.is_mobile(), |job| {
            job.add_step(steps::fastlane(platform, channel, tag))
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
    fn publish_release_counts_every_required_target() {
        let named = publish_release(Channel::Stable, None);
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

    /// The manifest write sits in the same job as the record publish, after
    /// it — the run's true last write — and resolves its own store
    /// credential, so the job declares the release environment the same
    /// way every other store-touching job does.
    #[test]
    fn the_manifest_write_follows_the_record_publish_in_the_same_job() {
        let named = publish_release(Channel::Stable, None);
        let value = serde_json::to_value(&named.job).expect("job serializes");

        assert_eq!(value["environment"]["name"], "App Release");

        let steps = value["steps"].as_array().expect("steps");
        let publish_index = steps
            .iter()
            .position(|step| step["id"] == "publish-release")
            .expect("publish-release step");
        let manifest_index = steps
            .iter()
            .position(|step| step["id"] == "write-manifest")
            .expect("write-manifest step");
        assert!(
            manifest_index > publish_index,
            "the manifest write must follow the record publish"
        );

        let manifest = &steps[manifest_index];
        assert_eq!(
            manifest["run"].as_str().unwrap(),
            r#"cargo xtask release-manifest --tag "${TAG}""#
        );
        assert!(manifest["env"].get("OP_SERVICE_ACCOUNT_TOKEN").is_some());
    }

    /// The upload step is a single one-line `xtask` dispatch: it resolves
    /// the store credential itself, through secretspec's Rust API, so no
    /// separate minting step exists for it to read a derived credential
    /// back from, and no step in the job exports anything to `$GITHUB_ENV`.
    /// See the design's "The store is reached through opendal in-process,
    /// from one client constructor" decision.
    #[test]
    fn the_upload_step_is_a_one_line_dispatch_resolving_its_own_credential() {
        let named = build(
            Channel::Stable,
            &WorkflowContext::fixture(),
            Platform::macOS,
            None,
        );
        let value = serde_json::to_value(&named.job).expect("job serializes");

        assert_eq!(value["environment"]["name"], "App Release");
        assert!(
            !serde_json::to_string(&value)
                .unwrap()
                .contains("GITHUB_ENV")
        );

        let steps = value["steps"].as_array().expect("steps");
        assert!(
            steps
                .iter()
                .all(|step| step["id"] != "mint-release-credential"),
            "no separate minting step should exist"
        );

        let upload = steps
            .iter()
            .find(|step| step["id"] == "upload-to-store")
            .expect("upload-to-store step");
        assert_eq!(
            upload["run"].as_str().unwrap(),
            r#"cargo xtask release-upload --platform macos --tag "${TAG}""#
        );
        assert!(upload["env"].get("OP_SERVICE_ACCOUNT_TOKEN").is_some());
        for leftover in [
            "R2_SESSION_ACCESS_KEY_ID",
            "R2_SESSION_SECRET_ACCESS_KEY",
            "R2_SESSION_TOKEN",
            "R2_SESSION_ENDPOINT",
        ] {
            assert!(upload["env"].get(leftover).is_none(), "{leftover}");
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
