//! Meeting built-in type - A meeting with attendees and notes.
use super::{objective, person, status_multi_select, tags_feature};
use domain::{
    Chart, ChartID, Date, DateGranularity, Feature, FeatureKind, Icon, Reference, Select,
    SelectGroup, SelectOption,
    icon_presets::{self, charts::objective},
};

/// Meeting - A meeting with attendees, agenda, and notes
pub fn chart() -> Chart {
    let status = status_multi_select();

    let chart = Chart::builder()
        .with_name("Meeting")
        .with_description("A meeting with attendees, agenda, and notes")
        .with_icon(icon_presets::charts::meeting())
        .add_feature(
            Feature::builder()
                .with_name("Date & Time")
                .with_kind(
                    Date::builder()
                        .with_granularity(DateGranularity::Minute)
                        .build()
                        .into(),
                )
                .build()
                .keyed("date-time"),
        )
        .add_feature(
            Feature::builder()
                .with_name("Status")
                .with_kind(status.into())
                .build()
                .keyed("status"),
        )
        .add_feature(
            Feature::builder()
                .with_name("Agenda")
                .with_kind(Reference::builder().with_display_card_sm().build().into())
                .build()
                .keyed("agenda"),
        )
        .add_feature(
            Feature::builder()
                .with_name("Attendees")
                .with_kind(
                    Reference::builder()
                        .add_allowed_chart(person::type_id())
                        .with_display_avatar()
                        .build()
                        .into(),
                )
                .build()
                .keyed("attendees"),
        )
        .add_feature(
            Feature::builder()
                .with_name("Action Items")
                .with_kind(
                    Reference::builder()
                        .add_allowed_chart(objective::type_id())
                        .with_display_card_sm()
                        .build()
                        .into(),
                )
                .build()
                .keyed("action-items"),
        )
        .add_feature(
            Feature::builder()
                .with_name("Location")
                .with_kind(FeatureKind::Text)
                .build()
                .keyed("location"),
        )
        .add_feature(tags_feature())
        .build();

    Chart {
        id: type_id(),
        ..chart
    }
}

/// Get the fixed TypeID for the Meeting type.
pub fn type_id() -> ChartID {
    ChartID::builtin("meeting")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meeting_chart() {
        let meeting = chart();
        assert_eq!(meeting.name, "Meeting");
        assert!(meeting.is_builtin());
        assert_eq!(meeting.features.len(), 7);
        assert!(meeting.features.values().any(|f| f.name == "Date & Time"));
        assert!(meeting.features.values().any(|f| f.name == "Status"));
        assert!(meeting.features.values().any(|f| f.name == "Attendees"));
        assert!(meeting.features.values().any(|f| f.name == "Agenda"));
        assert!(meeting.features.values().any(|f| f.name == "Action Items"));
        assert!(meeting.features.values().any(|f| f.name == "Location"));
    }

    #[test]
    fn meeting_chart_id() {
        assert_eq!(chart().id, type_id());
    }
}
