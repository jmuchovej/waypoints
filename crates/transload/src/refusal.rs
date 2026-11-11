// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Refusals as data.
//!
//! Ingest collects every problem it finds — per UUID, per Chart, per line — and
//! returns them as a structure a caller can render. A refused Bundle leaves the
//! store untouched.

use derive_more::Display;
use domain::{ChartID, FeatureID, MarkID};

/// Where in a Bundle something was found.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Location {
    /// The Bundle-relative file path.
    pub file: String,
    /// The 1-indexed line, where the carrier has lines to cite.
    pub line: Option<usize>,
}

impl Location {
    /// A whole-file location.
    pub fn file(path: impl Into<String>) -> Self {
        Self {
            file: path.into(),
            line: None,
        }
    }

    /// A located line, 1-indexed.
    pub fn line(path: impl Into<String>, line: usize) -> Self {
        Self {
            file: path.into(),
            line: Some(line),
        }
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(line) => write!(f, "{}:{line}", self.file),
            None => write!(f, "{}", self.file),
        }
    }
}

/// One `id` carried with divergent content by more than one file — Bundle
/// surgery, or a hand-merge that went wrong.
#[derive(Clone, Debug, PartialEq, Eq, Display)]
#[display("{id} differs between {}", files.join(", "))]
pub struct DuplicateConflict {
    /// The Mark id that disagrees with itself.
    pub id: MarkID,
    /// Every file that carried it, in path order. A shared Mark is byte-copied
    /// on export, so more than one file is normal — differing content is not.
    pub files: Vec<String>,
}

/// A Bundle's builtin Chart asking for more than the application's builtin can
/// give. Refusal, not silent narrowing.
#[derive(Clone, Debug, PartialEq, Eq, Display)]
// Raw id, not `Display`, which strips `builtin.`.
#[display(
    "builtin Chart `{}` requires features this build lacks: {}",
    crate::layout::chart_id(chart),
    missing_features.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ")
)]
pub struct ChartConflict {
    /// The builtin binding the Bundle named.
    pub chart: ChartID,
    /// Features the Bundle's schema requires that the application's builtin
    /// does not define.
    pub missing_features: Vec<FeatureID>,
}

/// A Bundle that does not hold together.
#[derive(Clone, Debug, PartialEq, Eq, Display)]
#[display("{at}: {kind}")]
pub struct Malformed {
    /// Where the problem is.
    pub at: Location,
    /// What the problem is.
    pub kind: MalformedKind,
}

/// The ways a Bundle can fail to be one.
#[derive(Clone, Debug, PartialEq, Eq, Display)]
pub enum MalformedKind {
    /// An identity comment keyed by importer provenance (`src`) rather than an
    /// `id`: Fragment grammar, refused rather than resolved.
    #[display("a `src`-keyed identity, which an export-Bundle never carries")]
    SourceKeyedIdentity,
    /// An annotative `terrain.jsonl` line whose natural key matches no edge
    /// reconstructible from the projection.
    #[display(
        "an annotative `{kind}` line ({src} → {dst}, order {}) matching no projected edge",
        order.map(|ordinal| ordinal.to_string()).unwrap_or_else(|| "—".to_string())
    )]
    OrphanAnnotativeEdge {
        /// The line's source endpoint.
        src: MarkID,
        /// The line's destination endpoint.
        dst: MarkID,
        /// The line's edge kind, as written.
        kind: String,
        /// The ordinal among parallel edges, as written.
        order: Option<u32>,
    },
    /// A reading that violates its Chart's schema.
    #[display("reading `{feature}` violates its Chart's schema: {detail}")]
    SchemaViolation {
        /// The feature whose reading is invalid.
        feature: FeatureID,
        /// What the validator said.
        detail: String,
    },
    /// Content the Bundle format does not define here — a reserved directory
    /// that was emitted, or a file the layout has no place for.
    #[display("unknown content: {detail}")]
    UnknownContent {
        /// What was found.
        detail: String,
    },
    /// The dialect, frontmatter, or sidecar grammar did not parse.
    #[display("{detail}")]
    Grammar {
        /// What failed to parse.
        detail: String,
    },
}

impl MalformedKind {
    /// See [`Self::OrphanAnnotativeEdge`].
    pub fn orphan_annotative_edge(
        src: MarkID,
        dst: MarkID,
        kind: impl Into<String>,
        order: Option<u32>,
    ) -> Self {
        Self::OrphanAnnotativeEdge {
            src,
            dst,
            kind: kind.into(),
            order,
        }
    }

