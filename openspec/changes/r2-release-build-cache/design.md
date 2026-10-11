## Context

See proposal.md — Why. The pieces that shape the approach:

- **Build jobs are generated.** `jobs::build` in `tooling/xtask/src/tasks/workflows/_jobs.rs` assembles every `build-*` job; step constructors live in `_steps.rs`; `Profile` (with `name()` and `environment()`) and the `resolving` helper (provider selection and vault credential, always set together) live in `_helpers.rs`. `devenv_update.rs` is the one other resolving workflow. Nothing in `.github/workflows` is edited by hand.
- **The build runs inside `xtauri`'s own `secretspec run`.** `just tauri::build <target> <flavor>` calls `tooling/bin/xtauri`, which wraps `cargo tauri build` in `secretspec run --profile release --scope <target>`. On macOS, iOS, and Android it also runs the build under `env -i`, passing only the `FORWARDED` names and `FORWARDED_PREFIXES` (`MISE_`). `xtauri` also selects the C compilers: `CC_<triple>`/`CXX_<triple>` and `CARGO_TARGET_*_LINKER` for iOS (`/usr/bin/clang`) and Android (the NDK's per-target clang); macOS and Linux use the system `cc` on a `PATH` it orders.
- **The generator forbids caches in publishing jobs.** `environments_follow_the_vault_credential` in `workflows.rs` fails any job that declares an environment and has a step whose `uses:` contains `cache` or whose `with.cache` is true. `.github/zizmor.yml` silences zizmor's `cache-poisoning` audit for the preview and stable workflows on the strength of that test.
- **Tools reach CI through the mise projection.** `devenv.nix` declares every tool with its nix package, mise spec, and lanes; devenv writes `.config/mise.<lane>.{toml,lock}`. `mise-action` installs the selected lanes in one step, with an optional built-in cache of its install directory (the Actions cache, keyed by config hash); that cache is off on build jobs today because of the guard above.
- **The generator already wraps and exports.** `steps::SecretSpec::run(cmd)` renders `secretspec run --profile <p> [--scope <s>] -- <cmd>`; `SecretSpec::export()` renders `secretspec export ... --format gha`. `with_scope` exists and is `#[allow(dead_code)]`. The published secret-provisioning spec prefers wrapping: export "widens the blast radius to the rest of the job".
- **secretspec scopes** are a top-level `[scopes.<name>] secrets = [...]` table; `--scope` narrows a profile's resolution to the listed names. The release profile already uses one scope per platform.
- **sccache's S3 backend** reads `SCCACHE_BUCKET`, `SCCACHE_ENDPOINT`, `SCCACHE_REGION`, `SCCACHE_S3_KEY_PREFIX`, and the standard `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` pair — in the **server** process, which starts lazily from the first client invocation unless started explicitly with `sccache --start-server`. The client (what `RUSTC_WRAPPER` names) talks to the server over a local port and carries no storage configuration. For R2 the endpoint is `https://<account-id>.r2.cloudflarestorage.com` and the region is `auto`.
- **cc-rs's wrapper contract** is `CC="sccache <compiler>"`: it recognises `sccache` (with `ccache`, `distcc`, `icecc`, `cachepot`, `buildcache`, `kache`) as a leading wrapper token and invokes it with the real compiler as the first argument.
- **rclone** speaks S3 to R2 with `provider=Cloudflare`, can take its credentials from `AWS_*` (`env_auth=true`), and accepts a remote as a connection string, so no config file is needed.
- **`hashFiles()`** is available in workflow expressions and is evaluated by the runner, identically on every platform.
- **The Actions cache is at 8.9 GiB of its 10 GB default** in five entries (three test jobs, the nightly planner, one mise lane), all on `main`. GitHub evicts idle entries after seven days and least-recently-used above the quota; low-trust triggers get read-only access to the default-branch scope by default.
- **The Android project** under `src-tauri/gen/android` is committed; devenv generates `gradle/libs.versions.toml`, `gradle/wrapper/gradle-wrapper.properties` (Gradle 8.14.4, no `distributionSha256Sum`), and `local.properties`. No dependency verification is configured.
- **The vault.** `r2-ci-cache` in `wp-prod` (the former `r2-releases-sccache`, renamed, token reused) is an API-credential item: `username` is the S3 access key id, `credential` the secret access key, `endpoint` the account's S3 URL without a bucket path, and `account-id` is also stored. `botswain` holds the App id and private key the `automation` profile reads today.

