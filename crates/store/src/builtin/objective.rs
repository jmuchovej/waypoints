//! Objective built-in type - A todo item with completion tracking.
use super::{person, status_multi_select, tags_feature};
use domain::{
    Chart, ChartID, Date, Feature, FeatureKind, GlyphID, Icon, Reference, Select, SelectGroup,
    SelectOption, icon_presets,
};
use std::str::FromStr;

/// Objective
pub fn chart() -> Chart {
    let status = status_multi_select();

    let priority_group = SelectGroup::builder()
        .with_name("Priority")
        .with_option(SelectOption::builder().with_name("Low").build())
        .with_option(SelectOption::builder().with_name("Medium").build())
        .with_option(SelectOption::builder().with_name("High").build())
        .build();

    let chart = Chart::builder()
        .with_name("Objective")
        .with_description("A task or todo item that can be completed")
        .with_icon(icon_presets::charts::objective())
        .add_feature(
            Feature::builder()
                .with_name("Status")
                .with_kind(status.into())
                .build(),
        )
        .add_feature(
            Feature::builder()
                .with_name("Due Date")
                .with_kind(Date::builder().build().into())
                .build(),
        )
        .add_feature(
            Feature::builder()
                .with_name("Priority")
                .with_kind(
                    Select::builder()
                        .with_group(priority_group, true.into())
                        .with_single_selection()
                        .build()
                        .into(),
                )
                .build(),
        )
        .add_feature(
            Feature::builder()
                .with_name("Assignee")
                .with_kind(
                    Reference::builder()
                        .add_allowed_chart(person::type_id())
                        .with_single_reference()
                        .with_display_inline()
                        .build()
                        .into(),
                )
                .build(),
        )
        .add_feature(tags_feature())
        .build();

    Chart {
        id: type_id(),
        ..chart
    }
}

/// Get the fixed ChartID for the Task type.
pub fn type_id() -> ChartID {
    ChartID::builtin("objective")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objective_chart() {
        let obj = chart();
        assert_eq!(obj.name, "Objective");
        assert!(obj.is_builtin());
        assert_eq!(obj.features.len(), 5);
        assert!(obj.features.values().any(|f| f.name == "Status"));
        assert!(obj.features.values().any(|f| f.name == "Due Date"));
        assert!(obj.features.values().any(|f| f.name == "Priority"));
        assert!(obj.features.values().any(|f| f.name == "Assignee"));
    }

    #[test]
    fn objective_chart_id() {
        assert_eq!(chart().id, type_id());
    }
}
