//! Loaded-model handles: the `ort` session seam. Execution providers are
//! registered here (and only here) once EP cargo features land (`coreml` on
//! Apple, `qnn`/`xnnpack` on Android, `openvino`/`cuda` on desktop); today
//! sessions run on the CPU provider. Hands `instruments` an opaque handle;
//! never interprets tensors.
//!
//! Linking: dev builds link dynamically against the devenv-provided ONNX
//! Runtime (`ORT_LIB_LOCATION` + `ORT_PREFER_DYNAMIC_LINK`, see devenv.nix).
//! App bundles must ship `libonnxruntime` — stores prohibit downloading
//! executable code at runtime.

use std::path::Path;

use ort::session::Session;

use crate::Error;

/// Opaque handle to a loaded model. `instruments` receives this and nothing
/// else; it runs forward passes via [`LoadedModel::session_mut`].
pub struct LoadedModel {
    session: Session,
}

impl LoadedModel {
    /// Load an ONNX model from a file materialized by [`crate::hub::Hub`].
    pub fn from_file(path: &Path) -> Result<Self, Error> {
        let session = Session::builder()?.commit_from_file(path)?;
        Ok(Self { session })
    }

    /// The underlying ort session, for instruments to run forward passes.
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }
}

#[cfg(test)]
mod tests {
    /// Smoke test that the ONNX Runtime library links and its C API version
    /// is compatible — catches ORT_LIB_LOCATION / nixpkgs-version drift at
    /// `cargo test` time rather than in a running app.
    #[test]
    fn ort_links_and_initializes() {
        ort::init().commit();
        ort::session::Session::builder().expect("ONNX Runtime failed to initialize");
    }
}
