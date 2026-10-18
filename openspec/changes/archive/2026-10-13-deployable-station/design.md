## Context

See proposal.md — Why. What the tree holds on 2026-10-10, read from source:

- **The CLI.** `crates/outpost/src/bin/outpost/commands/station.rs`: `--data` (default `.outpost-station`), `--orderer`, repeatable `--advertise <SocketAddr>`. `station_key` is load-or-generate of 32 raw bytes at `<data>/station.key`. `--orderer` calls `Station::hosting_orderer` with `SigningKey::generate` — a fresh fleet key per run — and that constructor passes `routable = false`, so an orderer today binds loopback only. The only shutdown path is `tokio::signal::ctrl_c()`. The address is written to `<data>/station.addr` and only its _path_ is printed. `main.rs`'s module docs still describe a `station-run` subcommand and a one-Territory limit, both stale.
- **The endpoint.** `crates/outpost-fabric/src/endpoint.rs`: `bind` is the `Minimal` preset, loopback-only, portmapper off, built with `clear_ip_transports` + two `bind_addr` calls (so the dual-family pattern exists). `bind_routable` is `Minimal` with no `bind_addr` at all — iroh picks the port. `advertised_addr` emits only `TransportAddr::Ip`. `Endpoint::close` exists and awaits `iroh::Endpoint::close`, whose docs say to await it so peers see a close rather than a timeout.
- **The Station.** `crates/outpost/src/station.rs`: `hosting`/`hosting_routable`/`hosting_orderer` all run `scan_holds` before returning, so "constructed" already means "stored Territories loaded". `serve_loop` returns once `accept` yields `None`, which it does after `endpoint.close()`. `endpoint()` is public. `SyncPolicy` has `Members` and `Everyone` only; the roster-peer arm is a doc comment. The accept-only pin (`tests/transport.rs::station_source_opens_no_outbound_connections`) counts `.connect(` in `station.rs`.
- **Who reads the roster.** Only `Vessel::sync_with_fleet`/`register_with_fleet` (`vessel.rs`) check a `FleetConfig`; `cartograph` never constructs one and holds a single `station: Option<EndpointAddr>`. `Replica::bootstrap_with_fleet` verifies order proofs against a fleet key — which is why a per-run fleet key is a correctness bug for any persistent deployment, not just an inconvenience.
- **Dependencies.** iroh `1.2.0` (`presets::{Empty, Minimal, N0, N0DisableRelay}`, `RelayMode::Custom`, `TransportAddr::Relay`; `iroh-relay` 1.2.0 is already in `Cargo.lock` transitively). `tokio/signal` is already in the `cli` feature. `clap` is `derive`-only at the workspace; `tracing-subscriber` is `env-filter`-only. `axum 0.8.9` is a workspace dependency used by `tooling/xtask`. No `libc`/`nix` crate anywhere. The only git dependency in the lockfile is `gh-workflow` (the `nix-flake-overlay` package carries its `outputHashes`); `turso`/`turso_core` are registry pins.
- **Nix.** `flake.nix` is flake-parts, packaging-only, three systems; `nix/overlay.nix` is pure and takes `root` (a path) and `rustToolchain`. `nix/packages/waypoints/package.nix` is the model: `lib.fileset` over `Cargo.toml`, `Cargo.lock`, `crates/`, `tooling/`, `vendor/`, and `src-tauri/{…}` (every workspace member's manifest is needed for cargo to read the workspace), `cargoLock.outputHashes` for `gh-workflow`, `doCheck = false`. `nix/packages/outpost/package.nix` exists in the working copy as an empty `{ }`, uncommitted. No CI builds flake packages today.
- **Storage.** Crash recovery _is_ verified: `storage` spec, _Committed writes survive abrupt process termination_, from `durability-crash-recovery` (archived 2026-09-29), with a SIGKILLed victim binary (`crates/bedrock/tests/crash_recovery.rs` spawns `CARGO_BIN_EXE_crash-victim` and `Child::kill`s it — the pattern this change's SIGTERM test reuses). `architecture.md` §11 item 8 still says "unverified"; it is stale. _Closing a territory releases connections without checkpointing_ means dropping holds on shutdown is correct and the WAL replays on reopen.
- **Probes.** A kubelet HTTP probe is sent to the pod IP from the node, not from inside the pod. A loopback listener would never be reachable; the listener must bind a pod-reachable address.
- **Dev shell.** No `kubeconform`, `kubectl`, or `kind`. mise's registry resolves all three through aqua (`yannh/kubeconform`, `kubernetes-sigs/kind`, `kubernetes/kubernetes/kubectl`). `waypoints.mise.environments` is the list a new lane joins.

## Goals / Non-Goals

**Goals:**

- A Station that an orchestrator can start, probe, stop, and restart without losing identity, ordering authority, or data.
- One image, one kustomize base, both validated — the base on `kind` by hand, the rendering by `kubeconform` in a `just` recipe.
- Zero change to the sync protocol, the Station's admission behaviour, or the accept-only invariant.

**Non-Goals:**

- Beyond the proposal's: no change to how the app configures its Station (it keeps pasting the published address); no `Station` API beyond one additional constructor and one additional bind; no attempt to make the manifests portable across every cloud's load balancer.

## Decisions

### D1 — Shutdown closes the endpoint and awaits it, bounded

**Chosen** (2026-10-10). `station::run` selects over SIGTERM (`tokio::signal::unix::signal(SignalKind::terminate())`, `#[cfg(unix)]`) and SIGINT (`ctrl_c`), then `station.endpoint().close().await` under a `tokio::time::timeout` of 10 s, then returns `Ok(())`. `serve_loop` ends on its own once `accept` yields `None`; the spawned session tasks end when their streams error on the closed endpoint; holds are dropped. A second signal during the close exits immediately.

**Why.** iroh's own documentation: not awaiting `close` makes the remote end time out instead of observing the close — exactly the Vessel-side behaviour the sync-protocol spec's liveness requirements would then misread as a dead session. Ten seconds is comfortably inside a 60 s grace period and far shorter than the idle timeout. Holds need no flush: the storage spec says close never checkpoints and the WAL replays.

**Rejected.** _Exit on signal with no close_ — the status quo for SIGINT, and the timeout problem above. _Drain in-flight syncs before closing_ — a reconcile can be long; a QUIC close mid-reconcile is already a case the Vessel handles ("reconnect is not a special case"), so draining buys nothing the next sync does not.

### D2 — A fixed port binds both wildcard families; `--port`, not `--bind`

`Endpoint::bind_routable_at(secret, port)` builds `Minimal` with `clear_ip_transports().bind_addr(("0.0.0.0", port))?.bind_addr(("[::]", port))?`, mirroring the loopback `bind`. The CLI exposes it as `--port <u16>` (`OUTPOST_PORT`), default 6782 (D12). `--port 0` means the kernel picks the number — through the TCP bind that comes first (D5) — and iroh then binds that same number, so the two sockets share a port in every mode. The existing port-0 `bind_routable` stays for library callers and tests; the CLI no longer uses it. A bind failure on a configured port propagates: the Station does not start.

**Why `--port` rather than `--bind <SocketAddr>`.** A pod has one interface and needs one port on it; letting an operator name an interface invites the question of what a v4 wildcard means for v6 and back. One number, both families, no policy.

**Risk to settle in the task.** Whether iroh's v6 wildcard socket is `IPV6_V6ONLY` on Linux. If it is not, binding `[::]:port` after `0.0.0.0:port` fails with address-in-use; the fallback is binding `[::]` alone (a dual-stack socket serves both) and the task records which it was.

### D3 — Key files are raw seeds; an explicit path must exist; `outpost keygen`

`--key-file <path>` (`OUTPOST_KEY_FILE`) and `--fleet-key-file <path>` (`OUTPOST_FLEET_KEY_FILE`) read today's format: 32 raw bytes, the Ed25519 seed. An explicitly given path that is absent or not 32 bytes is an error naming the path. `--key-file` unset keeps load-or-generate at `<data>/station.key`. `--fleet-key-file` set turns the orderer on; `--orderer` is deleted. `outpost keygen --out <path>` writes a fresh seed with mode `0600` and refuses to overwrite. One subcommand for both key kinds: the bytes are identical, and the role is decided by which flag the file is handed to.

**Why not generate at a Secret mount.** The mount is read-only; the only possible outcomes are a write error or, worse, a successful write to a path that is not the Secret. Refusing is the honest behaviour and matches what "I provisioned this key" means.

**Why not the environment for the key bytes.** `kubectl describe pod` and `/proc/<pid>/environ` print environment variables; file mounts do not.

**Why raw bytes, not a wrapped format.** It is what every existing key file is and what `SigningKey::from_bytes` takes. A format with a header is a migration for no present benefit.

### D4 — The orderer gets a routable constructor; nothing else in the library moves

`Station::hosting_orderer_routable(secret, fleet_key, storage)` is `build(…, true)` beside the three existing `hosting*` constructors. The CLI always binds routable (it did already for the storage-only path). `scan_holds` is unchanged; readiness (D5) hangs on the constructor returning.

**Rejected.** _A builder replacing the four constructors_ — a refactor of a public API this change does not need, touching every test that constructs a Station.

### D5 — Probe listener: always on, TCP at the QUIC port number, started before the scan

Bind order is TCP first. The binary binds a `TcpListener` on the wildcard address at the value of `--port` and reads back the local port `P` — the configured number, or the kernel's pick when `--port 0` — then constructs the Station on `P` (`Station::hosting_*` at `P`, D4) and serves axum on the listener with `/healthz` → `200` and `/readyz` → `503` until an `Arc<AtomicBool>` is flipped after the constructor returns and `serve_loop` is spawned; then `200`. Everything else is `404` with an empty body. There is no flag: a Station always has probe routes, and they are always at the QUIC port's number, ephemeral or not. The TCP listener is up before the storage scan in every mode, so a long scan shows as `503`, never as a refused connection. If the UDP bind at `P` fails: with a configured port the Station exits naming the port, exactly as for TCP; with port 0 the TCP listener is dropped and the pair is retried on a fresh kernel pick, a bounded number of times (eight), before giving up — the operator asked for any port, so a different one betrays nothing. The TCP socket binds the same wildcard families D2 settles on for UDP (task 1.2 audits both). `axum` joins the `cli` feature from the workspace table; it does not enter `station`/`vessel`.

**Why TCP first.** The TCP bind is the only one of the two whose port can be read back before the Station's constructor runs its storage scan; binding UDP first would put the probe listener up only after the scan, and ephemeral mode would behave differently from a fixed port. One order, one number, one code path.

**Amendment (2026-10-13, found by the audit).** Bound is not served. The first implementation bound the listener before the scan but spawned the server on it only after the Station was built, so during a long scan a probe's connection sat in the backlog and timed out — never a 503. It surfaced only once the kind pod homed 300 Territories (`tests/audit.rs::register_many`) and the replacement was sampled through its five-second scan; a startup delay would not have shown it, because a sleep behaves the same whichever side of the `spawn` it sits on. The server is now spawned on the listener before the Station is constructed, and aborted (dropping the listener) on the ephemeral-port retry. `tests/readiness.rs` homes 300 Territories through an in-process Station and asserts the 503 phase against the real binary. Measured: about 16 ms per hold on the kind pod (arm64 release build) and 8 ms on the Mac debug build, two file descriptors per hold.

**Why one port number.** TCP and UDP port spaces are independent, so 6782/udp for QUIC and 6782/tcp for probes coexist. An operator reads one number off a Service, a firewall rule, or an `--advertise` value, and the probe is at that number too. The manifests declare both protocols on `containerPort: 6782`, the Service stays UDP-only, and any NetworkPolicy must allow both.

**Why always on, unscoped.** Probes are how a supervisor tells a live Station from a stuck one; a Station without them is not a deployable one, and nothing is gained by letting a dev run omit them. Scoping the bind address or gating the routes is access control — defence in depth behind the encrypted envelopes that are the actual protection — and it is deferred until there is a threat to answer. The routes leak nothing (see the spec's _The listener reveals nothing_), so an open listener is a status page, not a surface.

**Why axum.** Already a workspace dependency, already compiled for `xtask`; HTTP semantics done correctly by someone else. The binary is a server; closure size is not a concern the app's is.

**Why not loopback.** Kubelet probes the pod IP. A loopback listener would need an exec probe, which needs a client in the image that talks to the running process — over a socket, which is the thing being avoided.

**Rejected.** _A separate probe port_ — a second number to read, write, and allow, for no isolation TCP-versus-UDP does not already give. _Routes on the QUIC socket itself_ — that is HTTP/3, and kubelet and load-balancer health checkers speak HTTP/1.1 over TCP; the prober rules it out, not us. _An exec probe that dials QUIC_ — the most honest readiness check, but it forks a process per probe, needs a subcommand because the image has no shell, and leaves a health-checking load balancer without a TCP target. _A hand-rolled responder on `TcpListener`_ — forty lines of HTTP nobody wants to review. _`/metrics`_ — nothing to export yet; a route with no content is a promise, not a feature.

### D6 — Environment fallbacks through clap's `env` feature

`clap` gains `env` at the workspace table (additive; `xtask` and `src-tauri` do not use it). Every `StationArgs` field gets `env = "OUTPOST_…"`; `--advertise` additionally takes `value_delimiter = ','` so one variable can carry several sockets. Flags win over the environment, which is clap's default.

### D7 — Structured logs through `tracing-subscriber`'s `json` feature, selected by flag

`--log-format {text,json}` (`OUTPOST_LOG_FORMAT`, default `text`). The `json` feature is enabled on `outpost`'s own `tracing-subscriber` entry under `cli`, not at the workspace table, so the app's build is untouched. `RUST_LOG` keeps driving the filter in both formats. The startup lines that `println!` today (address, data dir, orderer on/off) become `tracing::info!` events so they are structured too; the full advertised address JSON is logged, which is the operator's copy for the app.

### D8 — `outpost` package: `buildRustPackage`, `-p outpost --features cli`, tree sources

`nix/packages/outpost/package.nix` takes `root`, `rustToolchain`, `lib`, `rustPlatform`, `makeRustPlatform`. `src` is the fileset `Cargo.toml`, `Cargo.lock`, `crates/`, `tooling/`, `vendor/`, `src-tauri/{Cargo.toml,build.rs,src}` (workspace parsing needs every member; the desktop crate's heavy inputs are never built because only `outpost` is). `cargoBuildFlags = [ "-p" "outpost" ]`, `buildFeatures = [ "cli" ]`, `cargoLock.outputHashes` copied from the app package until `in-tree-workflow-generator` removes the git dependency, `doCheck = false` (consistent with the app package; the test suite runs under cargo in CI), `version` from `crates/outpost/Cargo.toml` via `fromTOML` (TOML 1.0-parseable; it already is), `meta.mainProgram = "outpost"`. `nix/overlay.nix` adds `outpost = final.callPackage ./packages/outpost/package.nix { inherit root rustToolchain; }`; `flake.nix` exposes `packages.outpost = overlaid.outpost` (no `stamp`: the binary has no build-metadata stamp to feed).

**Interaction with `nix-release-index`.** That change moves the desktop packages to `fetchFromGitHub` by index entry. `outpost` has no release channel, so it keeps building from `root`; when it gains one, it joins the index in its own change.

### D9 — Image: `dockerTools.buildLayeredImage`, Linux-only `perSystem`, not in the overlay

`nix/packages/outpost/_image.nix` (underscore: an intermediate the flake exposes directly, never a package in the overlay) calls `dockerTools.buildLayeredImage { name = "outpost"; tag = outpost.version; contents = [ outpost ]; config = { Entrypoint = [ "outpost" "station" ]; Env = [ "OUTPOST_DATA=/data" ]; User = "65534:65534"; WorkingDir = "/data"; Volumes."/data" = {}; ExposedPorts = { "6782/udp" = {}; "6782/tcp" = {}; }; }; }`. `flake.nix` adds `outpost-image` to `perSystem.packages` under `lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux`. The image carries no shell, no CA bundle, no `/etc/passwd` entry (the binary resolves no user names). `/data` is created by the volume mount; `fsGroup` in the manifests makes it writable.

**Why `buildLayeredImage` over `streamLayeredImage`.** `kind load image-archive ./result` wants an archive on disk, which is the one consumer this change has. **Why not nix2container.** A flake input for incremental-layer benefits an 8 MB binary does not need.

**Practical note.** The image is a Linux derivation; on the maintainer's darwin hosts it builds through a Linux remote builder (`nix.linux-builder` on nix-darwin) or on a Linux machine. The tasks say which was used.

**Amendment (2026-10-10, at implementation).** `Entrypoint = [ "outpost" ]` with `Cmd = [ "station" ]`, not a single two-word `Entrypoint`: Kubernetes `args` replaces `Cmd` and appends to `Entrypoint`, so the manifests' `args: [station, --port=6782, …]` would otherwise run `outpost station station …`. `WorkingDir` is dropped — `OUTPOST_DATA=/data` is absolute, and a working directory that does not exist until a volume is mounted is a runtime-dependent edge the image need not carry.

### D10 — Manifests: a kustomize base plus a `kind` overlay

```
deploy/
  justfile                      # lint, kind-up, kind-load, kind-apply
  kubernetes/
    base/
      kustomization.yaml        # the --advertise two-step is explained here
      statefulset.yaml
      service.yaml              # type: LoadBalancer, UDP 6782
      configmap.yaml            # OUTPOST_ADVERTISE, empty by default
      secrets.example.yaml      # shape only; never applied as-is
    kind/
      kustomization.yaml        # base + NodePort patch
      kind-config.yaml          # extraPortMappings: UDP 30782 → host 6782
```

StatefulSet: `replicas: 1`, `volumeClaimTemplates` RWO at `/data`, two Secret volumes at `/keys/station.key` and `/keys/fleet.key` (`defaultMode: 0400`), container args `station --port 6782 --key-file /keys/station.key --fleet-key-file /keys/fleet.key --log-format json`, `containerPort: 6782` declared twice (`protocol: UDP` for QUIC, `protocol: TCP` for probes), `OUTPOST_ADVERTISE` from the ConfigMap (empty means "no `--advertise`", which the CLI must treat as absent), startup probe `httpGet /readyz` on port 6782 (period 5 s, failure threshold 60), readiness `/readyz`, liveness `/healthz`, `terminationGracePeriodSeconds: 60`, `securityContext` `runAsNonRoot`, `runAsUser/runAsGroup/fsGroup: 65534`, `readOnlyRootFilesystem: true`, `allowPrivilegeEscalation: false`, `capabilities.drop: [ALL]`, `seccompProfile: RuntimeDefault`.

**Why kustomize, not Helm.** One replica, three resources, no conditionals. Helm arrives with the scaling change where replica count, identity pools, and the orderer split make values real.

**Why a `kind` overlay.** `kubectl port-forward` does not forward UDP, so the Vessel-to-pod check needs a real UDP path to the host: a NodePort plus kind's `extraPortMappings`. The overlay is the reproducible form of that check, not a deployment target.

**Why `type: LoadBalancer` in the base.** It is the one Service type that yields a stable external UDP socket on MetalLB and most clouds; the operator reads it and sets `OUTPOST_ADVERTISE`. `hostPort` on a pinned node is the air-gapped alternative and is a one-line patch away; it is not the base.

### D11 — Tooling: a `deploy` mise lane and a `deploy` justfile module

`devenv.nix` adds `kubeconform`, `kubectl`, and `kind` to `waypoints.mise.tools` with `envs = [ "deploy" ]` and `"deploy"` to `waypoints.mise.environments`; `check-generated` will show the projected mise file change. `justfile` gains `mod deploy "deploy/justfile"` with `lint` (`kubectl kustomize deploy/kubernetes/base | kubeconform -strict -summary`), `kind-up`, `kind-load`, and `kind-apply`. `lint` is the recipe the spec's validation requirement names.

### D12 — Ports spell their owner on a T9 keypad

| Port  | Owner                                   | Spelling           | Bound by this change                          |
| ----- | --------------------------------------- | ------------------ | --------------------------------------------- |
| 6782  | Station: QUIC over UDP, probes over TCP | OSTA (6·7·8·2)     | Yes — the `--port` default, both protocols    |
| 6875  | Vessel                                  | OVSL (6·8·7·5)     | No — reserved; Vessels keep an ephemeral bind |
| 30782 | kind NodePort → 6782                    | arbitrary in range | kind overlay only                             |

**Why a convention at all.** A Service, a firewall rule, a `--advertise` value, and a README all need the same number; a memorable one that encodes which process it belongs to stops the number from being looked up each time. T9 is the keypad everyone can reconstruct (2 ABC · 3 DEF · 4 GHI · 5 JKL · 6 MNO · 7 PQRS · 8 TUV · 9 WXYZ).

**Why the Vessel's port is reserved but not bound.** Two flavors of the app are co-installable and may run at once (`distribution/nix-overlay`, _Nightly and stable coexist_); a fixed Vessel port would make the second one fail to bind, and a fixed port buys a Vessel nothing today — it dials out and is never dialed. When a fixed Vessel port is wanted (a corporate egress rule, say), the shape is "prefer 6875, fall back to ephemeral", which is a Vessel change with its own spec scenario and is not this one.

**Why the probes have no number of their own.** They ride the Station's number over TCP (D5). One port per process is the whole point of spelling it.

## Risks / Trade-offs

- [iroh's v6 wildcard socket may not be `V6ONLY`, making the dual bind fail with address-in-use on Linux] → D2's fallback; the task tests on Linux (in the kind pod, not only on darwin) and records the outcome in `endpoint.rs`'s doc comment.
- [`kind load image-archive` may reject dockerTools' gzipped archive] → `docker load < result` then `kind load docker-image`; the `kind-load` recipe does whichever works and says so.
- [A cloud load balancer may not offer UDP, or may rewrite source addresses] → Out of scope for identity (QUIC authenticates the Station, not the path). The base is written for MetalLB-class behaviour; the kustomization comment says so.
- [The `--advertise` two-step is a manual edit after the Service gets an address] → Accepted for one Station; MetalLB's address-pool annotation makes it a static choice. Relay-assisted addressing removes the step and is the follow-up change's subject.
- [Another process holds TCP 6782 while UDP 6782 is free, or the reverse] → The Station refuses to start and names the port, the same rule as for the QUIC socket; no half-bound Station.
- [`axum` in the `cli` feature pulls `hyper`/`tower` into the server binary] → Server binary, already-compiled workspace dependency; no effect on the app or the role features.
- [`readOnlyRootFilesystem` with a binary that wants to write somewhere unexpected] → The only writes are under `--data`; the kind run with the manifests' security context is the test.
- [`clap`'s `env` feature at the workspace table changes nothing for other binaries but is a shared-table edit] → Additive feature; `cargo metadata` diff is the check.
- [`scan_holds` opening every hold eagerly may exceed the pod's file-descriptor limit on a large Fleet] → Not this change's problem at one Station with a developer's Territories; the startup probe's budget is the knob until a lazy scan exists. Measured on kind: 300 holds take about five seconds and 600 descriptors, so the five-minute startup budget covers on the order of fifteen thousand holds and the descriptor limit (2³¹ on containerd) is not the binding constraint.
- [Removing `--orderer` breaks anyone's dev invocation] → Nothing in the tree invokes it (checked: no justfile, task, or doc runs `outpost station --orderer`); `main.rs`'s docs are rewritten with it.

## Migration Plan

1. Land the library bits (D2, D4) with tests; no behaviour changes for existing callers.
2. Land the CLI (D1, D3, D5, D6, D7) with the process-level tests. `--orderer` disappears here.
3. Land the package and overlay entry (D8); `nix build .#outpost` on darwin and Linux.
4. Land the image (D9) and manifests (D10, D11); run the kind check and record it in the tasks.
5. Rollback is per step: each is a self-contained commit, and none changes the wire protocol or on-disk formats.

## Open Questions

- Image tag policy once the image is published somewhere (crate version, git rev, both). Does not affect the manifests, which reference an image name the overlay user sets.
- Whether `terminationGracePeriodSeconds` should exceed 60 s for Fleets with many parked subscriptions. The close bound is 10 s regardless; 60 s is slack.
