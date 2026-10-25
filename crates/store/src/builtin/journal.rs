//! Daily Note built-in type - A daily journal entry.
use super::tags_feature;
use domain::icon_presets;
use domain::{
    Chart, ChartID, Date, Feature, FeatureKind, Icon, Reference, Select, SelectGroup, SelectOption,
};

/// Daily Note - A daily journal entry
pub fn chart() -> Chart {
    let mood_group = SelectGroup::builder()
        .with_name("Mood")
        .with_option(
            SelectOption::builder()
                .with_name("Great")
                .with_icon(Icon::system_emoji("smiling-face-with-smiling-eyes"))
                .build()
                .keyed("great"),
        )
        .with_option(
            SelectOption::builder()
                .with_name("Good")
                .with_icon(Icon::system_emoji("slightly-smiling-face"))
                .build()
                .keyed("good"),
        )
        .with_option(
            SelectOption::builder()
                .with_name("Okay")
                .with_icon(Icon::system_emoji("neutral-face"))
                .build()
                .keyed("okay"),
        )
        .with_option(
            SelectOption::builder()
                .with_name("Not Great")
                .with_icon(Icon::system_emoji("slightly-frowning-face"))
                .build()
                .keyed("not-great"),
        )
        .with_option(
            SelectOption::builder()
                .with_name("Rough")
                .with_icon(Icon::system_emoji("pensive-face"))
                .build()
                .keyed("rough"),
        )
        .build()
        .keyed("group/mood");
    let chart = Chart::builder()
        .with_name("Journal")
        .with_description("A daily journal entry for tracking thoughts and activities.")
        .with_icon(icon_presets::charts::journal())
        .add_feature(
            Feature::builder()
                .with_name("Date")
                .with_kind(Date::builder().build().into())
                .build()
                .keyed("date"),
        )
        .add_feature(
            Feature::builder()
                .with_name("Mood")
                .with_kind(
                    Select::builder()
                        .with_group(mood_group, true.into())
                        .with_single_selection()
                        .build()
                        .into(),
                )
                .build()
                .keyed("mood"),
        )
        .add_feature(
            Feature::builder()
                .with_name("Highlights")
                .with_kind(Reference::builder().build().into())
                .build()
                .keyed("highlights"),
        )
        .add_feature(
            Feature::builder()
                .with_name("Goals for Tomorrow")
                .with_kind(Reference::builder().build().into())
                .build()
                .keyed("goals-for-tomorrow"),
        )
        .add_feature(tags_feature())
        .build();

    Chart {
        id: type_id(),
        ..chart
    }
}

/// Get the fixed ChartID for the Journal type.
pub fn type_id() -> ChartID {
    ChartID::builtin("journal")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_chart() {
        let journal = chart();
        assert_eq!(journal.name, "Journal");
        assert!(journal.is_builtin());
        assert_eq!(journal.features.len(), 5);
        assert!(journal.features.values().any(|f| f.name == "Date"));
        assert!(journal.features.values().any(|f| f.name == "Mood"));
        assert!(journal.features.values().any(|f| f.name == "Highlights"));
        assert!(
            journal
                .features
                .values()
                .any(|f| f.name == "Goals for Tomorrow")
        );
    }

    #[test]
    fn journal_chart_id() {
        assert_eq!(chart().id, type_id());
    }
}
