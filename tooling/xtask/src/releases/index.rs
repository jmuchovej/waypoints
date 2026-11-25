// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The release index: one committed file per channel under `nix/releases/`,
//! recording the tag that channel last published and what a Nix build needs
//! to fetch exactly that tree. See `distribution/release-index`.
//!
//! `cargo xtask release record --tag v…` records a tag that already exists.
//! It classifies the channel from the version, checks that the tagged tree's
//! manifest names the same channel, prefetches the tree's hash, and writes the
//! channel's file to the working tree, for a pull request to land on `main`.
//! The write is a blind replace of one file. Nothing is built: the tree carries
//! its own recipe and dependency pins.
//!
//! An entry lands on `main` strictly after the tag it describes, so to the
//! planners it looks like work that landed. [`product_commits`] is the one
//! history walk both planners read, with that bookkeeping excluded: the group
//! file names the index directory as bookkeeping, and [`history`] drops it.

use crate::{
    Channel,
    constants::{APP_MANIFEST, INDEX_DIR, RELEASE_INDEX_HEAD_PREFIX},
    releases::{history, landing::Landing, version::Version},
    tools::{git, github, nix},
};
use anyhow::{Context as _, Result, bail};

/// One channel's entry, serialized field-for-field as its file.
///
/// The field order is the file's key order, and `nix/releases/default.nix`
/// requires exactly these keys — there is one writer and one reader, and a
/// change to the shape changes both.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct Entry {
    pub tag: String,
    pub rev: String,
    pub hash: String,
}

impl Entry {
    pub fn path(channel: Channel) -> String {
        format!("{INDEX_DIR}/{channel}.json")
    }

    /// The file's bytes: two-space indented with a trailing newline, which is
    /// what treefmt leaves, so a write never also reads as a reformat.
    pub fn render(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self).context("Failed to serialize the entry")? + "\n")
    }

    /// The commit that records this entry on `main`.
    pub fn commit_message(&self, channel: Channel) -> String {
        crate::templates::must(
            "commit.index",
            tera::context! {
                tag => &self.tag,
                channel => &channel.to_string(),
            },
        )
    }

    /// The pull request that lands this entry. One branch per channel, so two
    /// channels publishing at once never overwrite each other's write.
    pub fn landing(&self, channel: Channel) -> Landing {
        Landing::new(
            format!("{RELEASE_INDEX_HEAD_PREFIX}{channel}"),
            self.commit_message(channel),
            &[Self::path(channel)],
        )
    }
}

/// The tag's channel, provided the manifest it points at names the same one.
///
/// A tag is minted from the manifest it points at, so the two can only
/// disagree through a tag minted on the wrong branch — a nightly on a
/// preview series, a stable tag on `main` — and such a tag must not reach
/// the index under either name.
pub fn classify(tag: &str, version: &Version, manifest: &str) -> Result<Channel> {
    let channel = version.channel();
    let declared = manifest_version(manifest)?;
    let declared_channel = declared.channel();
    if declared_channel != channel {
        bail!(
            "{tag} reads as a {channel} release, but its tree's manifest declares {declared}, a {declared_channel}; refusing to record it"
        );
    }
    Ok(channel)
}

/// The `[package] version` of an app manifest's text.
pub fn manifest_version(manifest: &str) -> Result<Version> {
    let table: toml::Table = manifest
        .parse()
        .with_context(|| format!("{APP_MANIFEST} is not TOML"))?;
    let version = table
        .get("package")
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .with_context(|| format!("{APP_MANIFEST} declares no [package] version"))?;
    Version::parse(version).with_context(|| format!("{APP_MANIFEST} version is not semver"))
}

/// The commit `reference` resolves to — a tag, or for a tag that does not
/// exist yet the commit it would point at — and the app manifest in that tree.
pub fn manifest_at(reference: &str) -> Result<(String, String)> {
    let rev = git::run(&["rev-parse", &format!("{reference}^{{commit}}")])?;
    let manifest = git::run(&["show", &format!("{rev}:{APP_MANIFEST}")])
        .with_context(|| format!("{reference} carries no {APP_MANIFEST}"))?;
    Ok((rev, manifest))
}

