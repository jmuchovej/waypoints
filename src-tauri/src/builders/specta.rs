use specta_typescript::Typescript;
use tauri_specta::{Builder, collect_commands};

use waypoints_domain::{
    Color, Icon, IconNamespace,
    chart::{
        Chart, ChartID, Feature, FeatureID, FeatureKind, GlyphID,
        feature_kind::{SelectGroup, SelectOption},
        ids::Base36ID,
    },
    mark::{
        Content, Landmark, Mark, MarkData, MarkID, Reading, ReferenceDisplayMode, RichReference,
        RichText, TextMark, TextMarkKind,
        mark_data::{MathEngine, Size},
    },
    terrain::{Terrain, TerrainKind},
};

pub(crate) fn builder() -> Builder<tauri::Wry> {
    let builder = Builder::<tauri::Wry>::new()
        .commands(collect_commands![])
        // _shared
        .typ::<Color>()
        .typ::<Icon>()
        .typ::<IconNamespace>()
        // chart
        .typ::<Base36ID>()
        .typ::<ChartID>()
        .typ::<FeatureID>()
        .typ::<GlyphID>()
        .typ::<Chart>()
        .typ::<Feature>()
        .typ::<FeatureKind>()
        .typ::<SelectOption>()
        .typ::<SelectGroup>()
        // mark
        .typ::<MarkID>()
        .typ::<Mark>()
        .typ::<MarkData>()
        .typ::<Content>()
        .typ::<Size>()
        .typ::<MathEngine>()
        .typ::<Landmark>()
        .typ::<Reading>()
        .typ::<RichText>()
        .typ::<TextMark>()
        .typ::<TextMarkKind>()
        .typ::<RichReference>()
        .typ::<ReferenceDisplayMode>()
        // terrain
        .typ::<Terrain>()
        .typ::<TerrainKind>();

    #[cfg(debug_assertions)]
    builder
        .export(Typescript::default(), "../src/app/bindings.g.ts")
        .expect("failed to export specta bindings");

    builder
}
