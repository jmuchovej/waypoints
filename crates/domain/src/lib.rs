//! Waypoints' core architecture for block-based thought tools.
//!
//! At it's core, any "document" in Waypoints is an ordered list of [`Marks`][M]. [`Marks`][M] come
//! in two flavors: [`Content`][M:C] and [`Landmark`][M:L].
//! 1. [`Content`][M:C] marks are atomic units of content (e.g., paragraphs, headings, etc.)
//! 1. [`Landmark`][M:L] marks are instances of [`Charts`][C] (we'll describe [`Charts`][C] in more
//!    detail later).
//!
//! While the majority of the marks in Waypoints are likely to be [`Content`][M:C], [`Charts`][C]
//! (and thereby [`Landmarks`][M:L]) provide a way to capture semantic objects rather than having
//! every "document" be treated as the same.
//!
//! More concretely, suppose we had a "Person" [`Chart`][C], one of its [`Landmarks`][M:L] would be
//! "Isaac Asimov". Similarly, if we had a "Book" [`Chart`][C], one of its [`Landmarks`][M:L] would
//! be "Foundation".
//!
//! [`Charts`][C] are, essentially, "types" (or "structs"). They represent a semantic object and can
//! be arbitrarily defined! If you want to define a new semantic class of [`Landmark`][M:L], you would
//! do this by creating a [`Chart`][C] to define that semantic class, then you could synthesize new
//! [`Landmarks`][M:L].
//!
// TODO: Remove this note once arbitrary Charts are supported
//! **NOTE:** Arbitrary [`Charts`][C] is under development as of 26 Nov 2025
//!
//! The core data models for Waypoints!
//! - [`Chart`][C] enables defining custom "schemas" (types) for various [`Landmarks`][M:L].
//! - Fundamentally, everything in Waypoints is a [`Mark`][M]. [`Marks`][M] can either be
//!   [`Content`][M:C] or a [`Landmark`][M:L].
//! - [`Terrain`][T] – suppose that [`Marks`][M] are nodes in a graph, then [`Terrain`][T] are the
//!   edges. In Waypoints, multiple edges may exist between two [`Marks`][M], thus Waypoints forms a
//!   multigraph when viewed through a graphical lens.
//!
//!
//! [M]: crate::mark::Mark
//! [C]: crate::chart::Chart
//! [T]: crate::terrain::Terrain
//! [M:L]: crate::mark::Landmark
//! [M:C]: crate::mark::Content
//! [F]: crate::chart::Feature
//! [R]: crate::mark::Reading

//! Shared utilities and types for the waypoints domain.
pub(crate) mod _shared {
    pub type Json = serde_json::Value;
    pub type DateTime = chrono::NaiveDateTime;
    pub type Uuid = uuid::Uuid;

    pub(crate) mod color;
    pub(crate) mod error;
    pub mod fractional_index;
    pub(crate) mod icon;

    pub use color::Color;
    pub use error::{DomainError, DomainResult};
    pub use icon::{HasIcon, Icon, IconNamespace, presets as icon_presets};
}
pub use _shared::{
    Color, DateTime, DomainError, DomainResult, HasIcon, Icon, IconNamespace, Json, Uuid,
    fractional_index, icon_presets,
};

pub(crate) mod mark {
    #[allow(clippy::module_inception)]
    mod mark;
    pub(crate) mod mark_data;
    pub(crate) mod reading;
    pub(crate) mod rich_reference;
    pub(crate) mod rich_text;

    pub use mark::{Mark, MarkID, now_ms};
    pub use mark_data::{Content, Landmark, MarkData, MathEngine, Size};
    pub use reading::Reading;
    pub use rich_reference::{ReferenceDisplayMode, RichReference};
    pub use rich_text::{InlineMark, RichText, TextMarkKind, TextSegment};

    pub(crate) mod reading_validators;

    mod builders {
        use super::mark;
        pub use mark::MarkBuilder;
    }
    pub use builders::MarkBuilder;
}
pub use mark::{
    Content, InlineMark, Landmark, Mark, MarkBuilder, MarkData, MarkID, MathEngine, Reading,
    ReferenceDisplayMode, RichReference, RichText, Size, TextMarkKind, TextSegment, now_ms,
};

pub(crate) mod chart {
    pub(crate) mod ids;
    pub use ids::{Base36ID, ChartID, FeatureID, GlyphID};

    mod feature_kind;
    pub use feature_kind::{
        Date, DateGranularity, FeatureKind, Number, Reference, Select, SelectGroup, SelectOption,
    };

    pub mod feature;
    pub use feature::Feature;

    #[allow(clippy::module_inception)]
    mod chart;
    pub use chart::Chart;

    mod builders {
        use super::chart;
        pub use chart::ChartBuilder;

        use super::feature_kind;
        pub use feature_kind::date_builder::DateBuilder;
        pub use feature_kind::number_builder::NumberBuilder;
        pub use feature_kind::reference_builder::ReferenceBuilder;
        pub use feature_kind::select_builder::SelectBuilder;
    }
    pub use builders::{ChartBuilder, DateBuilder, NumberBuilder, ReferenceBuilder, SelectBuilder};
}
pub use chart::{
    Base36ID, Chart, ChartBuilder, ChartID, Date, DateBuilder, DateGranularity, Feature, FeatureID,
    FeatureKind, GlyphID, Number, NumberBuilder, Reference, ReferenceBuilder, Select,
    SelectBuilder, SelectGroup, SelectOption,
};

pub(crate) mod terrain {
    #[allow(clippy::module_inception)]
    mod terrain;
    pub(crate) mod terrain_id;
    pub(crate) mod terrain_kind;

    pub use terrain::Terrain;
    pub use terrain_id::TerrainID;
    pub use terrain_kind::TerrainKind;

    mod builders {
        use super::terrain;
        pub use terrain::TerrainBuilder;
    }
    pub use builders::TerrainBuilder;
}
pub use terrain::{Terrain, TerrainBuilder, TerrainID, TerrainKind};

pub(crate) mod sync {
    pub(crate) mod identity;

    pub use identity::{AccountID, DeviceID, OpID, RealmID};
}
pub use sync::{AccountID, DeviceID, OpID, RealmID};