/// The NAR hash of the tree at `rev`, as the overlay's `fetchTree` will
/// verify it. Learned through the same github fetcher, so a private
/// repository needs only the `access-tokens` nix already reads — and nothing
/// is built.
pub fn source_hash(rev: &str) -> Result<String> {
    let repository = github::repository()?;
    let reported = nix::run(&[
        "flake",
        "prefetch",
        "--json",
        &format!("github:{repository}/{rev}"),
    ])?;
    let value: serde_json::Value =
        serde_json::from_str(&reported).context("`nix flake prefetch` did not emit JSON")?;
    value["hash"]
        .as_str()
        .map(str::to_owned)
        .with_context(|| format!("`nix flake prefetch` reported no hash for {rev}"))
}

/// The subjects of the commits in `range` that count for any group, oldest
/// first: [`history::walk`] over `main`'s first-parent history, with the
/// bookkeeping the group file names left out.
///
/// Filtered by path rather than by message: whether a commit is bookkeeping
/// is a question about what it touched, and a path answer survives any
/// change to how the index commit is worded.
pub fn product_commits(range: &str) -> Result<Vec<String>> {
    let metadata = crate::workspace::load()?;
    let groups = crate::release_groups::load(&metadata)?;
    let ignored = history::ignored_revisions(metadata.workspace_root.as_std_path(), &groups)?;
    Ok(history::product_subjects(&history::walk(
        range, &groups, &ignored,
    )?))
}

/// A throwaway git repository, for proving what the planners' walk sees.
#[cfg(test)]
pub(crate) mod fixture {
    use super::{INDEX_DIR, history};
    use crate::{constants::BLAME_IGNORE_FILE, release_groups::GroupFile};
    use std::{
        fs,
        path::PathBuf,
        process::Command,
        sync::atomic::{AtomicUsize, Ordering},
    };

    /// Groups for a fixture tree: three libraries, and the app owning `src/`
    /// beside its crates. Parsed rather than loaded, so nothing has to exist
    /// on disk.
    pub(crate) const GROUPS: &str = r#"
        [bookkeeping]
        paths = [ "nix/releases/" ]
        bot-subjects = [ "chore(release):" ]

        [group.bedrock]
        version = "0.1.0"
        members = [ "crates/bedrock", "crates/bedrock-core" ]
        tag = "bedrock-v{version}"
        fixtures = "covenant/fixtures/bedrock/"

        [group.outpost]
        version = "0.1.0"
        members = [ "crates/outpost" ]
        tag = "outpost-v{version}"
        fixtures = "covenant/fixtures/outpost/"

        [group.domain]
        version = "0.1.0"
        members = [ "crates/domain" ]
        tag = "domain-v{version}"
        fixtures = "covenant/fixtures/domain/"

        [group.waypoints]
        version = "0.1.0-nightly"
        members = [ "src-tauri" ]
        internals = [ "crates/store" ]
        tag = "v{version}"
        fixtures = "covenant/fixtures/waypoints/"
        paths = [ "src/" ]
    "#;

    pub(crate) struct Repo {
        dir: PathBuf,
    }

    impl Repo {
        /// The commit `HEAD` names.
        pub(crate) fn head(&self) -> String {
            self.git(&["rev-parse", "HEAD"])
        }

        /// Starts `name` at `HEAD` and checks it out.
        pub(crate) fn branch(&self, name: &str) {
            self.git(&["checkout", "-q", "-b", name]);
        }

        /// Lands `branch` on `main` the way a pull request does: a merge
        /// commit carrying `message`.
        pub(crate) fn merge(&self, branch: &str, message: &str) {
            self.git(&["checkout", "-q", "main"]);
            self.git(&["merge", "-q", "--no-ff", "-m", message, branch]);
        }

        /// Lists `sha` as a treewide pass.
        pub(crate) fn ignore(&self, sha: &str) {
            let file = self.dir.join(BLAME_IGNORE_FILE);
            let mut text = fs::read_to_string(&file).unwrap_or_default();
            text.push_str(sha);
            text.push('\n');
            fs::write(&file, text).unwrap();
        }