## Goals / Non-Goals

**Goals:**

- Every release `build-*` job compiles through sccache and restores its fetched directories from one R2 bucket.
- The store credential reaches only the sccache server and the rclone processes — never the job environment, the build, or anything that runs inside it.
- Every restored byte is one the build's pins would have verified: compilation content-addressed, directories keyed by hash-bearing manifests, Gradle made to bear hashes.
- CI's non-signing credentials live in one profile, grouped by consumer, resolved only by scope.
- Everything is derivable: tool versions from devenv, workflow YAML from xtask, cache keys from committed manifests.

**Non-Goals:**

- Those in the proposal.
- Replacing `Swatinem/rust-cache` on the test jobs, or sharing `target/` between jobs.
- A general-purpose cache action. The restore/save pair handles a known list of directories with known keys.

## Decisions

### D1 — The credential is wrapped, never exported; the server holds it

**Chosen** (2026-10-10, replacing two earlier drafts that exported the profile into `$GITHUB_ENV`). Three kinds of step touch the store, and each runs the contacting process under `secretspec run --profile ci --scope cache --`:

- `sccache-start`: `sccache --start-server`, with the non-secret store configuration (D2) on the step's `env`. The server daemonises with the resolved credential in its environment and outlives the step.
- `cache-restore-*` / `cache-save-*` (D9): the `rclone` invocation inside each script is the wrapped command; `tar`, logging, and the step output are not.
- `sccache-stats` (D6) needs no credential; it talks to the server's local port.

The build step gets `RUSTC_WRAPPER=sccache` and nothing else from this change. The client finds the server on its default local port, submits compilations, and never learns where they are stored. On every platform the build's environment, and so every `build.rs` and every `cc` invocation inside it, is free of the key pair.

**Why.** The token is an integrity credential: with it, an attacker writes objects that a later signed release restores. `export --format gha` would place it in `$GITHUB_ENV`, where every later step — upload actions, fastlane, and on the scrubbed targets everything `xtauri` forwards — can read it; the earlier draft forwarded `AWS_*` into the clean room, handing it to several hundred third-party build scripts. Wrapping confines it to two processes we invoke. This is also the shape the published spec prefers, and it removes the forwarding (D5) and the "widened blast radius" the export needed defending.

**Cost.** Five to seven `secretspec run` invocations per job, each a vault round-trip of a second or two. Accepted.

**Alternatives rejected.**

- _`SecretSpec::export()` of the cache scope before the first cache step._ Fewer vault calls; the credential in `$GITHUB_ENV` for the rest of the job and, forwarded, inside the compile. Rejected for the reason above.
- _`cachix/secretspec-action`._ Export with a second secretspec install; rejected earlier and moot now.
- _Adding the cache secrets to the `release` profile's platform scopes._ They would ride `xtauri`'s own `secretspec run` — straight into the compile environment, the opposite of the goal — and the `rclone` steps outside `xtauri` would still need them.

### D2 — The server-start step carries the store configuration; the build step carries only the wrapper

`sccache-start`'s `env` is `SCCACHE_BUCKET=ci-cache`, `SCCACHE_S3_KEY_PREFIX=sccache/`, `SCCACHE_REGION=auto`, `SCCACHE_IGNORE_SERVER_IO_ERROR=1`, `SCCACHE_IDLE_TIMEOUT=0`; the endpoint and key pair arrive through the wrap. These are configuration, not secrets, and belong in the YAML where a reader can see where the build caches. The build step's `env` gains `RUSTC_WRAPPER=sccache` and nothing else — emitted by `steps::with_sccache(step)`, the single place the wrapper is enabled, so a job cannot wrap the compiler without going through it, which is what the guard (D7) checks.

The idle timeout is zero because `pnpm install` and the cargo registry restore run between the server start and the first `rustc`; the stats step (D6) stops the server explicitly. The bucket name is configuration rather than a vault field because it is not secret and cannot change — R2 buckets are not renameable. The key prefix keeps sccache's objects apart from the directory snapshots (D9) so lifecycle rules can treat them differently.

**Why `ci-cache`.** The bucket is named for its writer class, not its contents or first consumer. "Only CI writes" is the invariant every trust argument here rests on, and it is meant to hold permanently: a developer machine holding the credential would be a second, less controlled writer, which is exactly what the design says cannot exist. The intent is for every cache CI uses — release workflows now, test and pull-request workflows later (Follow-ups) — to live in this one bucket. Release artifacts will be hosted from a different bucket in a later change. `build-cache` (contents) was the alternative; it already names the spec area, where it describes the capability rather than the store.

