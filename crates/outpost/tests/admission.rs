//! Admission policy (design doc
//! `docs/dev/outpost-live-push-and-station-persistence.md` §3) over real
//! QUIC. Registration: only an identity holding **Owner in the presented
//! log's fold** may home a new Territory (a later-promoted Owner passes
//! identically to the creator — authority comes from the ACL). Sync: the
//! **two-phase gate** serves the ACL phase to any authenticated identity but
//! refuses the Notice phase to non-members — a stranger walks away with the
//! ACL suffix and zero Notices. `Anyone`/`Everyone` restore the open
//! behavior. (The invited-joiner flow through the gate is covered end to end
//! by `acl_sync.rs`.)

use std::sync::Arc;

use ed25519_dalek::SigningKey;
use outpost::protocol::identity::KeyRoot;
use outpost::protocol::{EpochID, NoticeID};
use outpost::replica::Replica;
use outpost::station::{RegisterPolicy, Station, SyncPolicy};
use outpost::vessel::{Vessel, VesselError};
use rand::SeedableRng;

fn actors() -> (
    outpost::protocol::identity::Identity,
    outpost::protocol::identity::DeviceKey,
    outpost::protocol::identity::Identity,
    outpost::protocol::identity::DeviceKey,
) {
    let owner = KeyRoot::from_seed([1; 64]);
    let stranger = KeyRoot::from_seed([9; 64]);
    (
        owner.identity(),
        owner.device(0),
        stranger.identity(),
        stranger.device(0),
    )
}

#[tokio::test]
async fn only_an_owner_per_the_fold_may_register() {
    let mut rng = rand::rngs::StdRng::from_seed([7; 32]);
    let dir = tempfile::tempdir().unwrap();
    let (owner_id, owner_dev, stranger_id, stranger_dev) = actors();

    let owner = Vessel::create_territory(&owner_id, &owner_dev, ":memory:", &mut rng)
        .await
        .unwrap();
    let territory = owner.territory_id().await.unwrap();
    let root = owner.replica().hold().acl_records().await.unwrap();

    // The stranger holds the same public root — enough to *replicate*, not
    // to *register*: it is not an Owner in the log it presents.
    let stranger = Vessel::from_identity(
        &stranger_id,
        &stranger_dev,
        Replica::bootstrap(":memory:", &root).await.unwrap(),
    )
    .await
    .unwrap();

    let station = Arc::new(
        Station::hosting(&SigningKey::from_bytes(&[100; 32]), dir.path())
            .await
            .unwrap(),
    );
    let addr = station.endpoint().direct_addr();
    let serving = station.clone();
    let serve_loop = tokio::spawn(async move {
        loop {
            let _ = serving.serve_once().await;
        }
    });

    let refused = stranger.register(addr.clone(), territory).await;
    assert!(
        matches!(refused, Err(VesselError::RegisterRejected)),
        "a non-owner cannot home the Territory: {refused:?}"
    );
    assert!(!station.serves(&territory));

    // The actual owner registers the same Territory fine.
    owner.register(addr.clone(), territory).await.unwrap();
    assert!(station.serves(&territory));

    serve_loop.abort();
}

