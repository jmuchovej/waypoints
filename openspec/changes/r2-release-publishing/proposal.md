## Why

Nothing has ever been published. `gh release list` is empty, the two tags that exist (`v0.1.0-nightly.1+2026.10.10`, `v0.1.0-nightly.2+2026.10.11`) point at no release, and the `releases` R2 bucket that `tooling/infra` already declares — along with its bucket-scoped read-write token — has nothing writing to it. The pipeline that would fill it builds five platforms and attaches their artifacts to a GitHub release, which the website then tries to discover through `ungh.cc`: an unauthenticated third-party proxy that cannot read a private repository, so the download page resolves to nothing in production regardless of what the builds produce.

Two defects are load-bearing rather than incidental. The publish gate is `!failure() && !cancelled()`, and in GitHub Actions a _skipped_ job is neither a failure nor a cancellation — so a dispatch that selects a platform subset satisfies the gate, flips the draft public, and leaves a tag standing for a release no complete build ever produced. And a run that fails after its tag exists leaves that tag behind permanently: nightly mints its tag through the refs API before any build runs, so every failed nightly burns a version number that nothing can reuse and nothing describes.

This change makes a release an all-or-nothing event: artifacts land in the store, a channel begins advertising them only once every target for that channel has landed, and a run that falls short withdraws what it created.

## What Changes

- **Built artifacts upload to the `releases` bucket, not to the GitHub release.** Each build writes its bundle through the S3 endpoint under the existing bucket-scoped token. The series prefix collapses patch, prerelease and build metadata, so a minor line is one prefix and one lifecycle rule governs it wholesale.
- **The GitHub release becomes the release _record_, and holds no bytes.** It carries the tag and the `git-cliff` notes, drafted before the builds and published after them. Splitting record from bytes is what lets the notes stay where they are while distribution moves.
- **A per-channel manifest is the commit point.** `{channel}.json` names the channel's current version and its platform entries. Artifacts are immutable and may be uploaded in any order; a channel advertises nothing until every target for that channel has uploaded and the manifest is written last. Atomicity therefore needs no deletion — a shortfall simply leaves unreferenced objects, which the withdrawing run removes best-effort and which nothing serves either way.
- **Election is stated per channel.** Nightly publishes only when product work landed since the last one; preview and stable publish the tag that triggered them. A build-only run — an empty `tag` input — elects nothing and uploads nothing.
- **A run that does not produce every target for its channel publishes nothing.** The gate counts _successful_ target uploads against the channel's required set rather than trusting the absence of failure, so a skipped platform blocks the manifest and the record exactly as a failed one does.
- **A run withdraws only what it created.** On failure or shortfall the draft record is discarded, the objects the run uploaded are deleted best-effort, and the tag is deleted only when that run minted it — which is nightly, through the refs API. Preview and stable are triggered _by_ a tag push, so the tag predates the run and is left alone; deleting it could not un-fetch it from anyone who already had it.
- **Every uploading job mints its own expiring, channel-scoped credential.** The vault holds one long-lived parent; `xtask` resolves it through secretspec's Rust API and derives a session credential locally — signing a scoped JWT rather than calling R2's temporary-credentials endpoint, so no service sits between a finished build and its upload. The derived credential is confined to the channel's key prefix, permits writing objects without administering the bucket, and expires after 3 hours by default, configurable through `xtask` because a new dependency can lengthen a compile without warning. It applies to every release job that uploads through the bundler pipeline, on all three channels.
- **Nothing published is ever deleted; cold releases change storage class.** Every version ever published is kept, on every target — including the mobile ones that have no download link — so the store is an archival registry of what was built and released. Space is reclaimed by transitioning an object to Infrequent Access: three lifecycle rules on the bucket, one per channel prefix, at **30 days for nightly, 90 for preview, 270 for stable**, declared in `tooling/infra`. Nothing in the pipeline prunes.
- **A versionless `latest` URL per channel, served as a redirect.** `…/{channel}/latest/{os}/{arch}/waypoints-latest.{ext}` resolves the channel's manifest and redirects to the versioned URL, so a mutable pointer sits in front of an immutable resource and the download page can be prerendered and cached whole. The saved file keeps the resolved version's name.
- **The object key becomes `{channel}/{series}/{os}/{arch}/waypoints-{version}.{ext}`,** an exact mirror of the download URL. The channel leads because retention is per-channel and R2 lifecycle rules filter by prefix only — a series prefix mixes all three channels, so without it "tier nightlies after weeks, stable after years" cannot be expressed. The uploader never accepts a channel as input: it derives one from the version's tag grammar, and the download surface recomputes it and refuses any request whose channel segment disagrees with its version.
- **The website enumerates releases from the store.** Versions and platform availability come from the bucket; `ungh.cc` is dropped. Release notes continue to come from the GitHub releases API, read server-side with the existing token, so a private repository is no obstacle.
- **The target set is the host architecture of each platform.** macOS arm64, Linux amd64, Windows amd64, plus Android and iOS — the five jobs that exist. "Every target" means that set, and the download surface advertises exactly it.
- **Artifact formats are the ones the bundler already produces.** macOS `.dmg`, Linux `.AppImage`, Windows `.exe`. The naming scheme's extension is per-platform rather than a fixed trio.
- **BREAKING (for anything linking a GitHub release asset):** release assets stop being attached to GitHub releases. The download surface is the site's API in front of the private bucket; a GitHub release carries notes only.

