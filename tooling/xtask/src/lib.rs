//! `xtask` — the Rust-based dev-toolchain for Waypoints.
//!
//! Developer tooling lives here as tasks (`cargo xtask <task>`, or the
//! `just xtask::*` recipes) rather than as separate workspace crates: workflow
//! generation, CI test planning, devenv file listing, TypeScript binding
//! generation, and the browser-dev HTTP server. Nothing in this crate ships in
//! a release artifact, so a task can depend on app crates (e.g. `cartograph`)
//! without gating one.

#![allow(clippy::disallowed_macros)]

pub mod tasks {
    pub mod dev_server;
    pub mod devenv_files;
    pub mod icongen;
    pub mod nightly_plan;
    pub mod release_index;
    pub mod release_manifest;
    pub mod release_plan;
    pub mod release_store;
    pub mod release_upload;
    pub mod release_withdraw;
    pub mod test_plan;
    pub mod typegen;
    pub mod workflows;

    pub use dev_server::DevServerArgs;
    pub use devenv_files::DevenvFilesArgs;
    pub use icongen::IcongenArgs;
    pub use nightly_plan::NightlyPlanArgs;
    pub use release_index::ReleaseIndexArgs;
    pub use release_manifest::ReleaseManifestArgs;
    pub use release_plan::ReleasePlanArgs;
    pub use release_upload::ReleaseUploadArgs;
    pub use release_withdraw::ReleaseWithdrawArgs;
    pub use test_plan::TestPlanArgs;
    pub use typegen::TypegenArgs;
    pub use workflows::WorkflowArgs;
}
pub mod utils {
    pub mod devenv;
    pub mod github;
    pub mod mise;
}
pub mod botswain;
pub mod shell;
pub mod workspace;
