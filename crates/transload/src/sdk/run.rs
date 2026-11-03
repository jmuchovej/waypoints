//! What the exports do, as plain functions over bytes: the manifest, the
//! lifecycle, the finalize-time checks, and the census. The `importer!`
//! macro's exports are thin wrappers over a [`Runtime`]; a plugin's host-side
//! tests drive a [`Runtime`] directly.

use std::collections::{BTreeMap, BTreeSet};

use super::importer::{Importer, Manifested};
use crate::{
    abi::{Diagnostic, Envelope, FinalizeOutput, Init, Output, Summary},
    bundle::Census,
    fragment::{ContentFragment, Fragments, SourceKey},
};

/// The live instances of one plugin: one per `init`, each addressed by the
/// state handle `init` returned. A run uses one.
pub struct Runtime<T> {
    slots: Vec<Option<Slot<T>>>,
}

struct Slot<T> {
    importer: T,
    seen: Seen,
}

impl<T> Default for Runtime<T> {
    fn default() -> Self {
        Self { slots: Vec::new() }
    }
}

impl<T: Importer + Manifested> Runtime<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// The manifest JSON. A manifest the SDK cannot serve is a panic — there
    /// is no envelope to carry the error, and a plugin's native tests catch it
    /// first through [`Importer::manifest`].
    pub fn manifest(&self) -> Vec<u8> {
        let manifest = T::manifest().unwrap_or_else(|error| panic!("{error}"));
        serde_json::to_vec(&manifest).expect("a manifest is JSON")
    }

    /// `wp_init`: configuration JSON in, `{"ok": {"state": …}}` or `{"err": …}` out.
    pub fn init(&mut self, config: &[u8]) -> Vec<u8> {
        let result = (|| {
            let config: T::Config = serde_json::from_slice(config).map_err(|error| {
                Diagnostic::error(format!("configuration is not valid: {error}"))
            })?;
            let importer = T::init(config)?;
            let slot = Slot {
                importer,
                seen: Seen::default(),
            };
            let state = self.store(slot);
            Ok(Init { state })
        })();
        envelope(result)
    }

    /// `wp_entry`: one entry in, `{"ok": <output>}` or `{"err": …}` out.
    pub fn entry(&mut self, state: u32, name: &str, bytes: &[u8]) -> Vec<u8> {
        let result = (|| {
            let slot = self.slot(state)?;
            let output = slot.importer.entry(name, bytes)?;
            slot.seen.record(&output.fragments);
            Ok(output)
        })();
        envelope(result)
    }

    /// `wp_finalize`: the handle is consumed; `{"ok": <finalize output>}` or
    /// `{"err": …}` out. The SDK's protochart checks run after the plugin's
    /// own `finalize`, over everything the run emitted.
    pub fn finalize(&mut self, state: u32) -> Vec<u8> {
        let result = (|| {
            let Slot { importer, mut seen } = self.take(state)?;
            let output = importer.finalize()?;
            seen.record(&output.fragments);
            seen.check()?;
            Ok(FinalizeOutput::stamp(output, seen.census, seen.summary))
        })();
        envelope(result)
    }

    fn store(&mut self, slot: Slot<T>) -> u32 {
        if let Some(index) = self.slots.iter().position(Option::is_none) {
            self.slots[index] = Some(slot);
            return index as u32 + 1;
        }
        self.slots.push(Some(slot));
        self.slots.len() as u32
    }

    fn slot(&mut self, state: u32) -> Result<&mut Slot<T>, Diagnostic> {
        state
            .checked_sub(1)
            .and_then(|index| self.slots.get_mut(index as usize))
            .and_then(Option::as_mut)
            .ok_or_else(|| {
                Diagnostic::error(format!("state handle {state} names no live instance"))
            })
    }

    fn take(&mut self, state: u32) -> Result<Slot<T>, Diagnostic> {
        state
            .checked_sub(1)
            .and_then(|index| self.slots.get_mut(index as usize))
            .and_then(Option::take)
            .ok_or_else(|| {
                Diagnostic::error(format!("state handle {state} names no live instance"))
            })
    }
}

