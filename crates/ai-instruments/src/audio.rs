//! Shared audio frontend, backend-independent: capture (cpal), file decode
//! (symphonia), resampling to 16 kHz mono, VAD, and log-mel features.
//! Written once; every audio instrument consumes it.

/// PCM audio normalized to what the instruments expect: mono f32 samples.
#[derive(Debug, Clone, PartialEq)]
pub struct PcmAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}
