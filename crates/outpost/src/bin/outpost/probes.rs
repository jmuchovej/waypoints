//! The probe routes: `/healthz` and `/readyz` over plain HTTP, on TCP at the
//! Station's own port number. A UDP/QUIC server offers an orchestrator
//! nothing it can probe, so this listener is what a supervisor sees. It is
//! always on, and it says nothing beyond the two statuses — no addresses,
//! no Territories, no keys.

use axum::{Router, http::StatusCode, routing::get};
use std::{
    io,
    net::{Ipv4Addr, Ipv6Addr},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::net::TcpListener;

/// The flag the Station flips once its stored Territories are loaded and
/// its endpoint is accepting; `/readyz` reads it.
#[derive(Clone, Default)]
pub(crate) struct Readiness(Arc<AtomicBool>);

impl Readiness {
    pub(crate) fn mark_ready(&self) {
        self.0.store(true, Ordering::Release);
    }

    fn is_ready(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// Bind the probe listener on the wildcard address at `port` (0 lets the
/// kernel choose). `[::]` is tried first — dual-stack by default on Linux
/// and macOS, so it answers over IPv4 too — with `0.0.0.0` as the fallback
/// on a host without IPv6. The *port* never falls back: an address-in-use
/// error is returned as is.
pub(crate) async fn bind(port: u16) -> io::Result<TcpListener> {
    match TcpListener::bind((Ipv6Addr::UNSPECIFIED, port)).await {
        Ok(listener) => Ok(listener),
        Err(e) if e.kind() == io::ErrorKind::AddrInUse => Err(e),
        Err(_) => TcpListener::bind((Ipv4Addr::UNSPECIFIED, port)).await,
    }
}

/// `/healthz` answers whenever the process runs; `/readyz` answers once
/// `readiness` is marked, and `503` before that; everything else is an
/// empty `404`.
pub(crate) fn router(readiness: Readiness) -> Router {
    Router::new()
        .route("/healthz", get(|| async { StatusCode::OK }))
        .route(
            "/readyz",
            get(move || {
                let readiness = readiness.clone();
                async move {
                    if readiness.is_ready() {
                        StatusCode::OK
                    } else {
                        StatusCode::SERVICE_UNAVAILABLE
                    }
                }
            }),
        )
        .fallback(|| async { StatusCode::NOT_FOUND })
}

/// Serve the probe routes on `listener` for the life of the process.
pub(crate) async fn serve(listener: TcpListener, readiness: Readiness) {
    if let Err(e) = axum::serve(listener, router(readiness)).await {
        tracing::error!(error = %e, "probe listener stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};

    /// A bare HTTP/1.1 GET: status code and body. No client dependency —
    /// the point is to see exactly what a kubelet sees.
    async fn get(addr: SocketAddr, path: &str) -> (u16, String) {
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        stream
            .write_all(
                format!("GET {path} HTTP/1.1\r\nHost: probe\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .await
            .unwrap();
        let mut raw = String::new();
        stream.read_to_string(&mut raw).await.unwrap();
        let (head, body) = raw.split_once("\r\n\r\n").unwrap();
        let status: u16 = head.split(' ').nth(1).unwrap().parse().unwrap();
        (status, body.to_owned())
    }

    #[tokio::test]
    async fn routes_answer_health_then_readiness_and_nothing_else() {
        let readiness = Readiness::default();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(serve(listener, readiness.clone()));

        assert_eq!(get(addr, "/healthz").await.0, 200);
        assert_eq!(
            get(addr, "/readyz").await.0,
            503,
            "not ready until the Station says so"
        );
        readiness.mark_ready();
        assert_eq!(get(addr, "/readyz").await.0, 200);

        let (status, body) = get(addr, "/anything").await;
        assert_eq!(status, 404);
        assert!(
            body.is_empty(),
            "an unknown path describes nothing: {body:?}"
        );
        let (status, body) = get(addr, "/").await;
        assert_eq!((status, body.as_str()), (404, ""));
    }

    #[tokio::test]
    async fn bind_refuses_a_port_in_use_rather_than_moving() {
        // The holder sits on the same wildcard `bind` would pick, so the
        // second bind is an exact duplicate — refused on every platform.
        // (A loopback holder would not do: tokio sets `SO_REUSEADDR`, and
        // BSD then lets a wildcard bind coexist with a specific one.)
        let holder = bind(0).await.unwrap();
        let port = holder.local_addr().unwrap().port();
        let err = bind(port).await.expect_err("port is held");
        assert_eq!(err.kind(), io::ErrorKind::AddrInUse);
    }
}