_Alternative rejected:_ letting the client auto-start the server from the build step. The server would then inherit the build's environment, which is deliberately credential-free, and fall back to a local disk cache — a silent miss of the whole point. Starting it explicitly, outside the build, is what makes the credential-free build possible.

### D3 — `SCCACHE_IGNORE_SERVER_IO_ERROR=1`: an unreachable bucket costs time, not the release

Without it, the server refuses to start when the storage check fails, `sccache-start` would fail, and the release with it. With it, the server starts, logs the storage error, and compiles uncached. If the server nonetheless fails to start, the first `rustc` auto-starts one from the build's environment, which has no store configuration and so caches to local disk for the rest of that job. Either way the build completes, and the stats step (D6) prints the cache location, which is how the rollout tells `s3` from a silent fallback. **Verify at implementation** that the variable exists under that name in the sccache version devenv pins.

### D4 — sccache and rclone are projected mise tools in `build-desktop` and `build-mobile`

`devenv.nix` gains `sccache = { package = pkgs.sccache; spec = "aqua:mozilla/sccache"; envs = [ "build-desktop" "build-mobile" ]; }` and `rclone = { package = pkgs.rclone; spec = "aqua:rclone/rclone"; envs = [ "build-desktop" "build-mobile" ]; }`. The projection regenerates both lanes' `toml` and `lock`; `check-generated` covers them. `RUSTC_WRAPPER=sccache` then resolves through the mise shim on `PATH`, which crosses the clean room because `MISE_*` is already forwarded.

_Alternative rejected:_ `mozilla-actions/sccache-action`. Two real objections: it installs a version the repo does not declare, against "a tool's version has a single source", and it duplicates what the projection already does.

Two lints would also fire on it, and both are false positives here, not reasons. The generator's `environments_follow_the_vault_credential` rejects any `uses:` containing `cache` in a job that declares an environment — a substring check that cannot tell a cache-restoring action from one that installs a cache client. zizmor's `cache-poisoning` audit (verified against its source, 2026-10-09) lists `Mozilla-Actions/sccache-action` in its table of cache-aware actions and fires on workflows it deems publishing — tag-push or release triggers, or a known publisher action. Preview and stable trigger on tag pushes and are already in the audit's ignore list; nightly triggers on schedule and dispatch and publishes through a plain `gh release` step, so zizmor does not classify it as publishing at all. Both lints could legitimately be silenced for this step, because the threat they model is absent: cache poisoning needs a writer less trusted than the reader. For GitHub's own cache that was once the default state and is now a misconfiguration: GitHub gives low-trust triggers read-only access to the default-branch cache scope by default, and only trusted triggers can write it. The R2 bucket is stricter still: exactly one class of writer — a job holding the environment credential — which is the same class that signs. Restating the generator's guard (D7) records this reasoning in the test rather than leaving an exemption to be rediscovered.

The same fact bears on `rust_cache()`'s `save-if: main`: with GitHub's default in force it is belt-and-braces rather than the thing keeping the test-job cache clean. It stays, because it also keeps feature branches from churning the shared entry, but it is not a security control.

### D5 — `xtauri` forwards `RUSTC_WRAPPER` and wraps the C compilers it selects

`FORWARDED` gains `RUSTC_WRAPPER`. Nothing else: the client carries no store configuration and no credential, so the `SCCACHE_*` and `AWS_*` prefixes an earlier draft forwarded are not needed and would be wrong to forward. The comment above the list explains the addition. Linux and Windows are unscrubbed and inherit the ambient environment already, which under D1 contains no credential either.

**C and C++.** The workspace compiles C on every platform through cargo build scripts — `ring`, `zstd-sys`, `libmimalloc-sys`, `blake3`, and `embed-resource` on Windows — and sccache caches those invocations as readily as `rustc`'s. `xtauri` is where the wrapping belongs, because `xtauri` already owns compiler selection. The rule: when `RUSTC_WRAPPER` is set, `xtauri` prefixes every C and C++ compiler it selects with that wrapper, and sets `CC="<wrapper> cc"` / `CXX="<wrapper> c++"` for the desktop targets where it selected none. `CARGO_TARGET_*_LINKER` is never prefixed — sccache does not cache links, and the linker must stay a bare path.

