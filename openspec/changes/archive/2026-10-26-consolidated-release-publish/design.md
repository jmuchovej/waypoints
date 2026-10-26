## Context

See proposal.md — Why. The current shape, from the archived `r2-release-publishing` change: `prepare` (nightly) → `test-*` → `create-release` (drafts the record and its git-cliff notes) → `build-*` ×5, each of which uploads to the store through `cargo xtask release-upload`, records its target, and — on Android and iOS — runs `xfastlane publish` → `publish-release` (counted gate; flips the draft public, writes `{channel}.json`) and `withdraw-release` (the opposite gate).

Constraints that shape the approach:

- The iOS lane builds nothing: `upload_to_testflight` uploads an ipa `cargo tauri ios build` already produced, authenticating with an App Store Connect API key. Off macOS, fastlane uploads through Apple's Transporter for Linux, and refuses unless an `AppStoreInfo.plist` sits beside the ipa (`fastlane_core/itunes_transporter.rb`, added in fastlane 987a0105b, 2025-12-15; Transporter ≥ 4.1 requires it).
- `cargo tauri ios build` hands `xcodebuild -exportArchive` the repository's `src-tauri/gen/apple/ExportOptions.plist`, merging in only `method` (from `--export-method`) and the signing keys it derives; any other key in that file passes through.
- Apple distributes Transporter for Linux as a Makeself 2.5.0 self-extracting script at one unversioned URL (4.2.0.16 on 2026-10-21, `iTMSTransporter_installer_linux_x64_4.2.0.16.sh`, sha256 `f94a1ee8…159ea`). Its install script only asks for licence acceptance twice and copies `itms/` to `/usr/local/`. The `itms/` tree carries its own JRE and its launcher resolves everything relative to itself, so it runs from any directory.
- Every tool CI uses comes from the devenv projection into mise, and the projection takes each tool's version from a nix package (`toolchain/tool-projection`). nixpkgs has no Transporter.
- `xfastlane publish` today renders the store assets (typst, ImageMagick), writes the listing copy and release notes, and runs the lane — so a failed render or an over-long listing field surfaces inside the submission step.
- A dispatched nightly is always `--build-only`, and only `main`'s schedule publishes a nightly. The live publish path can therefore first be exercised after merge.
- Store build numbers are dated and single-use; a submission cannot be retried under the same number.

## Goals / Non-Goals

**Goals:**

- Every publishing act behind the completeness gate, none beside it.
- One place where a run either publishes or withdraws, with the point of no return explicit in the generated workflow.
- Reuse the archived change's key logic, client constructor, manifest writer, and withdraw command unchanged in behaviour.
- Nothing that can fail for a retractable reason runs after the point of no return.
- Every fastlane concern stays inside `xfastlane`.

**Non-Goals:**

- Moving release-note generation. The draft and its notes stay in `create-release`, before the builds: drafting is fully retractable (`discard_release`), and keeping it early means a notes failure stops the run before any build minutes are spent.
- Resuming a partially published run automatically.

## Decisions

### Builds stage, the publishing job publishes (2026-10-21, flattened 2026-10-22)

