//! The release index: one committed file per channel under `nix/releases/`,
//! recording the tag that channel last published and what a Nix build needs
//! to fetch exactly that tree. See `distribution/release-index`.
//!
//! `cargo xtask release-index --tag v…` records a tag that already exists.
//! It classifies the channel from the version, checks that the tagged tree's
//! manifest names the same channel, prefetches the tree's hash, and commits the
//! channel's file to `main` through the Contents API — the cross-branch
//! write a cut already performs. The write is a blind replace of one file,
//! so it needs no checkout of `main` and no committer identity. Nothing is
//! built: the tree carries its own recipe and dependency pins.
//!
//! An entry lands on `main` strictly after the tag it describes, so to the
//! planners it looks like work that landed. [`product_commits`] is the one
//! `git log` both planners read, with that bookkeeping excluded.

use super::{
    release_plan::{self, APP_MANIFEST},
    workflows::Channel,
};
use crate::shell;
use anyhow::{Context as _, Result, bail};
use cargo_metadata::semver::Version;
use clap::Parser;

/// Where the index lives, relative to the repository root.
pub const INDEX_DIR: &str = "nix/releases";

/// The pathspec that drops index-only commits. A list of exclusions alone
/// matches nothing in git; the positive `.` is what gives it anything to
/// exclude from.
const PRODUCT_PATHSPEC: [&str; 3] = ["--", ".", ":(exclude)nix/releases/"];

#[derive(Parser)]
pub struct ReleaseIndexArgs {
    /// The tag to record, exactly as created: `v{version}`.
    #[arg(long)]
    tag: String,
    /// Compute and print the entry without committing it.
    #[arg(long)]
    dry_run: bool,
}

/// One channel's entry, serialized field-for-field as its file.
///
/// The field order is the file's key order, and `nix/releases/default.nix`
/// requires exactly these keys — there is one writer and one reader, and a
/// change to the shape changes both.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
struct Entry {
    tag: String,
    rev: String,
    hash: String,
}

impl Entry {
    fn path(channel: Channel) -> String {
        format!("{INDEX_DIR}/{channel}.json")
    }

    /// The file's bytes: two-space indented with a trailing newline, which is
    /// what treefmt leaves, so a write never also reads as a reformat.
    fn render(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self).context("Failed to serialize the entry")? + "\n")
    }
}

/// The tag's channel, provided the manifest it points at names the same one.
///
/// A tag is minted from the manifest it points at, so the two can only
/// disagree through a tag minted on the wrong branch — a nightly on a
/// preview series, a stable tag on `main` — and such a tag must not reach
/// the index under either name.
fn classify(tag: &str, version: &Version, manifest: &str) -> Result<Channel> {
    let channel = release_plan::channel_of(version);
    let declared = manifest_version(manifest)?;
    let declared_channel = release_plan::channel_of(&declared);
    if declared_channel != channel {
        bail!(
            "{tag} reads as a {channel} release, but its tree's manifest declares {declared}, a {declared_channel}; refusing to record it"
        );
    }
    Ok(channel)
}

/// The `[package] version` of an app manifest's text.
fn manifest_version(manifest: &str) -> Result<Version> {
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

/// The NAR hash of the tree at `rev`, as the overlay's `fetchTree` will
/// verify it. Learned through the same github fetcher, so a private
/// repository needs only the `access-tokens` nix already reads — and nothing
/// is built.
fn source_hash(rev: &str) -> Result<String> {
    let repository = shell::env("GITHUB_REPOSITORY")?;
    let reported = shell::nix(&[
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

pub fn run(args: ReleaseIndexArgs) -> Result<()> {
    let version = args
        .tag
        .strip_prefix('v')
        .with_context(|| format!("{} does not start with `v`", args.tag))
        .and_then(|version| {
            Version::parse(version).with_context(|| format!("{} is not a semver tag", args.tag))
        })?;
    let rev = shell::git(&["rev-parse", &format!("{}^{{commit}}", args.tag)])?;
    let manifest = shell::git(&["show", &format!("{rev}:{APP_MANIFEST}")])
        .with_context(|| format!("{} carries no {APP_MANIFEST}", args.tag))?;
    let channel = classify(&args.tag, &version, &manifest)?;
    println!("{tag} is a {channel} release at {rev}", tag = args.tag);

    let hash = source_hash(&rev)?;
    println!("  source: {hash}");

    let entry = Entry {
        tag: args.tag.clone(),
        rev,
        hash,
    };
    let path = Entry::path(channel);
    let content = entry.render()?;
    println!("{path}:\n{content}");

    if args.dry_run {
        println!("  (dry run — nothing written)");
        return Ok(());
    }

    // The same cross-branch write a cut performs, with the same caveat: if
    // `main` ever becomes a protected branch, this needs a PR path too.
    let commit = release_plan::commit_file(
        &path,
        "main",
        &content,
        &format!(
            "chore(release): record {tag} as the published {channel}",
            tag = args.tag
        ),
    )?;
    println!("  recorded on main at {commit}");
    Ok(())
}

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
    use indoc::indoc;
    use std::{fs, path::Path};

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    fn entry() -> Entry {
        Entry {
            tag: "v0.2.0-nightly.7+2026.08.29".to_owned(),
            rev: "9f3c1a".to_owned(),
            hash: "sha256-source=".to_owned(),
        }
    }

    fn manifest(version: &str) -> String {
        format!("[package]\nname = \"Waypoints\"\nversion = \"{version}\"\n")
    }

    /// Each channel's tag is recorded under its own file when the manifest
    /// it points at agrees, and a tag minted on the wrong branch is recorded
    /// nowhere.
    #[test]
    fn a_tag_is_filed_by_the_channel_its_manifest_names() {
        let nightly = v("0.2.0-nightly.7+2026.08.29");
        assert_eq!(
            classify(
                "v0.2.0-nightly.7+2026.08.29",
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
                  "tag": "v0.2.0-nightly.7+2026.08.29",
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
