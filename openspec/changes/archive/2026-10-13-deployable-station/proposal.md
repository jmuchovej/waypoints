## Why

`outpost station` is a development server. It keeps a plaintext identity key inside its data directory, mints a fresh fleet key on every `--orderer` run (so ordered ACL records from one run do not verify against the next), binds whatever UDP port iroh picks, stops only on Ctrl-C, and offers nothing a supervisor can probe. None of that survives contact with a container orchestrator, and nothing in the tree builds a Station into an image or says how to run one.

The hosted tier is a Fleet of Stations on Kubernetes, and self-hosters are first-class — every tier is self-hostable, so the image and the manifests belong in the public repository. The smallest useful slice is a **one-Station Fleet**: one process holding the storage and orderer roles, deployable on Kubernetes from a Nix-built image, surviving pod deletion with its data and identity intact. That slice is worth shipping on its own, and it is the floor every later scaling step (replication, identity pools, relay reachability) stands on.

## What Changes

- **The Station process behaves under supervision.** `outpost station` handles SIGTERM as well as SIGINT and closes its iroh endpoint before exiting, so in-flight sessions end cleanly instead of timing out on the Vessel. It binds a fixed UDP port, 6782 by default (`--port`; OSTA spelled on a T9 keypad), so a Service can route to it. It always serves `/healthz` and `/readyz` over plain HTTP on TCP at that same port number (TCP and UDP port spaces are independent), where ready means the storage scan has finished and the endpoint is accepting — the only way a UDP/QUIC server can answer an orchestrator's probes, and one number for the operator to know. Logs can be emitted as JSON (`--log-format json`). Every flag reads from an `OUTPOST_*` environment variable as well.
- **Keys are provisioned, not improvised.** `--key-file` points the Station at its identity key; an explicitly given file must exist (a mounted Secret is read-only, and silently generating a new identity there would be a bug), while the default path inside the data directory keeps today's load-or-generate convenience for development. `--fleet-key-file` replaces `--orderer`: the orderer role is on exactly when a fleet key is supplied, and that key persists across restarts. **BREAKING** for the CLI: `--orderer` is removed. `outpost keygen` writes a fresh Ed25519 seed to a path so an operator can create Secrets before first boot.
- **The Station is a Nix package and a container image.** `nix/packages/outpost/package.nix` builds the `outpost` binary (`cli` feature) through the overlay, following the `nix-flake-overlay` layout. The image is a `dockerTools` layered image exposed as a Linux-only `perSystem` package, not through the overlay, so the overlay stays self-contained. No Dockerfile; non-root user; `/data` is the volume.
- **Reference Kubernetes manifests** under `deploy/kubernetes/`: a kustomize base for a one-Station Fleet — StatefulSet with a block-storage PVC at `/data`, the identity and fleet keys from Secrets, a UDP Service for the QUIC port, probes against the TCP side of the same port, a generous termination grace period, non-root security context. The base validates with `kubeconform` and is exercised on `kind`. The operator still supplies `--advertise` once the Service has an address; that is the no-relay reachability path and it is documented as such.
- **The dev shell gains `kubeconform`, `kubectl`, and `kind`** in a `deploy` mise lane so the manifests can be checked and the kind run reproduced.

## Capabilities

This change lands in two domains. **`sync-protocol`** already carries the Station's operational contract (an advertised, routable address; accept-only; stable firewall identity for development builds), and the process-level requirements here are more of the same kind. **`distribution`** covers packaging and installation outside the Tauri bundler; the overlay gains a server package, and the image-plus-manifests contract is a new area under it.

### New Capabilities

