//! Third-party actions, pinned in exactly one place.
//!
//! A workflow never spells out `owner/repo@version` inline. Bumping an action
//! is a one-line edit here that lands in every workflow on the next
//! `cargo xtask workflows`.

use gh_workflow::{Step, Use};

/// A `uses:` reference — `{owner}/{repo}@{version}`.
pub(crate) struct Action {
    owner: &'static str,
    repo: &'static str,
    version: &'static str,
}

impl Action {
    /// A `Step<Use>` invoking this action, labelled `name`.
    pub(crate) fn step(&self, name: impl ToString) -> Step<Use> {
        Step::new(name).uses(self.owner, self.repo, self.version)
    }
}

pub(crate) const CHECKOUT: Action = Action {
    owner: "actions",
    repo: "checkout",
    version: "v4",
};

pub(crate) const INSTALL_NIX: Action = Action {
    owner: "cachix",
    repo: "install-nix-action",
    version: "v31",
};

pub(crate) const CACHIX: Action = Action {
    owner: "cachix",
    repo: "cachix-action",
    version: "v17",
};
