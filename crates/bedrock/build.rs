//! Records the resolved `turso_core` version as `BEDROCK_ENGINE_VERSION`, so
//! a durability fault can report which engine it was built against.
//!
//! The lockfile, not the manifest, is the source: the workspace clamps both
//! the `turso` facade and `turso_core` exactly, but the lockfile is what
//! names the engine actually compiled in — it stays truthful even if the
//! clamps drift or are lifted. Neither crate exposes its version at runtime
//! (the `turso_version()` SQL function reports a SQLite-style file-header
//! stamp, not the release).

fn main() {
    // Walk up from the crate to the workspace root's lockfile.
    let lockfile = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .map(|dir| dir.join("Cargo.lock"))
        .find(|path| path.exists())
        .expect("Cargo.lock in an ancestor of CARGO_MANIFEST_DIR");
    println!("cargo:rerun-if-changed={}", lockfile.display());

    let lockfile = cargo_lock::Lockfile::load(&lockfile).expect("parse Cargo.lock");
    let engine = lockfile
        .packages
        .iter()
        .find(|package| package.name.as_str() == "turso_core")
        .expect("turso_core resolved in Cargo.lock");

    println!("cargo:rustc-env=BEDROCK_ENGINE_VERSION={}", engine.version);
}
