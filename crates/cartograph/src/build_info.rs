// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

/// The identity of a compiled build — what it is, where it came from, and when.
///
/// Populated at compile time by the app's build script (`src-tauri/build.rs`)
/// and surfaced through the `build_info` command. This is the single structured
/// source for diagnostics, crash context, and provenance: a bug report carries
/// its `sha`, so it reproduces by `git checkout <sha>` with no build-string
/// parsing and no reliance on a tag still existing.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    /// Semver of the build — the version the updater compares.
    pub version: String,
    /// Release channel: `stable` | `preview` | `nightly` | `development`.
    pub channel: String,
    /// Full git commit SHA the build was produced from.
    pub sha: String,
    /// RFC 3339 UTC timestamp of when the build was compiled (empty for local
    /// dev builds, which don't stamp a time).
    pub built_at: String,
    /// Git ref the build was produced from (branch name, or the tag on a tagged
    /// release build).
    pub branch: String,
    /// The version of every library group the binary carries, by group.
    pub groups: Vec<GroupVersion>,
}

/// One library group's version in a build: the tag it was built from — the
/// pin on a series, the tag or a `git describe` past it on `main` — or, where
/// the source carries no tags, the value injected into the build, else the
/// bare version its manifest declares.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GroupVersion {
    /// `bedrock`, `outpost`, `domain`, `transload`.
    pub group: String,
    /// `bedrock-v0.4.0`; `bedrock-v0.4.0-3-gabc1234` three commits past the
    /// tag on `main`; `0.4.0` where only the manifest could say.
    pub version: String,
}