Each build job runs a new `cargo xtask release-stage --platform <p> [--tag <t>]`, which renames the platform's published and updater artifacts out of the bundler's nested output (`dmg/`, `macos/`, `appimage/`, `nsis/`, Gradle's `outputs/`) into one flat `release/`, each as `waypoints-{tag}.{ext}` — the name it is stored under. Every expected output must match exactly one file, so a change in what Tauri exports fails the build that produced it. The job uploads `release/` as its one workflow artifact, `waypoints-{platform}-{channel}`, replacing the nested upload of the bundle output. Staging needs no credential.

Every run stages, build-only included, so the rename is exercised by every dispatch. A build-only run has no tag; a nightly names its files for the tag the planner would mint (`v` and its version, which a build-only plan carries), and preview and stable fall back to `v` and the app's own version.

Every platform's extensions are its own, so the flat directories merge without collision, and a file's extension alone files it under `{os}/{arch}`. `release-upload --from release/` maps each `waypoints-{tag}.{ext}` to `{channel}/{series}/{os}/{arch}/` (channel derived from the version, as now), refuses a subdirectory or any other file, and writes each with `if_not_exists`. The bundler's globs stay in the build; the publishing job needs only the extensions `Platform` already declares.

The iOS build also stages its `AppStoreInfo.plist` in `release/`, beside the ipa, where fastlane reads it. It is upload metadata for the App Store, not a published artifact: `release-upload` skips exactly that name and never sends it to the store.

Linux builds and publishes three installers: the AppImage, then the deb and the rpm, the same way Android's aab follows its apk. The AppImage stays first, so it remains what the download link serves. The deb and rpm get stable stored addresses for package-manager channels (a Debian repository, an AUR PKGBUILD). Each is signed for the updater and its `.sig` is staged too. Tauri's updater (2.12) in an app installed from a deb or rpm asks for its own format's entry (`linux-x86_64-deb`) before the bare `linux-x86_64`, and refuses a payload that is not its own format. The bundler builds the rpm in-process, so the runner needs no `rpmbuild`.

`release-withdraw` deletes the updater artifacts along with the installers. Before, it attempted only the installer keys, which left a shortfall run's signatures and macOS payload behind.

Rejected: staging into the store's nested key layout. Downloaded artifacts would keep a five-level tree, and the per-platform trees would merge only by luck of disjoint paths. Also rejected: renaming in the publishing job, which would need every platform's bundler globs and would see a changed export only after every build had finished.

### The iOS export writes its App Store description (2026-10-21)

`src-tauri/gen/apple/ExportOptions.plist` gains `generateAppStoreInformation = true`, so the export writes `AppStoreInfo.plist` next to the ipa. Tauri merges the file rather than replacing it, so the key survives with or without `--export-method`, whose only effect is `method` — already `app-store-connect`.

Rejected: `--export-method release-testing`. Despite the CLI help, it is Xcode 15.4's name for ad hoc distribution, which App Store Connect does not accept as a TestFlight build.

### The publishing job runs on Linux (2026-10-21)

One job, so the retractability order is a step order the runner enforces rather than a `needs:` chain across jobs. Nothing in it needs macOS once the ipa arrives with its `AppStoreInfo.plist`. Order, each a separate step:

1. download every platform's staged artifact, merged into one flat `release/`;
2. `xfastlane prepare <flavor>` — renders, writes, checks, and fetches everything both submissions need (see below); resolves no credential;
3. `release-upload --from release/` — derives the one store credential;
4. `gh release edit --draft=false`;
5. mark the point of no return (writes a job output, see below);
6. `xfastlane publish android`, then `xfastlane publish ios` — each resolving only its platform's release scope;
7. `release-manifest` — derives its own store credential.

`prepare` runs before the store upload rather than just before the mark: it touches nothing outside the runner, so running it first means a render or fetch failure costs the run before a single object reaches the store.

The job runs its commands inside the devenv shell, as `release-index` and `check-generated` do. No mise lane carries typst or ImageMagick, and typst's fonts are nix store paths, so the shell is the only place `prepare` can render. It also supplies `cargo`, `fastlane`, `git-cliff`, `secretspec`, and `op`. xtask is compiled once up front through the build jobs' compiler cache, and that cache's server is stopped before the first publishing step. The store steps resolve the `release` profile in-process, and that profile's default provider is the vault, so under devenv they carry the vault credential alone.

`withdraw-release` gets the same checkout and shell. Its `cargo xtask release-withdraw` previously ran on a runner with no checkout and no toolchain, as did the old `publish-release` job's manifest write. Neither had run yet, because no run had published.
Rejected: a macOS publishing job. It needs no extra tooling, but pays macOS minutes for a job that compiles nothing. Also rejected: splitting the iOS submission into its own job, which reintroduces a cross-job ordering between steps 3, 5, and 6, and two jobs each able to fail after the other has passed the point of no return.

### `xfastlane` splits into `prepare` and `publish` (2026-10-21)

`xfastlane prepare [flavor] [locale]` does everything a submission needs that touches nothing outside the runner: renders the store assets, writes the listing copy and release notes (failing on an over-long field as `metadata` does today), and — on Linux only — fetches and unpacks Transporter into `fastlane/.transporter/`. It covers both stores in one call, because the assets and copy are shared and a failure in either should stop both submissions before either starts.

`xfastlane publish <platform> [flavor]` only runs the lane. It refuses to run if `prepare` has not, rather than preparing implicitly — an implicit prepare would move rendering and fetching back past the point of no return the moment a caller forgot the first step. On Linux, `publish ios` sets `FASTLANE_ITUNES_TRANSPORTER_USE_SHELL_SCRIPT=true` and `FASTLANE_ITUNES_TRANSPORTER_PATH` to the prepared tree itself, so the workflow step names no fastlane internals. Locally, the two commands run in sequence.

`publish --from <dir>` takes the package from the staged directory instead of the build output, which the publishing job does not have: it finds the one aab or ipa in `release/` and hands it to the lane as `WAYPOINTS_STORE_PACKAGE`, which only `xfastlane` and the Fastfile name. fastlane reads `AppStoreInfo.plist` only from the ipa's own directory (`ipa_upload_package_builder.rb`), which in the flat `release/` is where the iOS build staged it. `--from` defaults to `release/`, and the Fastfile has no other source: the lanes' globs over the build output are gone (2026-10-23), since staging is what checks the export and names the file, and a local submission stages first.

Rejected: a second artifact mirroring the build paths, so the Fastfile's globs find the packages unchanged — the ipa and aab would cross artifact storage twice.

Rejected: fetching Transporter in its own workflow step outside `xfastlane`. It spreads fastlane's requirements across the generator and the script, and a step only the iOS-on-Linux path needs would sit in a job that otherwise knows nothing about fastlane. Also rejected: fetching it inside `publish ios`, which would put a download — the likeliest failure of the lot — after the Android submission.

### Transporter is fetched at run time, as a recorded exception (2026-10-21)

Transporter is the one tool this pipeline uses that is not projected. `toolchain/tool-projection` allows that only for a tool meeting every one of its conditions, justified here and nowhere else:

- **Only a current build is published, at an unversioned location.** Apple serves Transporter for Linux from one URL whose file is replaced in place (`Transporter__Linux/bin`, 4.2.0.16 on 2026-10-21); a replaced build cannot be fetched again, so a pin would break on every Apple release with nothing to fall back to.
- **It updates itself when run.** Apple's Transporter guide describes its own automatic software update, so a pinned download would not determine what executes.
- **The fetch precedes every irreversible act.** It runs in `prepare`, step 2, before the store upload.
- **HTTPS, failing on error, with the artifact's own checks on.** `curl --fail --proto '=https'`, then Makeself's `--noexec --target` unpack without `--nocheck`, so its embedded SHA-256 of the payload is verified. This catches a truncated or corrupt download, not a malicious one; authenticity rests on TLS to Apple, which is also all a pin would have added beyond change detection.
- **The upload can be completed by hand.** The ipa and its `AppStoreInfo.plist` are retained as the iOS build's workflow artifact, and can be uploaded from a Mac if the Linux upload fails.

The unpack skips Apple's install script, which only asks for licence acceptance twice and copies `itms/` to `/usr/local/` — so no root is needed, and the bundled JRE runs from where it was unpacked.

Rejected: a pinned nix package projected through mise's `http` backend. It needs `nix/devenv/mise.nix` to read backend options from packages and to keep host-unavailable packages out of the shell, and the pin would break on every Apple release while not pinning what runs.

### The counted gate stays, on the publishing job (2026-10-21)

The publishing job keeps the archived change's gate: each build still records its target, and the job's `if:` requires every required target to be recorded and a tag to be present. GitHub documents that a job whose dependency is skipped is itself skipped unless its condition overrides the default — but the publishing job must carry an explicit `if:` for the tag condition anyway, and how an explicit condition interacts with a skipped dependency is exactly what the archived design declined to rely on. The counted gate is already implemented and tested, costs one output per build, and makes the question moot.

Rejected: dropping the count in favour of `needs:` plus `success()`. Viable if verified, but the verification is a live run with a skipped build and a tag — which the existing dispatch guard refuses — so it cannot be checked without disabling a safety check.

### Withdrawal reads the point of no return (2026-10-21)

Step 5 writes `submitted=true` as a job output immediately before the first submission. The withdraw job's gate becomes: a tag is present, and either the count fell short or the publishing job failed, and `needs.publish.outputs.submitted != 'true'`. Marking _before_ the first submission is deliberately conservative: a submission that failed partway may still have reached the store, so it is treated as past the line. A run past the line fails without withdrawing; its remaining steps (the second submission, the manifest) are re-run by hand.

Withdrawal itself is unchanged: discard the draft, `release-withdraw` (best-effort delete of every key the tag could have produced), and delete the tag only when the nightly planner job (`prepare` in the workflow — not `xfastlane prepare`) minted it.

Rejected: inferring the line from step conclusions in the withdraw job. Step outcomes are not visible across jobs; a job output is.

### Credentials are scoped per step (2026-10-21)

Every resolving step goes through `helpers::resolving` with only its own scope: the store scope on steps 3 and 7, `release`/`android` on the Android submission, `release`/`ios` on the iOS one, `GITHUB_TOKEN` on step 4, and none on step 2. Steps 3 and 7 each derive their own short-lived credential rather than sharing one, so no store credential survives across the submissions. The job declares the release environment, as every store-touching job does today.

## Risks / Trade-offs

- **A new Transporter build can break the iOS upload** → Unpinned, a release can be the first to meet Apple's new build, and an upload failure lands past the point of no return with Android already submitted. Accepted under the exception's conditions: the run fails visibly, nothing is withdrawn, and the retained ipa is uploaded by hand. A fetch or unpack failure, by contrast, lands in `prepare` and withdraws cleanly.
- **Which Transporter ran is not recorded by default** → `prepare` should print the unpacked build's version, so a failed upload can be attributed to our change or Apple's.
- **Transporter's self-update in CI is unverified** → Whether it updates non-interactively on a fresh runner is first seen on the first upload.
- **The Linux upload path is unexercised here** → fastlane documents it and the code enforces its preconditions, but neither has been run against this project. The build-only nightly verifies `AppStoreInfo.plist` is produced and staged; the upload itself first runs on the first scheduled publish after merge.
- **fastlane clears `~/.appstoreconnect/private_keys/` after an upload** → Harmless on an ephemeral runner.
- **All five builds' artifacts pass through workflow-artifact storage** → An extra transfer per release, and the artifacts sit in GitHub's storage for its retention period. Acceptable: they are the same bytes about to be published, and nothing secret is in them.
- **A failure past the point of no return needs a human** → The second submission or the manifest write must be re-run by hand. Accepted: automating it would mean retrying a store submission, which is the act that cannot be retried safely.
- **One runner holds every publishing credential in turn** → Step scoping shortens each credential's exposure, but a step that tampers with the runner could reach what a later step resolves. Mitigated by the release environment's protection rules and by the publishing job running no third-party build code.
- **The live publish path is unexercised before merge** → Covered by generator tests over the job's shape, gate, and step order; the first scheduled nightly after merge is the live check.

## Migration Plan

No data migrates; the store and its keys do not change.

1. Split `xfastlane` into `prepare` and `publish`, with the Linux Transporter fetch in `prepare`.
2. Add `generateAppStoreInformation` to the export options.
3. Add `release-stage` and `release-upload --from`, with unit tests over staging and path refusal.
4. Rewire the generator: build jobs stage and lose their store and fastlane steps; the publishing job replaces `publish-release`; the withdraw gate reads `submitted`. Regenerate; `check-generated` proves the committed workflows match.
5. Dispatch a build-only nightly and confirm every build uploads a flat, renamed artifact, the iOS one carries `AppStoreInfo.plist`, and nothing publishes.
6. Merge; the first scheduled nightly is the live publish check.

Rollback: steps 1–4 are confined to `xtask`, `xfastlane`, and one plist; reverting and regenerating restores the per-build publishing of the archived change.
