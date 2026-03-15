//! Person built-in type - A person with contact information.
use super::tags_feature;
use domain::icon_presets;
use domain::{Chart, ChartID, Feature, FeatureKind};

/// Person - A person with contact information
pub fn chart() -> Chart {
    let chart = Chart::builder()
        .with_name("Person")
        .with_description("A person with contact information and relationships")
        .with_icon(icon_presets::charts::person())
        .add_feature(
            Feature::builder()
                .with_name("Email")
                .with_kind(FeatureKind::Email)
                .build(),
        )
        .add_feature(
            Feature::builder()
                .with_name("Phone")
                .with_kind(FeatureKind::Phone)
                .build(),
        )
        .add_feature(
            Feature::builder()
                .with_name("Company")
                .with_kind(FeatureKind::Text)
                .build(),
        )
        .add_feature(
            Feature::builder()
                .with_name("Role")
                .with_kind(FeatureKind::Text)
                .build(),
        )
        .add_feature(
            Feature::builder()
                .with_name("Nickname")
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

/// Get the fixed ChartID for the Person type.
pub fn type_id() -> ChartID {
    ChartID::builtin("person")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn person_chart() {
        let person = chart();
        assert_eq!(person.name, "Person");
        assert!(person.is_builtin());
        assert_eq!(person.features.len(), 6);
        assert!(person.features.values().any(|f| f.name == "Email"));
        assert!(person.features.values().any(|f| f.name == "Phone"));
        assert!(person.features.values().any(|f| f.name == "Company"));
        assert!(person.features.values().any(|f| f.name == "Role"));
        assert!(person.features.values().any(|f| f.name == "Nickname"));
    }

    #[test]
    fn person_chart_id() {
        assert_eq!(chart().id, type_id());
    }
}
