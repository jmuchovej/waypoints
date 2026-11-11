// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! GitHub, from inside a workflow run: the Actions environment a job is
//! handed, the step outputs it writes back, and the repository API.
//!
//! The API is reached through `octocrab` rather than the `gh` CLI, so a call
//! is a typed request and a typed answer instead of argument formatting and
//! `--jq` on the way back. It authenticates with the `GH_TOKEN` the step sets
//! (falling back to `GITHUB_TOKEN`), the same variable `gh` would read, and
//! addresses the repository the runner names in `GITHUB_REPOSITORY`.
//!
//! A planner writes only refs and pull-request state here. File writes reach
//! `main` and the series branches through a pull request instead, committed by
//! the step that opens it, so a planner needs no `user.name`, no `user.email`,
//! and no push credentials on the checkout.

use anyhow::{Context as _, Result, bail};
use octocrab::{
    Octocrab,
    models::IssueState,
    params::{pulls::MergeMethod, repos::Reference},
};
use serde::Serialize;
use serde_json::Value;
use std::{fs, io::Write as _, path::Path, time::Duration};

/// A variable the Actions runner sets, read as an error when it is absent
/// rather than as an empty string.
pub fn env(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("${name} is not set"))
}

/// The repository a run is for, as `owner/name`.
pub fn repository() -> Result<String> {
    env("GITHUB_REPOSITORY")
}

fn owner_and_name() -> Result<(String, String)> {
    let repository = repository()?;
    repository
        .split_once('/')
        .map(|(owner, name)| (owner.to_owned(), name.to_owned()))
        .with_context(|| format!("$GITHUB_REPOSITORY is not `owner/name`: {repository}"))
}

/// A client authenticated as the step: `GH_TOKEN`, or the runner's own
/// `GITHUB_TOKEN`.
fn client() -> Result<Octocrab> {
    let token = env("GH_TOKEN")
        .or_else(|_| env("GITHUB_TOKEN"))
        .context("neither $GH_TOKEN nor $GITHUB_TOKEN is set")?;
    Octocrab::builder()
        .personal_token(token)
        .build()
        .context("Failed to build the GitHub client")
}

/// Every planner is synchronous; the client is not.
fn block_on<T>(future: impl Future<Output = Result<T>>) -> Result<T> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build a tokio runtime")?
        .block_on(future)
}

/// The mutation that has GitHub merge a pull request once it can. The REST API
/// has no auto-merge endpoint.
fn auto_merge_request(pull_request_id: &str) -> Value {
    serde_json::json!({
        "query": "mutation($id: ID!) { enablePullRequestAutoMerge(input: { pullRequestId: $id, mergeMethod: MERGE }) { clientMutationId } }",
        "variables": { "id": pull_request_id },
    })
}

/// Has GitHub merge pull request `number` as soon as its required checks allow.
///
/// GitHub refuses the mutation when there is nothing left to wait for, so a
/// refusal falls through to merging directly. Either way the landing is
/// GitHub's: the merge call answers to the same rulesets, and refuses a pull
/// request whose required checks have not passed.
pub fn enable_auto_merge(number: u64) -> Result<()> {
    let (owner, name) = owner_and_name()?;
    block_on(async {
        let client = client()?;
        let pulls = client.pulls(&owner, &name);
        let pull = pulls
            .get(number)
            .await
            .with_context(|| format!("Failed to read pull request #{number}"))?;
        // A refreshed pull request keeps what it already had.
        if pull.auto_merge.is_some() {
            return Ok(());
        }
        let id = pull
            .node_id
            .with_context(|| format!("Pull request #{number} has no node id"))?;

        let refusal = match client.graphql::<Value>(&auto_merge_request(&id)).await {
            Ok(_) => return Ok(()),
            Err(error) => error,
        };
        pulls
            .merge(number)
            .method(MergeMethod::Merge)
            .send()
            .await
            .with_context(|| {
                format!("GitHub refused auto-merge on #{number} ({refusal}) and the merge itself")
            })?;
        Ok(())
    })
}

/// How often [`wait_for_merge`] looks, and how many looks in a row may fail. The
/// checks take tens of minutes, and a merged pull request nobody saw leaves a
/// release without its tag, so one bad response must not end the wait.
const MERGE_POLL: Duration = Duration::from_secs(30);
const MERGE_POLL_FAILURES: u32 = 10;

