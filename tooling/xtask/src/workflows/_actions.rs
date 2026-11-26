// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The setup prefixes the jobs share, generated as composite actions under
//! `.github/actions` — GitHub's one step-level reuse. A job checks out as
//! its own first step, with the depth, tags, and ref it needs, then runs one
//! of these by the self-repository reference, which resolves to the running
//! commit. What brackets a job's work — the snapshot pair, the compiler-cache
//! server — and what holds its environment's credential stays in the job.

use super::steps;
use crate::constants::ACTIONS_DIR;
use gh_workflow::{Expression, FluentBuilder as _, Step, StepType, StepValue};
use indexmap::IndexMap;
use serde::Serialize;
use std::path::PathBuf;

pub const SETUP_MISE: &str = "setup-mise";
pub const SETUP_NIX: &str = "setup-nix";

/// The file each action is written as, under its own directory.
pub const ACTION_FILE: &str = "action.yaml";

/// Every action the generator emits.
pub fn all() -> Vec<CompositeAction> {
    vec![setup_mise(), setup_nix()]
}

/// One `action.yaml`: the metadata GitHub reads, and the steps that were a
/// job's prefix.
#[derive(Serialize)]
pub struct CompositeAction {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    pub inputs: IndexMap<String, ActionInput>,
    pub runs: Runs,
}

#[derive(Serialize)]
pub struct ActionInput {
    pub description: String,
    #[serde(skip_serializing_if = "is_false")]
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Serialize)]
pub struct Runs {
    pub using: String,
    pub steps: Vec<StepValue>,
}

impl CompositeAction {
    fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_owned(),
            description: description.to_owned(),
            inputs: IndexMap::new(),
            runs: Runs {
                using: "composite".to_owned(),
                steps: Vec::new(),
            },
        }
    }

    /// An input, required when it has no default.
    fn input(mut self, name: &str, description: &str, default: Option<&str>) -> Self {
        self.inputs.insert(
            name.to_owned(),
            ActionInput {
                description: description.to_owned(),
                required: default.is_none(),
                default: default.map(str::to_owned),
            },
        );
        self
    }

    fn add_step<S: Into<Step<T>>, T: StepType>(mut self, step: S) -> Self {
        self.runs.steps.push(T::to_value(step.into()));
        self
    }

    /// Where the action is written.
    pub fn path(&self) -> PathBuf {
        PathBuf::from(ACTIONS_DIR)
            .join(&self.name)
            .join(ACTION_FILE)
    }
}

/// `inputs.<name>` as a `with:` or `env:` value inside the action.
fn input(name: &str) -> String {
    format!("${{{{ inputs.{name} }}}}")
}

/// Frees the runner's disk as asked, installs the mise the lockfile was
/// written by, and restores the compiled registry dependencies for a job that
/// publishes nothing.
fn setup_mise() -> CompositeAction {
    CompositeAction::new(
        SETUP_MISE,
        "Sets up the mise toolchain: frees the runner's disk as asked, installs the mise release the lockfile was written by, and restores the compiled Rust dependencies for a job that publishes nothing",
    )
    .input(
        "mise-version",
        "The mise release to install: the one that writes `.config/mise/mise.lock`",
        None,
    )
    .input("cache", "Keep mise's own install cache between runs", Some("false"))
    .input(
        "free-disk",
        "Reclaim the runner image's unused toolchains first: `none`, `all`, or `keep-android` for a job that builds for Android",
        Some("none"),
    )
    .input(
        "rust-cache",
        "Restore the compiled registry dependencies, written only from `main`; never for a job that publishes",
        Some("false"),
    )
    .add_step(
        steps::free_disk_space("${{ inputs.free-disk == 'all' }}")
            .if_condition(Expression::new("inputs.free-disk != 'none'")),
    )
    .add_step(steps::setup_mise_from(&input("mise-version"), &input("cache")))
    .add_step(steps::rust_cache().if_condition(Expression::new("inputs.rust-cache == 'true'")))
}

