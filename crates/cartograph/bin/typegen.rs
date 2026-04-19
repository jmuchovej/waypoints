use specta::TypeCollection;
use specta_typescript::Typescript;

use cartograph::territory::{Territory, TerritoryKind};
use cartograph::{
    CreateResult, CursorState, MutationResult, RenderNode, SplitResult, UndoRedoResult,
};

fn main() {
    let mut collection = TypeCollection::default();

    // territory
    collection.register::<Territory>();
    collection.register::<TerritoryKind>();

    // editor
    collection.register::<RenderNode>();
    collection.register::<MutationResult>();
    collection.register::<CreateResult>();
    collection.register::<SplitResult>();
    collection.register::<CursorState>();
    collection.register::<UndoRedoResult>();

    // Note: TerritoryError, MarkError, and EditorError manually implement
    // specta::Type as PrimitiveType::String (they serialize as error message
    // strings). They don't implement NamedType, so they can't be registered
    // here — and they'd just export as `type XxxError = string` anyway.
    // They're inlined automatically in bindings.g.ts by tauri-specta.

    let out = format!("{}/typescript/types.g.ts", env!("CARGO_MANIFEST_DIR"));
    Typescript::default()
        .export_to(&out, &collection)
        .expect("failed to export cartograph types");

    println!("wrote {out}");
}
