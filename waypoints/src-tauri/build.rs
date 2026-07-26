use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Bake the build's identity into the binary for `cartograph::BuildInfo`. Each
/// value prefers the CI-provided env var and falls back to `git` for local dev;
/// the `build_info` command reads them back via `env!`.
fn emit_build_info() {
    let sha = env_var("GITHUB_SHA")
        .or_else(|| git(["rev-parse", "HEAD"]))
        .unwrap_or_else(|| "unknown".into());
    let branch = env_var("GITHUB_REF_NAME")
        .or_else(|| git(["rev-parse", "--abbrev-ref", "HEAD"]))
        .unwrap_or_else(|| "unknown".into());
    let channel = env_var("WAYPOINTS_FLAVOR").unwrap_or_else(|| "development".into());

    // Only stamp a real build time for distribution builds (WAYPOINTS_BUILD=1).
    // A per-second timestamp on every dev build would force a relink each time.
    let built_at_epoch = match env_var("WAYPOINTS_BUILD") {
        Some(_) => SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        None => 0,
    };

    println!("cargo:rustc-env=WAYPOINTS_GIT_SHA={sha}");
    println!("cargo:rustc-env=WAYPOINTS_GIT_BRANCH={branch}");
    println!("cargo:rustc-env=WAYPOINTS_CHANNEL={channel}");
    println!("cargo:rustc-env=WAYPOINTS_BUILT_AT_EPOCH={built_at_epoch}");

    // Keep the SHA/branch fresh across local commits, and re-read the CI env.
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-env-changed=GITHUB_REF_NAME");
    println!("cargo:rerun-if-env-changed=WAYPOINTS_FLAVOR");
    println!("cargo:rerun-if-env-changed=WAYPOINTS_BUILD");
}

fn env_var(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.is_empty())
}

fn git<const N: usize>(args: [&str; N]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn main() {
    // Android 15+ requires 16 KB ELF page alignment
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        println!("cargo:rustc-link-arg=-Wl,-z,max-page-size=16384");
    }

    emit_build_info();

    tauri_build::build()
}