Keying on `RUSTC_WRAPPER` rather than a second variable: there is one wrapper, and "the compiler wrapper is in effect" is one fact. A build with `RUSTC_WRAPPER` set and unwrapped C compilers is the inconsistent state, not a configuration anyone wants.

**Windows is excluded for C.** When `CC` is unset, cc-rs finds MSVC's `cl.exe` through `vswhom` and sets up its environment itself. Setting `CC="sccache cl"` makes `cl` a bare name that must already be on `PATH` in an initialised VS environment, which a hosted runner's shell is not. sccache supports MSVC, so this is solvable — resolving `cl.exe`'s path in the recipe, or entering a developer shell — but it is its own piece of work, and the Rust side of the Windows build still caches. Recorded as a follow-up.

_Alternative rejected:_ setting `CC`/`CXX` from the generator alongside `RUSTC_WRAPPER`. The generator does not know which compiler each target uses; `xtauri` does, and a generator-set `CC` would be overridden by `xtauri`'s per-target table on iOS and Android anyway.

### D6 — `sccache --show-stats && sccache --stop-server` after the build, `if: always()`

`steps::sccache_stats()` runs after `build-recipe` and before `upload-artifacts`, unconditionally. It needs no credential — both commands talk to the local port — so it is a plain step. `--show-stats` prints hits, misses, and errors with Rust and C/C++ counted separately, and the cache location, which the rollout checks reads `s3` and not local disk (D3). `--stop-server` ends the daemon the start step left running, so the credential it held is gone before the upload and publish steps run.

### D7 — The cache guard is restated, not relaxed

`environments_follow_the_vault_credential` keeps its `uses` / `with.cache` check untouched; every step this change adds is a `run:` step and does not meet it. It gains positive assertions, in any job:

- a step setting `RUSTC_WRAPPER` is preceded in the same job by a step whose `run` starts the sccache server under `secretspec run --profile ci --scope cache`;
- every step whose id starts `cache-restore-` / `cache-save-` carries the provider and credential itself (the existing pairing test already demands both) and its `run` resolves `--profile ci --scope cache`;
- any job containing either declares an environment;
- no step's `env` names `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, or `SCCACHE_ENDPOINT` — the credential is never configuration;
- `mise-action` with `cache: true` is allowed by name in such a job (D11).

Its doc comment and `.github/zizmor.yml`'s comment are rewritten to say that the caches a publishing job may touch are those whose write credential lives in the job's environment and reaches only the processes that contact the store, plus mise's own install cache under GitHub's default write rules. `release-nightly.yaml` is not in the zizmor ignore today; whether it needs to be is checked when the regenerated workflows are linted.

### D8 — The `ci` profile and its scopes in `secretspec.toml`

```toml
# Every non-signing credential CI uses, grouped by consumer. Never resolved
# unscoped: a job names the scope of the one thing it is doing.
[profiles.ci]
  WP_BOTSWAIN_APP_ID      = { description = "wp-botswain GitHub App ID", refs = { op = { item = "botswain", field = "app-id" }, sops = { item = "botswain-app-id" } } }
  WP_BOTSWAIN_PRIVATE_KEY = { description = "wp-botswain GitHub App Private Key", refs = { op = { item = "botswain", field = "private key" }, sops = { item = "botswain-private-key" } } }
  SCCACHE_ENDPOINT        = { description = "R2 S3 endpoint URL for the account owning the ci-cache bucket", refs = { op = { item = "r2-ci-cache", field = "endpoint" }, sops = { item = "ci-cache-endpoint" } } }
  AWS_ACCESS_KEY_ID       = { description = "R2 S3 access key id scoped to the ci-cache bucket", refs = { op = { item = "r2-ci-cache", field = "username" }, sops = { item = "ci-cache-access-key-id" } } }
  AWS_SECRET_ACCESS_KEY   = { description = "R2 S3 secret access key scoped to the ci-cache bucket", refs = { op = { item = "r2-ci-cache", field = "credential" }, sops = { item = "ci-cache-secret-access-key" } } }

  [profiles.ci.defaults]
    inherit   = false
    providers = [ "op" ]
    required  = true

[scopes.botswain]
  secrets = [ "WP_BOTSWAIN_APP_ID", "WP_BOTSWAIN_PRIVATE_KEY" ]

[scopes.cache]
  secrets = [ "SCCACHE_ENDPOINT", "AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY" ]
```

