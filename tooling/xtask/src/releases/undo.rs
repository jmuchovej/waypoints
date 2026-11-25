// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Undoing a withdrawn release's bump. Every tag but a nightly's is minted
//! from a version write that landed by pull request; when the release built
//! from it is withdrawn, the write goes back the same way — reverted into
//! the working tree for the pull-request action, since `main` and the series
//! are protected — or the tree keeps declaring a version no tag names, which
//! every bump refuses until the revert lands. A cut's `-pre.1` is the one
//! bump that also opened a series and minted library tags: undoing it deletes
//! those, and reverts the write the cut landed on `main`.

use crate::{
    constants::{APP_MANIFEST, GROUP_FILE, RELEASE_UNDO_HEAD_PREFIX},
    release_groups::GroupFile,
    releases::{index, landing::Landing, version::Version},
    templates,
};
use anyhow::{Context as _, Result, bail};
use std::{path::Path, process::Command};
use tera::context;

/// What undoing a tag's bump takes.
#[derive(Debug, PartialEq, Eq)]
pub struct Undo {
    pub version: Version,
    /// The merge commit of the write declaring `version`.
    pub commit: String,
    pub kind: Kind,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Kind {
    /// An advance, promotion, or patch: one write on its series, reverted
    /// there.
    Series { branch: String },
    /// A cut: its series write at the commit, whose first parent is the
    /// write the cut landed on `main`. The library tags minted at that parent
    /// and the series itself go, and `main`'s write is reverted.
    Cut {
        series: String,
        main_write: String,
        library_tags: Vec<String>,
    },
}

/// Plans the undo of `tag`'s bump at `commit`, in the repository at `root`.
/// Refuses a nightly, which no write declared, and a commit that is not the
/// merge of the write declaring the tag's version: the tree at the commit has
/// to declare it and its first parent has not to. `tags` are the repository's
/// tags as `(name, sha)`; a cut names the library tags among them pointing at
/// its `main` write.
pub fn plan(root: &Path, tag: &str, commit: &str, tags: &[(String, String)]) -> Result<Undo> {
    let version = Version::from_tag(tag)?;
    if version.is_nightly() {
        bail!(
            "{tag} is a nightly, which no write declared; a nightly is withdrawn by deleting its tag"
        );
    }
    let declared = app_version_at(root, commit)?;
    if declared != version {
        bail!(
            "{commit} declares {declared}, not {version}; the commit to undo is the merge of the write declaring {tag}"
        );
    }
    let parent = git(root, &["rev-parse", &format!("{commit}^1")])?;
    if app_version_at(root, &parent)? == version {
        bail!(
            "{commit}'s first parent already declares {version}; the commit to undo is the merge of the write declaring {tag}"
        );
    }

    let series = version.release_branch();
    let kind = if is_cut(&version) {
        let library_tags = tags
            .iter()
            .filter(|(name, sha)| *sha == parent && is_library_tag(name))
            .map(|(name, _)| name.clone())
            .collect();
        Kind::Cut {
            series,
            main_write: parent,
            library_tags,
        }
    } else {
        Kind::Series { branch: series }
    };
    Ok(Undo {
        version,
        commit: commit.to_owned(),
        kind,
    })
}

/// Reverts `commit`, a merge, into the working tree at `root` without
/// committing — the pull-request action commits what is left there — and
/// returns the paths the revert changed.
pub fn revert(root: &Path, commit: &str) -> Result<Vec<String>> {
    git(root, &["revert", "--no-commit", "-m", "1", commit])?;
    let changed = git(root, &["diff", "--cached", "--name-only"])?;
    Ok(changed.lines().map(str::to_owned).collect())
}

/// The headers a summary carries for the versions a revert restores: every
/// group whose `version` differs between the group file `before` the revert
/// and `after` it — a library at the version it goes back to, the app as
/// where `main` targets again.
pub fn restored_headers(before: &str, after: &str) -> Result<Vec<String>> {
    let before = GroupFile::parse(before)?;
    let after = GroupFile::parse(after)?;
    let app = after.app_group()?;
    let mut headers = Vec::new();
    for (name, group) in &after.group {
        let was = before.group.get(name).map(|group| group.version.as_str());
        if was == Some(group.version.as_str()) {
            continue;
        }
        let header = if name == app {
            templates::must(
                "header.main",
                context! { app => app, next => &group.version },
            )
        } else {
            templates::must(
                "header.group",
                context! { group => name, version => &group.version },
            )
        };
        headers.push(header);
    }
    Ok(headers)
}

impl Undo {
    /// The pull request that lands a revert on `base` — the series, or
    /// `main` for a cut's write there — committing `paths`. A revert that
    /// restores more than one group's version is a summary, with the
    /// `headers` [`restored_headers`] reads, as the check requires.
    pub fn landing(&self, base: &str, paths: &[String], headers: &[String]) -> Landing {
        let title = if headers.len() > 1 {
            templates::must(
                "commit.undo-summary",
                context! { version => &self.version.to_string() },
            )
        } else {
            templates::must(
                "commit.undo",
                context! { version => &self.version.to_string(), branch => base },
            )
        };
        Landing::new(format!("{RELEASE_UNDO_HEAD_PREFIX}{base}"), title, paths)
    }
}

/// A `-pre.1`: the first preview of a series, which only a cut mints.
fn is_cut(version: &Version) -> bool {
    version.pre.as_str() == "pre.1"
}

/// A `<group>-v<version>` tag, as the group file's templates spell them;
/// the app's own tags are bare `v<version>`.
fn is_library_tag(name: &str) -> bool {
    !name.starts_with('v') && name.contains("-v")
}

/// The app version the tree at `rev` declares.
fn app_version_at(root: &Path, rev: &str) -> Result<Version> {
    let manifest = git(root, &["show", &format!("{rev}:{APP_MANIFEST}")])
        .with_context(|| format!("{rev} carries no {APP_MANIFEST}"))?;
    index::manifest_version(&manifest)
}

/// The group file the tree at `rev` carries.
pub fn group_file_at(root: &Path, rev: &str) -> Result<String> {
    git(root, &["show", &format!("{rev}:{GROUP_FILE}")])
        .with_context(|| format!("{rev} carries no {GROUP_FILE}"))
}

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .with_context(|| format!("running `git {}`", args.join(" ")))?;
    if !output.status.success() {
        bail!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8(output.stdout)
        .context("git's output is not UTF-8")?
        .trim()
        .to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::releases::index::fixture::Repo;

    fn group_file(app: &str, bedrock: &str) -> String {
        format!(
            r#"
            [bookkeeping]
            paths = [ "nix/releases/" ]
            bot-subjects = [ "chore(release):" ]

            [group.bedrock]
            version = "{bedrock}"
            members = [ "crates/bedrock" ]
            tag = "bedrock-v{{version}}"
            fixtures = "covenant/fixtures/bedrock/"

            [group.waypoints]
            version = "{app}"
            members = [ "src-tauri" ]
            tag = "v{{version}}"
            fixtures = "covenant/fixtures/waypoints/"
            "#
        )
    }

    fn manifest(name: &str, version: &str) -> String {
        format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\n")
    }

    /// Writes the app and `bedrock` at the given versions, as one commit.
    fn declare(repo: &Repo, app: &str, bedrock: &str, subject: &str) {
        repo.write(GROUP_FILE, &group_file(app, bedrock));
        repo.write(APP_MANIFEST, &manifest("Waypoints", app));
        repo.write("crates/bedrock/Cargo.toml", &manifest("bedrock", bedrock));
        repo.commit_all(subject);
    }

    /// Lands `subject`'s write on `base` as a pull request does: on a branch,
    /// then a merge commit.
    fn land(repo: &Repo, base: &str, app: &str, bedrock: &str, subject: &str) -> String {
        repo.git(&["checkout", "-q", base]);
        repo.git(&["checkout", "-q", "-b", "write"]);
        declare(repo, app, bedrock, subject);
        repo.git(&["checkout", "-q", base]);
        repo.git(&["merge", "-q", "--no-ff", "-m", subject, "write"]);
        repo.git(&["branch", "-q", "-D", "write"]);
        repo.head()
    }

    fn tags(repo: &Repo) -> Vec<(String, String)> {
        repo.git(&["show-ref", "--tags"])
            .lines()
            .filter_map(|line| line.split_once(' '))
            .map(|(sha, reference)| {
                (
                    reference.trim_start_matches("refs/tags/").to_owned(),
                    sha.to_owned(),
                )
            })
            .collect()
    }

    /// A series write — an advance here — is reverted into the series'
    /// working tree, and its landing names the series.
    #[test]
    fn a_series_write_is_reverted_into_the_tree_with_its_landing() {
        let repo = Repo::new();
        declare(
            &repo,
            "0.2.0-pre.1",
            "0.1.0",
            "chore(release): v0.2.x preview 0.2.0-pre.1",
        );
        repo.branch("v0.2.x");
        let write = land(
            &repo,
            "v0.2.x",
            "0.2.0-pre.2",
            "0.1.0",
            "chore(release): v0.2.x preview 0.2.0-pre.2",
        );
        repo.tag("v0.2.0-pre.2");

        let undo = plan(repo.root(), "v0.2.0-pre.2", &write, &tags(&repo)).unwrap();
        assert_eq!(
            undo.kind,
            Kind::Series {
                branch: "v0.2.x".to_owned()
            }
        );

        let paths = revert(repo.root(), &write).unwrap();
        assert_eq!(paths, [APP_MANIFEST, GROUP_FILE]);
        let file = GroupFile::parse(&repo.read(GROUP_FILE)).unwrap();
        assert_eq!(file.app_version().unwrap().to_string(), "0.2.0-pre.1");

        let landing = undo.landing("v0.2.x", &paths, &[]);
        assert_eq!(landing.head, "botswain/release-undo/v0.2.x");
        assert_eq!(
            landing.title,
            "chore(release): revert 0.2.0-pre.2 on v0.2.x"
        );
        assert_eq!(
            landing.paths,
            "src-tauri/Cargo.toml,tooling/release-groups.toml"
        );
    }

    /// A cut's undo names the library tags at its `main` write and the
    /// series, and the `main` revert restores every version the cut moved,
    /// under a summary with one header per group.
    #[test]
    fn a_cut_names_its_library_tags_and_series_and_reverts_main() {
        let repo = Repo::new();
        declare(&repo, "0.2.0-nightly", "0.1.0", "feat: first");
        let main_write = land(
            &repo,
            "main",
            "0.3.0-nightly",
            "0.1.1",
            "summary: cut 0.2.0-pre.1",
        );
        repo.tag("bedrock-v0.1.1");
        repo.branch("v0.2.x");
        let series_write = land(
            &repo,
            "v0.2.x",
            "0.2.0-pre.1",
            "0.1.1",
            "chore(release): v0.2.x preview 0.2.0-pre.1",
        );
        repo.tag("v0.2.0-pre.1");

        let undo = plan(repo.root(), "v0.2.0-pre.1", &series_write, &tags(&repo)).unwrap();
        assert_eq!(
            undo.kind,
            Kind::Cut {
                series: "v0.2.x".to_owned(),
                main_write: main_write.clone(),
                library_tags: vec!["bedrock-v0.1.1".to_owned()],
            }
        );

        repo.git(&["checkout", "-q", "main"]);
        let before = repo.read(GROUP_FILE);
        let paths = revert(repo.root(), &main_write).unwrap();
        assert_eq!(
            paths,
            ["crates/bedrock/Cargo.toml", APP_MANIFEST, GROUP_FILE]
        );
        let headers = restored_headers(&before, &repo.read(GROUP_FILE)).unwrap();
        assert_eq!(
            headers,
            [
                "chore(bedrock): 0.1.0",
                "chore(waypoints): main now targets 0.2.0-nightly",
            ]
        );
        let landing = undo.landing("main", &paths, &headers);
        assert_eq!(landing.head, "botswain/release-undo/main");
        assert_eq!(landing.title, "summary: revert the cut of 0.2.0-pre.1");
    }

    /// A nightly has no write to revert; it is withdrawn by deleting its tag.
    #[test]
    fn a_nightly_tag_is_refused() {
        let repo = Repo::new();
        declare(&repo, "0.2.0-nightly", "0.1.0", "feat: first");
        let error = plan(repo.root(), "v0.2.0-nightly.20261125", &repo.head(), &[])
            .unwrap_err()
            .to_string();
        assert!(error.contains("nightly"), "{error}");
    }

    /// The commit has to be the merge of the write: its tree declares the
    /// version, and its first parent's does not.
    #[test]
    fn a_commit_that_is_not_the_write_s_merge_is_refused() {
        let repo = Repo::new();
        declare(
            &repo,
            "0.2.0-pre.1",
            "0.1.0",
            "chore(release): v0.2.x preview 0.2.0-pre.1",
        );
        repo.branch("v0.2.x");
        let write = land(
            &repo,
            "v0.2.x",
            "0.2.0-pre.2",
            "0.1.0",
            "chore(release): v0.2.x preview 0.2.0-pre.2",
        );
        repo.commit("docs/a.md", "docs: after the write");
        let later = repo.head();

        // A later commit declares the version, but so does its parent.
        let error = plan(repo.root(), "v0.2.0-pre.2", &later, &[])
            .unwrap_err()
            .to_string();
        assert!(error.contains("first parent already declares"), "{error}");

        // The write's merge declares a version the tag does not name.
        let error = plan(repo.root(), "v0.2.0-pre.3", &write, &[])
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("declares 0.2.0-pre.2, not 0.2.0-pre.3"),
            "{error}"
        );
    }
}
