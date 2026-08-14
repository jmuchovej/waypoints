//! Third-party actions, pinned in exactly one place.
//!
//! A workflow never spells out `owner/repo@version` inline. Bumping an action
//! is a one-line edit here that lands in every workflow on the next
//! `cargo xtask workflows`, and Renovate's custom manager (see `renovate.json`)
//! reads these literals directly.

use gh_workflow::{Step, Use};

/// A `uses:` reference — `{owner}/{repo}@{version}`.
pub struct Action {
    owner: &'static str,
    repo: &'static str,
    version: &'static str,
}

impl Action {
    /// A `Step<Use>` invoking this action, labelled `name`.
    pub fn step(&self, name: impl ToString) -> Step<Use> {
        Step::new(name).uses(self.owner, self.repo, self.version)
    }
}

pub const CHECKOUT: Action = Action {
    owner: "actions",
    repo: "checkout",
    version: "v6",
};

pub const INSTALL_NIX: Action = Action {
    owner: "cachix",
    repo: "install-nix-action",
    version: "v31",
};

pub const CACHIX: Action = Action {
    owner: "cachix",
    repo: "cachix-action",
    version: "v17",
};

pub const MISE: Action = Action {
    owner: "jdx",
    repo: "mise-action",
    version: "v2",
};

pub const SETUP_JAVA: Action = Action {
    owner: "actions",
    repo: "setup-java",
    version: "v5",
};

pub const SETUP_ANDROID: Action = Action {
    owner: "android-actions",
    repo: "setup-android",
    version: "v4",
};

pub const TAURI: Action = Action {
    owner: "tauri-apps",
    repo: "tauri-action",
    version: "v1",
};

pub const UPLOAD_ARTIFACT: Action = Action {
    owner: "actions",
    repo: "upload-artifact",
    version: "v4",
};

pub const GH_RELEASE: Action = Action {
    owner: "softprops",
    repo: "action-gh-release",
    version: "v3",
};
