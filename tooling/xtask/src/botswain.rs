use gh_workflow::{Run, Step};

const BOTSWAIN_GIT_USERNAME: &str = "wp-botswain[bot]";
const BOTSWAIN_GIT_EMAIL: &str = "3633092+wp-botswain[bot]@users.noreply.github.com";

pub(crate) trait BotswainGitIdentity {
    fn with_botswain_git_identity(self) -> Self;
}

impl BotswainGitIdentity for Step<Run> {
    fn with_botswain_git_identity(self) -> Self {
        self.add_env(("GIT_AUTHOR_NAME", BOTSWAIN_GIT_USERNAME))
            .add_env(("GIT_COMMITTER_NAME", BOTSWAIN_GIT_USERNAME))
            .add_env(("GIT_AUTHOR_EMAIL", BOTSWAIN_GIT_EMAIL))
            .add_env(("GIT_COMMITTER_EMAIL", BOTSWAIN_GIT_EMAIL))
    }
}
