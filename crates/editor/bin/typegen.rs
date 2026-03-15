use specta::TypeCollection;
use specta_typescript::Typescript;

use editor::{CreateResult, CursorState, MutationResult, RenderNode, SplitResult, UndoRedoResult};

fn main() {
    let mut collection = TypeCollection::default();

    collection.register::<RenderNode>();
    collection.register::<CursorState>();
    collection.register::<MutationResult>();
    collection.register::<SplitResult>();
    collection.register::<CreateResult>();
    collection.register::<UndoRedoResult>();

    let out = format!("{}/types.g.ts", env!("CARGO_MANIFEST_DIR"));
    Typescript::default()
        .export_to(&out, &collection)
        .expect("failed to export editor types");

    println!("wrote {out}");
}
