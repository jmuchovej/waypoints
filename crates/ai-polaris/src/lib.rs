//! polaris — the virtual-assistant integration layer.
//!
//! The crew member, not the tools: polaris composes `instruments` into
//! assistant pipelines — live voice input, intent interpretation, dialog
//! state, action routing into cartograph. Explicit tool use (e.g. meeting
//! transcription) does **not** route through here; cartograph drives
//! instruments directly for that.
//!
//! Deliberately near-empty while transcription is the primary aim: the
//! assistant milestone is out of scope, and this crate exists so the seam —
//! and the dependency direction — is pinned.
//!
//! See `.agents/polaris-on-device-audio-ml.md` §6 for the crate topology.

pub use instruments;
