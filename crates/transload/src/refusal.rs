//! Refusals as data.
//!
//! Ingest collects every problem it finds — per UUID, per Chart, per line — and
//! returns them as a structure a caller can render. A refused Bundle leaves the
//! store untouched.

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateConflict {
    /// The Mark id that disagrees with itself.
    pub id: MarkID,
    /// Every file that carried it, in path order. A shared Mark is byte-copied
    /// on export, so more than one file is normal — differing content is not.
    pub files: Vec<String>,
}

/// A Bundle's builtin Chart asking for more than the application's builtin can
/// give. Refusal, not silent narrowing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChartConflict {
    /// The builtin binding the Bundle named.
    pub chart: ChartID,
    /// Features the Bundle's schema requires that the application's builtin
    /// does not define.
    pub missing_features: Vec<FeatureID>,
}

/// A Bundle that does not hold together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Malformed {
    /// Where the problem is.
    pub at: Location,
    /// What the problem is.
    pub kind: MalformedKind,
}

/// The ways a Bundle can fail to be one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MalformedKind {
    /// An identity comment keyed by importer provenance (`src`) rather than an
    /// `id`: Fragment grammar, refused rather than resolved.
    SourceKeyedIdentity,
    /// An annotative `terrain.jsonl` line whose natural key matches no edge
    /// reconstructible from the projection.
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
    SchemaViolation {
        /// The feature whose reading is invalid.
        feature: FeatureID,
        /// What the validator said.
        detail: String,
    },
    /// Content the Bundle format does not define here — a reserved directory
    /// that was emitted, or a file the layout has no place for.
    UnknownContent {
        /// What was found.
        detail: String,
    },
    /// The dialect, frontmatter, or sidecar grammar did not parse.
    Grammar {
        /// What failed to parse.
        detail: String,
    },
}

impl std::fmt::Display for MalformedKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceKeyedIdentity => {
                write!(
                    f,
                    "a `src`-keyed identity, which an export-Bundle never carries"
                )
            }
            Self::OrphanAnnotativeEdge {
                src,
                dst,
                kind,
                order,
            } => write!(
                f,
                "an annotative `{kind}` line ({src} → {dst}, order {}) matching no projected edge",
                order
                    .map(|ordinal| ordinal.to_string())
                    .unwrap_or_else(|| "—".to_string())
            ),
            Self::SchemaViolation { feature, detail } => {
                write!(
                    f,
                    "reading `{feature}` violates its Chart's schema: {detail}"
                )
            }
            Self::UnknownContent { detail } => write!(f, "unknown content: {detail}"),
            Self::Grammar { detail } => write!(f, "{detail}"),
        }
    }
}

/// Everything wrong with one Bundle, collected rather than thrown one at a
/// time — a hand-edited Bundle deserves the whole list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        let mut line = |f: &mut std::fmt::Formatter<'_>| -> std::fmt::Result {
            if first {
                first = false;
                Ok(())
            } else {
                writeln!(f)
            }
        };
        for duplicate in &self.duplicates {
            line(f)?;
            write!(
                f,
                "{} differs between {}",
                duplicate.id,
                duplicate.files.join(", ")
            )?;
        }
        for conflict in &self.charts {
            line(f)?;
            let missing: Vec<_> = conflict
                .missing_features
                .iter()
                .map(ToString::to_string)
                .collect();
            write!(
                f,
                // Raw id, not `Display`, which strips `builtin.`.
                "builtin Chart `{}` requires features this build lacks: {}",
                crate::layout::chart_id(&conflict.chart),
                missing.join(", ")
            )?;
        }
        for malformed in &self.malformed {
            line(f)?;
            write!(f, "{}: {}", malformed.at, malformed.kind)?;
        }
        Ok(())
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
    use domain::Base36ID;
    use uuid::Uuid;

    fn mark(n: u128) -> MarkID {
        MarkID(Uuid::from_u128(n))
    }

    #[test]
    fn an_empty_refusal_is_not_an_error() {
        assert!(Refusal::default().into_result().is_ok());
    }

    #[test]
    fn a_duplicate_names_every_file_that_carried_it() {
        let refusal = Refusal {
            duplicates: vec![DuplicateConflict {
                id: mark(7),
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
            kind: MalformedKind::OrphanAnnotativeEdge {
                src: mark(1),
                dst: mark(2),
                kind: "child".into(),
                order: Some(0),
            },
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
                id: mark(1),
                files: vec!["a.md".into()],
            }],
            charts: vec![ChartConflict {
                chart: ChartID::builtin("meeting"),
                missing_features: vec![FeatureID(Base36ID("a1b2".into()))],
            }],
            malformed: vec![Malformed {
                at: Location::file("library/c4d8/x.md"),
                kind: MalformedKind::SourceKeyedIdentity,
            }],
        };
        let message = refusal.to_string();
        assert_eq!(message.lines().count(), 3, "got {message}");
    }
}
