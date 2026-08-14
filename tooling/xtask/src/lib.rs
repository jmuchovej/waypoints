//! `xtask` — the Rust-based dev-toolchain for Waypoints.
//!
//! Developer tooling lives here as tasks (`cargo xtask <task>`, or the
//! `just xtask::*` recipes) rather than as separate workspace crates: workflow
//! generation, CI test planning, devenv file listing, and the browser-dev HTTP
//! server. Nothing in this crate ships in a release artifact, so a task can
//! depend on app crates (e.g. `cartograph`) without gating one.

#![allow(clippy::disallowed_macros)]

pub mod tasks {
    pub mod dev_server;
    pub mod devenv_files;
    pub mod nightly_plan;
    pub mod release_plan;
    pub mod test_plan;
    pub mod workflows;

    pub use dev_server::DevServerArgs;
    pub use devenv_files::DevenvFilesArgs;
    pub use nightly_plan::NightlyPlanArgs;
    pub use release_plan::ReleasePlanArgs;
    pub use test_plan::TestPlanArgs;
    pub use workflows::WorkflowArgs;
}
pub mod utils {
    pub mod devenv;
    pub mod github;
}
pub mod botswain;
pub mod shell;
pub mod workspace;
