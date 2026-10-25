//! Bookmark built-in type - A saved link with metadata.
use super::{person, tags_feature};
use domain::icon_presets;
use domain::{Chart, ChartID, Feature, FeatureKind, Reference, Select, SelectGroup};

/// Bookmark - A saved link with tags and notes
pub fn chart() -> Chart {
    let chart = Chart::builder()
        .with_name("Bookmark")
        .with_description("A saved link with tags and notes.")
        .with_icon(icon_presets::charts::bookmark())
        .add_feature(
            Feature::builder()
                .with_name("URL")
                .with_kind(FeatureKind::URL)
                .build()
                .keyed("url"),
        )
        .add_feature(
            Feature::builder()
                .with_name("Description")
                .with_kind(FeatureKind::Text)
                .build()
                .keyed("description"),
        )
        .add_feature(
            Feature::builder()
                .with_name("Tags")
                .with_kind(Select::builder().with_unlimited_selections().build().into())
                .build()
                .keyed("tag-select"),
        )
        .add_feature(
            Feature::builder()
                .with_name("Author(s)")
                .with_kind(
                    Reference::builder()
                        .add_allowed_chart(person::type_id())
                        .with_display_avatar()
                        .build()
                        .into(),
                )
                .build()
                .keyed("authors"),
        )
        .add_feature(tags_feature())
        .build();

    Chart {
        id: type_id(),
        ..chart
    }
}

/// Get the fixed ChartID for the Bookmark type.
pub fn type_id() -> ChartID {
    ChartID::builtin("bookmark")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bookmark_chart() {
        let bookmark = chart();
        assert_eq!(bookmark.name, "Bookmark");
        assert!(bookmark.is_builtin());
        assert_eq!(bookmark.features.len(), 5);
        assert!(bookmark.features.values().any(|f| f.name == "URL"));
        assert!(bookmark.features.values().any(|f| f.name == "Tags"));
        assert!(bookmark.features.values().any(|f| f.name == "Description"));
        assert!(bookmark.features.values().any(|f| f.name == "Author(s)"));
    }

    #[test]
    fn bookmark_chart_id() {
        assert_eq!(chart().id, type_id());
    }
}
