//! The release index: one committed file per channel under `nix/releases/`,
//! recording the tag that channel last published and the hashes a Nix build
//! needs to fetch exactly that source. See `distribution/release-index`.
//!
//! An entry lands on `main` strictly after the tag it describes, so to the
//! planners it looks like work that landed. [`product_commits`] is the one
//! `git log` both planners read, with that bookkeeping excluded.

use crate::shell;
use anyhow::Result;

/// Where the index lives, relative to the repository root.
pub const INDEX_DIR: &str = "nix/releases";

/// The pathspec that drops index-only commits. A list of exclusions alone
/// matches nothing in git; the positive `.` is what gives it anything to
/// exclude from.
const PRODUCT_PATHSPEC: [&str; 3] = ["--", ".", ":(exclude)nix/releases/"];

/// Conventional-commit subjects in `range`, oldest first, skipping commits
/// that touch only the release index.
///
/// Filtered by path rather than by message: whether a commit is bookkeeping
/// is a question about what it touched, and a path answer survives any
/// change to how the index commit is worded.
pub fn product_commits(range: &str) -> Result<Vec<String>> {
    Ok(shell::git(&log_args(range))?
        .lines()
        .map(str::to_owned)
        .collect())
}

fn log_args(range: &str) -> [&str; 7] {
    let [sep, all, exclude] = PRODUCT_PATHSPEC;
    ["log", "--format=%s", "--reverse", range, sep, all, exclude]
}

/// A throwaway git repository, for proving what the planners' `git log` sees.
#[cfg(test)]
pub(crate) mod fixture {
    use super::{INDEX_DIR, log_args};
    use std::{
        fs,
        path::PathBuf,
        process::Command,
        sync::atomic::{AtomicUsize, Ordering},
    };

    pub(crate) struct Repo {
        dir: PathBuf,
    }

    impl Repo {
        pub(crate) fn new() -> Self {
            // Unique per call: the runner reuses threads, so a thread id would
            // let two tests collide.
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let dir = std::env::temp_dir().join(format!(
                "xtask-release-index-{pid}-{n}",
                pid = std::process::id(),
                n = NEXT.fetch_add(1, Ordering::Relaxed),
            ));
            fs::create_dir_all(&dir).unwrap();

            let repo = Self { dir };
            repo.git(&["init", "-q", "-b", "main"]);
            repo
        }

        /// Runs `git` in the fixture, with identity and signing pinned so the
        /// developer's global config cannot reach in.
        pub(crate) fn git(&self, args: &[&str]) -> String {
            let output = Command::new("git")
                .args([
                    "-c",
                    "user.name=xtask",
                    "-c",
                    "user.email=xtask@waypoints.so",
                ])
                .args(["-c", "commit.gpgsign=false", "-c", "tag.gpgsign=false"])
                .args(args)
                .current_dir(&self.dir)
                .output()
                .expect("git is installed");
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            String::from_utf8(output.stdout).unwrap().trim().to_owned()
        }

        fn commit(&self, path: &str, subject: &str) {
            let file = self.dir.join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, subject).unwrap();
            self.git(&["add", path]);
            self.git(&["commit", "-q", "-m", subject]);
        }

        /// A commit touching product code, under `subject`.
        pub(crate) fn product(&self, subject: &str) {
            self.commit("crates/domain/src/lib.rs", subject);
        }

        /// A commit touching only `channel`'s index file. Worded as a
        /// feature so that, were it to leak through, the version bumper
        /// would act on it.
        pub(crate) fn index(&self, channel: &str, tag: &str) {
            self.commit(
                &format!("{INDEX_DIR}/{channel}.json"),
                &format!("feat(release): record {tag} as the published {channel}"),
            );
        }

        pub(crate) fn tag(&self, tag: &str) {
            self.git(&["tag", tag]);
        }

        /// What [`super::product_commits`] would return here for `range`.
        pub(crate) fn product_commits(&self, range: &str) -> Vec<String> {
            self.git(&log_args(range))
                .lines()
                .map(str::to_owned)
                .collect()
        }
    }

    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fixture::Repo;

    /// The exclusion and the directory the index lives in are stated
    /// separately; this is what keeps them the same directory.
    #[test]
    fn the_pathspec_excludes_the_index_directory() {
        let [sep, all, exclude] = PRODUCT_PATHSPEC;
        assert_eq!(sep, "--");
        assert_eq!(all, ".");
        assert_eq!(exclude, format!(":(exclude){INDEX_DIR}/"));

        let args = log_args("v0.1.0..HEAD");
        assert_eq!(args[3], "v0.1.0..HEAD");
        assert_eq!(&args[4..], &PRODUCT_PATHSPEC);
    }

    /// The whole reason the pathspec exists: a history of nothing but index
    /// commits reads as empty.
    #[test]
    fn a_history_of_only_index_commits_is_empty() {
        let repo = Repo::new();
        repo.product("feat: the nightly's work");
        repo.tag("v0.2.0-nightly.1+2026.08.29");
        repo.index("nightly", "v0.2.0-nightly.1+2026.08.29");
        repo.index("stable", "v0.1.0");

        assert_eq!(
            repo.product_commits("v0.2.0-nightly.1+2026.08.29..HEAD"),
            Vec::<String>::new()
        );
    }

    /// A mixed history yields the product commits and nothing else, in the
    /// order they landed.
    #[test]
    fn index_commits_vanish_from_a_mixed_history() {
        let repo = Repo::new();
        repo.product("feat: charts");
        repo.tag("v0.1.0");
        repo.index("stable", "v0.1.0");
        repo.product("fix: a bug");
        repo.index("nightly", "v0.2.0-nightly.1+2026.08.29");
        repo.product("feat: landmarks");

        assert_eq!(
            repo.product_commits("v0.1.0..HEAD"),
            ["fix: a bug", "feat: landmarks"]
        );
    }

    /// Before the first tag the range is the whole history, and the
    /// exclusion applies there too.
    #[test]
    fn the_whole_history_is_filtered_the_same_way() {
        let repo = Repo::new();
        repo.index("nightly", "v0.1.0-nightly.1+2026.08.29");
        repo.product("feat: first");

        assert_eq!(repo.product_commits("HEAD"), ["feat: first"]);
    }
}
