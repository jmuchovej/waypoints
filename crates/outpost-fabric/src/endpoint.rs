//! iroh wiring: QUIC, hole-punching, relays when NAT wins. Stations
//! are always-reachable iroh endpoints listed in the Fleet config;
//! Vessels dial them and drive [`session`](crate::session) over one
//! bidirectional stream.
//!
//! Deliberately thin — everything protocol-shaped lives transport-generic
//! in [`session`](crate::session), so a `quinn`+static-address fallback
//! would replace only this file. iroh authenticates the remote
//! endpoint's Ed25519 id for free, so [`remote_key`] hands the session the
//! peer identity a roster check ([`FleetConfig::station`]) consumes.
//!
//! [`FleetConfig::station`]: crate::fleet::FleetConfig::station

use ed25519_dalek::VerifyingKey;
use iroh::{
    EndpointId, SecretKey, TransportAddr,
    endpoint::{Connection, RecvStream, SendStream},
};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

pub use iroh::EndpointAddr;

/// One protocol, one ALPN; wire-version negotiation happens inside the
/// session, not here.
pub const ALPN: &[u8] = b"outpost/sync/v1";

/// iroh's error surface is large and version-mobile; sessions only ever
/// retry-or-report, so a message is all a caller needs.
#[derive(Debug, thiserror::Error)]
#[error("transport: {0}")]
pub struct TransportError(String);

fn err(e: impl std::fmt::Display) -> TransportError {
    TransportError(e.to_string())
}

/// Rewrite an unspecified bind address to loopback; a wildcard bind is not
/// something a peer can dial.
fn to_dialable(socket: SocketAddr) -> SocketAddr {
    match socket.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), socket.port())
        }
        IpAddr::V6(ip) if ip.is_unspecified() => {
            SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), socket.port())
        }
        _ => socket,
    }
}

/// An open bidirectional stream plus the authenticated peer identity.
pub struct Session {
    pub connection: Connection,
    pub send: SendStream,
    pub recv: RecvStream,
}

impl Session {
    /// The remote peer's identity key, as authenticated by iroh's QUIC/TLS
    /// handshake — the input to a roster admission check.
    pub fn remote_key(&self) -> VerifyingKey {
        self.connection.remote_id().as_verifying_key()
    }

    /// Signal a clean end of session to the peer. The initiator calls this
    /// once it has read the final response; the responder should
    /// [`drain`](Self::drain) until it observes the close, so no in-flight
    /// frame is lost to an abrupt teardown.
    pub fn close(&self) {
        self.connection.close(0u32.into(), b"");
    }

    /// Wait for the peer to close the session — how a responder holds its
    /// connection open long enough for the initiator to read the last frame.
    pub async fn drain(&self) {
        let _ = self.connection.closed().await;
    }
}

/// A bound iroh endpoint with our ALPN.
pub struct Endpoint {
    inner: iroh::Endpoint,
}

impl Endpoint {
    /// Bind an endpoint whose identity is `secret` — a Station's roster key
    /// or a Vessel's device key. The `Minimal` preset is LAN-only
    /// (no relay, no DNS): the right posture for air-gapped Fleets
    /// and for loopback tests. Relay-assisted presets layer on later.
    ///
    /// Binds **loopback only** (`127.0.0.1`/`[::1]`), replacing iroh's
    /// default wildcard sockets, and disables the UPnP/NAT-PMP portmapper
    /// (whose SSDP probe binds its own wildcard socket): every current
    /// posture is same-host ([`direct_addr`](Self::direct_addr) rewrites a
    /// wildcard to loopback before publishing anyway), and both wildcard
    /// binds make macOS's application firewall prompt for every unsigned
    /// rebuilt binary. A deployment that must be reachable off-host uses
    /// [`bind_routable`](Self::bind_routable).
    pub async fn bind(secret: SecretKey) -> Result<Self, TransportError> {
        let inner = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
            .secret_key(secret)
            .alpns(vec![ALPN.to_vec()])
            .portmapper_config(iroh::endpoint::PortmapperConfig::Disabled)
            .clear_ip_transports()
            .bind_addr("127.0.0.1:0")
            .map_err(err)?
            .bind_addr("[::1]:0")
            .map_err(err)?
            .bind()
            .await
            .map_err(err)?;
        Ok(Self { inner })
    }

