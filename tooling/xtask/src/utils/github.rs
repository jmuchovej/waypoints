//! Writing back to the GitHub Actions runner. A task hands over a
//! serializable value; rendering it to `key=value` happens here.

use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;
use std::{fs, io::Write as _, path::Path};

/// Publishes `outputs` as the current step's outputs.
pub fn set_outputs<T: Serialize>(outputs: &T) -> Result<()> {
    let path = std::env::var("GITHUB_OUTPUT").context("$GITHUB_OUTPUT is not set")?;
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