- `distribution/station-deployment`: the deployable Station — what the container image contains and guarantees (entrypoint, non-root, data path, no network egress requirements beyond the QUIC port), and what the reference manifests must hold for a one-Station Fleet (persistence across pod deletion, keys from Secrets, probes against the Station's TCP routes, graceful termination). The manifests are reference material for self-hosters and the hosted tier alike; they are validated, not merely illustrative.

### Modified Capabilities

- `sync-protocol`: the requirement _A Station is reachable at an advertised address_ gains a fixed-port bind. New requirements: a Station's identity key is provisioned from a file and an explicitly named file is never silently created; the orderer's fleet key is supplied, not generated, so ordering authority survives restarts; a Station stops cleanly on a termination signal, closing its endpoint first; a Station always reports liveness and readiness over TCP at its own port number, with readiness gated on the storage scan.
- `distribution/nix-overlay`: _Flake overlay exposes desktop packages_ widens to include the `outpost` server package. Co-installability, wrapper layout, and the update requirements are unchanged and do not apply to a server binary.

## Non-goals

- **Scaling beyond one Station.** Station↔Station replication, the roster-peer `SyncPolicy` arm, pre-provisioned identity pools, Vessel-side multi-Station selection, and roster composition/signing tooling. The Station never consults the roster today — only a Vessel's `sync_with_fleet`/`register_with_fleet` do, and the app does not use those paths — so a roster CLI would produce a file nothing reads. These are the next change, and its first decision (whether Stations pull from roster peers) is still the owner's.
- **A separate orderer process.** The app configures exactly one Station address and reaches the orderer through it, so a one-Station Fleet runs both roles in one process. Splitting the orderer into its own singleton arrives with multi-Station Fleets.
- **Relay-assisted reachability.** iroh 1.2 ships the pieces (`RelayMode::Custom`, `TransportAddr::Relay`, the `iroh-relay` crate), and running our own relay is the leaning for the hosted tier, but it needs a spike. This change keeps the `Minimal` preset and per-pod public addressing.
- **Helm, an operator, CRDs, HPA, PodDisruptionBudgets, NetworkPolicy, or multi-zone spread.** A one-replica kustomize base needs none of them.
- **Admission quotas or metrics.** `RegisterPolicy::Owner` stays as is; a public hosted Fleet needs an admission arm tied to the revenue model, which is not this change. `/metrics` without any metric to export is not worth a route.
- **Backup or export tooling for Territory holds.** Note for the record: crash recovery against the storage engine _is_ verified (`storage` spec, _Committed writes survive abrupt process termination_); the architecture reference's "unverified" line predates `durability-crash-recovery` and is stale.
- **DNS names in `--advertise`.** iroh direct addresses are IP sockets; a hostname path is the relay's job.
- **Publishing the image from CI**, a NixOS module for the Station, or any change to the Tauri app.

## Impact

- `crates/outpost/src/bin/outpost/commands/station.rs` — signal handling, `--port`, `--key-file`, `--fleet-key-file` (replacing `--orderer`), `--log-format`, the always-on probe listener, env fallbacks, the published address logged at startup.
- `crates/outpost/src/bin/outpost/{main.rs,commands/keygen.rs}` — the `keygen` subcommand; `main.rs`'s module docs lose the "fresh dev fleet key" limit.
- `crates/outpost-fabric/src/endpoint.rs` — a routable bind at an operator-chosen port, beside the existing port-0 `bind_routable`.
- `crates/outpost/src/station.rs` — `hosting_orderer` gains a routable form (today it binds loopback only, which is why `--orderer` was dev-only). No other library change; the accept-only pin in `tests/transport.rs` is untouched.
- `crates/outpost/Cargo.toml`, `Cargo.toml` — `clap` gains `env`; `tracing-subscriber` gains `json` under the `cli` feature; `axum` (already a workspace dependency, used by `xtask`) joins the `cli` feature for the probe listener; `libc` as a dev-dependency for the SIGTERM test.
- `crates/outpost/tests/` — a process-level test that spawns the binary, sends SIGTERM, and asserts a clean exit; probe-listener tests; a key-file-must-exist test.
- `nix/packages/outpost/package.nix` (today an empty `{ }` placeholder, uncommitted), `nix/packages/outpost/_image.nix`, `nix/overlay.nix`, `flake.nix` — the package, the image, and their wiring. Until `in-tree-workflow-generator` lands, `cargoLock.outputHashes` must carry the `gh-workflow` git pin exactly as the app package does. Builds from the tree (`root`), not from the release index that `nix-release-index` introduces — `outpost` has no release channel yet.
- `deploy/kubernetes/base/` — kustomization, StatefulSet, Service, example Secret manifests, and a README-level comment block in the kustomization explaining the `--advertise` step.
- `devenv.nix` — `kubeconform`, `kubectl`, `kind` in a new `deploy` mise lane.
- `.agents/skills/waypoints-architecture/references/architecture.md` — correct the stale crash-recovery line while touching the deployment topic (one sentence; the skill is a living document and the change invalidates it).