#[tokio::test]
async fn a_stranger_gets_the_acl_phase_but_zero_notices() {
    let mut rng = rand::rngs::StdRng::from_seed([8; 32]);
    let (owner_id, owner_dev, stranger_id, stranger_dev) = actors();

    // Owner's Territory with real content and a membership change on the
    // log, so both phases have something to withhold or serve.
    let mut owner = Vessel::create_territory(&owner_id, &owner_dev, ":memory:", &mut rng)
        .await
        .unwrap();
    let (_invite_key, _secret, _record) = owner
        .invite(&mut rng, outpost::protocol::acl::Permissions::Writer)
        .await
        .unwrap();
    let notice = owner
        .author(
            &mut rng,
            vec![],
            EpochID::ZERO,
            "waypoints.page.v1",
            b"members only",
        )
        .await
        .unwrap();
    let notice_id = NoticeID::of(&notice).unwrap();
    let full_acl = owner.replica().hold().acl_records().await.unwrap();
    assert_eq!(full_acl.len(), 2, "root + invite");

    let station = Arc::new(
        Station::new(
            &SigningKey::from_bytes(&[100; 32]),
            Replica::bootstrap(":memory:", &full_acl).await.unwrap(),
        )
        .await
        .unwrap(),
    );
    station.replica().ingest(vec![notice]).await.unwrap();
    let addr = station.endpoint().direct_addr();
    let serving = station.clone();
    let serve_loop = tokio::spawn(async move {
        loop {
            let _ = serving.serve_once().await;
        }
    });

    // The stranger starts from the public root only.
    let root_only = &full_acl[..1];
    let mut stranger = Vessel::from_identity(
        &stranger_id,
        &stranger_dev,
        Replica::bootstrap(":memory:", root_only).await.unwrap(),
    )
    .await
    .unwrap();

    let refused = stranger.sync_with(addr.clone()).await;
    assert!(
        matches!(refused, Err(VesselError::SyncRefused)),
        "the Notice phase is refused: {refused:?}"
    );
    // Two-phase: the ACL suffix DID arrive (it is what admits joiners)…
    assert_eq!(stranger.replica().acl_len().await.unwrap(), 2);
    // …but not one Notice — no envelopes, no ciphertext, no DAG shape.
    assert!(
        stranger
            .replica()
            .hold()
            .get_notice(notice_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        stranger
            .replica()
            .hold()
            .held_ids()
            .await
            .unwrap()
            .is_empty()
    );

    serve_loop.abort();
}

#[tokio::test]
async fn the_escape_hatches_restore_open_behavior() {
    let mut rng = rand::rngs::StdRng::from_seed([9; 32]);
    let dir = tempfile::tempdir().unwrap();
    let (owner_id, owner_dev, stranger_id, stranger_dev) = actors();

    let mut owner = Vessel::create_territory(&owner_id, &owner_dev, ":memory:", &mut rng)
        .await
        .unwrap();
    let territory = owner.territory_id().await.unwrap();
    let notice = owner
        .author(
            &mut rng,
            vec![],
            EpochID::ZERO,
            "waypoints.page.v1",
            b"open wide",
        )
        .await
        .unwrap();
    let notice_id = NoticeID::of(&notice).unwrap();
    let root = owner.replica().hold().acl_records().await.unwrap();

    let station = Arc::new(
        Station::hosting(&SigningKey::from_bytes(&[100; 32]), dir.path())
            .await
            .unwrap()
            .with_register_policy(RegisterPolicy::Anyone)
            .with_sync_policy(SyncPolicy::Everyone),
    );
    let addr = station.endpoint().direct_addr();
    let serving = station.clone();
    let serve_loop = tokio::spawn(async move {
        loop {
            let _ = serving.serve_once().await;
        }
    });

    // Under Anyone, even the stranger may home the Territory…
    let stranger = Vessel::from_identity(
        &stranger_id,
        &stranger_dev,
        Replica::bootstrap(":memory:", &root).await.unwrap(),
    )
    .await
    .unwrap();
    stranger.register(addr.clone(), territory).await.unwrap();
    assert!(station.serves(&territory));

    // …the owner pushes content, and under Everyone the stranger pulls it
    // (ciphertext it cannot read — content-blindness is cryptographic, not
    // admission).
    owner.sync_with(addr.clone()).await.unwrap();
    let mut stranger = stranger;
    stranger.sync_with(addr.clone()).await.unwrap();
    assert!(
        stranger
            .replica()
            .hold()
            .get_notice(notice_id)
            .await
            .unwrap()
            .is_some()
    );

    serve_loop.abort();
}