        /// A commit of `path` under `message`, which may carry a body and
        /// trailers.
        pub(crate) fn commit_with(&self, path: &str, message: &str) {
            self.commit_as("xtask", path, message);
        }

        /// A commit of `path` under `message`, authored by `author`.
        pub(crate) fn commit_as(&self, author: &str, path: &str, message: &str) {
            let file = self.dir.join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, message).unwrap();
            self.git(&["add", path]);
            self.git(&[
                "-c",
                &format!("user.name={author}"),
                "commit",
                "-q",
                "-m",
                message,
            ]);
        }

        /// What [`history::walk`] sees here for `range`, under [`GROUPS`].
        pub(crate) fn walk(&self, range: &str) -> Vec<history::Commit> {
            self.walk_under(range, GROUPS)
        }

        /// What [`history::walk`] sees here for `range`, under the group file
        /// `toml`, with the revisions it and the blame file list left out.
        pub(crate) fn walk_under(&self, range: &str, toml: &str) -> Vec<history::Commit> {
            let groups = GroupFile::parse(toml).unwrap();
            let ignored = history::ignored_revisions(&self.dir, &groups).unwrap();
            let args = history::log_args(range);
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            let output = self.git(&args);
            history::parse(&output)
                .into_iter()
                .map(|raw| history::attribute(raw, &groups, &ignored))
                .collect()
        }

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
            fs::write(dir.join(BLAME_IGNORE_FILE), "").unwrap();

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

