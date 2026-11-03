use std::collections::BTreeMap;

use super::run::Difference;
use crate::{abi::Diagnostic, sdk::ManifestError};

/// Why a harness run could not be completed, or was not conformant.
#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Manifest(#[from] ManifestError),
    /// The options schema is not a valid Draft-07 document.
    #[error("the options schema is not valid Draft-07: {0}")]
    Schema(String),
    /// The configuration does not satisfy the options schema. `init` was
    /// never called.
    #[error("the configuration does not satisfy the options schema: {0}")]
    Config(String),
    /// A call returned bytes that are not the envelope the ABI defines.
    #[error("`{call}` returned a payload that is not an envelope: {error}")]
    Payload { call: String, error: String },
    /// A call returned `{"err": …}`.
    #[error("`{call}` failed: {diagnostic}")]
    Plugin {
        call: String,
        diagnostic: Diagnostic,
    },
    /// The bytes are not a module, or it could not be instantiated.
    #[error("the module could not be loaded: {0}")]
    Module(String),
    /// The module imports something; a plugin imports nothing.
    #[error("the module is not a plugin: it imports {}", .0.join(", "))]
    Imports(Vec<String>),
    /// The export set or a signature is not the ABI's.
    #[error("the module is not a plugin: expected exports {expected:?}, found {found:?}")]
    Shape {
        expected: BTreeMap<String, String>,
        found: BTreeMap<String, String>,
    },
    #[error("the module speaks ABI {found}, not {expected}")]
    Abi { expected: u32, found: u32 },
    /// A call trapped. `panic` is what `wp_panic` had to say afterwards.
    #[error("`{call}` trapped: {error}{}", match panic { Some(panic) => format!(" — {panic}"), None => String::new() })]
    Trap {
        call: String,
        error: String,
        panic: Option<String>,
    },
    /// Outputs differ from what they were compared with: goldens, a second
    /// run, or the other backend.
    #[error("{what}: {}", .differences.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"))]
    Mismatch {
        what: String,
        differences: Vec<Difference>,
    },
}
