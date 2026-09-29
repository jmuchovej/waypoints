use crate::{Maneuver, ManeuverTarget};
use domain::{FeatureID, MarkID, Reading, RichText};

fn title_op(mark_id: MarkID, old: &str, new: &str) -> Maneuver {
    Maneuver::UpdateLandmarkTitle {
        mark_id,
        old: RichText::from(old.to_string()),
        new: RichText::from(new.to_string()),
    }
}

fn reading_op(mark_id: MarkID, feature_id: FeatureID, old: Reading, new: Reading) -> Maneuver {
    Maneuver::UpdateReading {
        mark_id,
        feature_id,
        old,
        new,
    }
}

#[test]
fn update_landmark_title_double_inversion_is_identity() {
    let op = title_op(MarkID::new(), "before", "after");
    assert_ne!(op.invert(), op);
    assert_eq!(op.invert().invert(), op);
}

#[test]
fn update_reading_double_inversion_is_identity() {
    let op = reading_op(
        MarkID::new(),
        FeatureID::new(),
        Reading::Bool(false),
        Reading::Bool(true),
    );
    assert_ne!(op.invert(), op);
    assert_eq!(op.invert().invert(), op);
}

#[test]
fn update_reading_null_old_double_inversion_is_identity() {
    // A previously unset feature records `Reading::Null` as its prior value.
    let op = reading_op(
        MarkID::new(),
        FeatureID::new(),
        Reading::Null,
        Reading::Number(42.0),
    );
    assert_eq!(op.invert().invert(), op);
}

#[test]
fn two_features_of_one_landmark_have_distinct_targets() {
    let lm = MarkID::new();
    let f = FeatureID::new();
    let g = FeatureID::new();

    let op_f = reading_op(lm, f.clone(), Reading::Null, Reading::Bool(true));
    let op_g = reading_op(lm, g.clone(), Reading::Null, Reading::Bool(true));

    assert_ne!(op_f.target(), op_g.target());
    assert_eq!(op_f.target(), ManeuverTarget::Reading(lm, f));
    assert_eq!(op_g.target(), ManeuverTarget::Reading(lm, g));
}

#[test]
fn title_and_reading_on_one_landmark_have_distinct_targets() {
    let lm = MarkID::new();

    let title = title_op(lm, "a", "b");
    let reading = reading_op(lm, FeatureID::new(), Reading::Null, Reading::Bool(true));

    assert_ne!(title.target(), reading.target());
    assert_eq!(title.target(), ManeuverTarget::Title(lm));
}