## Capabilities

This change lands in **`distribution`** — packaging, installation and update delivery. `distribution` is a tree, so each area takes its own path. It splits along the seam the change itself introduces: one capability for producing a release, one for consuming it.

### New Capabilities

- `distribution/release-publishing`: what a release _is_ and how one comes to exist. Which channels elect to publish and on what evidence; the artifact store's key layout and the immutability of a key; the per-channel manifest as the single commit point, written last; the requirement that every target for a channel succeed before anything is advertised; and what a run withdraws when it falls short — including the rule that it deletes only a tag it minted itself. Also the integrity standing of the store's write credential: whoever holds it can place bytes that users install as a signed release, so it is resolved by the uploading step alone and never enters the job environment.
- `distribution/release-delivery`: how a published release is reached. The store is private and no client touches it directly; downloads pass through an API that derives the object from channel, series, platform and arch, validates every segment, and rejects a request whose parts disagree. Covers the saved-filename rule (friendlier than the version it came from, and never parsed back), what each channel's manifest guarantees to an updater, how the published set is enumerated without a third-party proxy, and that release notes are read from the forge rather than the store.

### Modified Capabilities

None. No published capability's requirements change. The store's write credential is a new consumer of `toolchain/secret-provisioning` rather than an amendment to it — it resolves under the environment its job declares, scoped to the resolving step, which is what that spec already requires of every credential. `distribution/nix-overlay` builds from the repository and is untouched by where release bytes are hosted.

## Non-goals

- **Widening architecture coverage.** macOS Intel, ARM Linux and ARM Windows stay unbuilt; the toolchain declares those targets but no job produces them, and adding jobs is a cost decision of its own. This change fixes the target set so "every target" is well defined, and nothing about it precludes growing that set later.
- **Changing what the bundler emits,** or adding portable archives beside the installers. Linux keeps `.deb` as a build output without becoming a published target.
- **Moving release notes into the store.** They stay in the GitHub release, read with the token the site already holds.
- **Provisioning the bucket or its token.** `tooling/infra` already manages both, and this change adds only the lifecycle rules beside them.
- **Access control over archived releases.** Retrieval cost is bounded by immutable caching — one retrieval per archived object per cache eviction, over a set no requester can enlarge — with a rate limit on the archived path as the next lever. Putting a published release behind a sign-in changes what "released" means and is not spent on a cost caching already bounds.
- **A local filename that keeps the `latest` suffix.** Because the alias redirects to the versioned URL, a fetcher deriving a filename from the URL it was handed receives a versioned name, and two such fetches at different times produce differently-named files. Recorded as a known limitation rather than addressed.
- **Mobile store distribution.** Android artifacts upload to the store like any other, but Play track promotion and an iOS App Store submission are untouched; iOS has no public listing and this change does not give it one.
- **The build cache.** A different bucket and a different credential, with no overlap beyond both being R2.
- **Backfilling.** Tags that predate this pipeline are not published retroactively, and the store starts empty.
- **The branch and version model.** `main` nightly, `v0.2.x` preview, `v0.1.x` stable, and the tag grammars for each, are recorded by this change and not redefined by it.

## Impact

- `tooling/xtask/src/tasks/workflows/_steps.rs` — the upload step (S3 write under the store scope), the manifest-write step, the target-completeness gate, and the withdraw step; `steps::upload_release_assets` loses its GitHub target.
- `tooling/xtask/src/tasks/workflows/_jobs.rs` — `build` gains the upload; `publish_release` gains the completeness gate in place of `!failure()`; a new withdraw job runs on failure or shortfall.
- `tooling/xtask/src/tasks/workflows/release.rs` — wires the withdraw job and the completeness dependency; the `platforms` dispatch input's interaction with a non-empty `tag` becomes an error rather than a partial publish.
- `tooling/xtask/src/tasks/workflows/_types.rs` — the per-channel required-target set, and the per-platform artifact extension for the published name.
- `.github/workflows/release-{nightly,preview,stable}.yaml` — regenerated; `check-generated` proves they match.
- `tooling/xtask/src/tasks/nightly_plan.rs` — the tag it mints becomes withdrawable, which means recording that this run created it.
- `secretspec.toml` — a scope carrying the `releases` bucket's S3 endpoint and the long-lived parent key pair, resolved only by the credential-minting step.
- `tooling/xtask` — a task that resolves the parent through secretspec's Rust API and derives a channel-scoped, expiring S3 session credential by local JWT signing, with the lifetime configurable and defaulting to 3 hours.
- `tooling/infra/r2-releases.tf` — a `cloudflare_r2_bucket_lifecycle` carrying one storage-class transition per channel prefix, computed from a day count because the provider's condition is in seconds.
- `website/` — `useReleases` stops fetching `ungh.cc` and reads the store; the download page narrows to the three host-arch desktop targets, links versionlessly through the `latest` alias, and resolves its displayed version client-side; release notes are fetched server-side from the forge.
- Operations, outside the repo: the service account behind the release environment must read the `r2-releases` vault item, and that item's token must write the `releases` bucket.
