//! Resolves the release tag for a Nightly workflow run.
//!
//! - **push** (a `v*-nightly.*` tag): the tag already exists, so pass it through.
//! - **workflow_dispatch**: use the supplied tag; empty means build-only.
//! - **schedule**: skip when nothing landed since the last nightly, otherwise
//!   derive `v{base}-nightly.{date}` from the workspace version and create it.
//!
//! GitHub is reached through the REST API rather than shelled-out git, so the
//! job needs only a shallow checkout and tag creation needs no committer
//! identity or push credentials.

use crate::shell;
use anyhow::{Context, Result, bail};
use cargo_metadata::semver::Version;
use chrono::{DateTime, Utc};
use clap::Parser;
use octocrab::{Octocrab, params::repos::Reference};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use tokio::runtime::{Builder, Runtime};

#[derive(Parser)]
pub struct NightlyPlanArgs {
    /// Tag for a manual dispatch; empty means build-only, no release. Ignored
    /// for schedule and push events.
    #[arg(long, default_value = "")]
    dispatch_tag: String,
}

/// What the planner decided, and what the workflow reads back as job outputs.
#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    /// Empty on a build-only run.
    tag: String,
    should_build: bool,
}

impl Plan {
    fn build(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            should_build: true,
        }
    }

    const SKIP: Self = Self {
        tag: String::new(),
        should_build: false,
    };
}

/// The GitHub operations the planner needs.
///
/// A trait so the decision logic can be exercised against stubs — the same
/// reason the TypeScript this replaces injected an `exists` predicate.
pub trait Forge {
    /// Every tag in the repository.
    fn tags(&self) -> Result<Vec<String>>;
    /// Commits on `head` not reachable from `base`.
    fn commits_since(&self, base: &str, head: &str) -> Result<u64>;
    /// Creates an annotated tag object and the ref pointing at it.
    fn create_tag(&self, tag: &str, sha: &str) -> Result<()>;
}

pub fn run(args: NightlyPlanArgs) -> Result<()> {
    let event = env("GITHUB_EVENT_NAME")?;
    let plan = match event.as_str() {
        "push" => Plan::build(env("GITHUB_REF_NAME")?),
        "workflow_dispatch" => Plan::build(args.dispatch_tag),
        _ => {
            let sha = env("GITHUB_SHA")?;
            let gh = GitHub::from_env()?;
            plan_scheduled(&gh, &sha, &workspace_version(Path::new("."))?, Utc::now())?
        }
    };

    emit(&plan)
}

/// The scheduled path, which is the only one that has to decide anything.
fn plan_scheduled(
    forge: &impl Forge,
    sha: &str,
    version: &str,
    now: DateTime<Utc>,
) -> Result<Plan> {
    let tags = forge.tags()?;

    if let Some(last) = latest_nightly_tag(&tags)
        && forge.commits_since(&last, sha)? == 0
    {
        println!("No new commits since {last}; skipping tonight's build.");
        return Ok(Plan::SKIP);
    }

    let taken: BTreeSet<&str> = tags.iter().map(String::as_str).collect();
    let tag = next_nightly_tag(version, &date_stamp(now), &taken)?;

    forge.create_tag(&tag, sha)?;
    println!("Tagged nightly {tag} at {sha}.");
    Ok(Plan::build(tag))
}

