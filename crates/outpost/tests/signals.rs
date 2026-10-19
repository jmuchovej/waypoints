//! The Station under a supervisor: a termination signal ends it cleanly —
//! connected Vessels see a close, not a timeout — and what it held comes
//! back on restart: the Territories, the identity, and the orderer's
//! authority, each from a provisioned file.

#![cfg(unix)]

mod common;

use common::process::{StationProcess, outpost};
use ed25519_dalek::SigningKey;
use outpost::{
    hold::TerritoryHold,
    protocol::{
        ACLPayload, EpochID, InviteKey, KeyRoot, NoticeID, Permissions, RecordBody, RecordID,
    },
    replica::Replica,
    vessel::{SubscriptionEvent, Vessel},
};
use rand::SeedableRng;
use std::{path::Path, time::Duration};
use tokio::time::timeout;

/// Exit after SIGTERM: generous for a debug build, far below the Vessel's
/// idle timeout (45 s) — a close observed inside this window is a close,
/// not a reap.
const EXIT_BOUND: Duration = Duration::from_secs(15);

fn keygen(path: &Path) {
    let output = outpost()
        .args(["keygen", "--out"])
        .arg(path)
        .output()
        .expect("run keygen");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn sigterm_closes_a_parked_subscription_and_exits_zero() {
    let mut rng = rand::rngs::StdRng::from_seed([21; 32]);
    let dir = tempfile::tempdir().unwrap();
    let mut station = StationProcess::spawn(dir.path(), &[]);

    let root_key = KeyRoot::from_seed([21; 64]);
    let laptop = Vessel::create_territory(
        &root_key.identity(),
        &root_key.device(0),
        ":memory:",
        &mut rng,
    )
    .await
    .unwrap();
    let territory = laptop.territory_id().await.unwrap();
    laptop
        .register(station.addr.clone(), territory)
        .await
        .unwrap();
    let (_sub, mut events) = laptop.subscribe(station.addr.clone()).await.unwrap();

    station.sigterm();

    // The Station awaited its endpoint close, so the parked session ends
    // with a transport close the Vessel reports promptly.
    let closed = timeout(Duration::from_secs(10), async {
        loop {
            match events.recv().await {
                Some(SubscriptionEvent::Closed) | None => break,
                Some(_) => continue,
            }
        }
    })
    .await;
    assert!(
        closed.is_ok(),
        "the Vessel observed a close within 10 s, not an idle reap"
    );

    let status = station.wait_exit(EXIT_BOUND).await;
    assert!(status.success(), "clean exit, got {status}");
}

#[tokio::test(flavor = "multi_thread")]
async fn sigterm_with_no_sessions_exits_promptly() {
    let dir = tempfile::tempdir().unwrap();
    let mut station = StationProcess::spawn(dir.path(), &[]);
    station.sigterm();
    let status = station.wait_exit(Duration::from_secs(2)).await;
    assert!(status.success(), "clean exit, got {status}");
}

#[tokio::test(flavor = "multi_thread")]
async fn data_and_identity_survive_a_signalled_stop() {
    let mut rng = rand::rngs::StdRng::from_seed([22; 32]);
    let dir = tempfile::tempdir().unwrap();
    let key = dir.path().join("station.key");
    keygen(&key);
    let key_arg = key.to_str().unwrap();

    let root_key = KeyRoot::from_seed([22; 64]);
    let mut laptop = Vessel::create_territory(
        &root_key.identity(),
        &root_key.device(0),
        ":memory:",
        &mut rng,
    )
    .await
    .unwrap();
    let territory = laptop.territory_id().await.unwrap();
    let notice = laptop
        .author(
            &mut rng,
            vec![],
            EpochID::ZERO,
            "waypoints.page.v1",
            b"survives",
        )
        .await
        .unwrap();
    let notice_id = NoticeID::of(&notice).unwrap();

    // First life: register and push the Notice.
    let mut first = StationProcess::spawn(dir.path(), &["--key-file", key_arg]);
    laptop
        .register(first.addr.clone(), territory)
        .await
        .unwrap();
    laptop.sync_with(first.addr.clone()).await.unwrap();
    first.sigterm();
    assert!(first.wait_exit(EXIT_BOUND).await.success());

    // Second life, same data directory and key file: same identity (the
    // dial would fail an identity mismatch otherwise), same Territory.
    let mut second = StationProcess::spawn(dir.path(), &["--key-file", key_arg]);
    assert_eq!(
        second.addr.id, first.addr.id,
        "the identity is the key file's"
    );
    laptop.sync_with(second.addr.clone()).await.unwrap();
    second.sigterm();
    assert!(second.wait_exit(EXIT_BOUND).await.success());

    // What the Station holds is on disk: one Territory hold — named by the
    // Territory's 64-hex-char id, beside the Station's own `station.db` —
    // with the Notice in it.
    let holds: Vec<_> = std::fs::read_dir(dir.path().join("territories"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.extension().is_some_and(|e| e == "db") && p.file_stem().is_some_and(|s| s.len() == 64)
        })
        .collect();
    assert_eq!(holds.len(), 1, "one Territory hold: {holds:?}");
    let hold = TerritoryHold::open(holds[0].to_str().unwrap())
        .await
        .unwrap();
    assert!(
        hold.get_notice(notice_id).await.unwrap().is_some(),
        "the Notice accepted before the signal is held after two restarts"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn ordering_authority_survives_a_restart_but_not_a_key_change() {
    let mut rng = rand::rngs::StdRng::from_seed([23; 32]);
    let dir = tempfile::tempdir().unwrap();
    let fleet_a = dir.path().join("fleet-a.key");
    let fleet_b = dir.path().join("fleet-b.key");
    keygen(&fleet_a);
    keygen(&fleet_b);
    let fleet_a_pub = {
        let seed: [u8; 32] = std::fs::read(&fleet_a).unwrap().try_into().unwrap();
        SigningKey::from_bytes(&seed).verifying_key()
    };
    let fleet_a_arg = fleet_a.to_str().unwrap();

    let owner_root = KeyRoot::from_seed([23; 64]);
    let (owner_id, owner_dev) = (owner_root.identity(), owner_root.device(0));
    let owner = Vessel::create_territory(&owner_id, &owner_dev, ":memory:", &mut rng)
        .await
        .unwrap();
    let territory = owner.territory_id().await.unwrap();
    let root = owner.replica().hold().acl_records().await.unwrap()[0].clone();

    // First life: the orderer on fleet key A countersigns an Invite.
    let mut first = StationProcess::spawn(dir.path(), &["--fleet-key-file", fleet_a_arg]);
    owner.register(first.addr.clone(), territory).await.unwrap();
    let invite_key =
        InviteKey::from_bytes(SigningKey::from_bytes(&[9; 32]).verifying_key().to_bytes());
    let invite = RecordBody {
        prev: Some(RecordID::of(&root).unwrap()),
        author: owner_id.verifying_key(),
        payload: ACLPayload::Invite {
            invite_key,
            permissions: Permissions::Writer,
        },
    }
    .sign(owner_id.signing_key())
    .unwrap();
    let proof = owner.submit_acl(first.addr.clone(), &invite).await.unwrap();
    proof
        .verify(&fleet_a_pub, RecordID::of(&invite).unwrap())
        .unwrap();
    first.sigterm();
    assert!(first.wait_exit(EXIT_BOUND).await.success());

    // Second life, same fleet key: a peer homed to fleet A pulls the record
    // over sync and accepts it, because the proof still verifies. The peer
    // is not a member, so the Notice phase is refused after the ACL phase —
    // that refusal is expected and the ACL is what we assert on.
    let mut second = StationProcess::spawn(dir.path(), &["--fleet-key-file", fleet_a_arg]);
    let peer_root = KeyRoot::from_seed([24; 64]);
    let mut peer = Vessel::from_identity(
        &peer_root.identity(),
        &peer_root.device(0),
        Replica::bootstrap_with_fleet(":memory:", std::slice::from_ref(&root), fleet_a_pub)
            .await
            .unwrap(),
    )
    .await
    .unwrap();
    let _ = peer.sync_with(second.addr.clone()).await;
    assert_eq!(
        peer.replica().acl_len().await.unwrap(),
        2,
        "the record ordered before the restart verifies after it"
    );
    second.sigterm();
    assert!(second.wait_exit(EXIT_BOUND).await.success());

    // Third life on a *different* fleet key: what it orders now does not
    // verify for a peer homed to fleet A.
    let mut third =
        StationProcess::spawn(dir.path(), &["--fleet-key-file", fleet_b.to_str().unwrap()]);
    let revoke = RecordBody {
        prev: Some(RecordID::of(&invite).unwrap()),
        author: owner_id.verifying_key(),
        payload: ACLPayload::RevokeInvite { invite_key },
    }
    .sign(owner_id.signing_key())
    .unwrap();
    let proof = owner.submit_acl(third.addr.clone(), &revoke).await.unwrap();
    assert!(
        proof
            .verify(&fleet_a_pub, RecordID::of(&revoke).unwrap())
            .is_err(),
        "a proof from fleet key B does not verify under A"
    );
    let _ = peer.sync_with(third.addr.clone()).await;
    assert_eq!(
        peer.replica().acl_len().await.unwrap(),
        2,
        "the fleet-A peer refuses the record proven under B"
    );
    third.sigterm();
    assert!(third.wait_exit(EXIT_BOUND).await.success());
}