    /// See [`Self::SchemaViolation`].
    pub fn schema_violation(feature: FeatureID, detail: impl Into<String>) -> Self {
        Self::SchemaViolation {
            feature,
            detail: detail.into(),
        }
    }

    /// See [`Self::UnknownContent`].
    pub fn unknown_content(detail: impl Into<String>) -> Self {
        Self::UnknownContent {
            detail: detail.into(),
        }
    }

    /// See [`Self::Grammar`].
    pub fn grammar(detail: impl Into<String>) -> Self {
        Self::Grammar {
            detail: detail.into(),
        }
    }
}

/// Everything wrong with one Bundle, collected rather than thrown one at a
/// time — a hand-edited Bundle deserves the whole list.
#[derive(Clone, Debug, Default, PartialEq, Eq, Display)]
#[display(
    "{}",
    duplicates.iter().map(ToString::to_string)
        .chain(charts.iter().map(ToString::to_string))
        .chain(malformed.iter().map(ToString::to_string))
        .collect::<Vec<_>>()
        .join("\n")
)]
pub struct Refusal {
    /// Same-`id` Marks whose content disagrees.
    pub duplicates: Vec<DuplicateConflict>,
    /// Builtin Charts the application cannot satisfy.
    pub charts: Vec<ChartConflict>,
    /// Grammar and structure failures, each located.
    pub malformed: Vec<Malformed>,
}

impl Refusal {
    /// Is there nothing to refuse?
    pub fn is_empty(&self) -> bool {
        self.duplicates.is_empty() && self.charts.is_empty() && self.malformed.is_empty()
    }

    /// Turn a non-empty collection into the error it is; `Ok(())` when clean.
    pub fn into_result(self) -> Result<(), IngestError> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(IngestError::Refused(Box::new(self)))
        }
    }
}

/// Why an ingest did not happen.
#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    /// The manifest declared a format this build does not ingest. Raised
    /// before any other file is read.
    #[error("bundle_format `{declared}` is not supported; this build ingests `{supported}`")]
    UnsupportedFormat {
        /// What the manifest said.
        declared: String,
        /// What this build writes and reads.
        supported: String,
    },
    /// The Bundle was read and refused. The store is untouched.
    #[error("the bundle was refused:\n{0}")]
    Refused(Box<Refusal>),
    /// The Bundle could not be read from disk.
    #[error("reading the bundle: {0}")]
    Io(#[from] std::io::Error),
    /// A write failed while applying the Bundle, so nothing was applied.
    #[error("applying the bundle: {0}")]
    Store(#[from] bedrock::BedrockError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::fixtures::{feature_id, mark_id};
    use indoc::indoc;

    #[test]
    fn an_empty_refusal_is_not_an_error() {
        assert!(Refusal::default().into_result().is_ok());
    }

    #[test]
    fn a_duplicate_names_every_file_that_carried_it() {
        let refusal = Refusal {
            duplicates: vec![DuplicateConflict {
                id: mark_id(7),
                files: vec!["library/c4d8/a.md".into(), "library/c4d8/b.md".into()],
            }],
            ..Refusal::default()
        };
        let message = refusal.to_string();
        assert!(message.contains("library/c4d8/a.md"), "got {message}");
        assert!(message.contains("library/c4d8/b.md"), "got {message}");
        assert!(refusal.into_result().is_err());
    }

    #[test]
    fn a_located_line_reads_as_file_and_line() {
        let malformed = Malformed {
            at: Location::line(crate::layout::TERRAIN, 12),
            kind: MalformedKind::orphan_annotative_edge(mark_id(1), mark_id(2), "child", Some(0)),
        };
        let message = Refusal {
            malformed: vec![malformed],
            ..Refusal::default()
        }
        .to_string();
        assert!(message.contains("terrain.jsonl:12"), "got {message}");
    }

    #[test]
    fn every_refusal_class_is_reported_together() {
        let refusal = Refusal {
            duplicates: vec![DuplicateConflict {
                id: mark_id(1),
                files: vec!["a.md".into(), "b.md".into()],
            }],
            charts: vec![ChartConflict {
                chart: ChartID::builtin("meeting"),
                missing_features: vec![feature_id("a1b2"), feature_id("c3d4")],
            }],
            malformed: vec![Malformed {
                at: Location::file("library/c4d8/x.md"),
                kind: MalformedKind::SourceKeyedIdentity,
            }],
        };
        assert_eq!(
            refusal.to_string(),
            indoc! {"
                00000000-0000-0000-0000-000000000001 differs between a.md, b.md
                builtin Chart `builtin.meeting` requires features this build lacks: a1b2, c3d4
                library/c4d8/x.md: a `src`-keyed identity, which an export-Bundle never carries"}
        );
    }
}
