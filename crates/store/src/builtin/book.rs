//! Bookmark built-in type - A saved link with metadata.
use super::{person, tags_feature};
use domain::icon_presets;
use domain::{Chart, ChartID, Feature, FeatureKind, Reference, Select, SelectGroup};

/// Bookmark - A saved link with tags and notes
pub fn chart() -> Chart {
    let chart = Chart::builder()
        .with_name("Book")
        .with_description("The dead-tree version of PDFs!.")
        .with_icon(icon_presets::charts::bookmark())
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
                .build(),
        )
        // .add_feature(
        //     Feature::builder()
        //         .with_name("PDF")
        //         .with_kind(FeatureKind::File)
        //         .build(),
        // )
        .add_feature(
            Feature::builder()
                .with_name("Summary")
                .with_kind(FeatureKind::Text)
                .build(),
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
    ChartID::builtin("book")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bookmark_chart() {
        let bookmark = chart();
        assert_eq!(bookmark.name, "Book");
        assert!(bookmark.is_builtin());
        assert_eq!(bookmark.features.len(), 4);
        assert!(bookmark.features.values().any(|f| f.name == "Author(s)"));
        assert!(bookmark.features.values().any(|f| f.name == "Summary"));
        // assert!(bookmark.features.values().any(|f| f.name == "Files"));
        assert!(bookmark.features.values().any(|f| f.name == "Tags"));
    }

    #[test]
    fn bookmark_chart_id() {
        assert_eq!(chart().id, type_id());
    }
}
