//! Artifact acquisition: `hf-hub` download on first use, with progress
//! reporting surfaced to the UI, then fully offline. Materializes a
//! [`crate::ModelSpec`]'s files into the cache.
//!
//! Weights are *data*, so runtime download is app-store-compliant; the
//! inference runtime itself (`libonnxruntime`) must ship in the app bundle.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use hf_hub::api::sync::{Api, ApiBuilder};
use hf_hub::{Cache, Repo, RepoType};

use crate::Error;
use crate::registry::ModelSpec;

/// Download progress observer. Mirrors hf-hub's shape so that crate stays an
/// implementation detail of quartermaster. Files download in
/// `ModelSpec::files` order; note hf-hub may emit `begin` more than once for
/// a single file (preflight + transfer), so treat `begin` as "reset the bar",
/// not as a file counter.
pub trait Progress {
    /// A file download is starting.
    fn begin(&mut self, filename: &str, total_bytes: usize);
    /// `delta_bytes` more of the current file have arrived.
    fn advance(&mut self, delta_bytes: usize);
    /// The current file finished downloading.
    fn complete(&mut self);
}

/// No-op observer for callers that don't surface progress.
impl Progress for () {
    fn begin(&mut self, _filename: &str, _total_bytes: usize) {}
    fn advance(&mut self, _delta_bytes: usize) {}
    fn complete(&mut self) {}
}

/// Adapts our [`Progress`] to the hf-hub trait, which is taken by value.
struct Forward<'a, P: Progress>(&'a mut P);

impl<P: Progress> hf_hub::api::Progress for Forward<'_, P> {
    fn init(&mut self, size: usize, filename: &str) {
        self.0.begin(filename, size);
    }

    fn update(&mut self, size: usize) {
        self.0.advance(size);
    }

    fn finish(&mut self) {
        self.0.complete();
    }
}

/// The on-disk artifacts of a materialized [`ModelSpec`].
#[derive(Debug, Clone)]
pub struct ModelArtifacts {
    files: HashMap<String, PathBuf>,
}

impl ModelArtifacts {
    /// Path of one artifact, keyed by its name in `ModelSpec::files`.
    pub fn file(&self, name: &str) -> Option<&Path> {
        self.files.get(name).map(PathBuf::as_path)
    }
}

/// Synchronous Hub client. Downloads block; run [`Hub::ensure`] on a
/// blocking thread (`tauri::async_runtime::spawn_blocking` or equivalent)
/// when calling from an async context.
pub struct Hub {
    api: Api,
    cache: Cache,
}

impl Hub {
    /// `cache_dir` is where artifacts land, laid out in hf-hub's cache
    /// format (see [`crate::cache`] for the default location).
    pub fn new(cache_dir: PathBuf) -> Result<Self, Error> {
        let api = ApiBuilder::new()
            .with_cache_dir(cache_dir.clone())
            .with_progress(false)
            .build()
            .map_err(|e| Error::Fetch(e.to_string()))?;
        Ok(Self {
            api,
            cache: Cache::new(cache_dir),
        })
    }

    /// Ensure every file of `spec` is present locally, downloading whatever
    /// is missing at the spec's pinned revision. Already-cached files are
    /// returned without touching the network, so this is a no-op (and fully
    /// offline) once a model is materialized.
    pub fn ensure<P: Progress>(
        &self,
        spec: &ModelSpec,
        progress: &mut P,
    ) -> Result<ModelArtifacts, Error> {
        let repo = Repo::with_revision(spec.repo.clone(), RepoType::Model, spec.revision.clone());
        let cached = self.cache.repo(repo.clone());
        let remote = self.api.repo(repo);

        let mut files = HashMap::with_capacity(spec.files.len());
        for name in &spec.files {
            let path = match cached.get(name) {
                Some(path) => path,
                None => remote
                    .download_with_progress(name, Forward(progress))
                    .map_err(|e| Error::Fetch(format!("{}/{name}: {e}", spec.repo)))?,
            };
            files.insert(name.clone(), path);
        }
        Ok(ModelArtifacts { files })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{License, ModelID, ModelSpec};

    #[test]
    #[ignore = "network: downloads from huggingface.co"]
    fn ensure_downloads_and_then_hits_cache() {
        let tmp = tempfile::tempdir().expect("temp cache dir");
        let tmp = tmp.path().to_path_buf();
        let spec = ModelSpec {
            id: ModelID("tiny-random-bert".into()),
            repo: "hf-internal-testing/tiny-random-bert".into(),
            revision: "main".into(),
            files: vec!["config.json".into()],
            license: License::Apache2,
        };

        struct Counts {
            begun: usize,
            finished: usize,
        }
        impl Progress for Counts {
            fn begin(&mut self, _: &str, _: usize) {
                self.begun += 1;
            }
            fn advance(&mut self, _: usize) {}
            fn complete(&mut self) {
                self.finished += 1;
            }
        }

        let hub = Hub::new(tmp).expect("hub construction");
        let mut counts = Counts {
            begun: 0,
            finished: 0,
        };
        let artifacts = hub.ensure(&spec, &mut counts).expect("download");
        let config = artifacts.file("config.json").expect("artifact path");
        assert!(config.is_file());
        assert!(counts.begun >= 1);
        assert!(counts.finished >= 1);

        // Second call must resolve from cache without re-downloading.
        let mut recounts = Counts {
            begun: 0,
            finished: 0,
        };
        let again = hub.ensure(&spec, &mut recounts).expect("cache hit");
        assert_eq!(again.file("config.json"), Some(config));
        assert_eq!(recounts.begun, 0);
    }
}