        pub(crate) fn commit(&self, path: &str, subject: &str) {
            let file = self.dir.join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, subject).unwrap();
            self.git(&["add", path]);
            self.git(&["commit", "-q", "-m", subject]);
        }

        /// The repository's directory.
        pub(crate) fn root(&self) -> &std::path::Path {
            &self.dir
        }

        /// Writes `content` at `path`, uncommitted.
        pub(crate) fn write(&self, path: &str, content: &str) {
            let file = self.dir.join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, content).unwrap();
        }

        /// The file at `path` as the working tree has it.
        pub(crate) fn read(&self, path: &str) -> String {
            fs::read_to_string(self.dir.join(path)).unwrap()
        }

        /// Commits every change in the working tree under `subject`.
        pub(crate) fn commit_all(&self, subject: &str) {
            self.git(&["add", "-A"]);
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
            history::product_subjects(&self.walk(range))
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
    use indoc::indoc;
    use std::{fs, path::Path};

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    fn entry() -> Entry {
        Entry {
            tag: "v0.2.0-nightly.20260829".to_owned(),
            rev: "9f3c1a".to_owned(),
            hash: "sha256-source=".to_owned(),
        }
    }

    fn manifest(version: &str) -> String {
        indoc::formatdoc! { r#"
            [package]
            name = "Waypoints"
            version = "{version}"
        "# }
    }

    /// Each channel's tag is recorded under its own file when the manifest
    /// it points at agrees, and a tag minted on the wrong branch is recorded
    /// nowhere.
    #[test]
    fn a_tag_is_filed_by_the_channel_its_manifest_names() {
        let nightly = v("0.2.0-nightly.20260829");
        assert_eq!(
            classify(
                "v0.2.0-nightly.20260829",
                &nightly,
                &manifest("0.2.0-nightly")
            )
            .unwrap(),
            Channel::Nightly
        );
        assert_eq!(
            classify("v0.2.0-pre.1", &v("0.2.0-pre.1"), &manifest("0.2.0-pre.1")).unwrap(),
            Channel::Preview
        );
        assert_eq!(
            classify("v0.2.0", &v("0.2.0"), &manifest("0.2.0")).unwrap(),
            Channel::Stable
        );

        // A preview tag pointing at main's tree, or a stable tag at a preview's.
        let error = classify(
            "v0.2.0-pre.1",
            &v("0.2.0-pre.1"),
            &manifest("0.2.0-nightly"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("a nightly"), "{error}");
        assert!(classify("v0.2.0", &v("0.2.0"), &manifest("0.2.0-pre.3")).is_err());
        assert!(classify("v0.2.0", &v("0.2.0"), "[dependencies]\n").is_err());
    }

    #[test]
    fn an_entry_renders_as_its_file() {
        assert_eq!(Entry::path(Channel::Nightly), "nix/releases/nightly.json");
        assert_eq!(Entry::path(Channel::Stable), "nix/releases/stable.json");
        assert_eq!(
            entry().render().unwrap(),
            indoc! {r#"
                {
                  "tag": "v0.2.0-nightly.20260829",
                  "rev": "9f3c1a",
                  "hash": "sha256-source="
                }
            "#}
        );
    }

    /// One writer, one reader: every key written here is one the reader
    /// names as required, so the shape cannot drift on one side alone.
    #[test]
    fn the_reader_requires_every_key_written() {
        let reader = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(INDEX_DIR)
            .join("default.nix");
        let reader = fs::read_to_string(&reader).unwrap();

        let written = serde_json::to_value(entry()).unwrap();
        for key in written.as_object().unwrap().keys() {
            assert!(reader.contains(&format!("\"{key}\"")), "{key}");
        }
    }

    /// The whole reason the exclusion exists: a history of nothing but index
    /// commits reads as empty.
    #[test]
    fn a_history_of_only_index_commits_is_empty() {
        let repo = Repo::new();
        repo.product("feat: the nightly's work");
        repo.tag("v0.2.0-nightly.20260829");
        repo.index("nightly", "v0.2.0-nightly.20260829");
        repo.index("stable", "v0.1.0");

        assert_eq!(
            repo.product_commits("v0.2.0-nightly.20260829..HEAD"),
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
        repo.index("nightly", "v0.2.0-nightly.20260829");
        repo.product("feat: landmarks");

        assert_eq!(
            repo.product_commits("v0.1.0..HEAD"),
            ["fix: a bug", "feat: landmarks"]
        );
    }

    /// An entry now lands as a merge commit over a branch commit. Neither may
    /// read as work, or the quiet-repository skip never fires again.
    #[test]
    fn a_merged_index_pull_request_is_not_product_work() {
        let repo = Repo::new();
        repo.product("feat: the nightly's work");
        repo.tag("v0.2.0-nightly.20260829");

        repo.branch("botswain/release-index/nightly");
        repo.index("nightly", "v0.2.0-nightly.20260829");
        repo.merge(
            "botswain/release-index/nightly",
            "feat(release): record v0.2.0-nightly.20260829 as the published nightly (#7)",
        );

        assert_eq!(
            repo.product_commits("v0.2.0-nightly.20260829..HEAD"),
            Vec::<String>::new()
        );
    }

    /// Work that lands while the entry's pull request waits on its checks still
    /// counts, and the entry still does not.
    #[test]
    fn work_landing_beside_a_merged_entry_still_counts() {
        let repo = Repo::new();
        repo.product("feat: the nightly's work");
        repo.tag("v0.2.0-nightly.20260829");

        repo.branch("botswain/release-index/nightly");
        repo.index("nightly", "v0.2.0-nightly.20260829");
        repo.git(&["checkout", "-q", "main"]);
        repo.commit("crates/domain/src/other.rs", "fix: landed meanwhile");
        repo.merge(
            "botswain/release-index/nightly",
            "feat(release): record v0.2.0-nightly.20260829 as the published nightly (#7)",
        );

        assert_eq!(
            repo.product_commits("v0.2.0-nightly.20260829..HEAD"),
            ["fix: landed meanwhile"]
        );
    }

    /// One pull request per channel, so one channel's stalled entry cannot be
    /// overwritten by another's.
    #[test]
    fn each_channel_lands_from_its_own_branch() {
        let landing = entry().landing(Channel::Nightly);
        assert_eq!(landing.head, "botswain/release-index/nightly");
        assert_eq!(landing.paths, "nix/releases/nightly.json");
        assert_eq!(landing.title, entry().commit_message(Channel::Nightly));
        assert_ne!(landing.head, entry().landing(Channel::Stable).head,);
    }

    /// Before the first tag the range is the whole history, and the
    /// exclusion applies there too.
    #[test]
    fn the_whole_history_is_filtered_the_same_way() {
        let repo = Repo::new();
        repo.index("nightly", "v0.1.0-nightly.20260829");
        repo.product("feat: first");

        assert_eq!(repo.product_commits("HEAD"), ["feat: first"]);
    }
}
