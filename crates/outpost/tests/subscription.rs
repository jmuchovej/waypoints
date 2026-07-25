//! Live-push subscriptions over real
//! QUIC. Two devices of one identity subscribe to a Station; one authors and
//! pushes, the other's doorbell rings and its hold converges **without a
//! re-dial**. A one-shot sync carrying a new ACL record fans out live to
//! subscribers (Acl event + log growth). `connected` reflects the live
//! session: true while subscribed, false after close, and the closed side
//! sees `Closed`.

use std::sync::Arc;
use std::time::Duration;

use ed25519_dalek::SigningKey;
use outpost::protocol::identity::KeyRoot;
use outpost::protocol::{EpochID, NoticeID};
use outpost::replica::Replica;
use outpost::station::Station;
use outpost::vessel::{SubscriptionEvent, Vessel};
use rand::SeedableRng;
use tokio::time::timeout;

const TICK: Duration = Duration::from_millis(50);
const WAIT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn subscribers_converge_live_without_redialing() {
    let mut rng = rand::rngs::StdRng::from_seed([7; 32]);
    let root_key = KeyRoot::from_seed([1; 64]);
    let identity = root_key.identity();
    let laptop_dev = root_key.device(0);
    let phone_dev = root_key.device(1);

    let laptop = Vessel::create_territory(&identity, &laptop_dev, ":memory:", &mut rng)
        .await
        .unwrap();
    let territory = laptop.territory_id().await.unwrap();
    let root = laptop.replica().hold().acl_records().await.unwrap();

    let station = Arc::new(
        Station::new(
            &SigningKey::from_bytes(&[100; 32]),
            Replica::bootstrap(":memory:", &root).await.unwrap(),
        )
        .await
        .unwrap(),
    );
    let addr = station.endpoint().direct_addr();
    let serving = Arc::clone(&station);
    let serve_loop = tokio::spawn(async move { serving.serve_loop().await });

    let phone = Vessel::from_identity(
        &identity,
        &phone_dev,
        Replica::bootstrap(":memory:", &root).await.unwrap(),
    )
    .await
    .unwrap();

    // Both devices subscribe.
    let (laptop_sub, _laptop_events) = laptop.subscribe(addr.clone()).await.unwrap();
    let (phone_sub, mut phone_events) = phone.subscribe(addr.clone()).await.unwrap();

    // Both are live in the device registry while subscribed.
    let sightings = station.sightings_for(&territory);
    assert_eq!(sightings.len(), 2);
    assert!(
        sightings.iter().all(|s| s.connected),
        "both devices are connected while subscribed"
    );

    // The laptop authors and pushes; the phone's doorbell rings and its hold
    // converges — no re-dial anywhere.
    let notice = laptop
        .author(
            &mut rng,
            vec![],
            EpochID::ZERO,
            "waypoints.page.v1",
            b"live!",
        )
        .await
        .unwrap();
    let notice_id = NoticeID::of(&notice).unwrap();
    laptop_sub.push(notice).await.unwrap();

    let event = timeout(WAIT, phone_events.recv())
        .await
        .expect("doorbell within the wait")
        .expect("channel open");
    match event {
        SubscriptionEvent::Notices { tracks } => assert!(!tracks.is_empty()),
        other => panic!("expected a Notices doorbell, got {other:?}"),
    }
    let stored = phone
        .replica()
        .hold()
        .get_notice(notice_id)
        .await
        .unwrap()
        .expect("the pushed notice landed in the phone's hold");
    assert_eq!(phone.decrypt(&stored.notice).unwrap(), b"live!");

    // An ACL record arriving by an ordinary one-shot sync fans out live: the
    // laptop authors an invite locally and syncs it in; the subscribed phone
    // receives it as an AclPush (Acl doorbell + log growth), no re-dial.
    let mut laptop = laptop;
    laptop
        .invite(&mut rng, outpost::protocol::acl::Permissions::Writer)
        .await
        .unwrap();
    laptop.sync_with(addr.clone()).await.unwrap();

    let event = timeout(WAIT, phone_events.recv())
        .await
        .expect("acl doorbell within the wait")
        .expect("channel open");
    assert_eq!(event, SubscriptionEvent::Acl);
    assert_eq!(phone.replica().acl_len().await.unwrap(), 2, "root + invite");

    // Close the phone's subscription: Closed fires, and the registry drops
    // back to disconnected once the station notices.
    phone_sub.close();
    let event = timeout(WAIT, phone_events.recv())
        .await
        .expect("closed event within the wait")
        .expect("channel open");
    assert_eq!(event, SubscriptionEvent::Closed);

    let phone_key = phone_dev.signing.verifying_key();
    let mut disconnected = false;
    for _ in 0..100 {
        let s = station.sightings_for(&territory);
        if s.iter()
            .find(|s| *s.device() == phone_key)
            .is_some_and(|s| !s.connected)
        {
            disconnected = true;
            break;
        }
        tokio::time::sleep(TICK).await;
    }
    assert!(disconnected, "the phone shows disconnected after close");

    drop(laptop_sub);
    serve_loop.abort();
}
