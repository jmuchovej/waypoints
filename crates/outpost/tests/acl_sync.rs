//! ACL propagation over sync, and the read-key auto-refresh it triggers.
//! A Vessel that joins a Territory knowing only its root ACL
//! record syncs *once* with a Station and comes fully current: it receives
//! the membership records that admitted it, automatically derives the read
//! key sealed to it, pulls the encrypted history, and reads it — with no
//! explicit key handling anywhere.
//!
//! This is the payoff of auto-triggering [`refresh_keys`]: the ACL now
//! reconciles as the first phase of a session (it must, since Notices pin
//! `acl_head`s), and `sync_with` refreshes keys afterward.
//!
//! [`refresh_keys`]: outpost::vessel::Vessel::refresh_keys

mod common;

use common::{TerritoryBuilder, kem, signing};
use outpost::{
    protocol::{EpochID, NoticeID, ReadKey},
    replica::Replica,
    station::Station,
    vessel::Vessel,
};
use rand::SeedableRng;

#[tokio::test]
async fn a_joining_vessel_syncs_acl_key_and_history_in_one_shot() {
    let k0 = ReadKey::from_bytes([0x42; 32]);

    // Full Territory: owner (1) admits member (3). The joiner's Approve seals
    // the epoch-0 key to member 3.
    let mut t = TerritoryBuilder::new(1, &k0);
    t.admit(1, 3, &k0);
    let full_acl = t.into_records();
    // The joiner's starting knowledge is only the root — as if it holds just
    // an invite and the Territory id, nothing more.
    let root_only = &full_acl[..1];

    // The owner (fully current) authors an encrypted Notice.
    let mut owner = Vessel::new(
        signing(1),
        kem(1),
        Replica::bootstrap(":memory:", &full_acl).await.unwrap(),
    )
    .await
    .unwrap();
    let mut rng = rand::rngs::StdRng::from_seed([7; 32]);
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

    // The Station holds the full ACL and the Notice, and serves — it cannot
    // read the Notice (identity 100 is not a member).
    let station = Station::new(
        &signing(100),
        Replica::bootstrap(":memory:", &full_acl).await.unwrap(),
    )
    .await
    .unwrap();
    station.replica().ingest(vec![notice]).await.unwrap();
    let station_addr = station.endpoint().direct_addr();

    // The joining Vessel starts behind: only the root ACL record, so it is
    // not yet a member and holds no key.
    let mut joiner = Vessel::new(
        signing(3),
        kem(3),
        Replica::bootstrap(":memory:", root_only).await.unwrap(),
    )
    .await
    .unwrap();
    assert!(
        joiner.readable_epochs().is_empty(),
        "before syncing, the joiner is not yet a member and holds no key"
    );
    assert_eq!(joiner.replica().acl_len().await.unwrap(), 1);

    // One sync: ACL records propagate first, then the Notice; sync_with
    // auto-refreshes keys afterward.
    let serve = tokio::spawn(async move { station.serve_once().await.unwrap() });
    joiner.sync_with(station_addr).await.unwrap();
    serve.await.unwrap();

    // The joiner is now fully current: it has the whole ACL, derived its key,
    // pulled the Notice, and can read it.
    assert_eq!(joiner.replica().acl_len().await.unwrap(), 4);
    assert_eq!(
        joiner.readable_epochs(),
        vec![EpochID::ZERO],
        "the joiner auto-derived the epoch-0 key sealed to it"
    );
    let stored = joiner
        .replica()
        .hold()
        .get_notice(notice_id)
        .await
        .unwrap()
        .expect("the joiner pulled the Notice");
    assert_eq!(joiner.decrypt(&stored.notice).unwrap(), b"members only");
}
