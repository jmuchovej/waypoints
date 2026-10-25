//! The hybrid logical clock on the wire: authored Notices carry a real,
//! strictly advancing clock; observing a remote clock advances the local
//! one past it; and the clock survives a restart. The field stays advisory
//! at the protocol layer — these tests cover the Vessel-side behavior only
//! (protocol indifference is pinned in `outpost-protocol`'s validate suite).

#[path = "common/territory.rs"]
mod territory;

use outpost::{
    hold::TerritoryHold,
    protocol::{
        Envelope, EnvelopeHeader, EpochID, HLC, NoticeID, ReadKey, SignedNotice, seal_payload,
    },
    replica::Replica,
    vessel::Vessel,
};
use rand::SeedableRng;
use territory::{TerritoryBuilder, kem, signing};

fn rng() -> rand::rngs::StdRng {
    rand::rngs::StdRng::from_seed([7; 32])
}

/// Territory owned by 1 with Writers 2 and 3.
fn acl() -> (ReadKey, Vec<outpost::protocol::ACLRecord>) {
    let k0 = ReadKey::from_bytes([0x42; 32]);
    let mut t = TerritoryBuilder::new(1, &k0);
    t.admit(1, 2, &k0);
    t.admit(1, 3, &k0);
    (k0, t.into_records())
}

async fn author(vessel: &Vessel, plaintext: &[u8]) -> SignedNotice {
    vessel
        .author(
            &mut rng(),
            vec![],
            EpochID::ZERO,
            "waypoints.page.v1",
            plaintext,
        )
        .await
        .unwrap()
}

fn timestamp_of(notice: &SignedNotice) -> i64 {
    notice.parse_and_verify().unwrap().header.timestamp
}

#[tokio::test]
async fn authored_notices_carry_a_ticking_clock() {
    let (_, acl) = acl();
    let vessel = Vessel::new(
        signing(2),
        kem(2),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();

    let first = timestamp_of(&author(&vessel, b"one").await);
    let second = timestamp_of(&author(&vessel, b"two").await);
    assert!(first > 0, "the placeholder clock must be gone");
    assert!(second > first, "authoring must tick strictly upward");
}

#[tokio::test]
async fn observing_an_ahead_clock_advances_authoring() {
    let (k0, acl) = acl();
    let vessel = Vessel::new(
        signing(2),
        kem(2),
        Replica::bootstrap(":memory:", &acl).await.unwrap(),
    )
    .await
    .unwrap();

    // Writer 3 claims a clock far ahead of any real wall clock (year ~2096),
    // so a mere local tick could never exceed it — only `receive` can.
    let ahead = HLC::pack(4_000_000_000_000, 7);
    let header = EnvelopeHeader {
        parents: vec![],
        acl_head: vessel.replica().acl_head().await.unwrap(),
        epoch: EpochID::ZERO,
        author: signing(3).verifying_key(),
        snapshot_base: NoticeID::SELF,
        is_snapshot: false,
        payload_type: "waypoints.page.v1".into(),
        timestamp: ahead.inner(),
    };
    let payload = seal_payload(&mut rng(), &k0, &header, b"from the future").unwrap();
    let remote = SignedNotice::sign(&Envelope { header, payload }, &signing(3)).unwrap();

    vessel.replica().ingest(vec![remote]).await.unwrap();

    let next = timestamp_of(&author(&vessel, b"after observing").await);
    assert!(
        HLC(next) > ahead,
        "authored clock {next} must exceed the observed {ahead}"
    );
}

#[tokio::test]
async fn clock_persists_across_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("territory.db");
    let path = path.to_str().unwrap();
    let (_, acl) = acl();

    let vessel = Vessel::new(
        signing(2),
        kem(2),
        Replica::bootstrap(path, &acl).await.unwrap(),
    )
    .await
    .unwrap();
    let authored = timestamp_of(&author(&vessel, b"before restart").await);
    drop(vessel);

    let replica = Replica::reopen(TerritoryHold::open(path).await.unwrap())
        .await
        .unwrap();
    // Reopen restores the persisted clock itself — not merely a value that
    // happens to be past it. Ingesting the authored Notice at startup also
    // advanced past `authored` via receive; either way, nothing regressed.
    assert!(
        replica.clock().await >= HLC(authored),
        "reopened clock must not regress below the last authored value"
    );

    let vessel = Vessel::new(signing(2), kem(2), replica).await.unwrap();
    let next = timestamp_of(&author(&vessel, b"after restart").await);
    assert!(next > authored, "post-restart authoring must tick onward");
}
