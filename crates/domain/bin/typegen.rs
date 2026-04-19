use specta::TypeCollection;
use specta_typescript::Typescript;

use domain::{
    Color, Icon, IconNamespace,
    chart::{
        Chart, ChartID, DateGranularity, Feature, FeatureID, FeatureKind, GlyphID,
        feature_kind::{SelectGroup, SelectOption},
        ids::Base36ID,
    },
    mark::{
        Content, InlineMark, Landmark, Mark, MarkData, MarkID, Reading, ReferenceDisplayMode,
        RichReference, RichText, TextMarkKind, TextSegment,
        mark_data::{MathEngine, Size},
    },
    terrain::{Terrain, TerrainKind},
};

fn main() {
    let mut collection = TypeCollection::default();

    // _shared
    collection.register::<Color>();
    collection.register::<Icon>();
    collection.register::<IconNamespace>();

    // chart
    collection.register::<Base36ID>();
    collection.register::<ChartID>();
    collection.register::<FeatureID>();
    collection.register::<GlyphID>();
    collection.register::<Chart>();
    collection.register::<Feature>();
    collection.register::<FeatureKind>();
    collection.register::<DateGranularity>();
    collection.register::<SelectOption>();
    collection.register::<SelectGroup>();

    // mark
    collection.register::<MarkID>();
    collection.register::<Mark>();
    collection.register::<MarkData>();
    collection.register::<Content>();
    collection.register::<Size>();
    collection.register::<MathEngine>();
    collection.register::<Landmark>();
    collection.register::<Reading>();
    collection.register::<RichText>();
    collection.register::<TextSegment>();
    collection.register::<InlineMark>();
    collection.register::<TextMarkKind>();
    collection.register::<RichReference>();
    collection.register::<ReferenceDisplayMode>();

    // terrain
    collection.register::<Terrain>();
    collection.register::<TerrainKind>();

    let out = format!("{}/typescript/types.g.ts", env!("CARGO_MANIFEST_DIR"));
    Typescript::default()
        .export_to(&out, &collection)
        .expect("failed to export domain types");

    simplify_json_value(&out);

    println!("wrote {out}");
}

/// Rewrite the generated `export type JsonValue = …` declaration to `unknown`.
fn simplify_json_value(path: &str) {
    let contents = std::fs::read_to_string(path).expect("read generated types.g.ts");
    let anchor = "export type JsonValue =";
    let start = contents
        .find(anchor)
        .expect("JsonValue declaration not found in generated output");
    // Specta emits each type alias on a single line (and without a trailing
    // semicolon — that's added later by treefmt), so replace through the end of
    // the declaration's line. (Matching on `;` is unsafe: the next type's doc
    // comment contains `&ndash;`, whose entity semicolon would match first.)
    let end = start
        + contents[start..]
            .find('\n')
            .unwrap_or(contents.len() - start);
    let rewritten = format!(
        "{}export type JsonValue = unknown;{}",
        &contents[..start],
        &contents[end..],
    );
    std::fs::write(path, rewritten).expect("write simplified types.g.ts");
}
