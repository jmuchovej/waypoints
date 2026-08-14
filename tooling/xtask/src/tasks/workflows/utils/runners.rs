//! Runner labels, so a fleet change is a one-line edit rather than a sweep
//! across every workflow.

use gh_workflow::RunsOn;

#[derive(Clone, Copy)]
pub(crate) struct Runner(&'static str);

impl From<Runner> for RunsOn {
    fn from(runner: Runner) -> Self {
        runner.0.into()
    }
}

/// The default for anything that isn't a platform-specific build.
pub(crate) const LINUX: Runner = Runner("ubuntu-latest");
