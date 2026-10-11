//! Shared test scaffolding: build Territories whose ACL records carry
//! *real* sealed boxes, so tests exercise the actual key-acquisition path
//! ([`Vessel::refresh_keys`](outpost::vessel::Vessel::refresh_keys)) rather
//! than hand-fed keys.
//!
//! A [`TerritoryBuilder`] threads the running head and a live ACL fold so
//! each record chains correctly and rotations land at the right epoch. It
//! is deterministic (seeded RNG) so the sealed boxes are reproducible.

#![allow(dead_code, reason = "each integration test uses a different subset")]

pub mod process;

use ed25519_dalek::{Signer, SigningKey};
use outpost::protocol::{
    ACLPayload, ACLRecord, ACLState, EpochID, InviteKey, KEMSecret, MemberID, Permissions, ReadKey,
    RecordBody, RecordID, join_binding, seal_key_to_member, wrap_prev_key,
};
use rand::{SeedableRng, rngs::StdRng};

/// A member's deterministic identity + KEM keypair, addressed by a seed.
pub fn signing(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

pub fn kem(seed: u8) -> KEMSecret {
    // KEM seed kept distinct from the signing seed so the two key spaces
    // never accidentally coincide in a test.
    KEMSecret::from_seed([seed.wrapping_add(0x80); 32])
}

pub fn member_id(seed: u8) -> MemberID {
    MemberID::from(&signing(seed).verifying_key())
}

/// Builds a chained ACL log with real sealed boxes and tracks the fold so
/// callers can assert against membership/epoch as they go.
pub struct TerritoryBuilder {
    rng: StdRng,
    records: Vec<ACLRecord>,
    fold: ACLState,
    head: RecordID,
}

impl TerritoryBuilder {
    /// Create a Territory owned by member `owner_seed`, sealing epoch 0's
    /// `read_key` to the owner's KEM.
    pub fn new(owner_seed: u8, read_key: &ReadKey) -> Self {
        let mut rng = StdRng::from_seed([0x11; 32]);
        let owner = signing(owner_seed);
        let owner_kem = kem(owner_seed);
        let sealed = seal_key_to_member(&mut rng, &owner_kem.public_key(), read_key).unwrap();
        let root = RecordBody {
            prev: None,
            author: owner.verifying_key(),
            payload: ACLPayload::Root {
                territory_seed: [0xA5; 32],
                owner_kem: owner_kem.public_key(),
                epoch0_key_wrapped: sealed,
            },
        }
        .sign(&owner)
        .unwrap();
        let fold = ACLState::genesis(&root).unwrap();
        let head = fold.head;
        Self {
            rng,
            records: vec![root],
            fold,
            head,
        }
    }

    fn push(&mut self, author: &SigningKey, payload: ACLPayload) {
        let record = RecordBody {
            prev: Some(self.head),
            author: author.verifying_key(),
            payload,
        }
        .sign(author)
        .unwrap();
        self.fold = self.fold.apply(&record).unwrap();
        self.head = self.fold.head;
        self.records.push(record);
    }

    /// Admit member `who_seed` as a Writer at the current epoch, sealing the
    /// current `read_key` to their KEM (the full Invite → Join → Approve
    /// sequence). `admin_seed` must have Admin+ authority.
    pub fn admit(&mut self, admin_seed: u8, who_seed: u8, read_key: &ReadKey) {
        let admin = signing(admin_seed);
        let who = signing(who_seed);
        let who_kem = kem(who_seed);

        // A fresh invite key per admission (invite seed derived from the
        // joiner so tests stay deterministic and collision-free).
        let invite_secret = signing(who_seed.wrapping_add(0x40));
        let invite = InviteKey::from_bytes(invite_secret.verifying_key().to_bytes());
        self.push(
            &admin,
            ACLPayload::Invite {
                invite_key: invite,
                permissions: Permissions::Writer,
            },
        );

        let proof = invite_secret.sign(&join_binding(
            &invite,
            &who.verifying_key(),
            &who_kem.public_key(),
        ));
        self.push(
            &who,
            ACLPayload::Join {
                invite_key: invite,
                member_kem: who_kem.public_key(),
                invite_proof: proof,
            },
        );

        let sealed = seal_key_to_member(&mut self.rng, &who_kem.public_key(), read_key).unwrap();
        self.push(
            &admin,
            ACLPayload::Approve {
                member: member_id(who_seed),
                permissions: Permissions::Writer,
                current_key_wrapped: sealed,
            },
        );
    }

    /// Remove member `who_seed`, rotating to `next_key`: seal it to every
    /// remaining member, and wrap the outgoing `prev_key` under it so the
    /// backwards chain stays walkable. `admin_seed` must outrank the
    /// target.
    pub fn remove(&mut self, admin_seed: u8, who_seed: u8, prev_key: &ReadKey, next_key: &ReadKey) {
        let admin = signing(admin_seed);
        let target = member_id(who_seed);
        let new_epoch = self.fold.epoch.next();

        // Seal the new key to each member that remains after the removal.
        let remaining: Vec<MemberID> = self
            .fold
            .members
            .keys()
            .copied()
            .filter(|m| *m != target)
            .collect();
        let mut new_key_wrapped = Vec::new();
        for member in remaining {
            let sealed = seal_for(&mut self.rng, member, next_key);
            new_key_wrapped.push((member, sealed));
        }
        let prev_key_wrapped = wrap_prev_key(&mut self.rng, next_key, prev_key);

        self.push(
            &admin,
            ACLPayload::Remove {
                member: target,
                new_epoch,
                new_key_wrapped,
                prev_key_wrapped,
            },
        );
    }

    pub fn records(&self) -> &[ACLRecord] {
        &self.records
    }

    pub fn into_records(self) -> Vec<ACLRecord> {
        self.records
    }

    pub fn epoch(&self) -> EpochID {
        self.fold.epoch
    }
}

/// Seal `key` to the KEM of the member behind `id`. The builder knows each
/// member's KEM by seed, but a `Remove`'s remaining set is `MemberID`s — so
/// map back through the seeds the builder uses. Tests only ever remove
/// members created via [`TerritoryBuilder::admit`]/`new`, whose seeds are
/// small, so a bounded scan recovers the KEM.
fn seal_for(rng: &mut StdRng, id: MemberID, key: &ReadKey) -> Vec<u8> {
    for seed in 0u8..=255 {
        if member_id(seed) == id {
            return seal_key_to_member(rng, &kem(seed).public_key(), key).unwrap();
        }
    }
    panic!("unknown member id in remaining set — not created via the builder");
}
