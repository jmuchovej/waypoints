//! The plugin's own dependency graph: a nested `cargo build` for
//! `wasm32-unknown-unknown` into this crate's own target directory — the
//! only build in which the plugin's graph is the plugin's graph — and the
//! module backend over the artifact.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use plain_markdown::PlainMarkdown;
use serde_json::json;
use transload::{
    harness::{self, HarnessError, golden, module::Plugin, native},
    sdk::Importer,
};

fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn build_module() -> Vec<u8> {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| crate_dir().join("target"))
        .join("fixture-modules");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(cargo)
        .args([
            "build",
            "--locked",
            "--release",
            "-p",
            "plain-markdown",
            "--target",
            "wasm32-unknown-unknown",
        ])
        .env("CARGO_TARGET_DIR", &target)
        .current_dir(crate_dir())
        .status()
        .expect("cargo runs");
    assert!(status.success(), "the nested build failed");
    let module = target.join("wasm32-unknown-unknown/release/plain_markdown.wasm");
    fs::read(&module).expect("the module was built")
}

fn entries() -> Vec<harness::Entry> {
    let manifest = PlainMarkdown::manifest().unwrap();
    harness::entries(&crate_dir().join("fixtures"), &manifest.source).unwrap()
}

#[test]
fn the_module_has_the_shape_and_agrees_with_the_native_run() {
    let bytes = build_module();
    let mut plugin = Plugin::load(&bytes).unwrap();
    assert_eq!(plugin.panic(), None);

    let config = json!({"title_from": "heading"});
    let from_module = plugin.run(&config, &entries()).unwrap();
    let from_native = native::run::<PlainMarkdown>(&config, &entries()).unwrap();
    assert_eq!(from_native.differences(&from_module), []);
    golden::check(&crate_dir().join("goldens"), &from_module).unwrap();
}

#[test]
fn a_panic_traps_and_wp_panic_keeps_its_message() {
    let bytes = build_module();
    let mut plugin = Plugin::load(&bytes).unwrap();
    let config = json!({"title_from": "heading", "fail_on_entry": "alpha.md"});
    let error = plugin.run(&config, &entries()).unwrap_err();
    let HarnessError::Trap { call, panic, .. } = &error else {
        panic!("{error}");
    };
    assert_eq!(call, "wp_entry");
    let panic = panic.as_deref().unwrap();
    assert!(panic.contains("asked to fail on `alpha.md`"), "{panic}");
    assert!(panic.contains("src/lib.rs"), "{panic}");
}

#[test]
fn something_that_is_not_a_plugin_is_reported() {
    let Err(error) = Plugin::load(b"not wasm") else {
        panic!("bytes that are not a module loaded");
    };
    assert!(matches!(error, HarnessError::Module(_)), "{error}");
}
