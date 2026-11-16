// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

#![allow(clippy::disallowed_macros)]

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

// Shared with the crate, which reads the baked value back and holds the
// resolution to its order in its tests.
#[path = "src/group_versions.rs"]
mod group_versions;

/// The group file: every library group, its tag template, and its anchor.
const GROUP_FILE: &str = "tooling/release-groups.toml";

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

/// The workspace root: the nearest ancestor holding the group file.
fn workspace_root() -> Option<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .find(|dir| dir.join(GROUP_FILE).exists())
        .map(Path::to_path_buf)
}

/// Every library group's version, as `group_versions` orders the sources —
/// injected, pinned, described, declared — rendered for the binary to read
/// back. The app's own group carries no library version. Each file read is
/// named for cargo to watch; a tree without the group file bakes nothing.
fn resolve_group_versions() -> String {
    let Some(root) = workspace_root() else {
        return String::new();
    };
    let group_file = root.join(GROUP_FILE);
    let lockfile = root.join("Cargo.lock");
    println!("cargo:rerun-if-changed={}", group_file.display());
    println!("cargo:rerun-if-changed={}", lockfile.display());

    let groups: toml::Table = fs::read_to_string(&group_file)
        .ok()
        .and_then(|text| text.parse().ok())
        .unwrap_or_default();
    let lock = fs::read_to_string(&lockfile).unwrap_or_default();
    let injected = env_var(group_versions::ENV)
        .map(|value| group_versions::parse(&value))
        .unwrap_or_default();

    let mut versions = BTreeMap::new();
    let named = groups.get("group").and_then(toml::Value::as_table);
    for (name, group) in named.into_iter().flatten() {
        // The app owns the remainder, and ships the libraries rather than
        // being one.
        if group.get("owns").is_some() {
            continue;
        }
        let (Some(template), Some(anchor)) = (
            group.get("tag").and_then(toml::Value::as_str),
            group.get("anchor").and_then(toml::Value::as_str),
        ) else {
            continue;
        };
        let Some((prefix, _)) = template.split_once("{version}") else {
            continue;
        };

        let manifest_path = root.join(anchor).join("Cargo.toml");
        println!("cargo:rerun-if-changed={}", manifest_path.display());
        let manifest: toml::Table = fs::read_to_string(&manifest_path)
            .ok()
            .and_then(|text| text.parse().ok())
            .unwrap_or_default();
        let package = manifest.get("package").and_then(toml::Value::as_table);
        let field = |key: &str| {
            package
                .and_then(|p| p.get(key))
                .and_then(toml::Value::as_str)
        };

        let group = group_versions::Group {
            name,
            prefix,
            anchor_package: field("name").unwrap_or(name),
            anchor_version: field("version").unwrap_or("unknown"),
        };
        let version = group_versions::resolve(&group, &injected, &lock, |prefix| {
            git(["describe", "--tags", "--match", &format!("{prefix}*")])
        });
        versions.insert(name.clone(), version);
    }
    group_versions::render(&versions)
}

/// The flavors a release ships as; `development` is not one of them.
const SHIPPED: [&str; 3] = ["nightly", "preview", "stable"];

/// The channel a version's prerelease names, read as `Version::channel` in
/// `xtask` reads it: none is stable, one led by `nightly` is nightly, any
/// other is a preview.
fn channel(pre: &str) -> &'static str {
    match pre.split('.').next().unwrap_or_default() {
        "" => "stable",
        "nightly" => "nightly",
        _ => "preview",
    }
}

/// Refuses flavors that cannot ship from this tree: two shipped flavors; one
/// that disagrees with the version's channel, unless `development` sits beside
/// it; and, on a release build, none.
fn check_flavors(
    enabled: &[&str],
    development: bool,
    channel: &str,
    release: bool,
) -> Result<(), String> {
    match enabled {
        [] if release => Err(format!(
            "a release build needs one shipped flavor: enable `nightly`, `preview`, or `stable` \
             (this tree's version names the `{channel}` channel)"
        )),
        [flavor] if *flavor != channel && !development => Err(format!(
            "the `{flavor}` flavor is enabled, but this tree's version names the `{channel}` \
             channel; enable `{channel}`, or add `development` to compile another channel on a \
             workstation"
        )),
        [] | [_] => Ok(()),
        _ => Err(format!(
            "the {} flavors are enabled; a build takes exactly one",
            enabled
                .iter()
                .map(|flavor| format!("`{flavor}`"))
                .collect::<Vec<_>>()
                .join(" and ")
        )),
    }
}

fn main() {
    let enabled: Vec<&str> = SHIPPED
        .into_iter()
        .filter(|flavor| env_var(&format!("CARGO_FEATURE_{}", flavor.to_uppercase())).is_some())
        .collect();
    let development = env_var("CARGO_FEATURE_DEVELOPMENT").is_some();
    let pre = std::env::var("CARGO_PKG_VERSION_PRE").unwrap_or_default();
    let release = env_var("WAYPOINTS_BUILD").is_some();
    if let Err(refusal) = check_flavors(&enabled, development, channel(&pre), release) {
        eprintln!("error: {refusal}");
        std::process::exit(1);
    }

    // Android 15+ requires 16 KB ELF page alignment
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        println!("cargo:rustc-link-arg=-Wl,-z,max-page-size=16384");
    }

    let sha = env_var("GITHUB_SHA")
        .or_else(|| git(["rev-parse", "HEAD"]))
        .unwrap_or_else(|| "unknown".into());
    let r#ref = env_var("GITHUB_REF_NAME")
        .or_else(|| git(["rev-parse", "--abbrev-ref", "HEAD"]))
        .unwrap_or_else(|| "unknown".into());
    let channel = env_var("WAYPOINTS_FLAVOR").unwrap_or_else(|| "development".into());
    let built_at = match env_var("WAYPOINTS_BUILD") {
        Some(_) => {
            let epoch = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0) as i64;
            chrono::DateTime::from_timestamp(epoch, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default()
        }
        None => String::new(),
    };

    let groups = resolve_group_versions();

    println!("cargo:rustc-env=WAYPOINTS_GIT_SHA={sha}");
    println!("cargo:rustc-env=WAYPOINTS_GIT_REF={ref}");
    println!("cargo:rustc-env=WAYPOINTS_CHANNEL={channel}");
    println!("cargo:rustc-env=WAYPOINTS_BUILT_AT={built_at}");
    println!("cargo:rustc-env={}={groups}", group_versions::ENV);

    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-env-changed=GITHUB_REF_NAME");
    println!("cargo:rerun-if-env-changed=WAYPOINTS_FLAVOR");
    println!("cargo:rerun-if-env-changed=WAYPOINTS_BUILD");
    println!("cargo:rerun-if-env-changed={}", group_versions::ENV);

    tauri_build::build()
}
