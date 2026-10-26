## Why

Every release build job currently publishes on its own: each derives a store credential and uploads, and the two mobile jobs submit to their app stores, all before anything has confirmed that every other target built. A run that falls short therefore has already reached the store and, worse, may already have handed a build to Play or the App Store — the one write nothing can take back. Moving all publishing into a single job that runs only after every target has built puts every irreversible act behind the completeness check instead of beside it.

## What Changes

- Build jobs stop publishing. Each uploads its release assets as a workflow artifact, already named as they will be stored, and holds no store or app-store credential.
- A single publishing job, gated on every required target having built and on the run carrying a tag, performs every publishing act as its own step, ordered from most to least retractable: upload to the artifact store, publish the release record, submit to the app stores, and write the channel's manifest last.
- Each step resolves only the credential it needs, immediately before it needs it; no step holds another's.
- Store submission is specified for the first time: it happens once per release, after the artifacts are in the store and the record is published, and never on a run that fell short.
- Withdrawal distinguishes the point of no return. A failure before store submission withdraws as today; a failure at or after it does not withdraw, and leaves the manifest write to be re-run.
- Withdrawal deletes the objects a shortfall uploaded — correcting the main spec's requirement text, which says they are left in place while its own scenario says they are deleted.
- The publishing job runs on Linux. The iOS export writes the App Store description Apple's Linux uploader requires.
- `xfastlane` splits into `prepare` (render assets, write listing copy, and on Linux fetch Apple's Transporter) and `publish` (run the lane), so everything that can fail retractably happens before the point of no return.
- Transporter is fetched at run time rather than projected. The rule for when a tool may be fetched this way is specified once, with conditions every such tool must meet on its own — so this exception cannot be cited for the next.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `distribution/release-publishing`: amends *A channel advertises a release only once every target has succeeded* (publishing moves into one job behind the completeness check), *A shortfall withdraws what the run created, and nothing else* (a point of no return, and the deletion rule made consistent), *The store's write credential is an integrity credential* and *An uploading job holds only a derived, expiring, scoped credential* (one publishing job instead of many uploading ones, step-scoped). Adds *A release reaches the app stores only after it is complete in the store* and *Publishing steps run in order of retractability*.

- `toolchain/tool-projection`: adds *A tool fetched at run time is a justified exception, never a precedent* — the conditions under which a tool may skip the projection, and the rule that each such tool is justified on its own.

`distribution/release-delivery` is unaffected: what the store holds and how the site reads it does not change.

## Non-goals

- Changing the key layout, the manifest's shape, the retention rules, or anything the download surface reads.
- Changing which channels elect what, or the tag rules (a run still deletes only a tag it minted).
- Retrying or resuming a partially published run automatically; a failure past the point of no return is surfaced, not repaired by the pipeline.
- Changing what the app stores receive or the store-listing content.

## Impact

- `tooling/xtask/src/tasks/workflows/{release.rs,_jobs.rs,_steps.rs}` — the build jobs lose their store and fastlane steps and gain an artifact upload; a new publishing job replaces `publish-release`; `withdraw-release` learns the point of no return. The committed `release-{nightly,preview,stable}.yaml` regenerate.
- `tooling/xtask/src/tasks/release_upload.rs` — uploads from a downloaded artifact directory rather than the live bundler output, and covers every platform in one invocation.
- `tooling/xtask/src/tasks/release_withdraw.rs` — unchanged in what it deletes; called from a different gate.
- `src-tauri/gen/apple/ExportOptions.plist` — gains `generateAppStoreInformation`.
- `tooling/bin/xfastlane` — `prepare` and `publish` split; the Linux Transporter fetch and environment live here.
- No change to `website/`, `tooling/infra`, or `secretspec.toml`'s scopes.
