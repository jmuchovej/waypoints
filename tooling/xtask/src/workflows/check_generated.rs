// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Asserts that everything we generate is in lockstep with the source it is
//! generated from — devenv's `files."..."`/`tasks."mise:lock"` outputs, the
//! TypeScript bindings `cargo xtask generate bindings` exports from `domain`
//! and `cartograph`, the app icons `cargo xtask generate icons` renders from
//! `src-tauri/res/`, and the workflows emitted by this very crate. The
//! recipe runs at a workstation's `cargo xtask`, `local-dev` included, since
//! the icons are among what it regenerates.

use super::{WorkflowContext, helpers, runners, steps};
use crate::constants::{GENERATED, MAIN};
use gh_workflow::{Event, Job, Level, Permissions, PullRequest, Push, Workflow, WorkflowDispatch};
use std::collections::BTreeSet;

/// What re-runs the check: every ledger entry's sources, then every entry's
/// outputs — so a hand edit to one is caught — then devenv's files from the
/// context, each once in first-seen order.
fn watched_paths(context: &WorkflowContext) -> Vec<String> {
    let mut seen = BTreeSet::new();
    GENERATED
        .iter()
        .flat_map(|entry| entry.sources.iter().copied())
        .chain(
            GENERATED
                .iter()
                .flat_map(|entry| entry.outputs.iter().copied()),
        )
        .map(str::to_owned)
        .chain(context.devenv_files.iter().cloned())
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

pub fn workflow(context: &WorkflowContext) -> Workflow {
    let paths = watched_paths(context);

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
                    .add_branch(MAIN),
            )
            .workflow_dispatch(WorkflowDispatch::default()))
        .concurrency(helpers::one_run_per_ref("generated"))
        .add_job("check-generated", check_generated)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scenarios _A generator is added_ and _The check and the listing agree_:
    /// the trigger is the ledger and the context, each path once, and nothing
    /// else.
    #[test]
    fn the_trigger_watches_the_ledger_and_the_context_and_nothing_else() {
        let mut context = WorkflowContext::fixture();
        // The first is also a ledger source, so it must appear once.
        context.devenv_files = vec![
            "devenv.lock".to_owned(),
            ".config/mise/conf.d/tools.toml".to_owned(),
        ];
        let value = serde_json::to_value(workflow(&context)).unwrap();

        let named: BTreeSet<&str> = GENERATED
            .iter()
            .flat_map(|entry| entry.sources.iter().chain(entry.outputs).copied())
            .chain(context.devenv_files.iter().map(String::as_str))
            .collect();
        for event in ["pull_request", "push"] {
            let watched: Vec<&str> = value["on"][event]["paths"]
                .as_array()
                .unwrap_or_else(|| panic!("{event} has no paths"))
                .iter()
                .map(|path| path.as_str().unwrap())
                .collect();
            let once: BTreeSet<&str> = watched.iter().copied().collect();
            assert_eq!(once.len(), watched.len(), "{event}: {watched:?}");
            assert_eq!(once, named, "{event}");
        }
    }
}
