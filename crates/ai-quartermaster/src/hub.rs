//! Artifact acquisition: `hf-hub` download on first use, with progress
//! reporting surfaced to the UI, then fully offline. Materializes a
//! [`crate::ModelSpec`]'s files into the cache.
//!
//! Weights are *data*, so runtime download is app-store-compliant; the
//! inference runtime itself (`libonnxruntime`) must ship in the app bundle.
//!
//! Not yet implemented — lands with the first `ort` integration.
