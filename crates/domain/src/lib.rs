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

mod _shared;
pub mod chart;
pub mod mark;
pub mod terrain;

pub use _shared::fractional_index;
pub use _shared::*;
pub use chart::builders::{
    ChartBuilder, DateBuilder, NumberBuilder, ReferenceBuilder, SelectBuilder,
};
pub use chart::{
    Chart, ChartID, Date, DateGranularity, Feature, FeatureID, FeatureKind, GlyphID, Number,
    Reference, Select, SelectGroup, SelectOption,
};
pub use mark::builders::MarkBuilder;
pub use mark::{Mark, MarkID};
pub use terrain::builders::TerrainBuilder;
pub use terrain::{Terrain, TerrainID, TerrainKind};