The `automation` profile is removed; its two declarations move here unchanged, coordinates and all. Standalone for the same reason `automation` was: with `required = true`, inheriting `default` would make every signing secret required. The `sops` coordinates keep the declaration's coordinates-per-store shape; a contributor who wants the cache locally points them at a bucket of their own.

**Why one profile with scopes rather than two profiles.** A profile is the unit a consumer asks for; a scope is the unit it receives. `release` already works this way — one profile, one scope per platform — and the published spec's guarantee is about what a resolution _requires_, which a scope delivers as well as a profile does. Grouping CI's non-signing credentials under one name keeps them findable as a set and gives the next one (a Cachix token, say) an obvious home and an obvious rule: add a scope, never widen one. The cost is a rule the generator must enforce — no unscoped resolution of `ci` — which D7 and the delta spec state.

The variable is named `SCCACHE_ENDPOINT` because sccache reads that name; `rclone` reads the same variable inside its wrapped shell (D9). The item's `account-id` is not used: neither tool speaks the Cloudflare API, and the endpoint already embeds it. `secretspec check --profile ci --scope cache --explain` reports a mislabelled field without revealing values.

### D9 — Directory caches are generated restore/save `run:` steps over `rclone`, keyed by `hashFiles`, with only the `rclone` call wrapped

For each cached directory, `steps::cache_restore(dir)` and `steps::cache_save(dir)` where `dir` is a `CachedDir { name, path, key_files, platforms }` entry in a table in `_steps.rs`. Both are `shell: bash` `run:` steps, ids `cache-restore-<name>` and `cache-save-<name>`, with `CACHE_KEY: <name>-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('<key files>') }}` and `CACHE_PATH` on the step `env`, and `helpers::resolving` applied so the provider and vault credential are on the step.

Inside each script, exactly one line contacts the store, and it is wrapped: `secretspec run --profile ci --scope cache -- bash -c '<rclone command>'`, single-quoted so `${SCCACHE_ENDPOINT}` expands inside the resolved child rather than before it. The remote is a connection string, so nothing is written to disk: `:s3,provider=Cloudflare,env_auth=true,endpoint=${SCCACHE_ENDPOINT}:ci-cache/dirs/${CACHE_KEY}.tar.gz`. Restore: wrapped `rclone copyto` the object to `$RUNNER_TEMP`, then unwrapped `tar -xzf` into the path's parent, log `hit`; on any failure log `miss`, remove the partial file, and exit 0. Save: skipped (`if:`) when restore reported a hit; otherwise wrapped `rclone lsf` the key, and only if absent `tar -czf` the path and wrapped `rclone copyto` it up; any failure is logged and exits 0. Hit or miss is published as a step output so the save's `if:` and the log both read it.

**Why `rclone`.** One static binary on every platform including Windows, already in nixpkgs and aqua, speaks R2's S3 with a documented provider setting, and needs no config file. The `aws` CLI is a Python distribution; `curl` with SigV4 is a signing implementation to maintain.

**Why `tar.gz` and not zstd.** Universality: GNU tar on Linux, bsdtar on macOS, and Git Bash's tar on Windows all handle gzip without another tool in the lane. The directories are hundreds of megabytes, not gigabytes, so compression speed is not the bottleneck. zstd is a one-line change in the two steps if it ever is.

**Why `hashFiles`.** Evaluated by the runner, identical on every platform, and it hashes exactly the committed files the key should depend on. A key computed in the generator would bake a hash into the YAML and change the committed workflows on every lockfile change.

**Why write-once.** A key is a function of the manifests that pin the directory's contents; two saves under one key should hold the same bytes, so the second is wasted work at best and a non-determinism at worst. Refusing to overwrite makes an entry a fixed snapshot — the property a cache needs to be a faithful substitute for a fresh fetch — and makes the store append-only from the builds' point of view, which is what a lifecycle rule should be able to assume.

**Alternatives rejected.**

- _`actions/cache` on GitHub's cache for the directories, R2 only for sccache._ Sound under GitHub's defaults and less work. Rejected on the proposal's grounds: quota pressure, ref scoping, seven-day eviction, and a second trust surface and store to reason about when the first already fits.
- _S3-compatible `actions/cache` replacements (`tespkg/actions-cache`, `everpcpc/actions-cache`)._ Marketplace actions to pin and audit, each containing `cache` in its name and so tripping the guard by substring, doing in JavaScript what two `run:` steps do in sight — and each needing the credential in its `with:` block, which is `$GITHUB_ENV` by another route.
- _Caching the Rust `target/` directory as a snapshot instead of sccache._ The snapshot would be gigabytes per platform per run and keyed on a lockfile that changes weekly; sccache keys per unit and moves only what changed.

