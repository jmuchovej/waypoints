//! The ACL fold, exercised as scenarios: a happy membership
//! lifecycle, then one test per rejection — every `ACLReject` variant is a
//! promised behavior, not an incidental one.

use ed25519_dalek::{Signer, SigningKey};
use outpost_protocol::{
    ACLPayload, ACLRecord, ACLReject, ACLState, InviteKey, OrderProof, Permissions, RecordBody,
    join_binding,
};
use outpost_protocol::{EpochID, MemberID, RecordID};
use outpost_protocol::{KEMPublic, KEMSecret};

fn key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

fn kem(seed: u8) -> KEMPublic {
    KEMSecret::from_seed([seed; 32]).public_key()
}

fn member(signing: &SigningKey) -> MemberID {
    MemberID::from(&signing.verifying_key())
}

/// Sign `payload` as `author`, chained onto `state` (or as a root).
fn record(author: &SigningKey, prev: Option<RecordID>, payload: ACLPayload) -> ACLRecord {
    RecordBody {
        prev,
        author: author.verifying_key(),
        payload,
    }
    .sign(author)
    .unwrap()
}

fn root(owner: &SigningKey) -> ACLRecord {
    record(
        owner,
        None,
        ACLPayload::Root {
            territory_seed: [0xA5; 32],
            owner_kem: kem(1),
            epoch0_key_wrapped: vec![0xEE; 8],
        },
    )
}

/// Owner → invite → join → approve, returning the running state and the
/// new member's signing key.
fn territory_with_writer() -> (ACLState, SigningKey, SigningKey) {
    let owner = key(1);
    let state = ACLState::genesis(&root(&owner)).unwrap();

    let invite_secret = key(9);
    let invite = InviteKey::from_bytes(invite_secret.verifying_key().to_bytes());
    let state = state
        .apply(&record(
            &owner,
            Some(state.head),
            ACLPayload::Invite {
                invite_key: invite,
                permissions: Permissions::Writer,
            },
        ))
        .unwrap();

    let joiner = key(2);
    let joiner_kem = kem(2);
    let proof = invite_secret.sign(&join_binding(&invite, &joiner.verifying_key(), &joiner_kem));
    let state = state
        .apply(&record(
            &joiner,
            Some(state.head),
            ACLPayload::Join {
                invite_key: invite,
                member_kem: joiner_kem,
                invite_proof: proof,
            },
        ))
        .unwrap();

    let state = state
        .apply(&record(
            &owner,
            Some(state.head),
            ACLPayload::Approve {
                member: member(&joiner),
                permissions: Permissions::Writer,
                current_key_wrapped: vec![0xEE; 8],
            },
        ))
        .unwrap();

    (state, owner, joiner)
}

fn remove_record(
    state: &ACLState,
    author: &SigningKey,
    target: MemberID,
    epoch: EpochID,
) -> ACLRecord {
    let wraps = state
        .members
        .keys()
        .copied()
        .filter(|m| *m != target)
        .map(|m| (m, vec![0xEE; 8]))
        .collect();
    record(
        author,
        Some(state.head),
        ACLPayload::Remove {
            member: target,
            new_epoch: epoch,
            new_key_wrapped: wraps,
            prev_key_wrapped: vec![0xEE; 8],
        },
    )
}

#[test]
fn lifecycle_invite_join_approve_remove() {
    let (state, owner, joiner) = territory_with_writer();
    assert_eq!(state.members.len(), 2);
    assert_eq!(
        state.members.get(&member(&joiner)),
        Some(&Permissions::Writer)
    );
    assert!(state.pending_invites.is_empty());
    assert!(state.pending_joins.is_empty());
    assert!(state.require_writer(&joiner.verifying_key()).is_ok());
    assert_eq!(state.epoch, EpochID::ZERO);

    // Removal rotates the epoch and drops the member.
    let state = state
        .apply(&remove_record(
            &state,
            &owner,
            member(&joiner),
            EpochID::ZERO.next(),
        ))
        .unwrap();
    assert_eq!(state.epoch, EpochID::ZERO.next());
    assert!(!state.members.contains_key(&member(&joiner)));
    assert_eq!(
        state.require_writer(&joiner.verifying_key()),
        Err(ACLReject::NotAMember)
    );
}

