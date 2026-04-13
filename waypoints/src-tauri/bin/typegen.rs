use std::path::Path;
use waypoints::builders::specta;

/// Generates TypeScript bindings from Rust command/type definitions.
///
/// This binary exists because `tauri-specta` `export()` runs at the Rust level,
/// not at compile time. Running it as a separate binary avoids depending on the
/// full Tauri app startup just to regenerate bindings.
fn main() {
    let builder = specta::build();

    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../app/bindings.g.ts");
    specta::export(&builder, out.to_str().unwrap());

    println!("bindings written to {}", out.display());
}
