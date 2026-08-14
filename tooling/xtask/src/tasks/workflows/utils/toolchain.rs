//! Reads `rust-toolchain.toml`, the single declaration of which targets we
//! build for.

use anyhow::{Context, Result, bail};
use std::{fs, path::Path};

/// Android ABIs to build, named as `tauri android build --target` wants them.
///
/// Derived rather than listed: `tauri android build` defaults to *all four*
/// ABIs it knows about, so a target dropped from `rust-toolchain.toml` without
/// a matching `--target` fails the build. Reading one file keeps the two in
/// step.
pub fn android_abis(root: &Path) -> Result<Vec<String>> {
    let targets = targets(root)?;

    let abis: Vec<String> = targets
        .iter()
        .filter_map(|target| match target.as_str() {
            "aarch64-linux-android" => Some("aarch64"),
            "armv7-linux-androideabi" => Some("armv7"),
            "i686-linux-android" => Some("i686"),
            "x86_64-linux-android" => Some("x86_64"),
            _ => None,
        })
        .map(str::to_owned)
        .collect();

    if abis.is_empty() {
        bail!("rust-toolchain.toml declares no Android targets");
    }

    Ok(abis)
}

fn targets(root: &Path) -> Result<Vec<String>> {
    let path = root.join("rust-toolchain.toml");
    let raw =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;

    let manifest: toml::Table = raw
        .parse()
        .with_context(|| format!("Failed to parse {}", path.display()))?;

    manifest
        .get("toolchain")
        .and_then(|toolchain| toolchain.get("targets"))
        .and_then(toml::Value::as_array)
        .map(|targets| {
            targets
                .iter()
                .filter_map(toml::Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .context("rust-toolchain.toml has no [toolchain].targets")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(contents: &str) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);

        let dir = std::env::temp_dir().join(format!(
            "xtask-toolchain-{pid}-{n}",
            pid = std::process::id(),
            n = NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("rust-toolchain.toml"), contents).unwrap();
        dir
    }

    #[test]
    fn maps_android_targets_to_tauri_abi_names() {
        let dir = fixture(indoc::indoc! {r#"
            [toolchain]
            targets = [
              "aarch64-apple-darwin",
              "aarch64-linux-android",
              "x86_64-linux-android",
              "aarch64-apple-ios",
            ]
        "#});

        assert_eq!(android_abis(&dir).unwrap(), ["aarch64", "x86_64"]);
    }

    /// The 32-bit ABIs still map, so re-adding one to `rust-toolchain.toml` is
    /// all it takes to ship it again.
    #[test]
    fn maps_the_32_bit_abis_too() {
        let dir = fixture(indoc::indoc! {r#"
            [toolchain]
            targets = ["armv7-linux-androideabi", "i686-linux-android"]
        "#});

        assert_eq!(android_abis(&dir).unwrap(), ["armv7", "i686"]);
    }

    #[test]
    fn errors_when_no_android_target_is_declared() {
        let dir = fixture(indoc::indoc! {r#"
            [toolchain]
            targets = ["aarch64-apple-darwin"]
        "#});

        let error = android_abis(&dir).unwrap_err();
        assert!(error.to_string().contains("no Android targets"));
    }

    #[test]
    fn errors_when_targets_are_absent() {
        let dir = fixture("[toolchain]\nchannel = \"stable\"\n");
        let error = android_abis(&dir).unwrap_err();
        assert!(error.to_string().contains("[toolchain].targets"));
    }
}
