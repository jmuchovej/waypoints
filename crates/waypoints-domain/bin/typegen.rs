use specta::TypeCollection;
use specta_typescript::Typescript;

use waypoints_domain::{
    Color, Icon, IconNamespace,
    chart::{
        Chart, ChartID, Feature, FeatureID, FeatureKind, GlyphID,
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

    let out = format!("{}/types.g.ts", env!("CARGO_MANIFEST_DIR"));
    Typescript::default()
        .export_to(&out, &collection)
        .expect("failed to export domain types");

    println!("wrote {out}");
}
