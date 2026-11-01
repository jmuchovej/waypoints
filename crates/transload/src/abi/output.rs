use serde::{Deserialize, Serialize};

use super::diagnostic::Diagnostic;
use crate::{bundle::Census, fragment::Fragments};

/// What every fallible call returns: `{"ok": …}` or `{"err": …}`, the error
/// being a diagnostic.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Envelope<T> {
    Ok(T),
    Err(Diagnostic),
}

impl<T> From<Result<T, Diagnostic>> for Envelope<T> {
    fn from(result: Result<T, Diagnostic>) -> Self {
        match result {
            Ok(value) => Envelope::Ok(value),
            Err(diagnostic) => Envelope::Err(diagnostic),
        }
    }
}

impl<T> Envelope<T> {
    pub fn into_result(self) -> Result<T, Diagnostic> {
        match self {
            Envelope::Ok(value) => Ok(value),
            Envelope::Err(diagnostic) => Err(diagnostic),
        }
    }
}

/// What `wp_init` returns inside `ok`: the state handle every later call
/// passes back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Init {
    pub state: u32,
}

/// What a plugin produces for one entry, and at finalize: Fragments and
/// diagnostics. The wire shape of `wp_entry`'s `ok` payload.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Output {
    #[serde(default, skip_serializing_if = "Fragments::is_empty")]
    pub fragments: Fragments,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
}

impl Output {
    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty() && self.diagnostics.is_empty()
    }
}

/// Counts over everything a run emitted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    pub landmarks: usize,
    pub content: usize,
    pub protocharts: usize,
    pub terrain: usize,
}

/// The wire shape of `wp_finalize`'s `ok` payload: the plugin's final output
/// plus what the SDK stamps — the envelope and grammar versions, the census
/// of everything the run emitted, and the counts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FinalizeOutput {
    #[serde(default, skip_serializing_if = "Fragments::is_empty")]
    pub fragments: Fragments,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
    pub fragment_format: String,
    pub domain_version: String,
    pub census: Census,
    pub summary: Summary,
}

impl FinalizeOutput {
    /// Stamp a plugin's final output with what the SDK computed over the run.
    pub fn stamp(output: Output, census: Census, summary: Summary) -> Self {
        Self {
            fragments: output.fragments,
            diagnostics: output.diagnostics,
            fragment_format: crate::fragment::FRAGMENT_FORMAT.to_string(),
            domain_version: domain::VERSION.to_string(),
            census,
            summary,
        }
    }
}
