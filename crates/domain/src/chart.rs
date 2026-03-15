pub mod ids;
pub use ids::{ChartID, FeatureID, GlyphID};

pub mod feature_kind;
pub use feature_kind::{
    Date, DateGranularity, FeatureKind, Number, Reference, Select, SelectGroup, SelectOption,
};

pub mod feature;
pub use feature::Feature;

#[allow(clippy::module_inception)]
pub mod chart;
pub use chart::Chart;

pub mod builders {
    use super::chart;
    pub use chart::ChartBuilder;

    use super::feature_kind;
    pub use feature_kind::date_builder::DateBuilder;
    pub use feature_kind::number_builder::NumberBuilder;
    pub use feature_kind::reference_builder::ReferenceBuilder;
    pub use feature_kind::select_builder::SelectBuilder;
}
