// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Every job the workflows are assembled from. A workflow module names jobs
//! and wires their `needs:`.

use super::{
    DescribeJob, GuardJob, MiseEnv, ReleaseRun, TestSuite, WorkflowContext, WorkflowInput, actions,
    actions::{FreeDisk, MiseSetup},
    helpers,
    helpers::{ANDROID_HOME, Environment, GITHUB_TOKEN},
    runners, steps,
    steps::Tools,
};
use crate::{
    Platform,
    constants::{MAIN, WORKFLOWS_DIR},
    templates,
};
use gh_workflow::{
    Environment as GhEnvironment, Expression, FluentBuilder, Job, JobOutput, Level, NamedJob,
    Permissions, Step, StepOutput,
};
use strum::IntoEnumIterator as _;

/// The environment variable a build reads its store build number from. One
/// name; [`build_code`] decides which value fills it.
pub const BUILD_CODE_ENV: &str = "WAYPOINTS_BUILD_CODE";

/// The environment variable a mobile build reads the app version from. Tauri
/// takes it as its config `version`, which the manifest leaves unset, so
/// without this the stores see Gradle's and Xcode's fallbacks (`1.0`,
/// `1.0.0`). iOS shows the release and the date as `0.1.0 (2026.10.10)`;
/// the Gradle project renders Android's `versionName` the same way.
pub const VERSION_ENV: &str = "WAYPOINTS_VERSION";

/// What a build job reads from the job that described its run: whom to wait
/// on, when to run, what to check out, and what to stamp and name.
pub struct BuildInputs {
    pub needs: String,
    pub cond: Expression,
    /// The ref to check out.
    pub reference: String,
    /// The flavor the build enables.
    pub flavor: String,
    /// The channel the staged artifact is named for.
    pub channel: String,
    /// The tag the staged files are named for.
    pub tag: String,
    pub version: JobOutput,
    pub version_code: JobOutput,
    pub bundle_version: JobOutput,
}

impl BuildInputs {
    /// From the builder's `describe`: everything is what it read from
    /// `reference`, and the gate is the caller's platforms alone.
    pub fn described(
        describe: &DescribeJob,
        reference: &WorkflowInput,
        platform: Platform,
    ) -> Self {
        Self {
            needs: describe.name().to_owned(),
            cond: Expression::new(helpers::selected(platform)),
            reference: reference.to_string(),
            flavor: describe.flavor.to_string(),
            channel: describe.channel.to_string(),
            tag: describe.tag.to_string(),
            version: describe.version.clone(),
            version_code: describe.version_code.clone(),
            bundle_version: describe.bundle_version.clone(),
        }
    }
}

/// The output carrying `platform`'s build number, or `None` where the
/// platform's store has no such concept. Android's `versionCode` is a single
/// integer; iOS's `CFBundleVersion` is dotted.
fn build_code(inputs: &BuildInputs, platform: Platform) -> Option<&JobOutput> {
    match platform {
        Platform::Android => Some(&inputs.version_code),
        Platform::iOS => Some(&inputs.bundle_version),
        _ => None,
    }
}

/// The output carrying the app version, for the platforms that stamp it: the
/// mobile ones, whose stores reject Tauri's fallbacks. The desktop bundles
/// take their version from the crate.
fn app_version(inputs: &BuildInputs, platform: Platform) -> Option<&JobOutput> {
    platform.is_mobile().then_some(&inputs.version)
}

/// Names the mise config environments a job installs. Additive: each one adds
/// its `[tools]` to the base, so a job declares what it does and gets exactly
/// that. Replaced a deny-list of tool specs, which named what to skip and so
/// had to be re-derived every time the tool set moved — and silently no-opped
/// when a spec it named had been renamed out from under it.
fn mise_env(job: Job, envs: &[MiseEnv]) -> Job {
    job.add_env(MiseEnv::select(envs))
}

/// Reads the ref into the build's identity — the tag, the version, the
/// channel, the flavor, and the store build codes — as `release describe`
/// publishes them. Everything downstream stamps what this read; nothing else
/// is told the ref.
pub fn describe(context: &WorkflowContext, reference: &WorkflowInput) -> DescribeJob {
    let registry = steps::cargo_registry(steps::CARGO_HOME);
    // Through the environment: a ref is free text as far as the shell is
    // concerned.
    let describe = steps::cargo_xtask("release describe", &[("--ref", Some("$REF"))])
        .id("describe")
        .add_env(("REF", reference.to_string()));
    let output = |name| StepOutput::new_trusted(&describe, name);
    let tag = output("tag");
    let version = output("version");
    let channel = output("channel");
    let flavor = output("flavor");
    let version_code = output("version_code");
    let bundle_version = output("bundle_version");

    let job = NamedJob::new(
        "describe",
        mise_env(Job::default(), &[MiseEnv::Xtask])
            .name("Describe the build")
            .permissions(Permissions::default().contents(Level::Read))
            // Compiles through the CI cache, so the vault credential comes
            // from the automation environment.
            .environment(GhEnvironment::from(Environment::Ci))
            .runs_on(runners::LINUX)
            // The ref's tree, and the tags, so a tag resolves whatever the
            // caller named.
            .add_step(
                steps::checkout()
                    .add_with(("ref", reference.to_string()))
                    .add_with(("fetch-depth", 0))
                    .add_with(("fetch-tags", true)),
            )
            .add_step(actions::setup_mise(
                MiseSetup::default(),
                &context.mise_version,
            ))
            // Compiling xtask is the whole job; the registry snapshot and the
            // compiler cache the build jobs fill answer most of it.
            .add_step(steps::cache_restore(&registry, Tools::Mise))
            .add_step(steps::sccache_start(Tools::Mise))
            .add_step(steps::with_sccache(describe))
            .add_step(steps::sccache_stats(Tools::Mise))
            .add_step(steps::cache_save(&registry, Tools::Mise))
            .outputs([
                tag.clone().as_output(),
                version.clone().as_output(),
                channel.clone().as_output(),
                flavor.clone().as_output(),
                version_code.clone().as_output(),
                bundle_version.clone().as_output(),
            ]),
    );

    DescribeJob::new(
        tag.as_job_output(&job),
        version.as_job_output(&job),
        channel.as_job_output(&job),
        flavor.as_job_output(&job),
        version_code.as_job_output(&job),
        bundle_version.as_job_output(&job),
        job,
    )
}

