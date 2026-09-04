//! Connection liveness for parked live-push sessions.
//!
//! A live [`Subscribe`] session can sit idle indefinitely — no reconcile,
//! no push — while remaining healthy. Nothing in the framing distinguishes
//! that from a peer that vanished (sleep, network change, dropped
//! cellular): a silently-severed QUIC stream yields no error until
//! something is written to it. So each side signals liveness on a timer and
//! reaps a session that goes quiet.
//!
//! **No new wire type.** The keepalive is an empty
//! [`NoticePush`](outpost_protocol::SyncMessage::NoticePush) — a frame both
//! roles already treat as a no-op — so the heartbeat rides the existing
//! session framing.
//!
//! [`Subscribe`]: outpost_protocol::SyncMessage::Subscribe

use crate::protocol::SyncMessage;
use std::time::Duration;

/// How often a parked session emits a keepalive.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);

/// How long a session may produce no frame at all (keepalive included)
/// before its peer treats it as dead. Comfortably more than two heartbeat
/// intervals, so an isolated lost keepalive does not reap a live session.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(45);

/// A session's liveness timings. The defaults ([`HEARTBEAT_INTERVAL`],
/// [`IDLE_TIMEOUT`]) suit production; tests inject short durations to
/// exercise reaping and reconnect without waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Liveness {
    /// How often this side emits a keepalive.
    pub heartbeat: Duration,
    /// How long this side tolerates silence before reaping the session.
    pub idle_timeout: Duration,
}

impl Default for Liveness {
    fn default() -> Self {
        Self {
            heartbeat: HEARTBEAT_INTERVAL,
            idle_timeout: IDLE_TIMEOUT,
        }
    }
}

/// The keepalive frame: an empty `NoticePush`, a no-op on both roles.
pub fn keepalive() -> SyncMessage {
    SyncMessage::NoticePush {
        notices: Vec::new(),
    }
}

/// Whether a received frame is a content-free keepalive (an empty
/// `NoticePush`). Any frame — this one included — proves the peer is alive
/// and resets the idle timer; this only tells a loop there is no payload to
/// act on.
pub fn is_keepalive(frame: &SyncMessage) -> bool {
    matches!(frame, SyncMessage::NoticePush { notices } if notices.is_empty())
}
