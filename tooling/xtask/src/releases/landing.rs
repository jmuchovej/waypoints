// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! What the pull-request step lands: a bot writes to the working tree and
//! publishes one of these, and the step commits the write through the API so it
//! arrives signed. `main` and the series branches are protected, so this is the
//! only way a bot write reaches them.

/// The pull request for a write left in the working tree.
///
/// Serialized field-for-field into `$GITHUB_OUTPUT`, so these names are the
/// output keys.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub struct Landing {
    /// The pull request's branch: one per purpose and target, reused across
    /// runs, so a stalled write is refreshed and two writes never overwrite
    /// each other.
    pub head: String,
    /// The pull request's title and its commit's subject.
    pub title: String,
    /// What the step commits, comma-separated.
    pub paths: String,
}

impl Landing {
    pub fn new(head: String, title: String, paths: &[String]) -> Self {
        Self {
            head,
            title,
            paths: paths.join(","),
        }
    }

    /// One line for a job log or a dry run.
    pub fn describe(&self, base: &str) -> String {
        format!(
            "pull request `{title}` from {head} into {base}, committing {paths}",
            title = self.title,
            head = self.head,
            paths = self.paths,
        )
    }
}
