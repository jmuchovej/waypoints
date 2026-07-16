//! Read-key acquisition from the ACL log (§7.2–7.3), proven end-to-end
//! through [`Vessel::refresh_keys`] — no hand-fed keys.
//!
//! The scenario spans two epochs:
//! - epoch 0: owner (1) creates the Territory and admits member (2).
//! - member (2) is removed → rotation to epoch 1 (key sealed to the owner).
//! - epoch 1: owner admits a *new* member (3).
//!
//! Three properties fall out:
//! - **Chain recovery**: member 3, admitted at epoch 1, reads epoch-0
//!   history it never had a sealed box for — by walking the backwards chain
//!   from the epoch-1 key it *was* sealed (§7.3).
//! - **Forward-only cutoff**: member 2, removed at the rotation, holds only
//!   epoch 0; it can still *receive* epoch-1 Notices but cannot read them
//!   (§7.2), the honest limit of removal.
//! - **Owner**: holds both epochs (Root + the rotation it authored).
//!
//! [`Vessel::refresh_keys`]: outpost::vessel::Vessel::refresh_keys

mod common;

use common::{TerritoryBuilder, kem, signing};
use outpost::protocol::crypto::ReadKey;
use outpost::protocol::{EpochID, NoticeID};
use outpost::replica::Replica;
use outpost::vessel::{Vessel, VesselError};
use rand::SeedableRng;

const EPOCH0: EpochID = EpochID::ZERO;

async fn vessel(seed: u8, acl: &[outpost::vessel::ACLRecord]) -> Vessel {
    let mut v = Vessel::new(
        signing(seed),
        kem(seed),
        Replica::bootstrap(":memory:", acl).await.unwrap(),
    )
    .await
    .unwrap();
    v.refresh_keys().await.unwrap();
    v
}

#[tokio::test]
async fn keys_flow_from_the_acl_log_with_chain_and_cutoff() {
    let epoch1 = EpochID::from(1);
    let k0 = ReadKey::from_bytes([0xA0; 32]);
    let k1 = ReadKey::from_bytes([0xB1; 32]);

    // Build the two-epoch Territory (real sealed boxes throughout).
    let mut t = TerritoryBuilder::new(1, &k0);
    t.admit(1, 2, &k0); // owner admits member 2 at epoch 0
    t.remove(1, 2, &k0, &k1); // remove member 2 → epoch 1
    t.admit(1, 3, &k1); // owner admits member 3 at epoch 1
    let acl = t.into_records();

    let mut owner = vessel(1, &acl).await;
    let member2 = vessel(2, &acl).await;
    let member3 = vessel(3, &acl).await;

    // Cache contents, derived purely from the log:
    assert_eq!(
        owner.readable_epochs(),
        vec![EPOCH0, epoch1],
        "owner holds both"
    );
    assert_eq!(
        member2.readable_epochs(),
        vec![EPOCH0],
        "removed member is capped at its cutoff"
    );
    assert_eq!(
        member3.readable_epochs(),
        vec![EPOCH0, epoch1],
        "late member recovers epoch 0 via the backwards chain"
    );

    // Prove the recovered keys are *correct*, not just present: the owner
    // authors a Notice under each epoch's key, and we check who can read it.
    let mut rng = rand::rngs::StdRng::from_seed([9; 32]);
    let n0 = owner
        .author(
            &mut rng,
            vec![],
            EPOCH0,
            "waypoints.page.v1",
            b"epoch-zero secret",
        )
        .await
        .unwrap();
    let n1 = owner
        .author(
            &mut rng,
            vec![],
            epoch1,
            "waypoints.page.v1",
            b"epoch-one secret",
        )
        .await
        .unwrap();
    // Distinct Notices (different epochs / payloads).
    assert_ne!(NoticeID::of(&n0).unwrap(), NoticeID::of(&n1).unwrap());

    // Member 3 (chain) reads BOTH — the epoch-0 key it recovered really works.
    assert_eq!(member3.decrypt(&n0).unwrap(), b"epoch-zero secret");
    assert_eq!(member3.decrypt(&n1).unwrap(), b"epoch-one secret");

    // Member 2 (cutoff) reads epoch 0 but is locked out of epoch 1 — it has
    // no epoch-1 key and no way to get one (removal is forward-only).
    assert_eq!(member2.decrypt(&n0).unwrap(), b"epoch-zero secret");
    assert!(matches!(member2.decrypt(&n1), Err(VesselError::NoKey(e)) if e == epoch1));
}

#[tokio::test]
async fn refresh_keys_is_idempotent() {
    let k0 = ReadKey::from_bytes([0xA0; 32]);
    let t = TerritoryBuilder::new(1, &k0);
    let acl = t.into_records();

    let mut owner = Vessel::new(
        signing(1),
        kem(1),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();
    owner.refresh_keys().await.unwrap();
    let once = owner.readable_epochs();
    owner.refresh_keys().await.unwrap();
    assert_eq!(
        once,
        owner.readable_epochs(),
        "a second refresh changes nothing"
    );
    assert_eq!(once, vec![EPOCH0]);
}
