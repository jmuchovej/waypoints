use schemars::JsonSchema;
use serde::de::DeserializeOwned;

use super::manifest::{self, ManifestError};
use crate::abi::{Diagnostic, Manifest, Output};

/// What a plugin implements. Plain Rust: a plugin's own tests call these
/// directly, with no WebAssembly runtime.
///
/// The `importer!` macro turns an implementation into the ABI's exports and
/// supplies the authored manifest ([`Manifested`]).
pub trait Importer: Sized {
    /// The plugin's configuration. Its JSON Schema (Draft-07) is generated
    /// from this type and served in the manifest; a host validates
    /// configuration against it before `init` ever sees it.
    type Config: DeserializeOwned + JsonSchema;

    /// Start a run with validated configuration.
    fn init(config: Self::Config) -> Result<Self, Diagnostic>;

    /// One entry — a name and its complete bytes — in the host's order:
    /// markers first, then sorted paths.
    fn entry(&mut self, name: &str, bytes: &[u8]) -> Result<Output, Diagnostic>;

    /// The end of the stream. May carry Fragments that depend on every entry.
    fn finalize(self) -> Result<Output, Diagnostic>;

    /// The manifest `wp_manifest` serves: the authored TOML plus the stamped
    /// fields. Fails, naming the field, when the TOML is not a manifest.
    fn manifest() -> Result<Manifest, ManifestError>
    where
        Self: Manifested,
    {
        manifest::build::<Self>()
    }
}

/// What the `importer!` macro supplies: the authored manifest's text and the
/// plugin crate's version.
pub trait Manifested {
    /// The TOML the author wrote, embedded at compile time.
    const AUTHORED: &'static str;
    /// `CARGO_PKG_VERSION` of the plugin crate.
    const VERSION: &'static str;
}
