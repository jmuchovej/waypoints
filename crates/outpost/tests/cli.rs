//! The operator-facing shape of the binary: the probe routes answer on TCP
//! at the Station's own port number, a port it cannot have is refused by
//! name, and `keygen` writes a key exactly once.

#![cfg(unix)]

#[path = "common/process.rs"]
mod process;

use process::{StationProcess, http_get, outpost};
use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener},
    time::Duration,
};

#[tokio::test(flavor = "multi_thread")]
async fn probes_answer_on_tcp_at_the_quic_port_number() {
    let dir = tempfile::tempdir().unwrap();
    let mut station = StationProcess::spawn(dir.path(), &[]);

    // Ephemeral mode, and still one number: every advertised QUIC socket
    // is on the logged port, and so are the probes.
    let ports: Vec<u16> = station.addr.ip_addrs().map(|s| s.port()).collect();
    assert!(!ports.is_empty());
    assert!(
        ports.iter().all(|p| *p == station.port),
        "QUIC sockets {ports:?} share the logged port {}",
        station.port
    );
    let probe = SocketAddr::from((Ipv4Addr::LOCALHOST, station.port));

    assert_eq!(http_get(probe, "/healthz").await.0, 200);
    assert_eq!(
        http_get(probe, "/readyz").await.0,
        200,
        "ready: the running line is logged after the Station is built and serving"
    );
    let (status, body) = http_get(probe, "/anything").await;
    assert_eq!(status, 404);
    assert!(body.is_empty(), "nothing is described: {body:?}");

    station.sigterm();
    assert!(station.wait_exit(Duration::from_secs(15)).await.success());
}

#[test]
fn a_busy_tcp_port_is_refused_by_name() {
    // Hold the same wildcard the Station binds, so the Station's bind is an
    // exact duplicate (refused everywhere), on a kernel-chosen port — never
    // 6782 itself, which a developer's Station may be using.
    let holder = TcpListener::bind((Ipv6Addr::UNSPECIFIED, 0))
        .or_else(|_| TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)))
        .unwrap();
    let port = holder.local_addr().unwrap().port();
    let dir = tempfile::tempdir().unwrap();

    let output = outpost()
        .args(["station", "--data"])
        .arg(dir.path())
        .args(["--port", &port.to_string()])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "a held port must not start a Station"
    );
    assert!(
        stderr.contains(&port.to_string()),
        "the error names the port {port}: {stderr}"
    );
}

#[test]
fn keygen_writes_once_and_refuses_to_overwrite() {
    use std::os::unix::fs::PermissionsExt as _;
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("fleet.key");

    let first = outpost()
        .args(["keygen", "--out"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(bytes.len(), 32);
    let mode = std::fs::metadata(&out).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "mode {mode:o}");

    let second = outpost()
        .args(["keygen", "--out"])
        .arg(&out)
        .output()
        .unwrap();
    assert!(
        !second.status.success(),
        "an existing key is never overwritten"
    );
    assert!(
        String::from_utf8_lossy(&second.stderr).contains("fleet.key"),
        "the refusal names the file"
    );
    assert_eq!(std::fs::read(&out).unwrap(), bytes, "the key is untouched");
}
