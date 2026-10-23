//! Driving the `outpost` binary the way a supervisor does: spawn a Station
//! in a temp directory on an ephemeral port, read its JSON log line by
//! line, send it a signal by pid, and wait for it to exit within a bound.
//!
//! Declared by the tests that drive the binary, not by a shared parent, so
//! a test that only builds Territories never compiles it.

#![allow(dead_code, reason = "each process test uses a different subset")]

use outpost::fabric::EndpointAddr;
use serde_json::Value;
use std::{
    io::{BufRead as _, BufReader},
    net::SocketAddr,
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

/// How long a debug-build Station may take to bind, scan, and log.
const START_BOUND: Duration = Duration::from_secs(30);

/// A `Command` for the built binary with every `OUTPOST_*` variable and
/// `RUST_LOG` scrubbed, so the test's environment cannot leak in.
pub fn outpost() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_outpost"));
    for var in [
        "RUST_LOG",
        "OUTPOST_STATION_DATA",
        "OUTPOST_STATION_KEY_FILE",
        "OUTPOST_STATION_FLEET_KEY_FILE",
        "OUTPOST_STATION_PORT",
        "OUTPOST_STATION_ADVERTISE",
        "OUTPOST_LOG_FORMAT",
    ] {
        command.env_remove(var);
    }
    command
}

/// A running Station process, observed through its JSON log.
pub struct StationProcess {
    child: Child,
    stderr: mpsc::Receiver<String>,
    /// The advertised address from the "station running" line — what an
    /// operator would paste into the app.
    pub addr: EndpointAddr,
    /// The port both sockets share, from the same line.
    pub port: u16,
}

impl StationProcess {
    /// `outpost --log-format json station --data <data> --port 0 <extra…>`,
    /// returned once the "station running" line has been read.
    pub fn spawn(data: &Path, extra: &[&str]) -> Self {
        let mut child = outpost()
            .args(["--log-format", "json", "station", "--data"])
            .arg(data)
            .args(["--port", "0"])
            .args(extra)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn outpost");

        let stdout = child.stdout.take().expect("piped stdout");
        let (lines_tx, lines) = mpsc::channel::<Value>();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if let Ok(value) = serde_json::from_str::<Value>(&line)
                    && lines_tx.send(value).is_err()
                {
                    break;
                }
            }
        });
        let stderr_pipe = child.stderr.take().expect("piped stderr");
        let (stderr_tx, stderr) = mpsc::channel::<String>();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr_pipe).lines() {
                let Ok(line) = line else { break };
                if stderr_tx.send(line).is_err() {
                    break;
                }
            }
        });

        let deadline = Instant::now() + START_BOUND;
        let running = loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match lines.recv_timeout(remaining) {
                Ok(value) if value["fields"]["message"] == "station running" => break value,
                Ok(_) => continue,
                Err(_) => {
                    let _ = child.kill();
                    let stderr: Vec<String> = stderr.try_iter().collect();
                    panic!(
                        "no \"station running\" line within {START_BOUND:?}; stderr: {stderr:#?}"
                    );
                }
            }
        };
        let addr_json = running["fields"]["address"]
            .as_str()
            .expect("address field is a JSON string");
        let addr: EndpointAddr = serde_json::from_str(addr_json).expect("address parses");
        let port = running["fields"]["port"].as_u64().expect("port field") as u16;
        Self {
            child,
            stderr,
            addr,
            port,
        }
    }

    pub fn sigterm(&self) {
        let pid = self.child.id() as libc::pid_t;
        // SAFETY: a plain signal to a child we spawned and still own.
        let rc = unsafe { libc::kill(pid, libc::SIGTERM) };
        assert_eq!(rc, 0, "kill(SIGTERM) failed");
    }

    /// Wait for the process to exit, panicking (and killing it) if it is
    /// still running after `bound`.
    pub async fn wait_exit(&mut self, bound: Duration) -> ExitStatus {
        let deadline = Instant::now() + bound;
        loop {
            if let Some(status) = self.child.try_wait().expect("try_wait") {
                return status;
            }
            if Instant::now() >= deadline {
                let _ = self.child.kill();
                let stderr: Vec<String> = self.stderr.try_iter().collect();
                panic!("station still running after {bound:?}; stderr: {stderr:#?}");
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}

impl Drop for StationProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

/// A bare HTTP/1.1 GET — status code and body — with no client library, so
/// the test sees exactly what a kubelet probe sees.
pub async fn http_get(addr: SocketAddr, path: &str) -> (u16, String) {
    try_http_get(addr, path)
        .await
        .expect("the probe listener answers")
}

/// [`http_get`] that reports a refused connection, or no answer within a
/// second, as `None` — what a probe sees from a process that is not
/// serving yet.
pub async fn try_http_get(addr: SocketAddr, path: &str) -> Option<(u16, String)> {
    tokio::time::timeout(Duration::from_secs(1), async {
        let mut stream = tokio::net::TcpStream::connect(addr).await.ok()?;
        stream
            .write_all(
                format!("GET {path} HTTP/1.1\r\nHost: probe\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .await
            .ok()?;
        let mut raw = String::new();
        stream.read_to_string(&mut raw).await.ok()?;
        let (head, body) = raw.split_once("\r\n\r\n")?;
        let status: u16 = head.split(' ').nth(1)?.parse().ok()?;
        Some((status, body.to_owned()))
    })
    .await
    .ok()
    .flatten()
}
