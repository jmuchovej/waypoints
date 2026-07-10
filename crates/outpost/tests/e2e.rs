//! Full-stack proof: two real app databases, two coordinators, the real
//! `HttpPeer`, and a live outpost — the complete sync path over
//! actual HTTP. Plus the gossip pump: two live outposts converging
//! through pull-only anti-entropy (PROTOCOL.md §1 "Gossip").
//!
//! NOTE: All tests in this file are ignored stubs — the original tests drove
//! `SyncCoordinator` and `store::query::device` directly, and those types were
//! removed with the legacy sync layer.
//! TODO: re-enable against outpost's sync layer.

#[tokio::test]
#[ignore = "legacy sync subsystem removed; awaiting outpost sync layer"]
async fn two_devices_sync_over_http() {}

#[tokio::test]
#[ignore = "legacy sync subsystem removed; awaiting outpost sync layer"]
async fn wrong_secret_is_rejected() {}

#[tokio::test]
#[ignore = "legacy sync subsystem removed; awaiting outpost sync layer"]
async fn pump_converges_two_outposts() {}

#[tokio::test]
#[ignore = "legacy sync subsystem removed; awaiting outpost sync layer"]
async fn bidirectional_pump_reaches_fixpoint() {}

#[tokio::test]
#[ignore = "legacy sync subsystem removed; awaiting outpost sync layer"]
async fn pump_all_bootstraps_fresh_mirror_via_extra_realms() {}