### D10 — What is cached as a directory, and by what key

| name             | path                                                                                      | key files                                                                                                                                                                                            | platforms |
| ---------------- | ----------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- |
| `pnpm-store`     | the pnpm store, pinned to `$RUNNER_TEMP/pnpm-store` via `npm_config_store_dir` on the job | `pnpm-lock.yaml`                                                                                                                                                                                     | all       |
| `cargo-registry` | `$CARGO_HOME/registry` (index, cache, src)                                                | `Cargo.lock`                                                                                                                                                                                         | all       |
| `gradle`         | `~/.gradle/caches/modules-2`, `~/.gradle/wrapper/dists`                                   | `src-tauri/gen/android/gradle/verification-metadata.xml`, `gradle/libs.versions.toml`, `gradle/wrapper/gradle-wrapper.properties`, `settings.gradle.kts`, `build.gradle.kts`, `app/build.gradle.kts` | Android   |
| `android-ndk`    | `$ANDROID_HOME/ndk/<version>`                                                             | the version string from `WorkflowContext` (no file; the key is the version)                                                                                                                          | Android   |

Each is eligible under the spec's hash-bearing rule: pnpm's lockfile carries integrity hashes; `Cargo.lock` carries checksums; Gradle's metadata carries per-artifact checksums once D12 lands and the wrapper properties carry `distributionSha256Sum`; the NDK is fetched by `sdkmanager`, which verifies the package checksum from Google's repository manifest before unpacking, and the key is the exact version.