/// What a look at a pull request found: its merge commit, or nothing yet. A pull
/// request closed unmerged will never have one.
fn merge_commit(
    number: u64,
    merged: bool,
    closed: bool,
    merge_commit_sha: Option<String>,
) -> Result<Option<String>> {
    if merged {
        return merge_commit_sha
            .map(Some)
            .with_context(|| format!("Pull request #{number} merged with no merge commit"));
    }
    if closed {
        bail!("Pull request #{number} closed without merging");
    }
    Ok(None)
}

/// Blocks until pull request `number` merges, returning its merge commit. A tag
/// made any earlier would name a commit the planners' first-parent walk never
/// reaches.
pub fn wait_for_merge(number: u64) -> Result<String> {
    let (owner, name) = owner_and_name()?;
    let look = || {
        block_on(async {
            let pull = client()?
                .pulls(&owner, &name)
                .get(number)
                .await
                .with_context(|| format!("Failed to read pull request #{number}"))?;
            Ok((
                pull.merged_at.is_some(),
                pull.state == Some(IssueState::Closed),
                pull.merge_commit_sha,
            ))
        })
    };

    let mut failures = 0;
    loop {
        match look() {
            Ok((merged, closed, sha)) => {
                failures = 0;
                if let Some(commit) = merge_commit(number, merged, closed, sha)? {
                    return Ok(commit);
                }
            }
            Err(error) if failures + 1 < MERGE_POLL_FAILURES => {
                failures += 1;
                println!("::warning::{error:#}");
            }
            Err(error) => return Err(error),
        }
        std::thread::sleep(MERGE_POLL);
    }
}

/// Creates `refs/heads/{name}` at `sha`.
pub fn create_branch(name: &str, sha: &str) -> Result<()> {
    create_ref(&Reference::Branch(name.to_owned()), sha)
}

/// Creates `refs/tags/{name}` at `sha`.
pub fn create_tag(name: &str, sha: &str) -> Result<()> {
    create_ref(&Reference::Tag(name.to_owned()), sha)
}

fn create_ref(reference: &Reference, sha: &str) -> Result<()> {
    let (owner, name) = owner_and_name()?;
    block_on(async {
        client()?
            .repos(&owner, &name)
            .create_ref(reference, sha)
            .await
            .with_context(|| format!("Failed to create {}", reference.full_ref_url()))?;
        Ok(())
    })
}

// ── Step outputs ─────────────────────────────────────────────────────────────

/// Publishes `outputs` as the current step's outputs.
pub fn set_outputs<T: Serialize>(outputs: &T) -> Result<()> {
    let path = env("GITHUB_OUTPUT")?;
    append_outputs(Path::new(&path), outputs)
}

/// [`set_outputs`], but a no-op off a runner — the runner always sets
/// `$GITHUB_OUTPUT`, so its absence means nothing is listening.
pub fn set_outputs_if_available<T: Serialize>(outputs: &T) -> Result<()> {
    match std::env::var_os("GITHUB_OUTPUT") {
        Some(_) => set_outputs(outputs),
        None => Ok(()),
    }
}

/// Appends to the file GitHub hands the step.
///
/// Appending, not writing: the file is the step's whole output channel, so
/// truncating would drop anything already recorded there.
pub fn append_outputs<T: Serialize>(path: &Path, outputs: &T) -> Result<()> {
    let rendered = render(outputs)?;

    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(rendered.as_bytes()))
        .with_context(|| format!("Failed to append to {}", path.display()))
}

