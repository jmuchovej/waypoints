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

use super::{MiseEnv, WorkflowContext, WorkflowInput, helpers, runners, steps};
use gh_workflow::{Event, Job, Level, Permissions, Workflow, WorkflowDispatch};

pub fn workflow(context: &WorkflowContext) -> Workflow {
    let transition = WorkflowInput::choices(
        "transition",
        crate::tasks::release_plan::transitions(),
        None::<String>,
    )
    .description("Which release-cycle transition to perform");

    let plan = Job::new("plan the transition")
        // A `cargo xtask` run and nothing more.
        .add_env(MiseEnv::select(&[MiseEnv::Xtask]))
        // Write: the planner creates branch and tag refs.
        .permissions(Permissions::default().contents(Level::Write))
        .runs_on(runners::LINUX)
        // Tags without history: the planner reads them to find the last
        // release, and walks commits only from there.
        .add_step(steps::checkout().add_with(("fetch-tags", true)))
        .add_step(steps::setup_mise(false.into(), &context.mise_version))
        .add_step(
            steps::cargo_xtask("release-plan", &[("${{ inputs.transition }}", None)])
                .id("release-plan")
                .add_env(("GH_TOKEN", helpers::GITHUB_TOKEN)),
        );

    helpers::named_workflow("mint the refs for a release-cycle transition")
        .on(Event::default().workflow_dispatch(
            WorkflowDispatch::default().inputs([(transition.name.into(), transition.input())]),
        ))
        // A half-applied transition leaves the branch and its tag disagreeing.
        .concurrency(helpers::serialize("release-plan"))
        .add_job("release-plan", plan)
}
