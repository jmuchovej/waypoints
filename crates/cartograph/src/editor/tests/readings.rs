use super::*;
use domain::{FeatureID, FeatureKind, Reading};
use rstest::rstest;

fn number_kind() -> FeatureKind {
    FeatureKind::Number {
        min: None,
        max: None,
        step: None,
        precision: None,
        format: None,
    }
}

fn reference_kind(allowed_types: Option<Vec<ChartID>>) -> FeatureKind {
    FeatureKind::Reference {
        allowed_types,
        max_references: None,
        display_field: "title".to_string(),
        display_mode: Default::default(),
        filter_criteria: None,
    }
}

async fn get_reading(svc: &EditorService, id: MarkID, feature: &FeatureID) -> Option<Reading> {
    let m = svc.get_mark(id).await.unwrap();
    match m.data {
        MarkData::Landmark(l) => l.readings.get(feature).cloned(),
        _ => panic!("expected landmark"),
    }
}

/// A chart with one Number feature; returns (landmark, feature_id).
async fn seed_number_landmark(db: &Store) -> (MarkID, FeatureID, Chart) {
    let chart = Chart::builder()
        .with_name("Test Chart")
        .with_features(vec![feature("count", number_kind())])
        .build();
    seed_chart(db, &chart).await;
    let fid = chart.features.keys().next().unwrap().clone();
    let lm = seed_landmark_of(db, chart.id.clone(), "Readings Page").await;
    (lm, fid, chart)
}

#[rstest]
#[tokio::test]
async fn undo_restores_changed_reading(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, fid, _) = seed_number_landmark(&db).await;

    svc.update_mark_reading(lm, fid.clone(), Reading::Number(1.0))
        .await
        .unwrap();
    svc.update_mark_reading(lm, fid.clone(), Reading::Number(2.0))
        .await
        .unwrap();

    svc.undo(lm).await.unwrap().unwrap();
    assert_eq!(
        get_reading(&svc, lm, &fid).await,
        Some(Reading::Number(1.0))
    );

    svc.redo(lm).await.unwrap().unwrap();
    assert_eq!(
        get_reading(&svc, lm, &fid).await,
        Some(Reading::Number(2.0))
    );
}

#[rstest]
#[tokio::test]
async fn undo_restores_cleared_reading(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, fid, _) = seed_number_landmark(&db).await;

    svc.update_mark_reading(lm, fid.clone(), Reading::Number(7.0))
        .await
        .unwrap();
    svc.update_mark_reading(lm, fid.clone(), Reading::Null)
        .await
        .unwrap();
    assert_eq!(get_reading(&svc, lm, &fid).await, None);

    svc.undo(lm).await.unwrap().unwrap();
    assert_eq!(
        get_reading(&svc, lm, &fid).await,
        Some(Reading::Number(7.0))
    );
}

#[rstest]
#[tokio::test]
async fn undo_removes_newly_set_reading(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, fid, _) = seed_number_landmark(&db).await;

    svc.update_mark_reading(lm, fid.clone(), Reading::Number(3.0))
        .await
        .unwrap();

    svc.undo(lm).await.unwrap().unwrap();
    assert_eq!(get_reading(&svc, lm, &fid).await, None);
}

#[rstest]
#[tokio::test]
async fn two_features_undo_independently(#[future] env: Env) {
    let (mut svc, db) = env.await;

    let chart = Chart::builder()
        .with_name("Two Features")
        .with_features(vec![
            feature("f", number_kind()),
            feature("g", FeatureKind::Checkbox),
        ])
        .build();
    seed_chart(&db, &chart).await;
    let mut fids = chart.features.keys().cloned();
    let (f, g) = (fids.next().unwrap(), fids.next().unwrap());
    let lm = seed_landmark_of(&db, chart.id.clone(), "Two Features Page").await;

    svc.update_mark_reading(lm, f.clone(), Reading::Number(1.0))
        .await
        .unwrap();
    svc.update_mark_reading(lm, g.clone(), Reading::Bool(true))
        .await
        .unwrap();

    // One undo restores G alone; F retains its newly set value.
    svc.undo(lm).await.unwrap().unwrap();
    assert_eq!(get_reading(&svc, lm, &g).await, None);
    assert_eq!(get_reading(&svc, lm, &f).await, Some(Reading::Number(1.0)));
}

#[rstest]
#[tokio::test]
async fn invalid_reading_rejected_and_nothing_pushed(#[future] env: Env) {
    let (mut svc, db) = env.await;

    let chart = Chart::builder()
        .with_name("Checkbox Chart")
        .with_features(vec![feature("done", FeatureKind::Checkbox)])
        .build();
    seed_chart(&db, &chart).await;
    let fid = chart.features.keys().next().unwrap().clone();
    let lm = seed_landmark_of(&db, chart.id.clone(), "Checkbox Page").await;

    let result = svc
        .update_mark_reading(lm, fid.clone(), Reading::Number(1.0))
        .await;
    assert!(result.is_err());

    assert_eq!(get_reading(&svc, lm, &fid).await, None);
    assert!(svc.undo(lm).await.unwrap().is_none());
}

#[rstest]
#[tokio::test]
async fn reference_outside_allowed_types_rejected(#[future] env: Env) {
    let (mut svc, db) = env.await;

    let allowed = Chart::builder().with_name("Allowed").build();
    let forbidden = Chart::builder().with_name("Forbidden").build();
    seed_chart(&db, &allowed).await;
    seed_chart(&db, &forbidden).await;

    let chart = Chart::builder()
        .with_name("Referencing")
        .with_features(vec![feature(
            "rel",
            reference_kind(Some(vec![allowed.id.clone()])),
        )])
        .build();
    seed_chart(&db, &chart).await;
    let fid = chart.features.keys().next().unwrap().clone();

    let lm = seed_landmark_of(&db, chart.id.clone(), "Referencing Page").await;
    let target = seed_landmark_of(&db, forbidden.id.clone(), "Forbidden Target").await;

    let result = svc
        .update_mark_reading(lm, fid.clone(), Reading::References(vec![target]))
        .await;
    assert!(result.is_err());

    assert_eq!(get_reading(&svc, lm, &fid).await, None);
    assert!(svc.undo(lm).await.unwrap().is_none());
}

/// Pins design.md Decision 2: `apply` does not revalidate, so an op recorded
/// under looser constraints stays undoable after the chart narrows them.
#[rstest]
#[tokio::test]
async fn undo_succeeds_after_constraints_narrow(#[future] env: Env) {
    let (mut svc, db) = env.await;
    let (lm, fid, chart) = seed_number_landmark(&db).await;

    svc.update_mark_reading(lm, fid.clone(), Reading::Number(5.0))
        .await
        .unwrap();
    svc.update_mark_reading(lm, fid.clone(), Reading::Number(15.0))
        .await
        .unwrap();

    // Narrow the feature so the earlier reading (5.0) would no longer validate.
    let mut narrowed = chart;
    narrowed.features.get_mut(&fid).unwrap().kind = FeatureKind::Number {
        min: Some(10.0),
        max: None,
        step: None,
        precision: None,
        format: None,
    };
    seed_chart(&db, &narrowed).await;

    // A fresh forward edit below the minimum is now rejected...
    assert!(
        svc.update_mark_reading(lm, fid.clone(), Reading::Number(4.0))
            .await
            .is_err()
    );

    // ...but undoing back to 5.0 succeeds: apply is mechanical.
    svc.undo(lm).await.unwrap().unwrap();
    assert_eq!(
        get_reading(&svc, lm, &fid).await,
        Some(Reading::Number(5.0))
    );
}