/// The maintained `[workspace.package].version` — our single source of truth.
pub fn workspace_version(root: &Path) -> Result<String> {
    let path = root.join("Cargo.toml");
    let raw =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;

    raw.parse::<toml::Table>()
        .with_context(|| format!("Failed to parse {}", path.display()))?
        .get("workspace")
        .and_then(|workspace| workspace.get("package"))
        .and_then(|package| package.get("version"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .context("Cargo.toml has no [workspace.package].version")
}

/// Today's UTC date as `YYYYMMDD`.
pub fn date_stamp(now: DateTime<Utc>) -> String {
    now.format("%Y%m%d").to_string()
}

/// Derives the next nightly tag: `v{base}-nightly.{stamp}`, where `base` is the
/// version with any pre-release suffix stripped.
///
/// A same-day collision — a manual run alongside the cron, say — gets `.2`,
/// `.3`, and so on, which are valid and higher-sorting pre-release identifiers.
pub fn next_nightly_tag(version: &str, stamp: &str, taken: &BTreeSet<&str>) -> Result<String> {
    let Ok(parsed) = Version::parse(version) else {
        bail!("workspace version \"{version}\" is not valid semver");
    };

    let base = format!(
        "v{}.{}.{}-nightly.{stamp}",
        parsed.major, parsed.minor, parsed.patch
    );
    if !taken.contains(base.as_str()) {
        return Ok(base);
    }

    Ok((2..)
        .map(|n| format!("{base}.{n}"))
        .find(|candidate| !taken.contains(candidate.as_str()))
        .expect("an unbounded range always yields a free candidate"))
}

/// The most recent `*-nightly.*` tag by semver precedence — the date lives in
/// the tag, so ordering it by version orders it by time.
pub fn latest_nightly_tag(tags: &[String]) -> Option<String> {
    tags.iter()
        .filter(|tag| tag.contains("-nightly."))
        .filter_map(|tag| {
            Version::parse(tag.strip_prefix('v').unwrap_or(tag))
                .ok()
                .map(|version| (version, tag))
        })
        .max_by(|(left, _), (right, _)| left.cmp(right))
        .map(|(_, tag)| tag.clone())
}

/// Writes the plan to `$GITHUB_OUTPUT` for the workflow to read.
fn emit(plan: &Plan) -> Result<()> {
    let path: PathBuf = env("GITHUB_OUTPUT")?.into();
    let rendered = format!(
        "tag={tag}\nshould-build={should_build}\n",
        tag = plan.tag,
        should_build = plan.should_build,
    );

    fs::write(&path, rendered).with_context(|| format!("Failed to write {}", path.display()))
}

fn env(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("${name} is not set"))
}

/// [`Forge`] backed by the GitHub REST API.
///
/// The trait is synchronous because everything calling it is; the runtime lives
/// here rather than infecting `main`, which has no other reason to be async.
struct GitHub {
    client: Octocrab,
    runtime: Runtime,
    owner: String,
    repo: String,
}

impl GitHub {
    fn from_env() -> Result<Self> {
        let repository = env("GITHUB_REPOSITORY")?;
        let (owner, repo) = repository
            .split_once('/')
            .with_context(|| format!("$GITHUB_REPOSITORY is not owner/repo: {repository:?}"))?;

        // `GH_TOKEN` matches what the `gh` CLI reads, so a local run needs no
        // separate setup; Actions supplies `GITHUB_TOKEN`.
        let token = env("GH_TOKEN").or_else(|_| env("GITHUB_TOKEN"))?;

        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Failed to start a Tokio runtime")?;

        // Built inside the runtime: octocrab's HTTP client registers with the
        // reactor as it is constructed, not when it is first used, so building
        // it outside panics on the first request rather than here.
        let client = runtime
            .block_on(async { Octocrab::builder().personal_token(token).build() })
            .context("Failed to build a GitHub client")?;

        Ok(Self {
            client,
            runtime,
            owner: owner.to_owned(),
            repo: repo.to_owned(),
        })
    }
}

/// The annotated tag object, which octocrab does not wrap — it exposes only the
/// GET. The two fields we care about of the several it returns.
#[derive(serde::Deserialize)]
struct AnnotatedTag {
    sha: String,
}

impl Forge for GitHub {
    fn tags(&self) -> Result<Vec<String>> {
        self.runtime.block_on(async {
            let first = self
                .client
                .repos(&self.owner, &self.repo)
                .list_tags()
                .per_page(100)
                .send()
                .await
                .context("Failed to list tags")?;

            Ok(self
                .client
                .all_pages(first)
                .await
                .context("Failed to page through tags")?
                .into_iter()
                .map(|tag| tag.name)
                .collect())
        })
    }

    fn commits_since(&self, base: &str, head: &str) -> Result<u64> {
        self.runtime.block_on(async {
            let comparison = self
                .client
                .commits(&self.owner, &self.repo)
                .compare(base, head)
                .send()
                .await
                .with_context(|| format!("Failed to compare {base}...{head}"))?;

            u64::try_from(comparison.ahead_by)
                .with_context(|| format!("ahead_by was negative: {}", comparison.ahead_by))
        })
    }

    fn create_tag(&self, tag: &str, sha: &str) -> Result<()> {
        self.runtime.block_on(async {
            // An annotated tag is two objects: the tag itself, then a ref
            // pointing at it. The ref must reference the tag object, never the
            // commit it wraps.
            let created: AnnotatedTag = self
                .client
                .post(
                    format!("/repos/{}/{}/git/tags", self.owner, self.repo),
                    Some(&serde_json::json!({
                        "tag": tag,
                        "message": format!("Nightly {tag}"),
                        "object": sha,
                        "type": "commit",
                    })),
                )
                .await
                .with_context(|| format!("Failed to create the tag object for {tag}"))?;

            self.client
                .repos(&self.owner, &self.repo)
                .create_ref(&Reference::Tag(tag.to_owned()), created.sha)
                .await
                .with_context(|| format!("Failed to create refs/tags/{tag}"))
                .map(|_| ())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn taken<'a>(tags: &'a [&'a str]) -> BTreeSet<&'a str> {
        tags.iter().copied().collect()
    }

    fn owned(tags: &[&str]) -> Vec<String> {
        tags.iter().map(|tag| (*tag).to_owned()).collect()
    }

    #[test]
    fn date_stamp_formats_a_utc_date() {
        let now = Utc.with_ymd_and_hms(2026, 7, 26, 23, 59, 59).unwrap();
        assert_eq!(date_stamp(now), "20260726");
    }

    #[test]
    fn next_tag_stamps_the_base_version() {
        let tag = next_nightly_tag("0.2.0", "20260726", &taken(&[])).unwrap();
        assert_eq!(tag, "v0.2.0-nightly.20260726");
    }

    #[test]
    fn next_tag_strips_a_pre_release_suffix() {
        let tag = next_nightly_tag("0.2.0-pre.3", "20260726", &taken(&[])).unwrap();
        assert_eq!(tag, "v0.2.0-nightly.20260726");
    }

    #[test]
    fn next_tag_disambiguates_a_same_day_collision() {
        let tag =
            next_nightly_tag("0.2.0", "20260726", &taken(&["v0.2.0-nightly.20260726"])).unwrap();
        assert_eq!(tag, "v0.2.0-nightly.20260726.2");
    }

    #[test]
    fn next_tag_walks_past_multiple_collisions() {
        let existing = taken(&["v0.2.0-nightly.20260726", "v0.2.0-nightly.20260726.2"]);
        let tag = next_nightly_tag("0.2.0", "20260726", &existing).unwrap();
        assert_eq!(tag, "v0.2.0-nightly.20260726.3");
    }

    #[test]
    fn next_tag_rejects_a_non_semver_version() {
        let error = next_nightly_tag("not-a-version", "20260726", &taken(&[])).unwrap_err();
        assert!(error.to_string().contains("not valid semver"));
    }

    /// The disambiguated form has to keep sorting above the tag it disambiguates,
    /// or `latest_nightly_tag` would walk backwards.
    #[test]
    fn disambiguated_tags_sort_above_their_base() {
        let tags = owned(&["v0.2.0-nightly.20260726", "v0.2.0-nightly.20260726.2"]);
        assert_eq!(
            latest_nightly_tag(&tags).as_deref(),
            Some("v0.2.0-nightly.20260726.2")
        );
    }

    #[test]
    fn latest_ignores_non_nightly_tags() {
        let tags = owned(&[
            "v0.2.0-nightly.20260725",
            "v0.1.0",
            "v0.2.0-nightly.20260726",
            "v0.2.0-nightly.20260726.2",
            "v0.2.0-pre.1",
        ]);
        assert_eq!(
            latest_nightly_tag(&tags).as_deref(),
            Some("v0.2.0-nightly.20260726.2")
        );
    }

    /// Ordering is by version, not lexical — `0.10.0` outranks `0.9.0` even
    /// though its date stamp is older.
    #[test]
    fn latest_orders_across_base_versions() {
        let tags = owned(&["v0.9.0-nightly.20260101", "v0.10.0-nightly.20251231"]);
        assert_eq!(
            latest_nightly_tag(&tags).as_deref(),
            Some("v0.10.0-nightly.20251231")
        );
    }

    #[test]
    fn latest_is_none_without_nightly_tags() {
        assert_eq!(
            latest_nightly_tag(&owned(&["v0.1.0", "v0.2.0-pre.1"])),
            None
        );
    }

    #[test]
    fn workspace_version_reads_the_inherited_version() {
        let dir = tempdir();
        fs::write(
            dir.join("Cargo.toml"),
            "[workspace.package]\nversion = \"0.3.1\"\n",
        )
        .unwrap();
        assert_eq!(workspace_version(&dir).unwrap(), "0.3.1");
    }

    #[test]
    fn workspace_version_errors_when_absent() {
        let dir = tempdir();
        fs::write(dir.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
        let error = workspace_version(&dir).unwrap_err();
        assert!(error.to_string().contains("[workspace.package].version"));
    }

    /// Records what it was asked, so a test can assert on the calls themselves.
    #[derive(Default)]
    struct StubForge {
        tags: Vec<String>,
        ahead_by: u64,
        created: std::cell::RefCell<Vec<(String, String)>>,
    }

    impl Forge for StubForge {
        fn tags(&self) -> Result<Vec<String>> {
            Ok(self.tags.clone())
        }

        fn commits_since(&self, _base: &str, _head: &str) -> Result<u64> {
            Ok(self.ahead_by)
        }

        fn create_tag(&self, tag: &str, sha: &str) -> Result<()> {
            self.created
                .borrow_mut()
                .push((tag.to_owned(), sha.to_owned()));
            Ok(())
        }
    }

    fn at(stamp: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(&format!(
            "{}-{}-{}T04:00:00Z",
            &stamp[..4],
            &stamp[4..6],
            &stamp[6..]
        ))
        .unwrap()
        .with_timezone(&Utc)
    }

    #[test]
    fn scheduled_skips_when_nothing_landed() {
        let forge = StubForge {
            tags: owned(&["v0.2.0-nightly.20260725"]),
            ahead_by: 0,
            ..Default::default()
        };

        let plan = plan_scheduled(&forge, "SHA", "0.2.0", at("20260726")).unwrap();
        assert_eq!(plan, Plan::SKIP);
        assert!(
            forge.created.borrow().is_empty(),
            "must not tag when skipping"
        );
    }

    #[test]
    fn scheduled_tags_when_commits_landed() {
        let forge = StubForge {
            tags: owned(&["v0.2.0-nightly.20260725"]),
            ahead_by: 3,
            ..Default::default()
        };

        let plan = plan_scheduled(&forge, "SHA", "0.2.0", at("20260726")).unwrap();
        assert_eq!(plan, Plan::build("v0.2.0-nightly.20260726"));
        assert_eq!(
            forge.created.borrow().as_slice(),
            [("v0.2.0-nightly.20260726".to_owned(), "SHA".to_owned())]
        );
    }

    /// The very first nightly has nothing to compare against, so it must build
    /// rather than skip.
    #[test]
    fn scheduled_tags_when_no_nightly_exists_yet() {
        let forge = StubForge {
            tags: owned(&["v0.1.0"]),
            ahead_by: 0,
            ..Default::default()
        };

        let plan = plan_scheduled(&forge, "SHA", "0.2.0", at("20260726")).unwrap();
        assert_eq!(plan, Plan::build("v0.2.0-nightly.20260726"));
    }

    /// A cron run landing on a day that already has a manual nightly.
    #[test]
    fn scheduled_disambiguates_against_existing_tags() {
        let forge = StubForge {
            tags: owned(&["v0.2.0-nightly.20260726"]),
            ahead_by: 5,
            ..Default::default()
        };

        let plan = plan_scheduled(&forge, "SHA", "0.2.0", at("20260726")).unwrap();
        assert_eq!(plan, Plan::build("v0.2.0-nightly.20260726.2"));
    }

    /// A directory unique to this call — the test runner reuses threads, so
    /// keying on the thread would let two tests collide.
    /// Read-only probe against the live API. Ignored by default; run with
    /// `cargo test -p xtask -- --ignored --nocapture`.
    #[test]
    #[ignore = "hits the network"]
    fn live_read_paths() {
        unsafe { std::env::set_var("GITHUB_REPOSITORY", "cli/cli") };
        let forge = GitHub::from_env().unwrap();

        let tags = forge.tags().unwrap();
        println!("tags: {} (first: {:?})", tags.len(), tags.first());
        assert!(tags.len() > 100, "expected pagination to kick in");

        let ahead = forge.commits_since("v2.0.0", "trunk").unwrap();
        println!("ahead_by: {ahead}");
        assert!(ahead > 0);
    }

    fn tempdir() -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);

        let dir = std::env::temp_dir().join(format!(
            "xtask-nightly-plan-{pid}-{n}",
            pid = std::process::id(),
            n = NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
