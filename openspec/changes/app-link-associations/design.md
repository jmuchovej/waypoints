## Context

See proposal.md (Why). These are the mechanics that constrain the approach. All were read from the tree or observed on 2026-10-26.

**Identifiers per flavor.** The flavor configs in `src-tauri/res/configs/flavors/*.toml` set `identifier`: `so.waypoints` (stable), `so.waypoints.preview`, `so.waypoints.nightly` and `so.waypoints.dev` (development). iOS takes its bundle identifier from that config. The App Store profiles in `secretspec.toml`, the `tooling/bin/sync-mobileprovision` map and the fastlane `Appfile` lanes all cover only stable, preview and nightly, so those three are the shipped set.

Android works differently. `xtauri build android` pins Tauri's identifier back to `so.waypoints`, and `build.gradle.kts` appends `.dev`/`.preview`/`.nightly` only when `WAYPOINTS_BUILD=1`, which only `xtauri build` sets. The `debug` build type adds `.debug` on top. A `just tauri::dev android` build is therefore `so.waypoints.debug` whatever flavor is named. A debug build of a release flavor is `so.waypoints.<flavor>.debug`. Debug builds sign with the machine's debug keystore. Release builds of every flavor sign with the one upload keystore (`ANDROID_KEYSTORE`). Once a package enrolls in Play App Signing, store-installed builds carry Google's per-package key instead.

**The deep-link plugin writes tracked files.** At build time, `tauri-plugin-deep-link`'s build script reads the merged Tauri config, including any `--config` flavor fragment, through `TAURI_DEEP_LINK_PLUGIN_CONFIG`. It rewrites the `DEEP LINK PLUGIN` block of `src-tauri/gen/android/app/src/main/AndroidManifest.xml` and the `com.apple.developer.associated-domains` key of `src-tauri/gen/apple/Waypoints_iOS/Waypoints_iOS.entitlements` as `applinks:<host>` for each mobile entry. Both files are tracked. The committed entitlements hold a `*` placeholder string, not the array a build writes. When no mobile entry is a custom scheme, the plugin removes `CFBundleURLTypes` on iOS, so no mobile build registers `waypoints://`. `[plugins.deep-link.desktop]` registers `waypoints://` for every desktop flavor, and nothing associates a desktop build with a host.

**Current declaration.** `Tauri.toml` has one mobile entry: host `waypoints.so`, `pathPrefix = ["/pair"]`, with default schemes `https` and `http`. A prefix match on Android also takes `/pairing` or `/pair/x`. The Apple file's `{"/": "/pair"}` matches exactly.

**The pairing link.** `crates/cartograph/src/sync/setup_code.rs` hard-codes `https://waypoints.so/pair` for emission and accepts only that host or `waypoints://pair` when classifying. `src-tauri/build.rs` already stamps `WAYPOINTS_CHANNEL`, which defaults to `development` when no flavor is named.

**The live hosts.** `waypoints.so` and `dev.waypoints.so` are behind Cloudflare Access: every path, including both well-known files, answers 302 to a Cloudflare Access login. `https://app-site-association.cdn-apple.com/a/v1/waypoints.so` returns `Not Found`. Google's `statements:list` for `https://waypoints.so` returns `ERROR_CODE_REDIRECT`. `preview.waypoints.so` and `nightly.waypoints.so` do not resolve. `dev.waypoints.so` resolves and is in use for something this change does not own. Neither `website/nuxt.config.ts` nor `tooling/infra` declares the Worker's hostnames or the Access policy, so both live in the Cloudflare dashboard.

**Platform floor.** Android `minSdk` is 35. Since API 31, App Links verification is per host, so one host failing verification never revokes another. iOS deployment target is 17 or later (`project.yml` says 17.0, `Tauri.toml` 18.0), so only the `components` form of the Apple file is needed and the legacy `paths` form is not.

## Goals / Non-Goals

**Goals:**

- Make the association files the only thing that varies by channel, so no flavor build carries per-channel link configuration.
- Make "add a link path" a one-entry change on the website, and make it visible, at review time, what that entry costs on each platform.
- Make each claim of trust over a link checkable without an install: the files are fetchable and checked against the platforms' schemas, and the app's declaration is checked against the table, all in tests.

**Non-Goals:**

- A shared source file that generates both the website table and the Tauri config. The two are kept in agreement by a test rather than generated from one file (see the declaration decision).
- Any change to how the app handles a link once received. `src/app/plugins/deep-link.client.ts` keeps classifying and dropping unknown links.

## Decisions

### Every shipped channel is associated; development and debug are not (2026-10-26)

