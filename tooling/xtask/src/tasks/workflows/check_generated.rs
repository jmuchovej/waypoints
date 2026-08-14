//! Asserts that everything we generate is in lockstep with the source it is
//! generated from — devenv's `files."..."`/`tasks."mise:lock"` outputs, and the
//! workflows emitted by this very crate.

use gh_workflow::{Event, Job, Level, Permissions, PullRequest, Push, Workflow, WorkflowDispatch};

use super::{WorkflowContext, helpers::one_run_per_ref, runners, steps};

/// Sources whose change can invalidate something we generate. A change to any
/// of these has to re-run the check.
const SOURCES: &[&str] = &[
    "devenv.nix",
    "devenv.yaml",
    "devenv.lock",
    "tooling/xtask/**",
];

/// Generated outputs, listed so that hand-editing one is caught. devenv's own
/// come from the context at generation time.
const OUTPUTS: &[&str] = &[".github/workflows/**"];

pub fn workflow(context: &WorkflowContext) -> Workflow {
    let paths: Vec<String> = SOURCES
        .iter()
        .chain(OUTPUTS)
        .map(|path| (*path).to_owned())
        .chain(context.devenv_files.iter().cloned())
        .collect();

    let check_generated = Job::new("check generated files match their sources")
        .runs_on(runners::LINUX)
        .add_step(steps::checkout())
        .add_step(steps::install_nix())
        .add_step(steps::cachix("devenv"))
        // The recipe shells into devenv, so both exist before the shell does.
        .add_step(steps::nix_profile_install(&["devenv", "just"]))
        .add_step(steps::just("check-generated").name("Check generated files"));

    Workflow::new("assert generated files remain in lockstep")
        .on(Event::default()
            .pull_request(
                paths
                    .iter()
                    .fold(PullRequest::default(), |pr, p| pr.add_path(p)),
            )
            .push(
                paths
                    .iter()
                    .fold(Push::default(), |push, p| push.add_path(p))
                    .add_branch("main"),
            )
            .workflow_dispatch(WorkflowDispatch::default()))
        .permissions(Permissions::default().contents(Level::Read))
        .concurrency(one_run_per_ref("generated"))
        .add_job("check-generated", check_generated)
}
