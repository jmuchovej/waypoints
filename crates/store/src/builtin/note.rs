//! Note built-in type - A simple note with rich content.
use super::tags_feature;
use domain::icon_presets;
use domain::{Chart, ChartID, Feature, FeatureKind, Select};

/// Note - A simple note with rich content
pub fn chart() -> Chart {
    let chart = Chart::builder()
        .with_name("Note")
        .with_description("A simple note for capturing thoughts and information")
        .with_icon(icon_presets::charts::note())
        .add_feature(
            Feature::builder()
                .with_name("Tags")
                .with_kind(Select::builder().with_unlimited_selections().build().into())
                .build()
                .keyed("tag-select"),
        )
        .add_feature(
            Feature::builder()
                .with_name("Source URL")
                .with_kind(FeatureKind::URL)
                .build()
                .keyed("source-url"),
        )
        .add_feature(tags_feature())
        .build();

    Chart {
        id: type_id(),
        ..chart
    }
}

/// Get the fixed TypeID for the Note type.
pub fn type_id() -> ChartID {
    ChartID::builtin("note")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_chart() {
        let note = chart();
        assert_eq!(note.name, "Note");
        assert!(note.is_builtin());
        assert_eq!(note.features.len(), 3);
        assert!(note.features.values().any(|f| f.name == "Tags"));
        assert!(note.features.values().any(|f| f.name == "Source URL"));
    }

    #[test]
    fn note_chart_id() {
        assert_eq!(chart().id, type_id());
    }
}