/// One `key=value` line per field.
///
/// Scalars only. GitHub needs a heredoc delimiter for a value spanning lines,
/// and a nested value has no unambiguous rendering — either would be written
/// here as something the runner silently misreads, so both are refused.
fn render<T: Serialize>(outputs: &T) -> Result<String> {
    let value = serde_json::to_value(outputs).context("Failed to serialize outputs")?;
    let Value::Object(fields) = value else {
        bail!("outputs must serialize to a JSON object, got: {value}");
    };

    fields
        .iter()
        .map(|(key, value)| {
            let rendered = match value {
                Value::String(text) => text.clone(),
                Value::Bool(flag) => flag.to_string(),
                Value::Number(number) => number.to_string(),
                other => bail!("output `{key}` is not a scalar: {other}"),
            };

            if rendered.contains('\n') {
                bail!("output `{key}` spans lines; GitHub needs a heredoc delimiter for that");
            }

            Ok(format!("{key}={rendered}\n"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct Outputs {
        tag: String,
        should_build: bool,
        count: u32,
    }

    fn sample() -> Outputs {
        Outputs {
            tag: "v0.2.0-nightly.20260823".to_owned(),
            should_build: true,
            count: 7,
        }
    }

    #[test]
    fn renders_one_line_per_field() {
        assert_eq!(
            render(&sample()).unwrap(),
            "tag=v0.2.0-nightly.20260823\nshould_build=true\ncount=7\n"
        );
    }

    /// The file is the step's whole output channel, so anything already there
    /// has to survive.
    #[test]
    fn appends_rather_than_truncating() {
        let path = tempdir().join("github-output");
        fs::write(&path, "existing=value\n").unwrap();

        append_outputs(&path, &sample()).unwrap();

        let written = fs::read_to_string(&path).unwrap();
        assert!(written.starts_with("existing=value\n"), "{written}");
        assert!(written.contains("tag=v0.2.0-nightly.20260823"), "{written}");
    }

    #[test]
    fn creates_the_file_when_absent() {
        let path = tempdir().join("github-output");
        append_outputs(&path, &sample()).unwrap();
        assert!(fs::read_to_string(&path).unwrap().contains("count=7"));
    }

    /// A newline would be read as the end of the value, silently truncating it
    /// and then parsing the remainder as further outputs.
    #[test]
    fn refuses_a_value_spanning_lines() {
        #[derive(Serialize)]
        struct Multiline {
            notes: &'static str,
        }

        let error = render(&Multiline {
            notes: "first\nsecond",
        })
        .unwrap_err();
        assert!(error.to_string().contains("spans lines"), "{error}");
    }

    #[test]
    fn refuses_a_nested_value() {
        #[derive(Serialize)]
        struct Nested {
            inner: Vec<u8>,
        }

        let error = render(&Nested { inner: vec![1, 2] }).unwrap_err();
        assert!(error.to_string().contains("not a scalar"), "{error}");
    }

    #[test]
    fn refuses_a_value_that_is_not_an_object() {
        let error = render(&["a", "b"]).unwrap_err();
        assert!(error.to_string().contains("JSON object"), "{error}");
    }

    /// `MERGE`, because the rulesets allow nothing else, and the planners'
    /// first-parent walk needs a merge commit to read.
    #[test]
    fn auto_merge_lands_as_a_merge_commit() {
        let request = auto_merge_request("PR_abc");
        assert!(
            request["query"].as_str().unwrap().contains(
                "enablePullRequestAutoMerge(input: { pullRequestId: $id, mergeMethod: MERGE })"
            ),
            "{request}"
        );
        assert_eq!(request["variables"]["id"], "PR_abc");
    }

    #[test]
    fn a_merged_pull_request_names_its_merge_commit() {
        let found = merge_commit(7, true, true, Some("abc".to_owned())).unwrap();
        assert_eq!(found.as_deref(), Some("abc"));
    }

    /// An open pull request has a `merge_commit_sha` too: GitHub's trial merge,
    /// which is not the commit that lands. Only `merged_at` makes it real.
    #[test]
    fn an_open_pull_request_keeps_the_wait_going() {
        let found = merge_commit(7, false, false, Some("trial-merge".to_owned())).unwrap();
        assert_eq!(found, None);
    }

    #[test]
    fn a_pull_request_closed_unmerged_ends_the_wait() {
        let error = merge_commit(7, false, true, None).unwrap_err();
        assert!(error.to_string().contains("#7"), "{error}");
    }

    #[test]
    fn a_merge_with_no_commit_is_an_error_rather_than_a_longer_wait() {
        assert!(merge_commit(7, true, true, None).is_err());
    }

    /// The refs API takes `heads/…` and `tags/…`; a bare name would be read
    /// as neither.
    #[test]
    fn refs_are_fully_qualified() {
        assert_eq!(
            Reference::Branch("v0.2.x".to_owned()).full_ref_url(),
            "refs/heads/v0.2.x"
        );
        assert_eq!(
            Reference::Tag("v0.2.0-pre.1".to_owned()).full_ref_url(),
            "refs/tags/v0.2.0-pre.1"
        );
    }

    /// A directory unique to this call — the test runner reuses threads, so
    /// keying on the thread would let two tests collide.
    fn tempdir() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);

        let dir = std::env::temp_dir().join(format!(
            "xtask-github-{pid}-{n}",
            pid = std::process::id(),
            n = NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }
}