/// Runs the suite on one platform, as two steps: the plan resolves the
/// filterset, and the run is skipped when it comes back empty. The tree is
/// `reference`'s where one is given — the builder's ref — and the run's own
/// otherwise.
pub fn test(
    suite: TestSuite,
    platform: Platform,
    context: &WorkflowContext,
    reference: Option<&str>,
) -> NamedJob {
    // The command publishes its own step output, so no shell redirection and
    // nothing spelled differently for PowerShell on Windows.
    let plan = steps::cargo_xtask("test-plan", &[(suite.flag(), None)]).id("test-plan");
    let filterset = StepOutput::new_trusted(&plan, "filterset");
    let flavor = StepOutput::new_trusted(&plan, "flavor");

    // Both suites run the same two commands, so both want the same environment
    // — unlike the deny-list this replaced, where the changed-suite job was
    // left with the full tool set because narrowing it meant naming tools.
    let job = mise_env(Job::default(), &[MiseEnv::Test])
        .name(format!("Test {platform}"))
        .permissions(Permissions::default().contents(Level::Read))
        .runs_on(platform.runner())
        // The change-scoped plan diffs against the merge base, which a shallow
        // clone does not contain.
        .add_step(
            steps::checkout()
                .when_some(reference, |step, reference| {
                    step.add_with(("ref", reference))
                })
                .add_with(("fetch-depth", 0)),
        )
        // The disk goes first on Linux: the workspace's debug test build
        // exhausts a stock image on its own, and this job never builds for
        // Android. mise's own cache stays off for the release suite, which
        // runs cold alongside the build jobs that publish. The compiled-
        // dependency cache applies to both suites: a test job publishes
        // nothing, and compiling the workspace is what makes these runs
        // 20–33 minutes.
        .add_step(actions::setup_mise(
            MiseSetup {
                cache: matches!(suite, TestSuite::Changed),
                free_disk: if platform.is_linux() {
                    FreeDisk::All
                } else {
                    FreeDisk::None
                },
                rust_cache: true,
            },
            &context.mise_version,
        ))
        // Tauri links WebKitGTK on Linux, so even a library test build needs it.
        .when(platform.is_linux(), |job| {
            job.add_step(steps::install_linux_deps())
        })
        .add_step(plan)
        .add_step(
            steps::cargo_nextest(suite, &flavor.to_string())
                .with_filter_expr(&filterset.to_string()),
        );

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
        .add_step(actions::setup_mise(
            MiseSetup::default(),
            &context.mise_version,
        ))
        .add_step(steps::zizmor());

    NamedJob::new("zizmor", job)
}

/// Holds the tree to the licensing policy, and runs the dependency license check
/// that otherwise runs only before a push. On mise until the project has a
/// cachix: entering devenv costs far more than the check itself.
pub fn license(context: &WorkflowContext) -> NamedJob {
    let job = mise_env(Job::default(), &[MiseEnv::Devtools])
        .name("Check licensing")
        .permissions(Permissions::default().contents(Level::Read))
        .runs_on(runners::LINUX)
        .add_step(steps::checkout())
        .add_step(actions::setup_mise(
            MiseSetup {
                rust_cache: true,
                ..MiseSetup::default()
            },
            &context.mise_version,
        ))
        .add_step(steps::cargo_xtask("check licenses", &[]))
        .add_step(steps::cargo_deny_licenses());

    NamedJob::new("license", job)
}

/// Guards the release run's inputs and reads its one flag. The refusals are
/// expressions over the event and the dispatch's inputs: a real dispatch
/// that names no tag, a real dispatch pairing a tag with a platforms subset
/// — which can never satisfy the required-target count, so the run would
/// build without ever publishing — and `build` off without `dry_run`, since a
/// tag with no artifacts behind it must not be published. The flag is whether
/// the run rehearses, read off the event once here so every later gate reads
/// one output: a dispatch rehearses unless it says otherwise, and a push is
/// the real thing.
///
/// The conditions sit on the steps, not the job: the builder `need`s this
/// job, and a job skipped by its own `if` skips everything that needs it.
pub fn guard(
    tag: &WorkflowInput,
    platforms: &WorkflowInput,
    dry_run: &WorkflowInput,
    build: &WorkflowInput,
) -> GuardJob {
    let dispatched = "github.event_name == 'workflow_dispatch'";
    let real = format!("{dispatched} && !{}", dry_run.expr());

    let flag =
        steps::read_rehearsal_flag(&format!("${{{{ {dispatched} && {} }}}}", dry_run.expr()));
    let rehearsing = StepOutput::new_trusted(&flag, "dry_run");

    let job = NamedJob::new(
        "guard",
        Job::default()
            .name("Guard the run")
            .permissions(Permissions::default())
            .runs_on(runners::LINUX)
            .add_step(flag)
            .add_step(
                steps::refuse(
                    "reject-untagged-run",
                    "Refuse a real run without a tag",
                    "A real run names the tag it publishes. Dispatch again with `tag`, or as a dry run.",
                )
                .if_condition(Expression::new(format!("{real} && {} == ''", tag.expr()))),
            )
            .add_step(
                steps::refuse(
                    "reject-contradiction",
                    "Refuse a tag with a platforms subset",
                    "A release tag cannot be combined with a platforms subset — a subset can never satisfy every required target, so the run would build without ever publishing. Dispatch again with platforms left at All, or as a dry run.",
                )
                .if_condition(Expression::new(format!(
                    "{real} && {} != '' && {} != 'All'",
                    tag.expr(),
                    platforms.expr()
                ))),
            )
            .add_step(
                steps::refuse(
                    "reject-no-build",
                    "Refuse no build without a dry run",
                    "A run that does not build can only be a dry run — a tag with no artifacts behind it must not be published. Dispatch again with `dry_run`, or with `build`.",
                )
                .if_condition(Expression::new(format!(
                    "{dispatched} && !{} && !{}",
                    build.expr(),
                    dry_run.expr()
                ))),
            )
            .outputs([rehearsing.clone().as_output()]),
    );

    GuardJob::new(rehearsing.as_job_output(&job), job)
}