The shipped set is stable, preview and nightly: those are the identifiers with store profiles and fastlane lanes. Each of them is associated. The Apple file drops `so.waypoints.dev`. The Android file grows from stable only to all three, each on its own host (next decision).

Development and debug builds are never named. On Android the reason is integrity. An association statement grants a package plus a certificate the right to handle a host. A debug keystore is unprotected by construction: it is generated per machine, with a well-known password, and is never treated as secret. Publishing its fingerprint would let anyone holding that file ship an app that verifies for the host and receives pairing payloads. The `.debug` suffix already keeps every debug build distinct from every release package, so leaving debug packages out costs nothing. On iOS, the app identifier is bound to the team by Apple's provisioning. A development-signed build of a shipped flavor carries that flavor's bundle identifier and will associate, which is acceptable because only the team can produce it. A development-flavor build (`so.waypoints.dev`) is simply not named.

Rejected: a development host such as `dev.waypoints.so` naming `so.waypoints.dev` and the debug fingerprints. That host is already in use for something else, and on Android it would publish an unprotected key for a host whose links carry the same kind of payload. Adding one later is additive: one channel-map row, one host entry and one bypass. Also rejected: listing debug fingerprints on the production hosts "for testing". That trades the pairing payload's integrity for a convenience that a nightly build provides anyway.

### A link opens the app of the channel whose host it is on (2026-10-26)

Three ways for side-by-side installs to resolve a link were weighed.

|                                    | Per-channel host                                                                     | Per-channel path on one host                                                           | OS behaviour on one path                                                |
| ---------------------------------- | ------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Link shape                         | `preview.waypoints.so/pair#…`                                                        | `waypoints.so/preview/pair#…`                                                          | `waypoints.so/pair#…` for all                                           |
| iOS routing                        | each host's file names one app ID                                                    | per-app `components` in one file                                                       | first matching `details` entry wins, so stable whenever it is installed |
| Android routing                    | each host's statement names one package                                              | every package verified for the host, routed by its own intent-filter paths             | every package verified for the same URL, so the user gets a chooser     |
| Android isolation between channels | yes: a package is trusted only for its own host                                      | **none**: verification grants the whole host, and a package may declare any path on it | none                                                                    |
| Website cost                       | two subdomains: DNS, Worker custom domains, Access bypass, a host-aware landing page | a `/{channel}/…` route per link path                                                   | none                                                                    |
| Adding a path                      | one table entry, same path on every host                                             | one table entry, prefixed per channel                                                  | one table entry                                                         |
| Prefix hazard                      | none                                                                                 | `/pair` as an Android `pathPrefix` would also take `/pair/preview`                     | none                                                                    |

Per-channel host is the recommendation. The deciding row is Android isolation. Digital Asset Links verifies a host, not a path. A package listed for `waypoints.so` is a verified handler for any path its own manifest declares there. On one shared host, a compromised nightly signing path (a CI secret, a leaked upload key) would yield an app that verifies for stable's `/pair` links, and the payload those links carry is as sensitive as a recovery phrase. Per-path routing looks isolated on iOS, where the server lists paths per app, but it is not isolated on Android. Accepting OS behaviour gives the two platforms opposite answers (iOS silently picks stable, Android asks) and has no way to target the channel that issued the link. That matters because a link is generated by a device of a particular channel for a device of the same channel.

Per-channel host also gives each channel its own failure domain. A broken preview statement cannot affect stable verification, which is per host from API 31. The cost is operational and paid once: two hostnames bound to the same Worker, plus the bypass. The stable host and its existing link shape are unchanged.

Rejected: per-channel path, for the isolation gap above. Rejected: OS behaviour, for the inconsistency and the lack of targeting. Also rejected: per-channel registrable domains (such as `waypoints-preview.so`). They give the same isolation as subdomains, plus extra certificates, registrar records and phishing lookalikes, for no added property.

### Every build declares the same link set; the files decide (2026-10-26)

The app side declares every channel host, each with every associated path, in `Tauri.toml`, and no flavor config overrides `plugins.deep-link`. Each app therefore asks to be verified for all three hosts. Only its own host's file names it, so only its own host verifies. A stable app on a device that also has preview attempts `preview.waypoints.so`, fails that verification, and stays the browser's. This is the intended outcome. It shows in `pm get-app-links` as a non-verified host, and the verification tasks spell out what to expect.

The alternative is a per-flavor `[plugins.deep-link]` in each flavor config, naming only that flavor's host. It is cleaner in what each app claims, but the plugin writes its output into two tracked files. Every build of a flavor other than the last committed one would rewrite `AndroidManifest.xml` and the entitlements, and jj would snapshot that churn into whatever change is open. A shared declaration makes the plugin's output the same for every flavor. The committed files can hold exactly what a build writes, and a build leaves the tree clean. Rejected for that reason. Also rejected: a Gradle `manifestPlaceholders` host (`android:host="${linkHost}"` written through the plugin config). It works on Android, but the same string reaches the iOS entitlements as a literal `applinks:${linkHost}`.

