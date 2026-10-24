//! Project built-in type - A project with timeline and team.
use super::{person, status_multi_select, tags_feature};
use domain::{
    Chart, ChartID, Color, Date, Feature, FeatureKind, GlyphID, Icon, Number, Reference, Select,
    SelectBuilder, SelectGroup, SelectOption, icon_presets,
};
use std::str::FromStr;

/// Project - A project with timeline, team, and deliverables
pub fn chart() -> Chart {
    let status = status_multi_select();

    let chart = Chart::builder()
        .with_name("Campaign")
        .with_description("Campaigns group objectives into timelines and deliverables")
        .with_icon(icon_presets::charts::charter())
        .add_feature(
            Feature::builder()
                .with_name("Status")
                .with_kind(status.into())
                .build()
                .keyed("status"),
        )
        .add_feature(
            Feature::builder()
                .with_name("Start Date")
                .with_kind(Date::builder().build().into())
                .build()
                .keyed("start-date"),
        )
        .add_feature(
            Feature::builder()
                .with_name("End Date")
                .with_kind(Date::builder().build().into())
                .build()
                .keyed("end-date"),
        )
        .add_feature(
            Feature::builder()
                .with_name("Team Members")
                .with_kind(
                    Reference::builder()
                        .add_allowed_chart(person::type_id())
                        .with_display_avatar()
                        .build()
                        .into(),
                )
                .build()
                .keyed("team-members"),
        )
        .add_feature(tags_feature())
        .build();

    Chart {
        id: type_id(),
        ..chart
    }
}

/// Get the fixed ChartID for the Project type.
pub fn type_id() -> ChartID {
    ChartID::builtin("campaign")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campaign_chart() {
        let campaign = chart();
        assert_eq!(campaign.name, "Campaign");
        assert!(campaign.is_builtin());
        assert_eq!(campaign.features.len(), 5);
        assert!(campaign.features.values().any(|f| f.name == "Status"));
        assert!(campaign.features.values().any(|f| f.name == "Start Date"));
        assert!(campaign.features.values().any(|f| f.name == "End Date"));
        assert!(campaign.features.values().any(|f| f.name == "Team Members"));
    }

    #[test]
    fn campaign_chart_id() {
        assert_eq!(chart().id, type_id());
    }
}