/// Calls the builder on `reference`'s tree for `platforms`, as one job. Its
/// permissions are the union of the builder's jobs', and its secrets are
/// inherited: the vault credential the builds resolve is an environment
/// secret, which reaches a called workflow only this way and through the
/// called job's own environment — the one finding zizmor cannot see past,
/// suppressed on the job.
pub fn call_builder(
    reference: &str,
    platforms: &str,
    guard: &GuardJob,
    cond: Expression,
) -> NamedJob {
    let mut job = Job::default()
        .name("Build")
        .permissions(Permissions::default().contents(Level::Write))
        .add_needs(guard.name().to_owned())
        .cond(cond)
        .uses(format!("$/{WORKFLOWS_DIR}/build-distributables.yaml"))
        .add_with(("ref", reference))
        .add_with(("platforms", platforms))
        .inherit_secrets()
        .zizmor_ignore(
            "secrets-inherit",
            "the vault credential the builds resolve is an environment secret, which reaches a called workflow only by inheritance and the called job's own environment",
        );
    // A calling job runs nowhere itself.
    job.runs_on = None;

    NamedJob::new("build", job)
}

/// Drafts the release every build uploads into, with git-cliff notes.
///
/// The tag guards sit on the steps, not the job: the publishing job `need`s
/// this one, and a skipped job would drag it down with it — a dry run lets
/// every step no-op, and renders its notes into the run's artifact instead.
pub fn create_release(run: &ReleaseRun, context: &WorkflowContext) -> NamedJob {
    let tag = &run.tag;
    let dry_run = &run.dry_run;
    let publishing = || helpers::publishing(tag, dry_run);

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
        // Reads the builder's identity and the guard's flag, so it waits on
        // both.
        .add_needs(run.build.clone())
        .add_needs(run.guard.clone())
        // git-cliff walks from the release tag back to the tag before it.
        .add_step(
            steps::checkout()
                .add_with(("ref", helpers::release_ref()))
                .add_with(("fetch-depth", 0))
                .add_with(("fetch-tags", true))
                .if_condition(publishing()),
        )
        .add_step(
            actions::setup_mise(MiseSetup::default(), &context.mise_version)
                .if_condition(publishing()),
        )
        .add_step(steps::release_notes(tag, dry_run))
        .add_step(steps::create_release(
            tag,
            dry_run,
            &format!(
                "${{{{ {} != 'stable' && '--prerelease' || '' }}}}",
                run.channel.expr()
            ),
        ));

    NamedJob::new("create-release", job)
}

/// The job id a platform's build runs under. Shared between [`build`], which
/// names the job, and the builder's outputs, which read its recorded target
/// back — so the convention is stated once.
pub(crate) fn build_job_name(platform: Platform) -> String {
    format!("build-{}", platform.slug())
}

/// The `needs.<build>.outputs.<slug> == '<slug>'` clauses, joined with `&&`,
/// that assert every required target has recorded in through the builder.
/// Shared between the publish gate and the withdraw gate, which assert the
/// fact and its negation respectively. Every channel requires every platform
/// today, so the count is the full set.
fn completeness_expr(run: &ReleaseRun) -> String {
    Platform::iter()
        .map(|platform| format!("{} == '{}'", run.target(platform), platform.slug()))
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
    job.add_step(actions::setup_nix(&["devenv"]))
}

