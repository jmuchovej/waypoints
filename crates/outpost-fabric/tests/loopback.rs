//! The session convergence flow over **real iroh QUIC on loopback**
//! — the same exchange `session.rs` proves over an in-memory duplex, now
//! across two bound endpoints with an authenticated handshake. This is the
//! transport integration proof: framing over QUIC streams, connect/accept,
//! and roster-key extraction from the authenticated connection.
//!
//! Kept minimal (empty holds, so the reconciliation is a no-op transfer):
//! the convergence *logic* is covered by `session.rs`; this test's job is
//! to prove the same code drives correctly over the real transport, and
//! that a Vessel can read the Station's authenticated identity for a roster
//! check.

use std::collections::BTreeSet;

use async_trait::async_trait;
use ed25519_dalek::{SigningKey, VerifyingKey};
use iroh::SecretKey;
use outpost_fabric::{
    Endpoint, SyncPeer,
    fleet::{FleetId, RosterBody, StationEntry, StationRoles},
    initiate, serve,
};
use outpost_protocol::{NoticeID, OrderedRecord, SignedNotice, TallyDigest, TrackID};

/// An empty peer: no Notices, so a session is a clean no-op round-trip.
/// Enough to exercise framing + connect/accept over real QUIC.
struct EmptyPeer;

#[async_trait]
impl SyncPeer for EmptyPeer {
    async fn digest(&self) -> TallyDigest {
        TallyDigest::of_heads(&std::collections::BTreeMap::new())
    }
    async fn track_notices(&self, _track: TrackID) -> Vec<SignedNotice> {
        Vec::new()
    }
    async fn held_ids(&self) -> BTreeSet<NoticeID> {
        BTreeSet::new()
    }
    async fn ingest(&self, _notices: Vec<SignedNotice>) {}
    async fn acl_len(&self) -> u64 {
        0
    }
    async fn acl_suffix(&self, _from: u64) -> Vec<OrderedRecord> {
        Vec::new()
    }
    async fn ingest_ordered_acl(&self, _records: Vec<OrderedRecord>) {}
}

#[tokio::test]
async fn session_runs_over_real_quic_loopback() {
    let station_secret = SecretKey::from_bytes(&[0x5A; 32]);
    let station_key: VerifyingKey = station_secret.public().as_verifying_key();

    let station = Endpoint::bind(station_secret).await.expect("bind station");
    let vessel = Endpoint::bind(SecretKey::from_bytes(&[0x11; 32]))
        .await
        .expect("bind vessel");
    // Dial the station's real bound socket directly — loopback needs no
    // relay/DNS discovery (air-gapped posture).
    let station_addr = station.direct_addr();

    // Both endpoints stay alive for the whole exchange: the initiator
    // reads the responder's last frame, so tearing either side down early
    // would abort the connection. join both, then close gracefully.
    let station_fut = async {
        let mut session = station
            .accept()
            .await
            .expect("accept")
            .expect("a connection");
        // The vessel's authenticated identity, for a Station's own roster
        // check in production.
        assert_eq!(
            session.remote_key(),
            SecretKey::from_bytes(&[0x11; 32])
                .public()
                .as_verifying_key()
        );
        serve(&EmptyPeer, &mut session.send, &mut session.recv)
            .await
            .expect("serve");
        // Hold the connection open until the vessel has read our last frame
        // and closed the session.
        session.drain().await;
    };

    let vessel_fut = async {
        let mut session = vessel.connect(station_addr).await.expect("connect");
        // iroh authenticates the station's roster identity in the handshake.
        assert_eq!(session.remote_key(), station_key);

        // A roster the vessel trusts lists `station_key`; the authenticated
        // remote identity is exactly what a roster admission check keys on.
        let fleet_key = SigningKey::from_bytes(&[0xF1; 32]);
        let roster = RosterBody {
            fleet: FleetId(fleet_key.verifying_key()),
            revision: 1,
            stations: vec![StationEntry {
                peer_key: station_key,
                addrs: vec![],
                roles: StationRoles {
                    storage: true,
                    orderer: true,
                    blob: false,
                },
            }],
        }
        .sign(&fleet_key);
        assert!(roster.verify().is_ok());
        assert!(roster.station(&session.remote_key()).is_some());

        initiate(&EmptyPeer, &mut session.send, &mut session.recv)
            .await
            .expect("initiate");
        // Read done; signal a clean end so the station stops draining.
        session.close();
    };

    tokio::join!(station_fut, vessel_fut);

    station.close().await;
    vessel.close().await;
}
