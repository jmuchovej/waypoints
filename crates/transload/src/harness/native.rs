//! The native backend: the trait, driven directly, with the configuration
//! validated against the generated options schema first.

use domain::Json;
use serde::de::DeserializeOwned;

use super::{
    error::HarnessError,
    run::{Entry, Run},
};
use crate::{
    abi::{Envelope, FinalizeOutput, Init, Output},
    sdk::{Importer, Manifested, Runtime},
};

/// Run a plugin natively over `entries`, in the order given.
pub fn run<T: Importer + Manifested>(
    config: &Json,
    entries: &[Entry],
) -> Result<Run, HarnessError> {
    let manifest = T::manifest()?;
    check_config(&manifest.options, config)?;

    let mut runtime = Runtime::<T>::new();
    let manifest = runtime.manifest();
    let init = runtime.init(&serde_json::to_vec(config).expect("configuration is JSON"));
    let state = unwrap::<Init>("wp_init", &init)?.state;
    let mut outputs = Vec::with_capacity(entries.len());
    for entry in entries {
        let output = runtime.entry(state, &entry.name, &entry.bytes);
        unwrap::<Output>("wp_entry", &output)?;
        outputs.push((entry.name.clone(), output));
    }
    let finalize = runtime.finalize(state);
    unwrap::<FinalizeOutput>("wp_finalize", &finalize)?;
    Ok(Run {
        manifest,
        entries: outputs,
        finalize,
    })
}

/// The options schema is valid Draft-07 and the configuration satisfies it.
pub(crate) fn check_config(options: &Json, config: &Json) -> Result<(), HarnessError> {
    jsonschema::draft7::meta::validate(options)
        .map_err(|error| HarnessError::Schema(error.to_string()))?;
    let validator = jsonschema::draft7::new(options)
        .map_err(|error| HarnessError::Schema(error.to_string()))?;
    validator
        .validate(config)
        .map_err(|error| HarnessError::Config(error.to_string()))
}

/// An `ok` payload, or the call's failure.
pub(crate) fn unwrap<T: DeserializeOwned>(call: &str, bytes: &[u8]) -> Result<T, HarnessError> {
    let envelope: Envelope<T> =
        serde_json::from_slice(bytes).map_err(|error| HarnessError::Payload {
            call: call.to_string(),
            error: error.to_string(),
        })?;
    envelope
        .into_result()
        .map_err(|diagnostic| HarnessError::Plugin {
            call: call.to_string(),
            diagnostic,
        })
}
