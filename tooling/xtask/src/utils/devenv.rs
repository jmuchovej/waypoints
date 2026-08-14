//! Queries devenv for what it manages.

use anyhow::{Context as _, Result, bail};
use std::{path::Path, process::Command};

use crate::shell;

/// [`managed_files`] minus anything symlinked into the store.
///
/// Touches the filesystem, so its answer depends on whether the devenv shell
/// has been entered. Not safe to bake into generated output.
pub fn copied_files() -> Result<Vec<String>> {
    Ok(managed_files()?
        .into_iter()
        .filter(|file| !Path::new(file).is_symlink())
        .collect())
}

/// Every file devenv materializes into the working tree, parsed from the
/// `# files` section of `devenv info`.
///
/// Never touches the filesystem, so its answer is stable enough to bake into
/// generated output.
pub fn managed_files() -> Result<Vec<String>> {
    let output = Command::new("devenv")
        .arg("info")
        .env("SECRETSPEC_REASON", REASON)
        .output()
        .context("Failed to run `devenv info` — is devenv on PATH?")?;

    if !output.status.success() {
        bail!(
            "`devenv info` failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8(output.stdout).context("`devenv info` emitted invalid UTF-8")?;

    let mut files: Vec<String> = stdout
        .lines()
        .skip_while(|line| line.trim() != "# files")
        .skip(1)
        .take_while(|line| !line.trim().is_empty())
        .filter_map(|line| line.trim().strip_prefix("- "))
        .map(str::to_owned)
        .collect();

    // An empty list means the `# files` section vanished or was renamed; a
    // workflow generated from it would silently watch nothing.
    if files.is_empty() {
        bail!("`devenv info` listed no managed files. Has its output format changed?");
    }

    files.sort();
    Ok(files)
}

/// Satisfies secretspec's `require_reason` policy, which gates every devenv
/// evaluation behind a stated purpose.
const REASON: &str = "xtask reads devenv's managed-file list and Android toolchain \
                      versions to generate CI workflows; no secret values are read";

/// The Android NDK version devenv installs, so CI can provision the same one.
pub fn android_ndk_version() -> Result<String> {
    let attribute = "android.ndk.version";
    let versions = shell::devenv(attribute)?;

    // The option is a list; CI builds against one NDK.
    versions
        .get(0)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .with_context(|| format!("{attribute} was not a non-empty list: {versions}"))
}

/// The JDK major version devenv installs, for `setup-java`. Only the major —
/// Zulu's release list may not carry the exact patch devenv pins.
pub fn android_jdk_major() -> Result<String> {
    let attribute = "languages.java.jdk.package.version";
    let version = shell::devenv(attribute)?;

    version
        .as_str()
        .and_then(|version| version.split('.').next())
        .map(str::to_owned)
        .with_context(|| format!("{attribute} was not a version string: {version}"))
}
