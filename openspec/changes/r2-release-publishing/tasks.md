## 1. Required targets and the counted publish gate

- [x] 1.1 Add the architecture dimension and published extension to the platform type in `workflows/_types.rs` — macOS arm64/`dmg`, Linux amd64/`AppImage`, Windows amd64/`exe`, plus Android and iOS — and verify `cargo test -p xtask` covers that each platform reports exactly one published target
- [x] 1.2 Declare each channel's required-target set on `Channel` and verify a unit test asserts all three channels require the full five-target set
- [x] 1.3 Have each `build` job record its target on success, and replace `publish_release`'s `!failure() && !cancelled()` condition with an assertion that the recorded set equals the channel's required set; verify a unit test over the generated job shows the gate reads the recorded set
- [x] 1.4 Add a generator test forbidding a bare `!failure()` (or `!cancelled()` alone) as a publishing job's gate in any generated workflow, and verify it fails when the old condition is restored
- [x] 1.5 Make a non-empty `tag` combined with a `platforms` subset fail early in the run, and verify a unit test asserts the guard step precedes every build job
- [x] 1.6 Regenerate with `cargo xtask workflows` and verify `just check-generated` reports no diff

## 2. Store credential and artifact upload

- [x] 2.1 Add a `secretspec.toml` scope carrying the `releases` bucket's S3 endpoint and the long-lived parent key pair, and verify `secretspec check --explain --profile release --scope <scope>` names exactly those keys without printing values
- [ ] 2.2 Add a shared xtask constructor (`create_r2_client(channel)` or similar) that resolves the parent through secretspec's Rust API, derives a session credential by local JWT signing — HS256 under the parent secret, temporary secret as the SHA-256 hex digest of the signed JWT, session token as `base64("jwt/" + jwt)` — scoped to one channel prefix with `object-read-write` and a configurable lifetime defaulting to 3 hours, drops the parent, and returns an `opendal` S3 `Operator` built from the derived triple with `disable_config_load`, `disable_ec2_metadata`, region `auto`, and `enable_exact_buf_write` set; add `opendal` to `tooling/xtask/Cargo.toml` with default features off and `services-s3`, `layers-retry`, `http-transport-reqwest-rustls-no-provider`, installing ring's provider at startup; verify unit tests cover the derivation shape, the prefix scope, the lifetime default, and that both disable flags are set
- [ ] 2.3 Verify against the live bucket that a derived credential can write under its own channel prefix, cannot write under another channel's, is refused once expired, and that `if_not_exists` is refused on an existing key; re-run this after any `opendal` bump
- [ ] 2.4 Add the upload subcommand that writes a built artifact to `{channel}/{series}/{os}/{arch}/waypoints-{version}.{ext}` through `write_with(..).if_not_exists(true)` on the client from 2.2 — channel derived from the version, never supplied — and verify a unit test asserts the parent is dropped before the client exists and that nothing is exported to `$GITHUB_ENV`
- [x] 2.5 Remove the GitHub-release asset upload from the `build` job so the record carries no bytes, and verify the generated build job contains no asset-upload step
- [ ] 2.6 Add a generator test asserting that every store-touching step is a one-line `xtask` dispatch resolving the parent scope and nothing else, that no step names a transfer tool, and that all of them sit in a job declaring the release environment
- [ ] 2.7 Regenerate and verify `just check-generated` is clean, then confirm with a build-only dispatch (empty `tag`) that neither a credential is minted nor an upload runs

## 3. Manifest commit point and withdrawal

- [ ] 3.1 Add the manifest-write step emitting `{channel}.json` with the channel's version and its target entries, ordered as the publishing run's final write, and verify a unit test asserts it depends on every build job
- [ ] 3.2 Record in `nightly_plan.rs` that this run minted the tag, and verify `cargo test -p xtask` covers the flag being set only on the mint path
- [ ] 3.3 Add the withdraw job — discard the drafted record always, best-effort delete the objects this run uploaded through `delete_iter` on the client from 2.2, and delete the tag only when this run minted it — gated on failure or shortfall; verify unit tests assert nightly's withdraw deletes the tag and preview's and stable's do not, and that a failed object delete does not fail the job
- [ ] 3.4 Regenerate and verify `just check-generated` is clean
- [ ] 3.5 Exercise both paths with a nightly dispatch: one complete run leaves a manifest, a published record, and a surviving tag; one run with a platform forced to fail leaves no manifest, no published record, and no tag — verify by reading the bucket and `gh release list`

## 4. Site reads the store

- [ ] 4.1 Grant the Worker a read-only `releases` bucket binding in the site's deploy config, and verify a deployed request for a published artifact streams bytes while the bucket remains anonymously unreadable
- [ ] 4.2 Rework the key and URL helpers to `{channel}/{series}/{os}/{arch}/waypoints-{version}.{ext}`, deriving the channel from the version at write time and recomputing it at read time, and verify unit tests cover a key/URL round-trip plus rejection of a channel/version mismatch
- [ ] 4.3 Replace `useReleases`' `ungh.cc` fetch with enumeration from the store plus notes read server-side from the forge, and verify the download and releases pages render against the private repository
- [ ] 4.4 Narrow the download page's advertised targets to those with a download path (macOS arm64, Linux amd64, Windows amd64) and verify no rendered link addresses an unadvertised target while mobile artifacts are still archived
- [ ] 4.5 Verify a release whose notes cannot be read still lists with its version and downloads

## 5. Versionless links and a cacheable page

- [ ] 5.1 Add the `latest` alias route that resolves the channel manifest and redirects to the versioned URL, refusing to serve bytes itself; verify the redirect target, that the alias is not cached as immutable, and that a channel with no manifest reports a missing release
- [ ] 5.2 Verify a download taken through the alias saves under the resolved version's name rather than the alias's
- [ ] 5.3 Point the download page's links at the alias and move the displayed version and date to a client-side manifest fetch, and verify the rendered markup contains no version string
- [ ] 5.4 Confirm the download page is prerendered and its markup is unchanged by a publish, and that an unresolvable displayed version still leaves the links working
- [ ] 5.5 Run `just website::typecheck` and the per-commit hooks, and verify clippy, cargo-test and treefmt are green across the change

## 6. Retention rules

Independent of the pipeline groups: these change what an object costs, never whether it resolves.

- [x] 6.1 Add a `cloudflare_r2_bucket_lifecycle` to `tooling/infra/r2-releases.tf` with one storage-class transition per channel prefix — nightly 30 days, preview 90, stable 270, computed from days because the provider's `max_age` is in seconds — verified by `tofu fmt -check` and `tofu validate`
- [ ] 6.2 Apply it and verify the three rules are live on the bucket, noting that this resource does not support `terraform import` so it must be created rather than adopted
- [ ] 6.3 Confirm against R2's documentation whether an Infrequent Access object is served without a restore step, and record the answer in the design's retention decision