    /// [`bind`](Self::bind) on the wildcard address (`0.0.0.0`/`[::]`) —
    /// reachable from other hosts. The posture for a deployed Station whose
    /// roster entry publishes real addresses; nothing same-host needs it.
    pub async fn bind_routable(secret: SecretKey) -> Result<Self, TransportError> {
        let inner = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
            .secret_key(secret)
            .alpns(vec![ALPN.to_vec()])
            .bind()
            .await
            .map_err(err)?;
        Ok(Self { inner })
    }

    /// [`bind`](Self::bind) from an Ed25519 identity key. The 32-byte seed
    /// is shared, so the endpoint's authenticated id equals the key's public
    /// half — the roster peer key. Lets the role crates bind without
    /// touching iroh directly (only fabric knows iroh).
    pub async fn bind_identity(key: &ed25519_dalek::SigningKey) -> Result<Self, TransportError> {
        Self::bind(SecretKey::from_bytes(&key.to_bytes())).await
    }

    /// [`bind_routable`](Self::bind_routable) from an Ed25519 identity key —
    /// the off-host counterpart of [`bind_identity`](Self::bind_identity).
    /// A Station uses it to accept from other hosts; a Vessel uses it so its
    /// outbound packets can leave the host at all (a loopback-bound source
    /// socket cannot dial a remote Station). Neither role gains inbound
    /// *requirements* from this: Vessels stay dial-only.
    pub async fn bind_routable_identity(
        key: &ed25519_dalek::SigningKey,
    ) -> Result<Self, TransportError> {
        Self::bind_routable(SecretKey::from_bytes(&key.to_bytes())).await
    }

    /// This endpoint's dialable address (id + discovered socket hints).
    /// For relay-assisted presets this may take a moment to populate; on a
    /// LAN/loopback Fleet prefer [`direct_addr`](Self::direct_addr).
    pub fn addr(&self) -> EndpointAddr {
        self.inner.addr()
    }

    /// This endpoint's identity as an iroh id — its roster peer key.
    pub fn id(&self) -> EndpointId {
        self.inner.id()
    }

    /// A dialable address built from the actual bound socket(s), skipping
    /// discovery — the reliable form for LAN/loopback Fleets whose Stations
    /// publish a fixed address in the roster. Unspecified bind
    /// addresses (`0.0.0.0` / `::`) are rewritten to loopback so the result
    /// is dialable on the same host.
    pub fn direct_addr(&self) -> EndpointAddr {
        let addrs = self
            .inner
            .bound_sockets()
            .into_iter()
            .map(|s| TransportAddr::Ip(to_dialable(s)));
        EndpointAddr::from_parts(self.inner.id(), addrs)
    }

    /// A dialable address advertising `public` — operator-configured
    /// sockets (a public IP or DNS-resolved host:port) — instead of the
    /// bound sockets. What a routable Station hands to Vessels on other
    /// hosts, where the wildcard-bound sockets are meaningless.
    pub fn advertised_addr(&self, public: impl IntoIterator<Item = SocketAddr>) -> EndpointAddr {
        EndpointAddr::from_parts(self.inner.id(), public.into_iter().map(TransportAddr::Ip))
    }

    /// Dial a peer and open the session's bidirectional stream.
    pub async fn connect(&self, addr: impl Into<EndpointAddr>) -> Result<Session, TransportError> {
        let connection = self.inner.connect(addr, ALPN).await.map_err(err)?;
        let (send, recv) = connection.open_bi().await.map_err(err)?;
        Ok(Session {
            connection,
            send,
            recv,
        })
    }

    /// Accept the next inbound session (connection + its first stream), or
    /// `None` once the endpoint is closed.
    pub async fn accept(&self) -> Result<Option<Session>, TransportError> {
        let Some(incoming) = self.inner.accept().await else {
            return Ok(None);
        };
        let connection = incoming.await.map_err(err)?;
        let (send, recv) = connection.accept_bi().await.map_err(err)?;
        Ok(Some(Session {
            connection,
            send,
            recv,
        }))
    }

    /// Close the endpoint, terminating all sessions.
    pub async fn close(&self) {
        self.inner.close().await;
    }
}
