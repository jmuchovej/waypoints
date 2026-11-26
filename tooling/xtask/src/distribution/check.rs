// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Refuses a flavored package manifest committed under `distribution/`: a
//! concrete release tag, a 64-hex digest, or a versioned download URL means a
//! file was stamped for one release instead of rendered from the record. The
//! release index under `distribution/nix/releases/` exists to carry exactly
//! those, and is skipped.

use crate::distribution::render::DIST_DIR;
use anyhow::Result;
use regex::Regex;
use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

/// The one place under `distribution/` that records releases.
const RELEASE_INDEX: &str = "distribution/nix/releases";

static STAMPS: LazyLock<[(Regex, &'static str); 3]> = LazyLock::new(|| {
    [
        (
            Regex::new(r"\bv\d+\.\d+\.\d+").unwrap(),
            "a concrete release tag",
        ),
        (Regex::new(r"\b[0-9a-fA-F]{64}\b").unwrap(), "a digest"),
        (
            Regex::new(r"/api/v1/releases/\w+/v\d+\.\d+\.x/").unwrap(),
            "a versioned download URL",
        ),
    ]
});

/// One stamped value: where, and what it looked like.
#[derive(Debug, PartialEq, Eq)]
pub struct Stamp {
    pub path: PathBuf,
    pub line: usize,
    pub kind: &'static str,
}

/// Every stamp under `root`'s `distribution/`, outside the release index.
pub fn scan(root: &Path) -> Result<Vec<Stamp>> {
    let index = root.join(RELEASE_INDEX);
    let mut stamps = Vec::new();
    let walk = ignore::WalkBuilder::new(root.join(DIST_DIR))
        .hidden(false)
        .filter_entry(move |entry| !entry.path().starts_with(&index))
        .build();
    for entry in walk {
        let entry = entry?;
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        let path = entry
            .path()
            .strip_prefix(root)
            .unwrap_or(entry.path())
            .to_path_buf();
        for (number, line) in text.lines().enumerate() {
            for (pattern, kind) in STAMPS.iter() {
                if pattern.is_match(line) {
                    stamps.push(Stamp {
                        path: path.clone(),
                        line: number + 1,
                        kind,
                    });
                }
            }
        }
    }
    Ok(stamps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_committed_tree_carries_no_stamp() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(scan(&root).unwrap(), []);
    }

    #[test]
    fn a_stamped_package_manifest_is_named() {
        let root = tempfile::tempdir().unwrap();
        let brew = root.path().join("distribution/brew");
        std::fs::create_dir_all(&brew).unwrap();
        std::fs::write(
            brew.join("waypoints.rb"),
            format!(
                "cask \"waypoints\" do\n  version \"v0.1.1\"\n  sha256 \"{}\"\nend\n",
                "ab".repeat(32)
            ),
        )
        .unwrap();
        let stamps = scan(root.path()).unwrap();
        assert_eq!(
            stamps,
            [
                Stamp {
                    path: "distribution/brew/waypoints.rb".into(),
                    line: 2,
                    kind: "a concrete release tag",
                },
                Stamp {
                    path: "distribution/brew/waypoints.rb".into(),
                    line: 3,
                    kind: "a digest",
                },
            ]
        );
    }

    #[test]
    fn a_versioned_url_is_a_stamp_and_the_release_index_is_not() {
        let root = tempfile::tempdir().unwrap();
        let winget = root.path().join("distribution/winget");
        let index = root.path().join(RELEASE_INDEX);
        std::fs::create_dir_all(&winget).unwrap();
        std::fs::create_dir_all(&index).unwrap();
        std::fs::write(
            winget.join("installer.yaml"),
            "InstallerUrl: https://waypoints.so/api/v1/releases/stable/v0.1.x/windows/amd64/x.exe\n",
        )
        .unwrap();
        std::fs::write(
            index.join("nightly.json"),
            "{ \"tag\": \"v0.1.0-nightly.20261116\" }\n",
        )
        .unwrap();
        let stamps = scan(root.path()).unwrap();
        assert_eq!(stamps.len(), 1, "{stamps:?}");
        assert_eq!(stamps[0].kind, "a versioned download URL");
    }
}
