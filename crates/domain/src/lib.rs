// SPDX-FileCopyrightText: 2025 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of Waypoints.
//
// Waypoints is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Waypoints is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with Waypoints. If not, see <https://www.gnu.org/licenses/>.
//
// Additional permission under GNU AGPL version 3 section 7: this Program
// may be conveyed through application stores on the terms set out in
// docs/licensing/COPYING.exceptions.

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

// Features are additive and flavors are not: one shipped flavor per build.
#[cfg(all(feature = "nightly", feature = "preview"))]
compile_error!(
    "domain: the `nightly` and `preview` flavors are both enabled; a build takes exactly one"
);
#[cfg(all(feature = "nightly", feature = "stable"))]
compile_error!(
    "domain: the `nightly` and `stable` flavors are both enabled; a build takes exactly one"
);
#[cfg(all(feature = "preview", feature = "stable"))]
compile_error!(
    "domain: the `preview` and `stable` flavors are both enabled; a build takes exactly one"
);

/// This crate's version — the grammar signal a Bundle records.
///
/// Any new `Content`, `Reading`, `FeatureKind`, or `TerrainKind` variant bumps
/// this, so it bounds what a writing build could have expressed (not tightly:
/// unrelated changes bump it too).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

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
    pub use mark_data::{BlockTheme, Content, Landmark, MarkData, MathEngine, Size};
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
    BlockTheme, Content, InlineMark, Landmark, Mark, MarkBuilder, MarkData, MarkID, MathEngine,
    Reading, ReferenceDisplayMode, RichText, Size, TextMarkKind, TextSegment, now_ms,
};

/// The markdown dialect `Content` and `RichText` project into, documented at
/// `docs/dialect.md`. Behind the `syntax` feature so other dependents skip it.
#[cfg(feature = "syntax")]
pub mod syntax {
    pub mod block;
    pub mod inline;
}

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
