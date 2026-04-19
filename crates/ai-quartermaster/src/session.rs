//! Loaded-model handles. Will own the `ort` environment and construct
//! sessions with per-target execution providers, selected at compile time
//! via cargo features (`coreml` on Apple, `qnn`/`xnnpack` on Android,
//! `openvino`/`cuda` on desktop). Hands `instruments` an opaque handle;
//! never interprets tensors.

/// Opaque handle to a loaded model. Will wrap the `ort` session(s) once the
/// runtime dependency lands; `instruments` receives this and nothing else.
#[derive(Debug)]
pub struct LoadedModel {
    _private: (),
}
