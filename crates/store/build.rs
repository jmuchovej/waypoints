//! Embeds the getting-started collection: every file under
//! `src/collections/getting-started/`, keyed by its Bundle path.

// Cargo reads build-script directives from stdout.
#![allow(clippy::disallowed_macros)]

use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read the collection") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/collections/getting-started");
    println!("cargo::rerun-if-changed={}", root.display());

    let mut found = Vec::new();
    files(&root, &mut found);
    found.sort();

    let mut table = String::from("pub(crate) const FILES: &[(&str, &str)] = &[\n");
    for path in found {
        let relative = path.strip_prefix(&root).expect("under the root");
        table.push_str(&format!(
            "    ({:?}, include_str!({:?})),\n",
            relative.to_string_lossy().replace('\\', "/"),
            path.display().to_string(),
        ));
    }
    table.push_str("];\n");

    let out = Path::new(&env::var("OUT_DIR").expect("OUT_DIR")).join("getting_started.rs");
    fs::write(out, table).expect("write the table");
}
