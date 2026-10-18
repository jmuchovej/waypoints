//! Mints the refs a release cycle moves through.
//!
//! Every transition is a deliberate decision rather than a schedule, so this
//! is dispatch-only. One branch exists per minor version: `cut` opens it as a
//! preview, `advance` takes the next candidate on it, `promote` makes it
//! stable in place, and `patch` fixes it afterwards. Which of those a run
//! performs is the `transition` input; the planner derives everything else
//! from the manifest version and the tags already published.
//!
//! Nightlies are minted by [`super::release`]'s `prepare` job instead — their
//! version is a timestamp, not a claim about content.
//!
//! Refs are minted with the wp-botswain App token rather than `GITHUB_TOKEN`:
//! a tag pushed with the latter raises no `push` event, so neither the
//! channel's release workflow nor the release index would run on it.

use super::{
    MiseEnv, WorkflowContext, WorkflowInput, helpers, helpers::Environment, runners, steps,
    steps::Tools,
};
use gh_workflow::{
    Environment as GhEnvironment, Event, Job, Level, Permissions, StepOutput, Workflow,
    WorkflowDispatch,
};

pub fn workflow(context: &WorkflowContext) -> Workflow {
    let transition = WorkflowInput::choices(
        "transition",
        crate::tasks::release_plan::transitions(),
        None::<String>,
    )
    .description("Which release-cycle transition to perform");

    let app_token = steps::app_token(&[("contents", "write")]);
    let token = StepOutput::new_trusted(&app_token, "token");
    let registry = steps::cargo_registry(steps::CARGO_HOME);

    let plan = Job::new("plan the transition")
        // A `cargo xtask` run and nothing more.
        .add_env(MiseEnv::select(&[MiseEnv::Xtask]))
        // Only what `checkout` needs: the branch, the version commit, and the
        // tag are created with the App token, which carries its own scope.
        .permissions(Permissions::default().contents(Level::Read))
        // Authenticates as the App and compiles through the CI cache, so the
        // vault credential comes from the automation environment.
        .environment(GhEnvironment::from(Environment::Ci))
        .runs_on(runners::LINUX)
        // Tags and the history between them: a cut walks from the last stable
        // release to HEAD with a pathspec that drops the index directory, and
        // in a shallow clone the tip is a root commit that appears to touch
        // every path — see the nightly planner.
        .add_step(
            steps::checkout()
                .add_with(("fetch-depth", 0))
                .add_with(("fetch-tags", true)),
        )
        .add_step(steps::setup_mise(false.into(), &context.mise_version))
        // As with the nightly planner: the cost here is compiling xtask, and
        // the build jobs' registry snapshot and compiler cache answer most of
        // it.
        .add_step(steps::cache_restore(&registry, Tools::Mise))
        .add_step(steps::sccache_start(Tools::Mise))
        .add_step(steps::botswain_credentials())
        .add_step(app_token)
        .add_step(steps::with_sccache(
            steps::cargo_xtask("release-plan", &[("${{ inputs.transition }}", None)])
                .id("release-plan")
                .add_env(("GH_TOKEN", token.to_string())),
        ))
        .add_step(steps::sccache_stats(Tools::Mise))
        .add_step(steps::cache_save(&registry, Tools::Mise));

    helpers::named_workflow("mint the refs for a release-cycle transition")
        .on(Event::default().workflow_dispatch(
            WorkflowDispatch::default().inputs([(transition.name.into(), transition.input())]),
        ))
        // A half-applied transition leaves the branch and its tag disagreeing.
        .concurrency(helpers::serialize("release-plan"))
        .add_job("release-plan", plan)
}