/// Publishes a complete run, in order of retractability: the store upload,
/// the public record, the point of no return, both store submissions, and
/// the manifest last. One job, so the order is a step order. On a dry run
/// every one of those stands down, and the counterparts after them render
/// the notes, the manifest, and the index entry into one directory, which
/// is uploaded as the run's artifact.
///
/// Completeness is counted, not inferred from the absence of failure: a
/// target skipped by a `platforms` subset blocks the gate exactly as one that
/// failed, because neither records the output the builder hands back — see
/// the design's "The counted gate stays" decision.
pub fn publish(run: &ReleaseRun) -> NamedJob {
    let tag = &run.tag;
    let dry_run = &run.dry_run;
    let channel = run.channel.to_string();
    let flavor = run.flavor.to_string();
    let completeness = completeness_expr(run);
    let expr = format!(
        "({completeness}) && ({tag})",
        tag = helpers::tag_condition(tag).0
    );

    let mark = steps::mark_submitted(dry_run);
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
        // The release's tree, with the history and tags the dry notes'
        // git-cliff walks back to the previous tag.
        .add_step(
            steps::checkout()
                .add_with(("ref", helpers::release_ref()))
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
        .add_step(steps::download_staged(&channel))
        .add_step(steps::prepare_listing(&flavor))
        .add_step(steps::with_sccache(steps::upload_to_store(tag, dry_run)))
        .add_step(steps::publish_release(tag, dry_run))
        .add_step(mark)
        .add_step(steps::submit(Platform::Android, &flavor, dry_run))
        .add_step(steps::submit(Platform::iOS, &flavor, dry_run))
        // Last: the manifest is what the updater and the download links
        // read, and it is idempotent, so a failure here is repaired by
        // running it again.
        .add_step(steps::with_sccache(steps::write_manifest(tag, dry_run)))
        // A dry run's counterparts, after the writes they stand in for.
        .add_step(steps::dry_notes(tag, dry_run))
        .add_step(steps::with_sccache(steps::dry_manifest(tag, dry_run)))
        .add_step(steps::with_sccache(steps::dry_index(tag, dry_run)))
        .add_step(steps::upload_dry_run(&channel, dry_run))
        .outputs([submitted.as_output()])
        // Reads the builder's identity and the guard's flag, so it waits on
        // both.
        .add_needs(run.build.clone())
        .add_needs(run.guard.clone());

    NamedJob::new(PUBLISH_JOB, job)
}

/// Becomes the record's merge commit body, so it says what the merge commit
/// is.
pub(crate) fn record_body() -> String {
    templates::must("pull-request.index", tera::Context::new())
}

/// Records the published tag in the release index, after the manifest: the
/// entry exists only for a tag whose release was committed, and a run that
/// withdrew never reaches this job. On a real run only; a dry run renders its
/// entry into the run's artifact from [`publish`]. `main` is protected, so
/// the entry lands through a pull request that merges itself once its checks
/// pass; the job does not wait for it.
pub fn record(run: &ReleaseRun, context: &WorkflowContext) -> NamedJob {
    // Inside the devenv shell, as `check-generated` runs: the task drives
    // `nix`, `git`, and `cargo`, and the shell is where they come from at the
    // versions the tree pins. The tag reaches the command through the
    // environment.
    let record = Step::new("cargo ci-task release record")
        .run("devenv shell -- cargo ci-task release record --tag \"$TAG\"")
        .shell("bash")
        .id("record")
        .add_env(("TAG", helpers::tag(&run.tag)))
        .add_env((
            "SECRETSPEC_REASON",
            "release record records a published tag in nix/releases; no secret values are read",
        ));

    // The task publishes these only when it wrote.
    let wrote = || {
        Expression::new(format!(
            "{} != ''",
            StepOutput::new_trusted(&record, "title").expr()
        ))
    };
    let credentials = steps::botswain_credentials(Tools::Devenv).if_condition(wrote());
    let app_token = steps::app_token(&[("contents", "write"), ("pull-requests", "write")])
        .if_condition(wrote());
    let token = StepOutput::new_trusted(&app_token, "token");
    let pull_request = steps::land_write(&record, &token, "index-pull-request", &record_body())
        .if_condition(wrote());
    // Landed and left: nothing downstream waits on the entry.
    let auto_merge = steps::merge_without_waiting(
        &StepOutput::new_trusted(&pull_request, "pull-request-number"),
        &token,
        Tools::Devenv,
    );

    // The devenv shell leaves `CARGO_HOME` unset, so cargo keeps its registry
    // at its default; the snapshot has to land there.
    let registry = steps::cargo_registry("${HOME}/.cargo");

    let job = Job::default()
        .name("Record the release")
        // Only what `checkout` needs: the entry lands with the App token, which
        // carries its own scope.
        .permissions(Permissions::default().contents(Level::Read))
        // Compiles through the CI cache and authenticates as the App, so the
        // vault credential comes from the automation environment.
        .environment(GhEnvironment::from(Environment::Ci))
        .runs_on(runners::LINUX)
        .cond(helpers::not_rehearsing(&run.dry_run))
        // On the branch the entry lands on, whatever ref started the run: the
        // pull-request step commits against what is checked out. The tag
        // resolves to its commit here; nothing walks history.
        .add_step(
            steps::checkout()
                .add_with(("ref", MAIN))
                .add_with(("fetch-tags", true)),
        );
    let job = with_devenv(job)
        // The task is a `cargo ci-task` compile before it is anything else; the
        // build jobs' registry snapshot and compiler cache pay for most of it.
        .add_step(steps::cache_restore(&registry, Tools::Devenv))
        .add_step(steps::sccache_start(Tools::Devenv))
        .add_step(steps::with_sccache(record))
        .add_step(steps::sccache_stats(Tools::Devenv))
        .add_step(steps::cache_save(&registry, Tools::Devenv))
        // Minted after the compile, which is most of this job's time.
        .add_step(credentials)
        .add_step(app_token)
        .add_step(pull_request)
        .add_step(auto_merge)
        // Waits on the publication it records, and on the guard whose flag
        // it reads.
        .add_needs(PUBLISH_JOB.to_owned())
        .add_needs(run.guard.clone());

    NamedJob::new("record", job)
}

/// Cleans up a run that failed or fell short before reaching a store, and
/// undoes the bump that minted its tag: discards the drafted record,
/// best-effort deletes the objects this run uploaded, and — when the run was
/// the tag's own push, so the tag was minted for this publication minutes
/// before and nothing has recorded it — deletes the tag and reverts the write
/// it was minted from through a pull request the bot opens and merges, so no
/// branch declares a version no tag names. A dispatched re-publication of an
/// existing tag leaves both the tag and its write, since that release may
/// already be out. Never on a dry run, which created nothing to withdraw.
///
/// Gated on the negation of the completeness count, ORed with `failure()`:
/// a skipped build already leaves its target unrecorded, which the negated
/// count alone catches, but `failure()` is kept explicit so a failure
/// elsewhere in the job's dependency chain — the builder, `create-release`,
/// or [`publish`] itself — still triggers cleanup. Never past the point of
/// no return: once [`publish`] has marked `submitted`, a store already has
/// the build. An explicit `if:` is required either way: the job's own
/// `needs` may have failed, and GitHub's default implicit gate would skip
/// this job exactly when it is needed.
pub fn withdraw_release(run: &ReleaseRun) -> NamedJob {
    // The tag the run was started by, not the builder's reading of it: the
    // builder may be what failed.
    let tag = helpers::triggering_tag();
    let completeness = completeness_expr(run);
    let expr = format!(
        "(({triggering}) != '') && ({real}) && (failure() || !({completeness})) && \
         needs.{PUBLISH_JOB}.outputs.submitted != 'true'",
        triggering = helpers::triggering_tag_expr(),
        real = helpers::not_rehearsing(&run.dry_run).0,
    );

    let pushed = helpers::pushed;
    let undo = steps::undo(&tag).if_condition(pushed());
    let credentials = steps::botswain_credentials(Tools::Devenv).if_condition(pushed());
    let app_token = steps::app_token(&[("contents", "write"), ("pull-requests", "write")])
        .if_condition(pushed());
    let token = StepOutput::new_trusted(&app_token, "token");
    let revert =
        steps::land_write_from_file(&undo, &token, "revert-pull-request").if_condition(pushed());
    // Waited for: the shared concurrency group holds the cycle until the
    // tree is consistent again.
    let merge = steps::merge(
        &StepOutput::new_trusted(&revert, "pull-request-number"),
        &token,
        Tools::Devenv,
    )
    .id("merge-revert");

    let job = Job::default()
        .name("Withdraw the release")
        .permissions(Permissions::default().contents(Level::Write))
        // The object delete below resolves the store credential, so the
        // vault credential comes from the same environment every other
        // store-touching job declares.
        .environment(GhEnvironment::from(Environment::AppRelease))
        .runs_on(runners::LINUX)
        .cond(Expression::new(expr))
        // `release undo` reads the write's merge and its parent, and checks
        // the branch it reverts on out afresh.
        .add_step(
            steps::checkout()
                .add_with(("fetch-depth", 0))
                .add_with(("fetch-tags", true)),
        );
    let job = with_devenv(job)
        .add_step(steps::discard_release(&tag))
        .add_step(steps::withdraw_from_store(&tag))
        .add_step(steps::delete_tag(&tag).if_condition(pushed()))
        .add_step(undo)
        .add_step(credentials)
        .add_step(app_token)
        .add_step(revert)
        .add_step(merge)
        .add_needs(run.build.clone())
        .add_needs(run.guard.clone());

    NamedJob::new("withdraw-release", job)
}

/// Builds one platform from `inputs`: checks the tree out, builds and signs
/// under the flavor, renames the distributables for the tag, and hands them
/// on as one artifact named for the channel, recording its target.
pub fn build(context: &WorkflowContext, platform: Platform, inputs: &BuildInputs) -> NamedJob {
    // Absent off nightly, where the tag's own semver suffices.
    let build_code = build_code(inputs, platform);
    let version = app_version(inputs, platform);

    // The mobile lanes need `xcodegen` and the nushell the `_build` recipes are
    // written in; the desktop ones do not.
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
        // Builds and signs, so the vault credential comes from the release
        // environment — for the signing scope the build resolves and the
        // cache scope the snapshot steps resolve alike.
        .environment(GhEnvironment::from(Environment::AppRelease))
        .runs_on(platform.runner())
        .cond(inputs.cond.clone())
        // The build stamps each library group's version from its tags on
        // `main` (`git describe`), which a shallow, tagless checkout cannot
        // name — it would fall back to the manifests' bare versions.
        .add_step(
            steps::checkout()
                .add_with(("ref", inputs.reference.clone()))
                .add_with(("fetch-depth", 0))
                .add_with(("fetch-tags", true)),
        )
        // The Ubuntu runners build Linux and Android, and both are large
        // enough to be worth the reclaim — before `setup-android`, so nothing
        // is removed after being installed, and Android keeps its runtime,
        // since `setup-android` configures the image's SDK rather than
        // fetching one. mise's own install cache is the one third-party cache
        // in this job: the credential resolver is itself a mise tool, so
        // nothing from the bucket can be restored before mise has run.
        // GitHub's default write rules keep low-trust triggers out of it, and
        // `mise.lock` pins what it holds, so it is sound on the same terms as
        // the snapshots below.
        .add_step(actions::setup_mise(
            MiseSetup {
                cache: true,
                free_disk: match platform {
                    Platform::Linux => FreeDisk::All,
                    Platform::Android => FreeDisk::KeepAndroid,
                    _ => FreeDisk::None,
                },
                rust_cache: false,
            },
            &context.mise_version,
        ))
        .when(platform.is_android(), |job| {
            job.add_step(steps::setup_java(&context.android_jdk))
                .add_step(steps::setup_android_sdk())
        })
        .when(platform.is_linux(), |job| {
            // Tauri's Linux backend is WebKitGTK, absent from the runner image.
            job.add_step(steps::install_linux_deps())
        })
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
            steps::with_sccache(steps::build(platform, &inputs.flavor))
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
        // Staged, not published: renamed flat and handed on, so the
        // publishing job uploads and submits once every build has recorded
        // in. Every run stages, dry runs included, so a dry run exercises the
        // rename too.
        .add_step(steps::stage_release(platform, inputs.tag.clone()))
        .add_step(steps::upload_staged(&inputs.channel, platform))
        .add_step(record_target)
        .outputs([target.clone().as_output()])
        // Reads the describing job's outputs, so it waits on it.
        .add_needs(inputs.needs.clone());

    NamedJob::new(build_job_name(platform), job)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};

    fn reference() -> WorkflowInput {
        WorkflowInput::string("ref", None)
    }

    /// A build's inputs, as the builder's `describe` hands them.
    fn inputs(platform: Platform) -> BuildInputs {
        let reference = reference();
        BuildInputs::described(
            &describe(&WorkflowContext::fixture(), &reference),
            &reference,
            platform,
        )
    }

    fn built(platform: Platform) -> NamedJob {
        build(&WorkflowContext::fixture(), platform, &inputs(platform))
    }

    /// The release run's handles, as `release.rs` wires them.
    fn run() -> ReleaseRun {
        let tag = WorkflowInput::string("tag", Some(String::new()));
        let platforms = WorkflowInput::string("platforms", Some("All".to_owned()));
        let dry_run = WorkflowInput::bool("dry_run", Some(true));
        let build = WorkflowInput::bool("build", Some(true));
        let guard = guard(&tag, &platforms, &dry_run, &build);
        let builder = call_builder("ref", "All", &guard, Expression::new("true"));
        ReleaseRun::new(&guard, &builder)
    }

    /// Swapping these would hand Play a value it cannot parse.
    #[test]
    fn each_platform_draws_its_own_build_code() {
        let inputs = inputs(Platform::Linux);

        assert_eq!(
            build_code(&inputs, Platform::Android).map(JobOutput::expr),
            Some(inputs.version_code.expr())
        );
        assert_eq!(
            build_code(&inputs, Platform::iOS).map(JobOutput::expr),
            Some(inputs.bundle_version.expr())
        );

        for platform in [Platform::macOS, Platform::Linux, Platform::Windows] {
            assert!(build_code(&inputs, platform).is_none(), "{platform}");
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
        let inputs = inputs(Platform::Linux);

        for platform in [Platform::Android, Platform::iOS] {
            assert_eq!(
                app_version(&inputs, platform).map(JobOutput::expr),
                Some(inputs.version.expr()),
                "{platform}"
            );
        }
        for platform in [Platform::macOS, Platform::Linux, Platform::Windows] {
            assert!(app_version(&inputs, platform).is_none(), "{platform}");
        }
    }

    /// `describe` republishes exactly the outputs `release describe` writes,
    /// under the names the builds and the caller read.
    #[test]
    fn describe_republishes_the_description_s_fields() {
        let named: NamedJob = describe(&WorkflowContext::fixture(), &reference()).into();
        let mut outputs: Vec<&str> = named
            .job
            .outputs
            .as_ref()
            .expect("outputs")
            .keys()
            .map(String::as_str)
            .collect();
        outputs.sort_unstable();
        assert_eq!(
            outputs,
            [
                "bundle_version",
                "channel",
                "flavor",
                "tag",
                "version",
                "version_code"
            ]
        );
    }

    /// Each build records its own slug; the build job's `outputs:` has to
    /// expose it for the builder's outputs to read back.
    #[test]
    fn a_build_records_its_target_as_a_job_output() {
        let named = built(Platform::macOS);
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

    /// Scenario _The caller counts the builder's targets_: completeness is
    /// counted against every target the builder hands back, by name, and
    /// never falls back to inferring it from `failure()`/`cancelled()`.
    #[test]
    fn publish_counts_every_target_the_builder_hands_back() {
        let named = publish(&run());
        let value = serde_json::to_value(&named.job).expect("job serializes");
        let cond = value["if"].as_str().expect("if");

        for platform in Platform::iter() {
            assert!(
                cond.contains(&format!(
                    "needs.build.outputs.{slug} == '{slug}'",
                    slug = platform.slug()
                )),
                "{platform}: {cond}"
            );
        }
        assert!(!cond.contains("failure()"), "{cond}");
        assert!(!cond.contains("cancelled()"), "{cond}");
    }

    /// A series' lockfile pins each library group to a tag of this
    /// repository, which is public: cargo fetches the pins with nothing
    /// configured, so no build step carries a git credential or a fetch
    /// override (library-minting design D7).
    #[test]
    fn a_build_fetches_a_series_pin_with_nothing_configured() {
        for platform in Platform::iter() {
            let job = serde_json::to_string(&built(platform).job).expect("job serializes");
            for forbidden in [
                "CARGO_NET_GIT_FETCH_WITH_CLI",
                "GIT_CONFIG",
                "GIT_ASKPASS",
                "insteadOf",
                "git-credential",
            ] {
                assert!(!job.contains(forbidden), "{platform}: {forbidden}");
            }
        }
    }

    /// The build stamps each library group's version from its tags, so every
    /// build job checks out the history and the tags a `git describe` reads.
    #[test]
    fn every_build_checks_out_full_history_and_tags() {
        for platform in Platform::iter() {
            let job = serde_json::to_value(&built(platform).job).expect("job serializes");
            let checkout = job["steps"]
                .as_array()
                .unwrap()
                .iter()
                .find(|step| step["id"] == "checkout")
                .unwrap_or_else(|| panic!("{platform}: no checkout"));
            assert_eq!(checkout["with"]["fetch-depth"], "0", "{platform}");
            assert_eq!(checkout["with"]["fetch-tags"], "true", "{platform}");
        }
    }

    fn publish_steps() -> Vec<serde_json::Value> {
        let named = publish(&run());
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

    /// The real writes an ordinary run makes, in the order it makes them.
    const REAL_WRITES: [&str; 6] = [
        "upload-to-store",
        "publish-release",
        "mark-submitted",
        "submit-android",
        "submit-ios",
        "write-manifest",
    ];

    /// What a dry run renders instead, in order, ending in the one upload.
    const DRY_COUNTERPARTS: [&str; 4] =
        ["dry-notes", "dry-manifest", "dry-index", "upload-dry-run"];

    /// Ordered by retractability: everything that only touches the runner,
    /// then the store, the record, the mark, the submissions, and the
    /// manifest last — nothing between them. The dry counterparts follow,
    /// after every write they stand in for; the index entry follows in its
    /// own job, which needs this one.
    #[test]
    fn the_publish_steps_run_in_order_of_retractability() {
        let order: Vec<&str> = ["download-staged", "prepare-listing"]
            .into_iter()
            .chain(REAL_WRITES)
            .chain(DRY_COUNTERPARTS)
            .collect();
        let steps = publish_steps();
        let ids = steps
            .iter()
            .filter_map(|step| step["id"].as_str())
            .skip_while(|id| *id != order[0])
            .collect::<Vec<_>>();
        assert_eq!(ids, order);

        let record = record(&run(), &WorkflowContext::fixture());
        assert!(
            record
                .job
                .needs
                .as_ref()
                .is_some_and(|needs| needs.contains(&PUBLISH_JOB.to_owned())),
            "{:?}",
            record.job.needs
        );
    }

    /// Scenario _The entry follows the manifest_ and _A withdrawn tag is not
    /// recorded_: the record runs after the publishing job, on a real run
    /// only, checks `main` out with the tags, and lands its entry with the
    /// App token without waiting; a run that withdrew never reaches it.
    #[test]
    fn the_record_follows_the_manifest_on_a_real_run() {
        let run = run();
        let named = record(&run, &WorkflowContext::fixture());
        let value = serde_json::to_value(&named.job).expect("job serializes");
        assert_eq!(value["if"], format!("{} != 'true'", run.dry_run.expr()));
        assert_eq!(value["environment"]["name"], "CI");
        assert_eq!(value["permissions"]["contents"], "read");

        let steps = value["steps"].as_array().expect("steps");
        let checkout = step(steps, "checkout");
        assert_eq!(checkout["with"]["ref"], "main");
        assert_eq!(checkout["with"]["fetch-tags"], "true");

        let record = step(steps, "record");
        assert_eq!(
            record["run"],
            "devenv shell -- cargo ci-task release record --tag \"$TAG\""
        );
        assert_eq!(record["env"]["TAG"], run.tag.to_string());

        let wrote = "steps.record.outputs.title != ''";
        for id in ["app-token", "index-pull-request"] {
            assert_eq!(step(steps, id)["if"], wrote, "{id}");
        }
        assert_eq!(
            step(steps, "index-pull-request")["with"]["token"],
            "${{ steps.app-token.outputs.token }}"
        );
        let merge = steps
            .iter()
            .find(|step| {
                step["run"]
                    .as_str()
                    .is_some_and(|run| run.contains("release merge"))
            })
            .expect("the merge");
        assert!(
            merge["run"].as_str().unwrap().ends_with("--no-wait"),
            "{merge}"
        );
    }

    /// Every real write stands down on a dry run, and every counterpart runs
    /// only then — in the publish job and in the draft job alike.
    #[test]
    fn every_real_write_stands_down_on_a_dry_run_and_its_counterpart_runs_only_then() {
        let run = run();
        let real = format!("{} != 'true'", run.dry_run.expr());
        let dry = format!("{} == 'true'", run.dry_run.expr());

        let named = publish(&run);
        let value = serde_json::to_value(&named.job).expect("job serializes");
        let steps = value["steps"].as_array().expect("steps");
        for id in REAL_WRITES {
            let cond = step(steps, id)["if"].as_str().unwrap_or("");
            assert!(cond.contains(&real), "{id}: {cond}");
        }
        for id in DRY_COUNTERPARTS {
            assert_eq!(step(steps, id)["if"], dry, "{id}");
        }

        let named = create_release(&run, &WorkflowContext::fixture());
        let value = serde_json::to_value(&named.job).expect("job serializes");
        let steps = value["steps"].as_array().expect("steps");
        for id in ["gen-release-notes", "draft-release"] {
            let cond = step(steps, id)["if"].as_str().unwrap_or("");
            assert!(cond.contains(&real), "{id}: {cond}");
        }
        assert_eq!(
            step(steps, "draft-release")["env"]["PRERELEASE"],
            format!(
                "${{{{ {} != 'stable' && '--prerelease' || '' }}}}",
                run.channel.expr()
            )
        );
    }

    /// The dry counterparts render into one directory and leave one artifact,
    /// named for the channel the builder read and kept a week; the index
    /// entry is rendered for the commit the run is on.
    #[test]
    fn a_dry_run_leaves_one_artifact() {
        let steps = publish_steps();
        for id in ["dry-notes", "dry-manifest", "dry-index"] {
            let run = step(&steps, id)["run"].as_str().unwrap();
            assert!(run.contains(steps::DRY_RUN_DIR), "{id}: {run}");
        }
        assert!(
            step(&steps, "dry-index")["run"]
                .as_str()
                .unwrap()
                .contains(r#"--rev "${REV}""#)
        );
        assert_eq!(step(&steps, "dry-index")["env"]["REV"], "${{ github.sha }}");
        let upload = step(&steps, "upload-dry-run");
        assert_eq!(
            upload["with"]["name"],
            "dry-run-${{ needs.build.outputs.channel }}"
        );
        assert_eq!(upload["with"]["path"], steps::DRY_RUN_ARTIFACT_PATH);
        assert_eq!(upload["with"]["retention-days"], "7");
    }

    /// The upload reads the merged staged directory, and the submissions
    /// take their packages from it, under the flavor the builder read.
    #[test]
    fn the_publish_job_works_from_the_staged_tree() {
        let steps = publish_steps();
        assert!(
            step(&steps, "upload-to-store")["run"]
                .as_str()
                .unwrap()
                .contains("cargo ci-task release distributables upload --from release --tag")
        );
        assert_eq!(step(&steps, "download-staged")["with"]["path"], "release");
        assert_eq!(
            step(&steps, "download-staged")["with"]["merge-multiple"],
            "true"
        );
        // The plist is staged beside the ipa, where fastlane looks for it.
        for (id, lane) in [("submit-ios", "ios"), ("submit-android", "android")] {
            let submit = step(&steps, id);
            let run = submit["run"].as_str().unwrap();
            assert!(
                run.contains(&format!(
                    r#"xfastlane publish {lane} "${{FLAVOR}}" --from release"#
                )),
                "{run}"
            );
            assert_eq!(
                submit["env"]["FLAVOR"], "${{ needs.build.outputs.flavor }}",
                "{id}"
            );
        }
    }

    /// Each step resolves only its own credential: the store on the upload
    /// and the manifest (derived in-process), each platform's release scope
    /// on its own submission, `GITHUB_TOKEN` on the record, and nothing on
    /// the preparation, the mark, or any dry counterpart.
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
        for id in DRY_COUNTERPARTS {
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

    /// The dry notes read git-cliff over the release's tree, which needs the
    /// history back to the previous tag.
    #[test]
    fn the_publish_job_checks_out_the_release_with_full_history_and_tags() {
        let steps = publish_steps();
        let checkout = step(&steps, "checkout");
        assert_eq!(checkout["with"]["ref"], helpers::release_ref());
        assert_eq!(checkout["with"]["fetch-depth"], "0");
        assert_eq!(checkout["with"]["fetch-tags"], "true");
    }

    /// The point of no return is a job output, which the withdraw gate can
    /// read across jobs.
    #[test]
    fn the_publish_job_exposes_its_point_of_no_return() {
        let named = publish(&run());
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
            let named = built(platform);
            let value = serde_json::to_value(&named.job).expect("job serializes");
            let steps = value["steps"].as_array().expect("steps");
            for step in steps {
                let run = step["run"].as_str().unwrap_or("");
                for publishing in [
                    "release distributables upload",
                    "release distributables commit",
                    "xfastlane",
                ] {
                    assert!(!run.contains(publishing), "{platform}: {run}");
                }
            }

            // Every run stages, dry runs included, so a changed export fails
            // the build that produced it.
            let stage = step(steps, "stage-release");
            assert!(stage["env"].get("OP_SERVICE_ACCOUNT_TOKEN").is_none());
            assert!(stage.get("if").is_none(), "{platform}: {stage}");

            // One artifact per build, and it is the flat staged directory.
            let uploads = steps
                .iter()
                .filter(|step| {
                    step["uses"]
                        .as_str()
                        .is_some_and(|uses| uses.starts_with("actions/upload-artifact@"))
                })
                .collect::<Vec<_>>();
            assert_eq!(uploads.len(), 1, "{platform}");
            assert_eq!(uploads[0]["with"]["path"], "release", "{platform}");
            assert_eq!(
                uploads[0]["with"]["name"],
                format!(
                    "waypoints-{}-${{{{ needs.describe.outputs.channel }}}}",
                    platform.slug()
                )
            );
        }
    }

    /// The staged files are named for the tag `describe` read — the ref's,
    /// or the one its tree would be tagged as — and `release distributables
    /// rename` still takes the app's own version when handed nothing.
    #[test]
    fn the_staged_files_are_named_for_the_run_s_tag() {
        let named = built(Platform::Linux);
        let value = serde_json::to_value(&named.job).expect("job serializes");
        let steps = value["steps"].as_array().expect("steps");
        let stage = step(steps, "stage-release");
        assert_eq!(stage["env"]["TAG"], "${{ needs.describe.outputs.tag }}");
        assert!(
            stage["run"]
                .as_str()
                .unwrap()
                .ends_with(r#"${TAG:+--tag "${TAG}"}"#)
        );
    }

    /// The publishing job merges exactly the channel's builds, by the name
    /// the builder read.
    #[test]
    fn the_publish_job_merges_its_own_channel_s_artifacts() {
        let steps = publish_steps();
        let download = step(&steps, "download-staged");
        assert_eq!(
            download["with"]["pattern"],
            "waypoints-*-${{ needs.build.outputs.channel }}"
        );
    }

    /// The record carries no bytes: a release build's artifacts reach only
    /// the store, never a GitHub release asset.
    #[test]
    fn the_build_job_has_no_github_release_asset_upload() {
        let named = built(Platform::macOS);
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

    /// The calling job carries only what a call may: `uses`, `with`,
    /// `secrets`, `permissions`, `needs`, and `if` — no runner, steps, or
    /// environment of its own — and inherits the secrets under the one
    /// suppression that inheritance earns.
    #[test]
    fn the_calling_job_carries_only_what_a_call_may() {
        let run = run();
        let guard = guard(
            &WorkflowInput::string("tag", Some(String::new())),
            &WorkflowInput::string("platforms", Some("All".to_owned())),
            &WorkflowInput::bool("dry_run", Some(true)),
            &WorkflowInput::bool("build", Some(true)),
        );
        let named = call_builder("ref", "All", &guard, Expression::new("true"));
        let value = serde_json::to_value(&named.job).expect("job serializes");
        let mut keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "if",
                "name",
                "needs",
                "permissions",
                "secrets",
                "uses",
                "with"
            ]
        );
        assert_eq!(
            value["uses"],
            "$/.github/workflows/build-distributables.yaml"
        );
        assert_eq!(value["secrets"], "inherit");
        assert_eq!(value["with"]["ref"], "ref");
        assert_eq!(value["needs"], serde_json::json!([run.guard]));
        let rules: Vec<&str> = named
            .job
            .lint_ignores
            .iter()
            .map(|ignore| ignore.rule.as_str())
            .collect();
        assert_eq!(rules, ["secrets-inherit"]);
    }

    /// The guard reads the rehearsal flag off the event — a dispatch
    /// rehearses unless it says otherwise, a push is real — and refuses the
    /// three contradictions on their steps, never on the job.
    #[test]
    fn the_guard_reads_the_flag_and_refuses_on_its_steps() {
        let guard = guard(
            &WorkflowInput::string("tag", Some(String::new())),
            &WorkflowInput::string("platforms", Some("All".to_owned())),
            &WorkflowInput::bool("dry_run", Some(true)),
            &WorkflowInput::bool("build", Some(true)),
        );
        let named: NamedJob = guard.into();
        let value = serde_json::to_value(&named.job).expect("job serializes");
        assert!(value.get("if").is_none(), "{value}");
        assert_eq!(
            value["outputs"]["dry_run"],
            "${{ steps.rehearsal.outputs.dry_run }}"
        );
        let steps = value["steps"].as_array().expect("steps");
        assert_eq!(
            step(steps, "rehearsal")["env"]["DRY_RUN"],
            "${{ github.event_name == 'workflow_dispatch' && inputs.dry_run }}"
        );
        for (id, needle) in [
            ("reject-untagged-run", "inputs.tag == ''"),
            ("reject-contradiction", "inputs.platforms != 'All'"),
            ("reject-no-build", "!inputs.build && !inputs.dry_run"),
        ] {
            let cond = step(steps, id)["if"].as_str().unwrap();
            assert!(
                cond.starts_with("github.event_name == 'workflow_dispatch'"),
                "{id}: {cond}"
            );
            assert!(cond.contains(needle), "{id}: {cond}");
        }
    }

    /// Scenarios _A pushed tag is withdrawn_, _The write is reverted_, and
    /// _A re-published tag survives_: the drafted record and the store
    /// objects go on every withdrawal; the tag, and the bump that minted it,
    /// go only when the run was the tag's own push — the revert as a pull
    /// request the bot opens, signs, and waits on.
    #[test]
    fn a_pushed_tag_is_withdrawn_with_its_write_and_a_dispatched_one_is_not() {
        let named = withdraw_release(&run());
        let value = serde_json::to_value(&named.job).expect("job serializes");
        assert_eq!(value["environment"]["name"], "App Release");
        let steps = value["steps"].as_array().expect("steps");

        let triggering = helpers::triggering_tag();
        for id in ["discard-release", "withdraw-from-store"] {
            let step = step(steps, id);
            assert!(step.get("if").is_none(), "{id}: {step}");
            assert_eq!(step["env"]["TAG"], triggering, "{id}");
        }
        for id in ["delete-tag", "undo", "app-token", "revert-pull-request"] {
            assert_eq!(step(steps, id)["if"], "github.event_name == 'push'", "{id}");
        }
        let credentials = steps
            .iter()
            .find(|step| step["name"] == "Resolve the botswain secrets")
            .expect("the credentials step");
        assert_eq!(credentials["if"], "github.event_name == 'push'");

        let undo = step(steps, "undo");
        assert_eq!(
            undo["run"],
            r#"devenv shell -- cargo ci-task release undo --tag "${TAG}" --commit "${COMMIT}""#
        );
        assert_eq!(undo["env"]["TAG"], triggering);
        assert_eq!(undo["env"]["COMMIT"], "${{ github.sha }}");
        let revert = step(steps, "revert-pull-request");
        assert_eq!(
            revert["with"]["body-path"],
            "${{ steps.undo.outputs.body_path }}"
        );
        assert_eq!(revert["with"]["branch"], "${{ steps.undo.outputs.head }}");
        assert_eq!(revert["with"]["sign-commits"], "true");
        // The merge stands on the pull request's number, which the push-only
        // step before it leaves empty on a dispatch.
        let merge = step(steps, "merge-revert");
        assert_eq!(
            merge["if"],
            "steps.revert-pull-request.outputs.pull-request-number != ''"
        );
        assert!(
            !merge["run"].as_str().unwrap().contains("--no-wait"),
            "the withdrawal waits for the revert to land"
        );
        assert_eq!(
            merge["env"]["PULL_REQUEST"],
            "${{ steps.revert-pull-request.outputs.pull-request-number }}"
        );
    }

    /// The withdraw gate is the negation of the publish gate's completeness,
    /// ORed with `failure()` — an explicit `if:`, since the job's own
    /// `needs` may have failed and GitHub's default implicit gate would
    /// skip it exactly when it is needed — on a run started with a tag, and
    /// never on a dry run, which created nothing to withdraw.
    #[test]
    fn withdraw_gates_on_failure_or_shortfall() {
        let run = run();
        let named = withdraw_release(&run);
        let value = serde_json::to_value(&named.job).expect("job serializes");
        let cond = value["if"].as_str().expect("if");
        assert!(cond.contains("failure()"), "{cond}");
        assert!(
            cond.starts_with(&format!("(({}) != '')", helpers::triggering_tag_expr())),
            "{cond}"
        );
        assert!(
            cond.contains(&format!("({} != 'true')", run.dry_run.expr())),
            "{cond}"
        );
        // A top-level conjunct, so neither a failure nor a shortfall can
        // override it: past the mark, a store already has the build.
        assert!(
            cond.ends_with(") && needs.publish.outputs.submitted != 'true'"),
            "{cond}"
        );
        for platform in Platform::iter() {
            assert!(
                cond.contains(&format!(
                    "needs.build.outputs.{slug} == '{slug}'",
                    slug = platform.slug()
                )),
                "{platform}: {cond}"
            );
        }
    }
}