#[test]
fn genesis_rejects_non_root_and_chained_roots() {
    let owner = key(1);
    let not_root = record(
        &owner,
        None,
        ACLPayload::Invite {
            invite_key: InviteKey::from_bytes([9; 32]),
            permissions: Permissions::Writer,
        },
    );
    assert_eq!(ACLState::genesis(&not_root), Err(ACLReject::RootMisplaced));

    let state = ACLState::genesis(&root(&owner)).unwrap();
    let second_root = record(
        &owner,
        Some(state.head),
        ACLPayload::Root {
            territory_seed: [0; 32],
            owner_kem: kem(1),
            epoch0_key_wrapped: vec![],
        },
    );
    assert_eq!(state.apply(&second_root), Err(ACLReject::RootMisplaced));
}

#[test]
fn broken_chain_is_rejected() {
    let (state, owner, _) = territory_with_writer();
    let stale = record(
        &owner,
        Some(RecordID::from_bytes([0xFF; 32])),
        ACLPayload::Invite {
            invite_key: InviteKey::from_bytes([9; 32]),
            permissions: Permissions::Reader,
        },
    );
    assert_eq!(state.apply(&stale), Err(ACLReject::BrokenChain));
}

#[test]
fn non_members_cannot_operate() {
    let (state, _, _) = territory_with_writer();
    let outsider = key(7);
    let attempt = record(
        &outsider,
        Some(state.head),
        ACLPayload::Invite {
            invite_key: InviteKey::from_bytes([9; 32]),
            permissions: Permissions::Reader,
        },
    );
    assert_eq!(state.apply(&attempt), Err(ACLReject::NotAMember));
}

#[test]
fn writers_cannot_invite_and_admins_cannot_mint_admins() {
    let (state, owner, joiner) = territory_with_writer();

    // The Writer tries to invite: insufficient authority.
    let attempt = record(
        &joiner,
        Some(state.head),
        ACLPayload::Invite {
            invite_key: InviteKey::from_bytes([9; 32]),
            permissions: Permissions::Reader,
        },
    );
    assert_eq!(state.apply(&attempt), Err(ACLReject::InsufficientAuthority));

    // Promote the Writer to Admin (Owner may).
    let state = state
        .apply(&record(
            &owner,
            Some(state.head),
            ACLPayload::ChangeRole {
                member: member(&joiner),
                permissions: Permissions::Admin,
            },
        ))
        .unwrap();

    // An Admin inviting an Admin: Owner-only.
    let attempt = record(
        &joiner,
        Some(state.head),
        ACLPayload::Invite {
            invite_key: InviteKey::from_bytes([9; 32]),
            permissions: Permissions::Admin,
        },
    );
    assert_eq!(state.apply(&attempt), Err(ACLReject::InsufficientAuthority));

    // And an Admin demoting another Admin: also Owner-only.
    let attempt = record(
        &joiner,
        Some(state.head),
        ACLPayload::ChangeRole {
            member: member(&joiner),
            permissions: Permissions::Reader,
        },
    );
    assert_eq!(state.apply(&attempt), Err(ACLReject::InsufficientAuthority));
}

#[test]
fn join_requires_a_live_invite_and_a_valid_proof() {
    let owner = key(1);
    let state = ACLState::genesis(&root(&owner)).unwrap();

    let invite_secret = key(9);
    let invite = InviteKey::from_bytes(invite_secret.verifying_key().to_bytes());
    let joiner = key(2);
    let joiner_kem = kem(2);

    // No such invite.
    let proof = invite_secret.sign(&join_binding(&invite, &joiner.verifying_key(), &joiner_kem));
    let attempt = record(
        &joiner,
        Some(state.head),
        ACLPayload::Join {
            invite_key: invite,
            member_kem: joiner_kem.clone(),
            invite_proof: proof,
        },
    );
    assert_eq!(state.apply(&attempt), Err(ACLReject::UnknownInvite));

    let state = state
        .apply(&record(
            &owner,
            Some(state.head),
            ACLPayload::Invite {
                invite_key: invite,
                permissions: Permissions::Writer,
            },
        ))
        .unwrap();

    // Proof signed for a DIFFERENT joiner identity: replay is rejected.
    let impostor = key(3);
    let stolen = record(
        &impostor,
        Some(state.head),
        ACLPayload::Join {
            invite_key: invite,
            member_kem: joiner_kem.clone(),
            invite_proof: proof,
        },
    );
    assert_eq!(state.apply(&stolen), Err(ACLReject::BadInviteProof));

    // Legitimate join consumes the invite (single-use).
    let state = state
        .apply(&record(
            &joiner,
            Some(state.head),
            ACLPayload::Join {
                invite_key: invite,
                member_kem: joiner_kem.clone(),
                invite_proof: proof,
            },
        ))
        .unwrap();
    assert!(state.pending_invites.is_empty());

    let rejoin = record(
        &key(4),
        Some(state.head),
        ACLPayload::Join {
            invite_key: invite,
            member_kem: joiner_kem,
            invite_proof: proof,
        },
    );
    assert_eq!(state.apply(&rejoin), Err(ACLReject::UnknownInvite));
}

