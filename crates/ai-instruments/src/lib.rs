//! instruments — inference-backed capabilities for Waypoints.
//!
//! One submodule per capability, each owning its complete toolchain:
//! preprocessing, forward passes against `quartermaster`-provided sessions,
//! decode loops, postprocessing. Output types are plain-data DTOs (serde +
//! specta) designed for consumers — they are what crosses the IPC boundary —
//! but this crate exposes **no transport-facing API of its own**: `cartograph`
//! drives it for explicit tool use, `polaris` for assistant flows.
//!
//! See `.agents/polaris-on-device-audio-ml.md` §6 for the crate topology.

pub mod asr;
pub mod audio;
pub mod embed;
pub mod tts;
