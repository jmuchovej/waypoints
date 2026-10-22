//! Asserts that everything we generate is in lockstep with the source it is
//! generated from — devenv's `files."..."`/`tasks."mise:lock"` outputs, the
//! TypeScript bindings `cargo xtask typegen` exports from `domain` and
//! `cartograph`, the app icons `cargo xtask icongen` renders from
//! `src-tauri/res/`, and the workflows emitted by this very crate.

use super::{WorkflowContext, helpers, runners, steps};
use gh_workflow::{Event, Job, Level, Permissions, PullRequest, Push, Workflow, WorkflowDispatch};

/// Sources whose change can invalidate something we generate. A change to any
/// of these has to re-run the check.
const SOURCES: &[&str] = &[
    "devenv.nix",
    "devenv.yaml",
    "devenv.lock",
    "tooling/xtask/**",
    "crates/domain/**",
    "crates/cartograph/**",
    "src-tauri/res/**",
];

/// Generated outputs, listed so that hand-editing one is caught. devenv's own
/// come from the context at generation time.
const OUTPUTS: &[&str] = &[
    ".github/workflows/**",
    "crates/domain/typescript/**",
    "crates/cartograph/typescript/**",
    "src-tauri/assets/icons/app/**",
    "src-tauri/gen/apple/Assets.xcassets/*.appiconset/**",
    "src-tauri/gen/android/app/src/*/res/**/ic_launcher*",
];

pub fn workflow(context: &WorkflowContext) -> Workflow {
    let paths: Vec<String> = SOURCES
        .iter()
        .chain(OUTPUTS)
        .map(|path| (*path).to_owned())
        .chain(context.devenv_files.iter().cloned())
        .collect();

    let check_generated = Job::new("check generated files match their sources")
        .permissions(Permissions::default().contents(Level::Read))
        .runs_on(runners::LINUX)
        .add_step(steps::checkout())
        // The recipe evaluates and realises the whole environment, Android
        // SDK and all, which a stock image cannot hold. Nothing here uses
        // the image's Android SDK, so it goes too.
        .add_step(steps::free_disk_space(true))
        .add_step(steps::install_nix())
        .add_step(steps::cachix("devenv"))
        // `just` as well as `devenv`, because the recipe is invoked as
        // `devenv shell -- just …` and something has to launch that. The recipe
        // body also shells into devenv for its `cargo xtask` calls, which is
        // redundant inside the outer shell but harmless, and keeps the recipe
        // runnable on its own.
        .add_step(steps::nix_profile_install(&["devenv", "just"]))
        .add_step(steps::just("check-generated").name("Check generated files"));

    helpers::named_workflow("assert generated files remain in lockstep")
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
        .concurrency(helpers::one_run_per_ref("generated"))
        .add_job("check-generated", check_generated)
}
