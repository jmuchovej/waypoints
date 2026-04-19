//! Speech → text. [`AsrEngine`] is the backend seam: `ort` primary, Candle
//! fallback, vendor-native a future Tier-3 slot — all behind the trait,
//! selected by cargo feature. Decode loops (Whisper autoregressive, Parakeet
//! TDT) live here in host Rust regardless of backend; ONNX only provides the
//! encoder/decoder forward passes.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::audio::PcmAudio;

/// A completed transcription.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct Transcript {
    pub segments: Vec<TranscriptSegment>,
    /// BCP-47 tag of the detected or requested language.
    pub language: Option<String>,
}

/// One time-aligned span of recognized speech.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
pub struct TranscriptSegment {
    /// Seconds from the start of the audio.
    pub start: f64,
    /// Seconds from the start of the audio.
    pub end: f64,
    pub text: String,
    /// Diarization label once speaker attribution runs (`None` for plain ASR).
    pub speaker: Option<String>,
}

/// The ASR backend seam. Implementations are feature-gated; constructors take
/// loaded sessions from `quartermaster` and never expose them.
pub trait AsrEngine {
    fn transcribe(&self, audio: &PcmAudio) -> Result<Transcript, Error>;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("model unavailable: {0}")]
    Model(#[from] quartermaster::Error),
    #[error("audio rejected: {0}")]
    Audio(String),
}
