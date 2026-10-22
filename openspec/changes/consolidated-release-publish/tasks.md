## 1. Split `xfastlane` into `prepare` and `publish`

- [x] 1.1 Add `xfastlane prepare [flavor] [locale]` that renders the store assets and writes the listing copy and release notes for both stores (what `publish` does before the lane today), failing on an over-long field; verify running it locally produces the same `metadata/` and `screenshots/` trees `publish` produced before (it calls the same `build` and `metadata` the old `publish` did; a local run rendered 9 assets and wrote 8 listing files)
- [x] 1.2 On Linux only, have `prepare` fetch Transporter with `curl --fail --proto '=https'` from Apple's URL, unpack it with Makeself's `--noexec --target fastlane/.transporter` (no `--nocheck`), and print the unpacked build's version; gitignore the directory; mark the fetch with a one-line comment naming the `toolchain/tool-projection` requirement it is an exception under; verify on a Linux machine that a corrupted download fails the unpack and an intact one leaves `itms/bin/iTMSTransporter` runnable from that directory (fetch and unpack exercised on macOS — both are POSIX sh and curl, not Linux-specific: an intact download unpacked with Makeself's SHA-256 and MD5 checks passing, a one-byte corruption failed on SHA-256 with exit 2; the build fetched on 2026-10-21 was 4.3.0.4, up from 4.2.0.16 the same morning. Running the x86_64 JRE needs an x86_64 Linux host, so `iTMSTransporter` itself first runs on the first publish)
- [x] 1.3 Reduce `xfastlane publish <platform> [flavor]` to running the lane, refusing to run when `prepare` has not, and on Linux setting `FASTLANE_ITUNES_TRANSPORTER_USE_SHELL_SCRIPT=true` and `FASTLANE_ITUNES_TRANSPORTER_PATH` to the prepared tree for `ios`; verify `publish` without `prepare` fails with a message naming `prepare`, and that a local macOS `prepare` then `publish` still submits as before (refusal verified, both with no marker and with a marker for another flavor; a local submission was not run, since it would be a real store upload — on macOS `publish` runs the same lane invocation as before, with no Transporter variables)

## 2. The iOS export writes its App Store description

- [x] 2.1 Add `generateAppStoreInformation = true` to `src-tauri/gen/apple/ExportOptions.plist`; verify a local `cargo tauri ios build` (or the next iOS build job) writes `AppStoreInfo.plist` beside the ipa (key added; a local build was not run, since it needs the distribution signing identity — the check is the iOS artifact in 7.2)

## 3. Staging and upload from a directory

- [ ] 3.1 Add `cargo xtask release-stage --platform <p> --tag <t>` that copies the platform's published and updater artifacts into `release/` under their final store keys, deriving the channel from the version as `release-upload` does, and for iOS also stages `AppStoreInfo.plist` beside the ipa outside `release/`; verify unit tests cover every platform's staged paths (including Android's two installers, macOS's separate updater payload, and the iOS plist) and that staging resolves no credential
- [ ] 3.2 Change `release-upload` to `--from <dir>`, walking the directory and writing every file with `if_not_exists` through one `create_r2_client`; verify unit tests refuse a path that is not a key the scheme would produce for the tag (wrong channel, wrong series, foreign file) before any client is built
- [ ] 3.3 Remove the per-platform glob path from `release-upload` once nothing calls it, and verify `cargo test -p xtask` and clippy are green

## 4. Build jobs stop publishing

- [ ] 4.1 Replace each build job's "Upload to the release store" step with `release-stage` plus an artifact upload named for the platform, gated on the tag as before; verify a generator test asserts no build job resolves the store scope or runs `release-upload`
- [ ] 4.2 Remove the fastlane step from the Android and iOS build jobs; verify a generator test asserts no build job resolves a `release` platform scope for submission
- [ ] 4.3 Keep `record-target` on every build; verify the existing completeness tests still pass unchanged

## 5. The publishing job

- [ ] 5.1 Add a `publish` job on Linux, in the release environment, needing every build and `create-release`, gated on the counted completeness and the tag, selecting the lane that carries fastlane, typst, ImageMagick, and git-cliff; verify a generator test asserts the gate counts every required target and is not a bare `!failure()`
- [ ] 5.2 Give it, in order: download-and-merge the staged artifacts and the iOS upload metadata, `xfastlane prepare`, `release-upload --from release/`, publish the draft, mark `submitted=true` as a job output, `xfastlane publish android`, `xfastlane publish ios`, `release-manifest`; check out with full history and tags, since `prepare` reads the release notes from git-cliff; verify a generator test asserts that exact step order, that `prepare` resolves no credential, and that no step outside `xfastlane` names a fastlane variable
- [ ] 5.3 Scope each resolving step to its own credential (store on upload and manifest, `release`/`android` and `release`/`ios` on the submissions, `GITHUB_TOKEN` on the draft publish); verify a generator test asserts no step resolves another step's scope
- [ ] 5.4 Remove the `publish-release` job it replaces; verify `release.rs` tests assert the manifest write lives in `publish` and depends on every build

## 6. Withdrawal and the point of no return

- [ ] 6.1 Gate `withdraw-release` on a tag being present and either a shortfall or a failed `publish`, and on `needs.publish.outputs.submitted != 'true'`; verify generator tests assert a failure past the mark does not withdraw and a shortfall still does
- [ ] 6.2 Keep nightly-only tag deletion and best-effort object deletion unchanged; verify the existing withdraw tests pass unchanged

## 7. Regenerate and verify

- [ ] 7.1 Regenerate with `cargo xtask workflows` and verify `just check-generated` is clean
- [ ] 7.2 Dispatch a build-only nightly and verify every build uploads its staged artifact, the iOS artifact carries `AppStoreInfo.plist` beside the ipa, `publish` and `withdraw-release` are skipped, and nothing reaches the store, the app stores, or the forge
- [ ] 7.3 After merge, verify the first scheduled nightly publishes end to end — staged artifacts in the store, a published record, both store submissions (the iOS one through Transporter on Linux, with `prepare`'s log naming the Transporter build used), `nightly.json` written last, and the tag kept. This is the first point at which a publishing run can be exercised: a dispatched nightly is always build-only. The withdraw path is covered by the generator tests in 6.1 until a real failure exercises it
