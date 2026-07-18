use specta::Types;
use specta_typescript::Typescript;

use cartograph::sync::{
    IdentityCreated, PairingCode, SetupCode, StationStatus, SyncDevice, SyncStatus,
};
use cartograph::territory::{Territory, TerritoryKind};
use cartograph::{
    CreateResult, CursorState, MutationResult, RenderNode, SplitResult, UndoRedoResult,
};

fn main() {
    let mut collection = Types::default();

    // territory
    collection.register_mut::<Territory>();
    collection.register_mut::<TerritoryKind>();

    // sync
    collection.register_mut::<SyncStatus>();
    collection.register_mut::<IdentityCreated>();
    collection.register_mut::<StationStatus>();
    collection.register_mut::<SyncDevice>();
    collection.register_mut::<PairingCode>();
    collection.register_mut::<SetupCode>();

    // editor
    collection.register_mut::<RenderNode>();
    collection.register_mut::<MutationResult>();
    collection.register_mut::<CreateResult>();
    collection.register_mut::<SplitResult>();
    collection.register_mut::<CursorState>();
    collection.register_mut::<UndoRedoResult>();

    // Note: TerritoryError, MarkError, and EditorError manually implement
    // specta::Type as PrimitiveType::String (they serialize as error message
    // strings). They don't implement NamedType, so they can't be registered
    // here — and they'd just export as `type XxxError = string` anyway.
    // They're inlined automatically in bindings.g.ts by tauri-specta.

    let out = format!("{}/typescript/types.g.ts", env!("CARGO_MANIFEST_DIR"));
    Typescript::default()
        .export_to(&out, &collection, specta_serde::PhasesFormat)
        .expect("failed to export cartograph types");

    collapse_phase_aliases(&out);
    println!("wrote {out}");
}

/// specta-serde's `PhasesFormat` emits `X = X_Serialize | X_Deserialize` for every
/// type whose serialize/deserialize shapes diverge. The frontend only ever holds
/// serialize-shaped data (the backend serializes across the IPC/HTTP boundary), so
/// collapse each base alias to its serialize phase — otherwise the union blocks
/// property access on divergent types. The `_Deserialize` types remain exported
/// for code that constructs send-payloads.
fn collapse_phase_aliases(path: &str) {
    let contents = std::fs::read_to_string(path).expect("read generated types.g.ts");
    let collapsed = contents
        .lines()
        .map(|line| {
            if let Some(rest) = line.strip_prefix("export type ")
                && let Some((name, rhs)) = rest.split_once(" = ")
                && rhs == format!("{name}_Serialize | {name}_Deserialize;")
            {
                format!("export type {name} = {name}_Serialize;")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(path, collapsed).expect("write collapsed types.g.ts");
}