fn envelope<P: serde::Serialize>(result: Result<P, Diagnostic>) -> Vec<u8> {
    serde_json::to_vec(&Envelope::from(result)).expect("an envelope is JSON")
}

/// What the SDK remembers across a run, for the finalize-time checks, the
/// census, and the summary.
#[derive(Default)]
struct Seen {
    /// Each protochart's features, each feature's option and group keys.
    protocharts: BTreeMap<SourceKey, BTreeMap<SourceKey, BTreeSet<SourceKey>>>,
    /// Each reference feature's allowed protocharts, by protochart and feature.
    allowed: Vec<(SourceKey, SourceKey, Vec<SourceKey>)>,
    /// Each Landmark's binding and the reading keys and values it used.
    landmarks: Vec<LandmarkUse>,
    census: Census,
    summary: Summary,
}

struct LandmarkUse {
    src: SourceKey,
    protochart: Option<SourceKey>,
    readings: Vec<(SourceKey, Vec<SourceKey>)>,
}

impl Seen {
    fn record(&mut self, fragments: &Fragments) {
        fragments.tally(&mut self.census);
        self.summary.landmarks += fragments.landmarks.len();
        self.summary.protocharts += fragments.protocharts.len();
        self.summary.terrain += fragments.terrain.len();
        for protochart in &fragments.protocharts {
            let features = self.protocharts.entry(protochart.src.clone()).or_default();
            for (feature, definition) in &protochart.features {
                features.insert(
                    feature.clone(),
                    definition.kind.option_keys().into_iter().cloned().collect(),
                );
                let allowed = definition.kind.allowed_protocharts();
                if !allowed.is_empty() {
                    self.allowed
                        .push((protochart.src.clone(), feature.clone(), allowed.to_vec()));
                }
            }
        }
        for landmark in &fragments.landmarks {
            self.summary.content += count_content(&landmark.body);
            self.landmarks.push(LandmarkUse {
                src: landmark.src.clone(),
                protochart: landmark.protochart.clone(),
                readings: landmark
                    .readings
                    .iter()
                    .map(|(feature, reading)| {
                        (
                            feature.clone(),
                            reading.option_keys().into_iter().cloned().collect(),
                        )
                    })
                    .collect(),
            });
        }
    }

    /// Every protochart key, reading key, select value, and allowed
    /// protochart names something the run defined. Mark references are not
    /// checked: a key this run never defined may be known from an earlier one.
    fn check(&self) -> Result<(), Diagnostic> {
        for landmark in &self.landmarks {
            let Some(protochart) = &landmark.protochart else {
                if let Some((feature, _)) = landmark.readings.first() {
                    return Err(Diagnostic::error(format!(
                        "Landmark `{}` binds no protochart but carries a reading `{feature}`",
                        landmark.src
                    )));
                }
                continue;
            };
            let Some(features) = self.protocharts.get(protochart) else {
                return Err(Diagnostic::error(format!(
                    "Landmark `{}` binds protochart `{protochart}`, which this run never defined",
                    landmark.src
                )));
            };
            for (feature, values) in &landmark.readings {
                let Some(options) = features.get(feature) else {
                    return Err(Diagnostic::error(format!(
                        "Landmark `{}` reads feature `{feature}`, which protochart `{protochart}` does not define",
                        landmark.src
                    )));
                };
                if let Some(value) = values.iter().find(|value| !options.contains(*value)) {
                    return Err(Diagnostic::error(format!(
                        "Landmark `{}` selects `{value}` for feature `{feature}`, which protochart `{protochart}` does not offer",
                        landmark.src
                    )));
                }
            }
        }
        for (protochart, feature, allowed) in &self.allowed {
            if let Some(missing) = allowed
                .iter()
                .find(|key| !self.protocharts.contains_key(*key))
            {
                return Err(Diagnostic::error(format!(
                    "protochart `{protochart}` feature `{feature}` allows protochart `{missing}`, which this run never defined"
                )));
            }
        }
        Ok(())
    }
}

fn count_content(blocks: &[ContentFragment]) -> usize {
    blocks
        .iter()
        .map(|block| 1 + count_content(&block.children))
        .sum()
}
