//! A Station with many stored Territories answers its probes *while* it
//! loads them: `/healthz` 200 and `/readyz` 503 through the scan, then 200.
//! The holds are real — homed over loopback by an in-process Station — so
//! the scan is the one a deployment pays, not a sleep standing in for it.

#![cfg(unix)]
#![allow(
    clippy::disallowed_macros,
    reason = "the observed window is for the person running the test; there is no runtime to swallow stdout"
)]

mod common;

use common::process::{outpost, try_http_get};
use ed25519_dalek::SigningKey;
use outpost::{protocol::KeyRoot, station::Station, vessel::Vessel};
use rand::SeedableRng;
use std::{
    net::{Ipv4Addr, SocketAddr, TcpListener, UdpSocket},
    process::Stdio,
    sync::Arc,
    time::{Duration, Instant},
};

/// Enough holds for the scan to outlast many 20 ms polls on a debug build;
/// on kind, 300 holds took a release binary about five seconds.
const HOLDS: u32 = 300;

/// A port free for both protocols right now — the binary binds both.
fn free_port() -> u16 {
    for _ in 0..32 {
        let tcp = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = tcp.local_addr().unwrap().port();
        if UdpSocket::bind((Ipv4Addr::LOCALHOST, port)).is_ok() {
            return port;
        }
    }
    panic!("no port free for both TCP and UDP");
}

#[tokio::test(flavor = "multi_thread")]
async fn readyz_answers_503_while_stored_territories_load() {
    let dir = tempfile::tempdir().unwrap();
    let territories = dir.path().join("territories");

    // Seed: an in-process Station homes HOLDS Territories, each registered
    // over loopback by its owner.
    {
        let station = Arc::new(
            Station::hosting(&SigningKey::from_bytes(&[42; 32]), &territories)
                .await
                .unwrap(),
        );
        let addr = station.endpoint().direct_addr();
        let serving = Arc::clone(&station);
        let serve = tokio::spawn(async move { serving.serve_loop().await });
        for seed in 1..=HOLDS {
            let mut bytes = [0x5Eu8; 64];
            bytes[..4].copy_from_slice(&seed.to_le_bytes());
            let root = KeyRoot::from_seed(bytes);
            let mut rng = rand::rngs::StdRng::from_seed(bytes[..32].try_into().unwrap());
            let owner =
                Vessel::create_territory(&root.identity(), &root.device(0), ":memory:", &mut rng)
                    .await
                    .unwrap();
            let territory = owner.territory_id().await.unwrap();
            owner.register(addr.clone(), territory).await.unwrap();
        }
        serve.abort();
        station.endpoint().close().await;
    }
    let holds = std::fs::read_dir(&territories)
        .unwrap()
        .filter(|e| {
            let p = e.as_ref().unwrap().path();
            p.extension().is_some_and(|x| x == "db") && p.file_stem().is_some_and(|s| s.len() == 64)
        })
        .count();
    assert_eq!(holds as u32, HOLDS, "seeded holds on disk");

    // The binary on a port known in advance, so the probe can be polled
    // from the first moment, before anything is logged.
    let port = free_port();
    let mut child = outpost()
        .args(["--log-format", "json", "station", "--data"])
        .arg(dir.path())
        .args(["--port", &port.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let probe = SocketAddr::from((Ipv4Addr::LOCALHOST, port));

    let started = Instant::now();
    let deadline = started + Duration::from_secs(120);
    let mut first_503 = None;
    let mut healthz_while_loading = None;
    let mut ready_at = None;
    while Instant::now() < deadline {
        match try_http_get(probe, "/readyz").await {
            None => {} // not bound yet
            Some((503, body)) => {
                assert!(
                    body.is_empty(),
                    "the Station's 503 carries no body: {body:?}"
                );
                first_503.get_or_insert_with(|| started.elapsed());
                if healthz_while_loading.is_none() {
                    healthz_while_loading = try_http_get(probe, "/healthz").await.map(|(c, _)| c);
                }
            }
            Some((200, _)) => {
                ready_at = Some(started.elapsed());
                break;
            }
            Some(other) => panic!("unexpected probe answer {other:?}"),
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let _ = child.kill();
    let _ = child.wait();

    let ready_at = ready_at.expect("the Station became ready");
    let first_503 = first_503.unwrap_or_else(|| {
        panic!("/readyz never answered 503 while {HOLDS} holds loaded (ready after {ready_at:.1?}): the probes are not served before the scan")
    });
    assert_eq!(
        healthz_while_loading,
        Some(200),
        "/healthz answers while loading"
    );
    println!("not-ready window observed from {first_503:.2?} to {ready_at:.2?} for {HOLDS} holds");
}