/// Frees the runner's disk — entering the devenv shell realises an
/// environment a stock image cannot hold — then installs Nix with the devenv
/// binary cache and the profile packages a job needs before its shell exists.
fn setup_nix() -> CompositeAction {
    CompositeAction::new(
        SETUP_NIX,
        "Sets up Nix for a job entered through the devenv shell: frees the runner's disk, installs Nix with the devenv binary cache, and installs the profile packages the job's first command needs",
    )
    .input(
        "packages",
        "The nixpkgs attributes to install into the ambient profile, space-separated",
        Some("devenv"),
    )
    .input("extra-nix-config", "Lines appended to `nix.conf`", Some(""))
    .add_step(steps::free_disk_space(true))
    .add_step(steps::install_nix().add_with(("extra_nix_config", input("extra-nix-config"))))
    .add_step(steps::cachix("devenv"))
    .add_step(steps::nix_profile_install_from(&input("packages")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn steps_of(action: &CompositeAction) -> Vec<Value> {
        action
            .runs
            .steps
            .iter()
            .map(|step| serde_json::to_value(step).expect("step serializes"))
            .collect()
    }

    /// Scenario _A checkout precedes the setup_, the action's half: no
    /// action checks out, so the job's own checkout — its depth, tags, and
    /// ref — is the one the setup follows.
    #[test]
    fn no_action_checks_out() {
        for action in all() {
            for step in steps_of(&action) {
                let uses = step["uses"].as_str().unwrap_or("");
                assert!(
                    !uses.starts_with("actions/checkout@"),
                    "{}: checks out",
                    action.name
                );
            }
        }
    }

    /// Scenario _A bracketing step stays in the job_: an action holds setup
    /// alone — nothing that restores before and saves after a job's work,
    /// nothing that starts the credentialed cache server, and nothing that
    /// carries the vault credential an environment grants.
    #[test]
    fn an_action_holds_setup_alone() {
        for action in all() {
            for step in steps_of(&action) {
                let label = format!("{}: {}", action.name, step["name"]);
                let run = step["run"].as_str().unwrap_or("");
                assert!(!run.contains("xcache "), "{label}: snapshots");
                assert!(!run.contains("sccache"), "{label}: runs the cache server");
                assert!(
                    step["env"].get("OP_SERVICE_ACCOUNT_TOKEN").is_none(),
                    "{label}: holds the vault credential"
                );
                assert!(
                    step["env"].get("SECRETSPEC_PROVIDER").is_none(),
                    "{label}: resolves secrets"
                );
            }
        }
    }

    /// Each input a job can set reaches the step it governs, and the mise
    /// release is the one input with no default: the job passes the version
    /// the lockfile was written by.
    #[test]
    fn the_mise_action_wires_its_inputs() {
        let action = super::setup_mise();
        let required: Vec<&str> = action
            .inputs
            .iter()
            .filter(|(_, input)| input.required)
            .map(|(name, _)| name.as_str())
            .collect();
        assert_eq!(required, ["mise-version"]);

        let steps = steps_of(&action);
        let free = &steps[0];
        assert_eq!(free["if"], "inputs.free-disk != 'none'");
        assert_eq!(free["with"]["android"], "${{ inputs.free-disk == 'all' }}");
        let mise = &steps[1];
        assert_eq!(mise["with"]["version"], "${{ inputs.mise-version }}");
        assert_eq!(mise["with"]["cache"], "${{ inputs.cache }}");
        let rust = &steps[2];
        assert_eq!(rust["if"], "inputs.rust-cache == 'true'");
        assert_eq!(steps.len(), 3);
    }

    /// The package list reaches the install through the environment, so a
    /// name is never spliced into the command line.
    #[test]
    fn the_nix_action_installs_its_packages_from_the_environment() {
        let action = super::setup_nix();
        let steps = steps_of(&action);
        let install = steps
            .iter()
            .find(|step| step["id"] == "setup-nix-profile")
            .expect("the profile install");
        assert_eq!(install["env"]["PACKAGES"], "${{ inputs.packages }}");
        assert!(
            !install["run"].as_str().unwrap().contains("inputs."),
            "{}",
            install["run"]
        );
        assert_eq!(action.inputs["packages"].default.as_deref(), Some("devenv"));
    }
}