Left out, and why: mise installs (D11); apt packages (a minute, and apt's cache is awkward to snapshot soundly); Xcode's DerivedData (small for this shell, and fragile); the Android SDK beyond the NDK (`setup-android` configures the image's preinstalled SDK rather than fetching one).

Order in `jobs::build`: `cache-restore-pnpm-store` before `pnpm-install` and its save after; `cache-restore-cargo-registry` before the build; on Android, `cache-restore-gradle` and `cache-restore-android-ndk` before `install_ndk`; `sccache-start` immediately before the build; `with_sccache` around the build; `sccache-stats` and the remaining saves after it and before `upload-artifacts`. The NDK restore replaces `sdkmanager`'s download on a hit; the install step stays and is a no-op when the directory exists.

### D11 — mise's install cache stays on the Actions cache, and says so

The credential resolver — `secretspec` and the `op` CLI — is installed by `mise-action`. A directory cache for mise's install directory on R2 would need those credentials before mise has run: a cycle. `mise-action`'s built-in `cache: true` is switched on for the build jobs instead. It is the Actions cache, keyed by the lane's config hash, written only by trusted-trigger runs under GitHub's default rules, and holding tools whose checksums `mise.lock` pins — sound on the same terms as the R2 snapshots, just in a store we do not own. The guard (D7) allows it by name and the zizmor comment records it as the one third-party cache in a publishing job.

_Alternative rejected:_ bootstrapping `secretspec` and `op` outside mise (a direct download in a `run:` step) so mise itself can be restored from R2. Two version pins outside the projection, against "a tool's version has a single source", to save the Actions cache one entry per lane.

### D12 — Gradle dependency verification, checksums only

`src-tauri/gen/android/gradle/verification-metadata.xml` is generated with `./gradlew --write-verification-metadata sha256 <tasks>` and committed, with `<verify-metadata>true</verify-metadata>` and `<verify-signatures>false</verify-signatures>`. Gradle then refuses any resolved artifact — dependency or plugin — whose checksum is absent or differs, naming it. A `just android::verification-metadata` recipe regenerates the file; the tasks it runs are the ones the release build runs, so every configuration the build resolves is covered. Whether the Tauri Gradle plugin's Rust build can be short-circuited during that run, or the recipe simply pays for one Android build, is settled at implementation.

The wrapper distribution is pinned the same way: `nix/devenv/android.nix` writes `distributionSha256Sum` into the generated `gradle-wrapper.properties`, taken from the hash nix already records for the same `gradle-<version>-bin.zip` it fetches, so the pin moves with the version and never by hand.

**When it goes stale.** `devenv-update` bumps Gradle, Kotlin, and the JDK by moving `libs.versions.toml` and the wrapper; the Android build on that pull request then fails on verification until the recipe is run. That is the intended failure: a dependency moved and nobody has looked at it yet. The `devenv-update` pull-request body gains a line saying so.

_Alternative rejected:_ also enabling signature verification. It needs a trusted-keys list that is its own curation task, and the directory cache needs only checksums.

### D13 — Windows runs the same bash steps

The restore/save and server steps declare `shell: bash` and run under the Git Bash the runner provides, as every other `run:` step in these workflows already does. `rclone`, `sccache`, `secretspec`, and `tar` are on `PATH` there; the paths in D10 are expressed through `$RUNNER_TEMP`, `$CARGO_HOME`, and `$HOME`, which Git Bash maps. The pnpm store is pinned to `$RUNNER_TEMP/pnpm-store` precisely so that its location is the same expression on every platform.

### D14 — In the generator, a job declares its environment; a profile and a scope describe a resolution

Today `Profile::environment()` derives the GitHub environment from the profile, which worked while each profile had one consumer in one environment. With `ci` resolved by build jobs (under `App Release`, for the cache) and by automation jobs (under `CI`, for botswain), that derivation is wrong in principle — the published spec already says the environment "follows from the work the job does" — and now wrong in practice. So:

- `Profile` becomes `Release | Ci`, with `name()` only.
- `Scope` is a small enum — `Platform(Platform)` for release, `Cache` and `Botswain` for ci — with `name()`; `SecretSpec::with_scope` takes it, loses its `#[allow(dead_code)]`, and `with_profile(Profile::Ci)` without a scope is made unrepresentable (a `ci(scope)` constructor rather than two setters).
- `Environment` is an enum, `AppRelease | Ci`, declared by the job constructors directly: `jobs::build` declares `AppRelease` because it signs and publishes; `devenv_update` declares `Ci` because it authenticates as the App. `Profile::environment()` is deleted.
- `helpers::resolving` is unchanged: provider and token on every resolving step; the token expression is the same in both environments.

The existing tests (`environments_follow_the_vault_credential`, `provider_and_credential_are_set_together`) hold without change; D7 extends the first.

## Risks / Trade-offs

- [A mise shim as `RUSTC_WRAPPER` adds a process hop per `rustc` invocation] → Measured, not assumed: compare wall-clock against a run with `RUSTC_WRAPPER` set to the resolved path (`mise which sccache`) during rollout; switch the helper to resolve the path in a preceding step if the shim costs more than a few percent.
- [The server started outside the clean room must be reachable from inside it] → It listens on a local port; the client needs no environment to find it. Verified by the stats step showing non-zero counts and an `s3` cache location on an iOS build during rollout.
- [The server exits before the build reaches it] → `SCCACHE_IDLE_TIMEOUT=0` on the start step; the stats step stops it explicitly. If it does die, the client auto-starts a credential-free server and the stats step shows a local-disk location (D3), which the rollout treats as a failure to fix, not a pass.
- [Several `secretspec run` invocations per job, each a vault round-trip] → A second or two each, five to seven per job, against builds measured in tens of minutes. Accepted as the price of never exporting.
- [`SCCACHE_IGNORE_SERVER_IO_ERROR` and `SCCACHE_IDLE_TIMEOUT` are named from memory] → Task 1.1 verifies the names against the pinned sccache before anything depends on them.
- [A wrapped C compiler changes what a build script sees — `CC` now has two tokens] → cc-rs handles the split itself for the wrappers it knows; a build script that reads `CC` raw and execs it as a path would break. None of the five C-compiling crates does (all go through `cc`). Verified per platform by the rollout: the stats step reports C/C++ hits separately from Rust.
- [sccache rejects a compiler it cannot classify and falls back to running it uncached] → That is the designed failure mode: the compile still succeeds and the stats step shows the miss reason. The NDK clang and Apple clang are both ordinary clang to sccache.
- [A poisoned cache entry would reach a signed artifact] → The threat model is unchanged from today: only the holder of the environment credential can write, which is the same set that can sign, and under D1 that credential is held by two processes we invoke rather than by the job. Write-once keys (D9) additionally mean an entry, once written, cannot be replaced by a later run even with the credential.
- [A directory snapshot restores files the consumer then treats as authoritative without re-verification] → By construction only directories whose manifest carries content hashes are cached (D10, D12), so the snapshot holds exactly the bytes a fresh fetch would have had to match. The cargo registry and pnpm store additionally re-verify on use: cargo checks `Cargo.lock` checksums against the registry cache, and pnpm checks integrity on link.
- [Gradle verification metadata goes stale and blocks the Android release] → Loud and specific: Gradle names the artifacts. The regeneration recipe is one command; the `devenv-update` body points at it. Better a blocked build than an unverified dependency in a release.
- [Two save steps race on the same key from concurrent jobs] → The nightly workflow is serialised; preview and stable build different tags and run rarely. If two jobs do race, `rclone lsf` then `copyto` is check-then-act and the second write replaces the first with bytes that are, by the key's construction, the same. Acceptable; noted so nobody adds a lock for it.
- [Folding `automation` into `ci` touches the one working secret-resolving workflow] → `devenv-update` changes by two flags on one step and nothing it receives changes; the first scheduled run after landing, or a manual dispatch, proves it. The `CI` service account's grants are untouched.
- [An unscoped `secretspec run --profile ci` would resolve everything] → Unrepresentable in the generator (D14) and asserted on the rendered YAML (D7); the delta spec states the rule so it outlives this change.
- [Bucket growth with no eviction] → sccache never evicts remotely and the snapshots are write-once. The bucket will carry lifecycle rules per prefix (`sccache/`, `dirs/`) declared in OpenTofu, in infrastructure tooling that is its own change; until then growth is bounded by what the builds write, which is a few gigabytes per platform plus a few hundred megabytes per manifest change.
- [Windows: sccache and MSVC `rustc`; `rclone`, `sccache`, and `tar` under Git Bash] → Supported upstream; the Windows build job is unscrubbed so there is no forwarding concern. Covered by the rollout check in tasks.

## Migration Plan

1. Land the tool projection, the `ci` profile and scopes with `automation` removed, the `xtauri` forwarding and C wrapping, Gradle verification, and the generator change together with the regenerated workflows (three release workflows and `devenv-update`); `check-generated` must be green.
2. Dispatch `devenv-update` and confirm the botswain scope resolves and the run completes as before.
3. Dispatch `release-nightly` with `platforms: All` (build-only, since it is a dispatch). Confirm every restore reports a miss and every save writes, and the stats step shows an `s3` cache location and sccache writes on every platform for Rust, and for C/C++ everywhere but Windows.
4. Dispatch `release-nightly` again against the same commit. Confirm every restore reports a hit and the stats step shows hits on every dependency compilation — the determinism probe. Any miss on an unchanged crate is investigated before step 5.
5. Dispatch `release-preview` build-only (empty tag) to confirm the cross-channel hit.
6. Rollback is removing `sccache-start`, `with_sccache`, and the cache steps from `jobs::build` and regenerating; today's cold builds come back with no other change. The `ci` profile, tool projection, and Gradle verification can stay in place harmlessly.

## Follow-ups

- Windows C compilation through sccache (D5). Needs `cl.exe` resolved to a path, or the build run inside an initialised developer environment, before `CC="sccache <path to cl.exe>"` is viable.
- Extending the store to the test jobs and pull-request workflows under the `CI` environment. Needs a decision about a read-only credential and who writes; the `ci` profile's `cache` scope, the `sccache/` and `dirs/` prefixes, and the job-declared environment (D14) are laid out so that is a token and a bucket-policy change.
- Token rotation, from the infrastructure tooling that will own the bucket. Shortens the window a leaked credential is useful; orthogonal to D1, which shortens the list of places it can leak from.
- Gradle signature verification, once a trusted-keys list is worth curating.

## Resolved

- _Which bucket, and what is it called?_ `ci-cache`, named for its writer class; `releases-sccache` has been deleted, since it could not be renamed and its name no longer described the store. The vault item was renamed to `r2-ci-cache` with its token reused; D8 names its existing fields.
- _Lifecycle rule?_ Yes, via OpenTofu in separate infrastructure tooling, which will also take over the bucket and token. Out of scope here; the prefixes in D2 and D9 are what it will key on.
- _Can Gradle be made to carry content hashes?_ Yes: dependency verification with checksums (D12) and `distributionSha256Sum` on the wrapper.
- _Export or wrap?_ Wrap (D1). The deciding fact was that the token is an integrity credential and the export would have carried it into every third-party build script.
- _One `ci` profile or a `ci-cache` profile?_ One `ci` profile with `cache` and `botswain` scopes (D8), the automation profile folded in, with "never unscoped" as the rule that keeps the guarantee.
