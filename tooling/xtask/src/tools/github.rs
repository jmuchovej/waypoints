//! GitHub, from inside a workflow run: the Actions environment a job is
//! handed, the step outputs it writes back, and the repository API.
//!
//! The API is reached through `octocrab` rather than the `gh` CLI, so a call
//! is a typed request and a typed answer instead of argument formatting and
//! `--jq` on the way back. It authenticates with the `GH_TOKEN` the step sets
//! (falling back to `GITHUB_TOKEN`), the same variable `gh` would read, and
//! addresses the repository the runner names in `GITHUB_REPOSITORY`.
//!
//! Every repository write goes through the Contents and Refs APIs, which
//! commit as the token's identity — so a planner needs no `user.name`, no
//! `user.email`, and no push credentials on the checkout.

use anyhow::{Context as _, Result, bail};
use octocrab::{Octocrab, params::repos::Reference};
use serde::Serialize;
use serde_json::Value;
use std::{fs, io::Write as _, path::Path};

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

/// One file as it is on `branch`: its blob sha (needed to replace it) and
/// its decoded text.
pub fn file(path: &str, branch: &str) -> Result<(String, String)> {
    let (owner, name) = owner_and_name()?;
    block_on(async {
        let mut items = client()?
            .repos(&owner, &name)
            .get_content()
            .path(path)
            .r#ref(branch)
            .send()
            .await
            .with_context(|| format!("Failed to read {path} on {branch}"))?;
        let item = items
            .take_items()
            .into_iter()
            .next()
            .with_context(|| format!("{path} on {branch} is not a file"))?;
        let content = item
            .decoded_content()
            .with_context(|| format!("{path} on {branch} has no decodable content"))?;
        Ok((item.sha, content))
    })
}

/// Commits `content` over `path` on `branch`, returning the new commit's sha.
///
/// The Contents API writes a file and commits it in one call, authored by
/// the token's identity. The file must already exist: its current blob sha
/// is what the API replaces.
pub fn update_file(path: &str, branch: &str, content: &str, message: &str) -> Result<String> {
    let (owner, name) = owner_and_name()?;
    let (sha, _) = file(path, branch)?;
    block_on(async {
        let update = client()?
            .repos(&owner, &name)
            .update_file(path, message, content, sha)
            .branch(branch)
            .send()
            .await
            .with_context(|| format!("Failed to commit {path} on {branch}"))?;
        update
            .commit
            .sha
            .with_context(|| format!("GitHub reported no commit for {path} on {branch}"))
    })
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
