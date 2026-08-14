//! The membership ceremony driven through real Vessel APIs: an owner
//! creates a Territory, invites a user, the user joins, the owner approves,
//! and the new member reads history — no hand-built ACL records. Then
//! removal rotates the epoch and cuts the removed member off, and an
//! unauthorized operation is refused.
//!
//! Records move between the two vessels by direct `ingest_acl` here (the
//! sync path that carries them is tested elsewhere); this keeps the test
//! focused on the authoring APIs.

use outpost::{
    protocol::{EpochID, KeyRoot, NoticeID, Permissions},
    vessel::Vessel,
};
use rand::SeedableRng;

fn identity(seed: u8) -> (outpost::protocol::Identity, outpost::protocol::DeviceKey) {
    let root = KeyRoot::from_seed([seed; 64]);
    (root.identity(), root.device(0))
}

#[tokio::test]
async fn owner_invites_user_joins_and_reads() {
    let mut rng = rand::rngs::StdRng::from_seed([1; 32]);
    let (owner_id, owner_dev) = identity(1);
    let (user_id, user_dev) = identity(2);

    // Owner creates the Territory and authors an epoch-0 Notice.
    let mut owner = Vessel::create_territory(&owner_id, &owner_dev, ":memory:", &mut rng)
        .await
        .unwrap();
    assert_eq!(owner.readable_epochs(), vec![EpochID::ZERO]);
    let secret_note = owner
        .author(
            &mut rng,
            vec![],
            EpochID::ZERO,
            "waypoints.page.v1",
            b"members only",
        )
        .await
        .unwrap();
    let note_id = NoticeID::of(&secret_note).unwrap();

    // Owner invites the user (Writer) and hands over the invite secret.
    let (invite_key, invite_secret, invite_rec) =
        owner.invite(&mut rng, Permissions::Writer).await.unwrap();

    // The user opens their Vessel over the Territory so far (root + invite),
    // then joins.
    let acl_so_far = owner.replica().hold().acl_records().await.unwrap();
    let user = Vessel::new(
        user_id.signing_key().clone(),
        user_id.secret_key(),
        outpost::replica::Replica::bootstrap(":memory:", &acl_so_far)
            .await
            .unwrap(),
    )
    .await
    .unwrap();
    let _ = (&user_dev, &invite_rec); // (device cert + record used via the log)
    let join_rec = user.join(invite_key, invite_secret).await.unwrap();

    // Owner receives the Join and approves it — sealing the current key to
    // the new member.
    owner.replica().ingest_acl(vec![join_rec]).await.unwrap();
    let approve_rec = owner
        .approve(&mut rng, user_id.member_id(), Permissions::Writer)
        .await
        .unwrap();

    // The user catches up on the Approve (+ owner's earlier Notice), derives
    // the key, and reads the members-only Notice.
    user.replica().ingest_acl(vec![approve_rec]).await.unwrap();
    user.replica().ingest(vec![secret_note]).await.unwrap();
    let mut user = user;
    user.refresh_keys().await.unwrap();
    assert_eq!(user.readable_epochs(), vec![EpochID::ZERO]);
    let stored = user
        .replica()
        .hold()
        .get_notice(note_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(user.decrypt(&stored.notice).unwrap(), b"members only");
}

#[tokio::test]
async fn remove_rotates_and_cuts_off() {
    let mut rng = rand::rngs::StdRng::from_seed([2; 32]);
    let (owner_id, owner_dev) = identity(1);
    let (user_id, _user_dev) = identity(2);

    let mut owner = Vessel::create_territory(&owner_id, &owner_dev, ":memory:", &mut rng)
        .await
        .unwrap();
    // Bring `user` in via invite/join/approve.
    let (ik, isec, _) = owner.invite(&mut rng, Permissions::Writer).await.unwrap();
    let acl = owner.replica().hold().acl_records().await.unwrap();
    let mut user = Vessel::new(
        user_id.signing_key().clone(),
        user_id.secret_key(),
        outpost::replica::Replica::bootstrap(":memory:", &acl)
            .await
            .unwrap(),
    )
    .await
    .unwrap();
    let join = user.join(ik, isec).await.unwrap();
    owner.replica().ingest_acl(vec![join]).await.unwrap();
    let approve = owner
        .approve(&mut rng, user_id.member_id(), Permissions::Writer)
        .await
        .unwrap();
    user.replica().ingest_acl(vec![approve]).await.unwrap();
    user.refresh_keys().await.unwrap();
    assert_eq!(user.readable_epochs(), vec![EpochID::ZERO]);

    // Owner removes the user → epoch 1. The owner rotates forward; the
    // removed user, syncing the Remove, gets no epoch-1 key.
    let remove = owner.remove(&mut rng, user_id.member_id()).await.unwrap();
    assert_eq!(
        owner.readable_epochs(),
        vec![EpochID::ZERO, EpochID::from(1)]
    );
    user.replica().ingest_acl(vec![remove]).await.unwrap();
    user.refresh_keys().await.unwrap();
    assert_eq!(
        user.readable_epochs(),
        vec![EpochID::ZERO],
        "the removed member is capped at its pre-removal epoch",
    );
}

#[tokio::test]
async fn unauthorized_operations_are_refused() {
    let mut rng = rand::rngs::StdRng::from_seed([3; 32]);
    let (owner_id, owner_dev) = identity(1);
    let (user_id, _) = identity(2);

    let mut owner = Vessel::create_territory(&owner_id, &owner_dev, ":memory:", &mut rng)
        .await
        .unwrap();
    let (ik, isec, _) = owner.invite(&mut rng, Permissions::Writer).await.unwrap();
    let acl = owner.replica().hold().acl_records().await.unwrap();
    let user = Vessel::new(
        user_id.signing_key().clone(),
        user_id.secret_key(),
        outpost::replica::Replica::bootstrap(":memory:", &acl)
            .await
            .unwrap(),
    )
    .await
    .unwrap();
    let join = user.join(ik, isec).await.unwrap();
    owner
        .replica()
        .ingest_acl(vec![join.clone()])
        .await
        .unwrap();
    owner
        .approve(&mut rng, user_id.member_id(), Permissions::Writer)
        .await
        .unwrap();

    // The now-approved Writer tries to invite — insufficient authority. The
    // local apply fails (the fold rejects it).
    let user = {
        let full = owner.replica().hold().acl_records().await.unwrap();
        Vessel::new(
            user_id.signing_key().clone(),
            user_id.secret_key(),
            outpost::replica::Replica::bootstrap(":memory:", &full)
                .await
                .unwrap(),
        )
        .await
        .unwrap()
    };
    assert!(
        user.invite(&mut rng, Permissions::Reader).await.is_err(),
        "a Writer may not invite",
    );
}