Web links are declared `https` only. The plugin's default adds `http`, which claims a scheme no link should use and that Apple ignores anyway. Paths are declared with `path` (exact), not `pathPrefix`, so Android matches what the Apple file's `{"/": "/pair"}` matches.

### One table on the website; the platforms pay differently per entry (2026-10-26)

`website/shared/utils/app-links.ts` holds two declarations. The channel map gives each channel its host, bundle identifier and package. The link table gives each path its state, `associated` or `reserved`. Both route handlers are thin wrappers over pure builders, `appleAssociation(host, teamId)` and `androidAssociation(host, fingerprints)`, that read only these declarations. A request's host selects one channel, and a host outside the map yields an empty association: `{"applinks":{"details":[]}}` and `[]`.

What one entry costs:

| Change                                           | Website                                                | iOS                                                                                                               | Android                                                                                                                                                                            | Desktop               |
| ------------------------------------------------ | ------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------- |
| Reserve a path                                   | one table entry (`reserved`)                           | nothing                                                                                                           | nothing                                                                                                                                                                            | nothing               |
| Associate a path (new, or reserved → associated) | one table entry                                        | no app config change; installed apps pick it up when Apple's CDN next refreshes the file; the app needs a handler | add the path to every host entry in `Tauri.toml` (the consistency test fails until it is there); takes effect only in builds shipped after; older installs leave it to the browser | nothing (scheme only) |
| Add a channel host                               | one channel-map row; DNS, custom domain, Access bypass | a `Tauri.toml` host entry changes the entitlement, so a new build is needed (the capability is already enabled)   | a `Tauri.toml` host entry, then a new build                                                                                                                                        | nothing               |

Order matters when associating a path. iOS starts routing a path as soon as the CDN refreshes, regardless of app version, so associate it only after a release that handles it has shipped. Until then, an installed app would receive a link it drops. Android cannot have this problem, because the path ships in the same build as its handler.

The table and `Tauri.toml` are two places that must agree. A website test parses `Tauri.toml` (with `confbox`, already in the lockfile through `c12`) and asserts that the declared hosts and paths equal the channel map's hosts crossed with the associated paths. Rejected: generating the Tauri deep-link config from the website table through `xtauri --config`. A plain `cargo tauri android build` would then silently ship no association, and the tracked files the plugin writes would depend on which wrapper ran.

### Two paths are reserved; matching is exact (2026-10-26)

`/pair` is associated. `/bind` and `/callback` are reserved: names held for a later flow that binds an account to an identity, and for a later authentication callback. Nothing routes, handles or associates them here. Reserved paths answer 404 ahead of the site's catch-all routes. Without that, `website/app/pages/[...slug].vue` would render whatever content page matched, and once a path renders a page, associating it later takes that page away from browser users. The names avoid `/auth/*` because `website/app/pages/auth/[...slug].vue` already renders a sign-in page for every path under it. They avoid `/api/*` because better-auth owns `/api/auth/*`.

Rejected: reserving by associating now. An associated path with no handler opens the app to do nothing, and on iOS that happens on every device as soon as the CDN refreshes.

### Fingerprints are per channel, deploy-time, and validated (2026-10-26)

`runtimeConfig.appAssociation.android` becomes `{ stable, preview, nightly }`, set as `NUXT_APP_ASSOCIATION_ANDROID_STABLE` and its siblings, each a comma-separated list. This replaces the single `androidCertFingerprints`. Each list holds the SHA-256 of the upload certificate, which signs the archived release artifact and any sideloaded build. Once the package enrolls in Play App Signing, the list also holds Play's app-signing certificate. Values are normalized to upper case and kept only if they match `^([0-9A-F]{2}:){31}[0-9A-F]{2}$`, and a dropped value is logged by its channel. A channel whose list ends up empty publishes no statement.

These values are public, since they appear in the served file. They stay deploy-time config, consistent with the existing approach, because rotation and Play enrollment happen outside the repository and should not need a commit. The Apple team identifier keeps its committed default in `runtimeConfig`.

Rejected: one fingerprint list shared by all packages. All three channels use the same upload key today, but Play App Signing gives each package its own key. A shared list would publish every channel's certificate for every package and would hide the case where only one channel is misconfigured. Rejected: committing fingerprints. Nothing secret is gained, and rotation would need a commit.

### Association files bypass Access on every link host (2026-10-26)

