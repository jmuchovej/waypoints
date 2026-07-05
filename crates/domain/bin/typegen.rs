use specta::Types;
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
    sync::{AccountID, DeviceID, HLC, OpID, RealmID},
    terrain::{Terrain, TerrainKind},
};

fn main() {
    let mut collection = Types::default();

    // _shared
    collection.register_mut::<Color>();
    collection.register_mut::<Icon>();
    collection.register_mut::<IconNamespace>();

    // chart
    collection.register_mut::<Base36ID>();
    collection.register_mut::<ChartID>();
    collection.register_mut::<FeatureID>();
    collection.register_mut::<GlyphID>();
    collection.register_mut::<Chart>();
    collection.register_mut::<Feature>();
    collection.register_mut::<FeatureKind>();
    collection.register_mut::<DateGranularity>();
    collection.register_mut::<SelectOption>();
    collection.register_mut::<SelectGroup>();

    // mark
    collection.register_mut::<MarkID>();
    collection.register_mut::<Mark>();
    collection.register_mut::<MarkData>();
    collection.register_mut::<Content>();
    collection.register_mut::<Size>();
    collection.register_mut::<MathEngine>();
    collection.register_mut::<Landmark>();
    collection.register_mut::<Reading>();
    collection.register_mut::<RichText>();
    collection.register_mut::<TextSegment>();
    collection.register_mut::<InlineMark>();
    collection.register_mut::<TextMarkKind>();
    collection.register_mut::<RichReference>();
    collection.register_mut::<ReferenceDisplayMode>();

    // terrain
    collection.register_mut::<Terrain>();
    collection.register_mut::<TerrainKind>();

    // sync
    collection.register_mut::<AccountID>();
    collection.register_mut::<RealmID>();
    collection.register_mut::<DeviceID>();
    collection.register_mut::<OpID>();
    collection.register_mut::<HLC>();

    let out = format!("{}/typescript/types.g.ts", env!("CARGO_MANIFEST_DIR"));
    Typescript::default()
        .export_to(&out, &collection, specta_serde::PhasesFormat)
        .expect("failed to export domain types");

    // `serde_json::Value` fields are annotated `#[specta(type = Option<Unknown>)]`
    // at the source, so they render directly as `unknown` — no post-processing
    // needed. (This also sidesteps specta-typescript's BigInt-forbidden guard,
    // which rejects the `i64`/`u64` inside `serde_json::Number`.)
    collapse_phase_aliases(&out);
    println!("wrote {out}");
}

/// specta-serde's `PhasesFormat` emits `X = X_Serialize | X_Deserialize` for every
/// type whose serialize/deserialize shapes diverge. The frontend only ever holds
/// serialize-shaped data (the backend serializes across the IPC/HTTP boundary), so
/// collapse each base alias to its serialize phase — otherwise the union blocks
/// property access on divergent types (`RichText`, `MarkData`, …). The
/// `_Deserialize` types remain exported for code that constructs send-payloads.
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
