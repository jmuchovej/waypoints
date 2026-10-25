//! Weekly refresh of the Nix inputs pinned in `devenv.lock`, and of everything
//! generated downstream of them.
//!
//! Renovate covers the language ecosystems on the same Monday cadence but has
//! no manager for `devenv.lock`, so nothing else moves these inputs.

use super::{
    WorkflowContext, helpers,
    helpers::{CiScope, Environment, Resolution},
    runners, steps,
};
use gh_workflow::{
    Environment as GhEnvironment, Event, Job, Level, Permissions, Schedule, StepOutput, Workflow,
    WorkflowDispatch,
};

/// 04:00 UTC on Mondays. `schedule` only fires on the default branch, which is
/// the only branch whose lockfile we want to move.
const CRON: &str = "0 4 * * 1";

/// Reused every run, so a still-unmerged update is refreshed in place rather
/// than stacking a second pull request behind the first.
const BRANCH: &str = "devenv/update";

const TITLE: &str = "chore(devenv): update locked inputs";

const BODY: &str = concat!(
    "Ran `devenv update`, then regenerated everything that derives from the ",
    "lockfile — devenv's `files.\"...\"` outputs, the mise configs and their ",
    "lockfiles, and the workflows under `.github/workflows`.\n\n",
    "Opened by the weekly schedule in `xtask::workflows::devenv_update`. ",
    "Review the toolchain moves before merging; `check-generated` proves only ",
    "that the tree is self-consistent, not that the new versions work.\n\n",
    "If Gradle, Kotlin, or the Android Gradle Plugin moved, the Android build ",
    "fails dependency verification until `just android::verification-metadata` ",
    "has been run and its output committed on this branch.",
);

/// Sources and outputs the update is allowed to touch. Anything else the run
/// happens to dirty stays out of the commit.
fn paths(context: &WorkflowContext) -> Vec<String> {
    ["devenv.lock", ".github/workflows"]
        .iter()
        .map(|path| (*path).to_owned())
        .chain(context.devenv_files.iter().cloned())
        .collect()
}

/// What the App token is allowed to do: push the branch, open the pull request,
/// and write workflow files. That last one is its own GitHub permission — an
/// update that moves the Android NDK or JDK regenerates `.github/workflows`,
/// and a token without it fails the push rather than the pull request.
const TOKEN_SCOPES: &[(&str, &str)] = &[
    ("contents", "write"),
    ("pull-requests", "write"),
    ("workflows", "write"),
];

pub fn workflow(context: &WorkflowContext) -> Workflow {
    let app_token = steps::app_token(TOKEN_SCOPES);
    let token = StepOutput::new_trusted(&app_token, "token");

    // Through the shell, like the step before it: `secretspec` and `op` are
    // both devenv's here, and nothing installs them any other way.
    let export = steps::SecretSpec::export()
        .resolving(Resolution::Ci(CiScope::Botswain))
        .inside_devenv()
        .with_reason(
            "CI resolves the ci profile's botswain scope to authenticate as the App it names",
        );

    let update = Job::new("refresh devenv.lock and everything generated from it")
        // Only what `checkout` needs. The branch and the pull request are
        // pushed with the App token, which carries its own write scopes.
        .permissions(Permissions::default().contents(Level::Read))
        // Authenticates as the App, so the vault credential comes from the
        // automation environment.
        .environment(GhEnvironment::from(Environment::Ci))
        .runs_on(runners::LINUX)
        .add_step(steps::checkout())
        // `devenv update` evaluates and realises the whole environment,
        // Android SDK and all, which a stock image cannot hold: the first run
        // died with the runner's own diagnostics unable to write. Nothing here
        // uses the image's Android SDK, so it goes too.
        .add_step(steps::free_disk_space(true))
        .add_step(steps::install_nix())
        .add_step(steps::cachix("devenv"))
        // devenv provides us with secretspec already.
        .add_step(steps::nix_profile_install(&["devenv"]))
        .add_step(steps::devenv_update())
        .add_step(export)
        .add_step(app_token)
        .add_step(steps::create_pull_request(
            &token,
            BRANCH,
            TITLE,
            BODY,
            &paths(context),
        ));

    helpers::named_workflow("update devenv inputs")
        .on(Event::default()
            .schedule(vec![Schedule::new(CRON)])
            .workflow_dispatch(WorkflowDispatch::default()))
        // Queued rather than cancelled — a run killed between the push and the
        // pull request leaves the branch stranded.
        .concurrency(helpers::serialize("devenv-update"))
        .add_job("devenv-update", update)
}