#[test]
fn approve_requires_a_pending_join() {
    let (state, owner, _) = territory_with_writer();
    let attempt = record(
        &owner,
        Some(state.head),
        ACLPayload::Approve {
            member: MemberID::from_bytes([9; 32]),
            permissions: Permissions::Reader,
            current_key_wrapped: vec![],
        },
    );
    assert_eq!(state.apply(&attempt), Err(ACLReject::UnknownJoin));
}

#[test]
fn owner_is_immutable() {
    let (state, owner, joiner) = territory_with_writer();

    let demote = record(
        &owner,
        Some(state.head),
        ACLPayload::ChangeRole {
            member: member(&owner),
            permissions: Permissions::Reader,
        },
    );
    assert_eq!(state.apply(&demote), Err(ACLReject::OwnerImmutable));

    let evict = remove_record(&state, &joiner, member(&owner), state.epoch.next());
    assert_eq!(state.apply(&evict), Err(ACLReject::OwnerImmutable));
}

#[test]
fn remove_must_rotate_exactly_once_and_cover_everyone() {
    let (state, owner, joiner) = territory_with_writer();

    // Wrong epoch: skipping ahead.
    let skip = remove_record(&state, &owner, member(&joiner), EpochID::from(2));
    assert_eq!(state.apply(&skip), Err(ACLReject::WrongEpoch));

    // Wraps that miss the remaining member set.
    let uncovered = record(
        &owner,
        Some(state.head),
        ACLPayload::Remove {
            member: member(&joiner),
            new_epoch: state.epoch.next(),
            new_key_wrapped: vec![],
            prev_key_wrapped: vec![0xEE; 8],
        },
    );
    assert_eq!(state.apply(&uncovered), Err(ACLReject::WrapCoverage));

    // Wraps that include the removed member: also wrong.
    let overcovered = record(
        &owner,
        Some(state.head),
        ACLPayload::Remove {
            member: member(&joiner),
            new_epoch: state.epoch.next(),
            new_key_wrapped: state
                .members
                .keys()
                .copied()
                .map(|m| (m, vec![0xEE; 8]))
                .collect(),
            prev_key_wrapped: vec![0xEE; 8],
        },
    );
    assert_eq!(state.apply(&overcovered), Err(ACLReject::WrapCoverage));
}

#[test]
fn epoch_pinning_is_current_or_older() {
    let (state, owner, joiner) = territory_with_writer();
    assert!(state.require_epoch_current_or_older(EpochID::ZERO).is_ok());
    assert_eq!(
        state.require_epoch_current_or_older(EpochID::from(1)),
        Err(ACLReject::WrongEpoch)
    );

    let state = state
        .apply(&remove_record(
            &state,
            &owner,
            member(&joiner),
            state.epoch.next(),
        ))
        .unwrap();
    assert!(state.require_epoch_current_or_older(EpochID::ZERO).is_ok());
    assert!(
        state
            .require_epoch_current_or_older(EpochID::from(1))
            .is_ok()
    );
}

#[test]
fn tampered_records_are_wire_rejects() {
    let (state, owner, _) = territory_with_writer();
    let good = record(
        &owner,
        Some(state.head),
        ACLPayload::Invite {
            invite_key: InviteKey::from_bytes([9; 32]),
            permissions: Permissions::Reader,
        },
    );
    let mut bytes = good.bytes.to_vec();
    *bytes.last_mut().unwrap() ^= 1;
    let tampered = ACLRecord {
        bytes: bytes.into(),
        signature: good.signature,
    };
    assert!(matches!(state.apply(&tampered), Err(ACLReject::Wire(_))));
}

#[test]
fn order_proofs_verify_and_bind_position() {
    let fleet = key(42);
    let (state, _, _) = territory_with_writer();
    let proof = OrderProof::sign(&fleet, state.head, 3);
    assert!(proof.verify(&fleet.verifying_key(), state.head).is_ok());

    // A different record or a shifted position is orderer equivocation
    // evidence, not a valid proof.
    assert!(
        proof
            .verify(&fleet.verifying_key(), RecordID::from_bytes([1; 32]))
            .is_err()
    );
    let shifted = OrderProof {
        position: 4,
        signature: proof.signature,
    };
    assert!(shifted.verify(&fleet.verifying_key(), state.head).is_err());
}
