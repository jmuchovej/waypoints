//! Page built-in type - A container for other blocks.
use super::tags_feature;
use waypoints_domain::{
    chart::{Chart, ChartID, Feature, Reference},
    icon_presets,
};

pub fn chart() -> Chart {
    let chart = Chart::builder()
        .with_name("Page")
        .with_description("The plainest document you could ever have!")
        .with_icon(icon_presets::charts::page())
        .build();

    Chart {
        id: type_id(),
        ..chart
    }
}

pub fn type_id() -> ChartID {
    ChartID::builtin("page")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_chart() {
        let page = chart();
        assert_eq!(page.name, "Page");
        assert_eq!(page.features.len(), 0); // Page has no custom fields - base fields are on Thing struct

        // Page should have no additional fields beyond the base Thing structure
        // (title, icon, children are guaranteed on every Thing instance)

        // Should have icon set
        assert_eq!(page.icon, Some(icon_presets::charts::page()));
        assert!(page.icon.is_some());
    }

    #[test]
    fn test_page_chart_id() {
        assert_eq!(chart().id, type_id());
    }

    #[test]
    fn test_page_icon_system() {
        use waypoints_domain::Icon;

        let page = chart();

        // Test that page has correct icon
        assert_eq!(page.icon, Some(icon_presets::charts::page()));

        // Demonstrate creating custom page with different icon types
        let custom_page = Chart::builder()
            .with_name("Custom Page")
            .with_icon(Icon::system_emoji("📋")) // Emoji icon
            .build();

        assert_eq!(custom_page.icon, Some(Icon::system_emoji("📋")));
    }
}
