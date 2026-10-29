## Why

The channels are meant to be alpha, beta, and production — nightly is where questionable code is allowed to go — and that only means something if each channel is its own build, compiled and tested as itself. Today a flavor is identity only: the release workflows already build each channel under `--features <flavor>`, but no Rust code gates on a shipped flavor, nothing stops a build from enabling two flavors or the wrong one for the tree it builds, and the suite that gates a release runs with no flavor at all, so the combination that ships is never the one that was tested. The library crates cannot gate anything to nightly because most declare no flavor: only `store` and `src-tauri` declare all four, and `src-tauri` forwards to `store` alone.

Why now: `library-minting` will measure every library's API surface under the stable flavor (`cargo semver-checks --features stable`), which needs `stable` to be a real feature on every library crate, and a series that pins a library tag compiles it under the series' flavor, which needs the flavor to reach it. That dependency runs toward this change; this change depends on nothing.

## What Changes

- **Flavors gate behaviour in a staircase.** Nightly compiles preview-gated code, preview compiles stable's ungated baseline, and stable _is_ the baseline but is still demanded, since it is how a build selects its channel. A feature graduates by loosening its gate until it has none.
- **Every group crate declares the four flavors** — `nightly`, `preview`, `stable`, `development` — and the flavor enabled on `src-tauri` reaches every group crate through the dependency graph, so a library can gate questionable work to nightly. Today `domain` lacks `nightly`, `cartograph` has `development` only, and `bedrock*`, `outpost*`, `transload` have none.
- **Exactly one shipped flavor per build.** Two shipped flavors is a `compile_error!` in every crate that declares them, since Cargo features are additive and flavors are not. `src-tauri/build.rs` refuses a release build whose enabled flavor disagrees with the channel the manifest version names, or that enables no shipped flavor; `development` beside one shipped flavor relaxes only the agreement check, so another channel can be compiled on a workstation.
- **The suites run under the flavor they gate.** The release test suite runs under the channel's flavor, and the pull-request suite under the flavor of the branch's channel, so the compiled combination that ships is the one that was tested. A promotion rebuilds under `stable` rather than relabelling the preview build — the release workflows already do this, and a generator test now holds them to it.
- **`development` is not a channel.** It adds (`store/mock`, dev tooling), never overrides a channel gate, never reaches a released artifact, and may sit beside one shipped flavor.
- Promotion stays a human decision after nightly soak; nothing here automates it.

## Capabilities

This change stays inside the `distribution` domain, which `nix-flake-overlay` introduced and `nix-release-index` extended with `release-planning` and `release-index`. Which flavor a channel's release is built and tested under is release planning: it constrains what a release claims to be.

### New Capabilities

None.

### Modified Capabilities

- `distribution/release-planning`: gains two `ADDED` requirements — _A channel's release is built and tested under its own flavor_ (a fresh build under the channel's flavor, a promotion rebuilt under stable, the suite run under the flavor it gates, gates forming a staircase) and _A build enables exactly one shipped flavor_ (two shipped flavors refused at compile time, the wrong flavor refused for the tree, `development` additive and never shipped). No published requirement is modified: _The manifest version names the channel_ is what the build guard reads, and _A series branch carries only cherry-picks of `main`_ already requires a tagged tree to build "under the channel its manifest version names".

## Impact

- **Manifests:** `crates/bedrock`, `crates/bedrock-core`, `crates/bedrock-derive`, `crates/outpost`, `crates/outpost-fabric`, `crates/outpost-hold`, `crates/outpost-protocol`, `crates/domain`, `crates/transload`, `crates/cartograph`, `crates/store`, and `src-tauri` each declare the four flavors and forward each to every workspace dependency that declares it.
- **Crate roots:** one `compile_error!` guard per declaring crate, refusing any two of `nightly`, `preview`, `stable`.
- **`src-tauri/build.rs`:** reads the enabled `CARGO_FEATURE_*` flavors and the manifest's channel from `CARGO_PKG_VERSION_PRE`, and refuses the combinations the design names.
- **`tooling/xtask`:** `test-plan` emits the tree's channel flavor beside its filterset; the nextest step passes it as `--features`; `workflows.rs` gains the generator tests that hold every suite to its flavor and every channel's build to its own; the three release workflows and `pull-request.yaml` are regenerated.
- **Ordering:** this change and `release-groups-guards` depend on nothing and on each other not at all. `protected-branches` depends on `release-groups-guards`; `library-minting` depends on all three, and on this one specifically for `stable` being a feature every library crate declares.

## Non-goals

- No change to flavor identity: bundle id, updater endpoint, data directory, icon, and product name stay in the Tauri flavor configs, `WAYPOINTS_FLAVOR`, and the Gradle resources.
- No gated code. This change makes gating possible and enforced; the first nightly-only test is its proof, not a feature.
- No automation of promotion, and no soak timer; readiness is a human reading.
- No channels for library groups and no library minting; that is `library-minting`.
- No change to the `development` chain (`development → store/development → store/mock`) beyond declaring the feature where it is missing.
- No flavor on the pre-commit hooks' `clippy` and `nextest` steps in `.config/hk.pkl`; they keep compiling the ungated baseline, and the pull-request suite is the flavored gate.
- No change to how a build-only dispatch picks its ref; one that pairs a channel's workflow with a tree of another channel is now refused by the guard rather than built, which is the guard working.