Apple's CDN and Google's verifier both refuse redirects. As long as Access fronts `/.well-known/*`, no association can verify, whatever the files contain. A Cloudflare Access bypass application covers exactly `/.well-known/apple-app-site-association` and `/.well-known/assetlinks.json` on `waypoints.so`, `preview.waypoints.so` and `nightly.waypoints.so`. Everything else on those hosts keeps whatever gating the site has. Universal Links and App Links open the app without fetching the landing page, so the gated landing page only matters to someone without the app, and serving that person is a launch question, not this change's.

The well-known routes also take `prerender: false` in `routeRules`. Their output depends on the request host, and a prerendered copy would freeze one host's answer for all of them.

Rejected: lifting Access site-wide, which is a launch decision rather than an association one. Rejected: serving the files as static assets, which sit behind the same gate and lose host awareness.

### A build writes its own channel's host; development writes the scheme (2026-10-26)

`encode_pair_url` takes a link base instead of a constant. `src-tauri` resolves the base from the stamped `WAYPOINTS_CHANNEL` and passes it to the sync service when it is constructed. Stable, preview and nightly map to their hosts. Development, including any build with no flavor named, maps to `waypoints://pair`. The xtask dev server passes the development base. Classification accepts `/pair#…` on any channel host and on the scheme, and nothing else. A cartograph test asserts that its host list equals the hosts declared in `src-tauri/Tauri.toml` (read with the workspace `toml` crate as a dev dependency), which closes the third place a host is named.

Rejected: development keeps emitting `waypoints.so`. A development pairing link would then open a shipped stable install's setup flow with a development payload. Rejected: classifying any `*.waypoints.so` host. That would accept `dev.waypoints.so` and any future host whose purpose is unrelated to links.

### Channel hosts serve only what links need (2026-10-26)

A server middleware runs on `preview.waypoints.so` and `nightly.waypoints.so`. Associated paths and the well-known files pass through. Reserved paths get their 404. Everything else gets a temporary redirect (302) to the same path on `waypoints.so`. The redirect is temporary so that a browser never caches a redirect for a path that is later associated. `pair.vue` reads its channel from the host. Its install button goes to `/download/<channel>` on the apex. Its copy button copies a link on `window.location.origin` instead of the hard-coded stable host.

## Risks / Trade-offs

- [Apple's CDN caches the current `Not Found` and refreshes on its own schedule] → After the bypass, poll `app-site-association.cdn-apple.com/a/v1/<host>` before testing on a device. Do not read a stale CDN as a broken file. For immediate iteration, a development-signed build can add `?mode=developer` to its entitlement on a device with developer mode on, which bypasses the CDN.
- [The Access bypass is scoped wider than two paths] → The verification tasks require an unauthenticated `/` on each host to still be gated, alongside the two files answering 200.
- [A plugin upgrade changes what it writes into the tracked manifest or entitlements] → Re-check after any `tauri-plugin-deep-link` bump that a build leaves both files clean.
- [Each app records failed verifications for the two foreign hosts] → Expected. The device task lists the per-package state to expect, so a reviewer does not chase it.
- [Stable store installs do not verify until Play App Signing's certificate is added] → Adding it is a deploy-time value, set once enrollment exists. Until then, the upload certificate covers sideloaded builds.
- [Preview and nightly pairing links already shared on `waypoints.so` keep opening stable] → Accepted and marked BREAKING in the proposal. Neither channel has a public install base.
- [The landing page's "Open in Waypoints" button emits `waypoints://`, which no mobile build registers] → Out of scope, as recorded in the proposal's Non-goals. Inside an installed app the association already opens the app before the page loads.

## Migration Plan

1. Ship the website change. The stable host's Apple file loses `.dev`. Set the new per-channel fingerprint variables before the deploy, and remove `NUXT_APP_ASSOCIATION_ANDROID_CERT_FINGERPRINTS` after it. Nothing verifies today, so no working association is lost.
2. Bind `preview.waypoints.so` and `nightly.waypoints.so` to the Worker, then add the Access bypass on all three hosts. Confirm with the CDN and the Digital Asset Links API.
3. Enable Associated Domains on the three App IDs, regenerate their App Store profiles, and resync them.
4. Land the app-side declaration and the cartograph link base. The next nightly carries them. Verify on devices.

Rollback: reverting the website restores the previous files. Builds already shipped declare hosts whose files decide everything, so a reverted file only removes associations and never misroutes a link.

## Open Questions

- Are `/bind` and `/callback` the names John wants held? Renaming a reserved path before it is associated changes one table entry.
- Should the stable host's statement list only the upload certificate until Play enrollment, or should it wait and publish nothing? The spec allows either. This design publishes the upload certificate.
