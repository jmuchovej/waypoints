//! Asserts that everything we generate is in lockstep with the source it is
//! generated from — devenv's `files."..."`/`tasks."mise:lock"` outputs, and the
//! workflows emitted by this very crate.

use gh_workflow::{Event, Job, Level, Permissions, PullRequest, Push, Workflow, WorkflowDispatch};

use super::{
    WorkflowContext,
    utils::{runners, steps, vars},
};

/// Sources whose change can invalidate something we generate. A change to any
/// of these has to re-run the check.
const SOURCES: &[&str] = &[
    "devenv.nix",
    "devenv.yaml",
    "devenv.lock",
    "tooling/xtask/**",
];

/// Generated outputs, listed so that hand-editing one is caught. devenv's own
/// outputs come from [`WorkflowContext::devenv_files`] rather than a copy of
/// the list in `devenv.nix`.
const OUTPUTS: &[&str] = &[".github/workflows/**"];

pub fn workflow(context: &WorkflowContext) -> Workflow {
    let paths: Vec<String> = SOURCES
        .iter()
        .chain(OUTPUTS)
        .map(|path| (*path).to_owned())
        .chain(context.devenv_files.iter().cloned())
        .collect();

    Workflow::new("assert generated files remain in lockstep")
        .on(Event::default()
            .pull_request(add_paths(PullRequest::default(), &paths))
            .push(add_paths(Push::default(), &paths).branches(vec!["main".to_string()]))
            .workflow_dispatch(WorkflowDispatch::default()))
        .permissions(Permissions::default().contents(Level::Read))
        .concurrency(vars::one_run_per_ref("generated"))
        .add_job("check-generated", check_generated())
}

fn check_generated() -> Job {
    Job::new("check generated files match their sources")
        .runs_on(runners::LINUX)
        .add_step(steps::checkout())
        .add_step(steps::install_nix())
        .add_step(steps::cachix("devenv"))
        // `just check-generated` shells into devenv, so devenv and just both
        // have to exist before the shell does.
        .add_step(steps::nix_profile_install(&["devenv", "just"]))
        .add_step(steps::just("check-generated").name("Check generated files"))
}

trait WithPaths {
    fn add_path(self, path: impl Into<String>) -> Self;
}

impl WithPaths for PullRequest {
    fn add_path(self, path: impl Into<String>) -> Self {
        PullRequest::add_path(self, path)
    }
}

impl WithPaths for Push {
    fn add_path(self, path: impl Into<String>) -> Self {
        Push::add_path(self, path)
    }
}

/// The hand-written YAML shared one path list between `push` and
/// `pull_request` through a YAML anchor. Generation has no anchors, so the list
/// is shared here instead and spelled out twice on the way out.
fn add_paths<E: WithPaths>(event: E, paths: &[String]) -> E {
    paths
        .iter()
        .fold(event, |event, path| event.add_path(path.clone()))
}
