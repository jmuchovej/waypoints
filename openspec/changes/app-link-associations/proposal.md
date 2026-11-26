## Why

No shipped mobile build verifies a link today, and none can until three gaps close. The website's two association files disagree with the apps and with each other. The Apple file names `so.waypoints`, `so.waypoints.preview` and `so.waypoints.dev`, so it leaves out `so.waypoints.nightly`, which ships, and includes `.dev`, which does not. The Android file names only `so.waypoints`, and its fingerprint list is empty. Then there is the host. `waypoints.so` sits behind Cloudflare Access, so every request for `/.well-known/*` gets a 302 to a login page. Apple's association CDN reports `Not Found` for the domain, and Google's Digital Asset Links API returns `ERROR_CODE_REDIRECT`. All of this has to be fixed before any store submission, because a submitted build's link claims can only change through another release.

Pairing is the only link today, and it carries a payload "as sensitive as your recovery phrase". When stable, preview and nightly are installed side by side, the OS has no rule for which one opens it. On Android, any package an association file names for a host can claim every path on that host. So the routing rule is also a trust rule.

## What Changes

- **Each shipped channel gets its own link host.** Stable keeps `waypoints.so`. Preview answers on `preview.waypoints.so` and nightly on `nightly.waypoints.so`. Each host's association files name only that channel's app. A pairing link therefore opens the channel that created it, or else that channel's landing page. It never opens a sibling channel.
- **One table on the website declares every app link.** Each entry pairs a path with its state, and each channel maps to a host, an Apple app ID and an Android package. Both association files are derived from this table. Adding a path takes one table entry. What that costs on each platform is recorded in the design.
- **Paths can be reserved.** A reserved path is held in the table but is not associated and is not served as an ordinary page. A later flow can then claim it without taking over a web page. Two such paths are reserved, and nothing is implemented behind them.
- **Android fingerprints are configured per channel.** Each package's statement carries the certificate fingerprints that installed builds of that package may be signed with. They are set as deploy-time runtime config, one value per channel, and a malformed value is never served. A channel with no fingerprints configured publishes no statement, rather than an empty one.
- **Development and debug builds are not associated.** `so.waypoints.dev` comes out of the Apple file. No `.debug` package and no debug-keystore fingerprint is ever named on any host.
- **The apps all declare one shared link set.** The deep-link configuration in `Tauri.toml` lists every channel host with the table's paths, and no flavor overrides it. The association files alone decide which app a host opens. The plugin rewrites the tracked `AndroidManifest.xml` and iOS entitlements on every build, and with a shared set those files are identical for every flavor. Web links are `https` only.
- **The app writes links on its own channel's host.** The pairing link a build emits uses its channel's host. Classifying a link accepts any channel's host, so a pasted link from another channel still routes to the setup flow. A development build emits the `waypoints://` form, which no association can open.
- **Association files are reachable without Access on every link host.** They are served directly, with no redirect, as `application/json`. Whether the rest of each host is gated is a separate policy.
- **The landing page knows which host it is on.** `/pair` on a channel host offers that channel's download and copies a link on that host. On a channel host, any path that is not a link or association path redirects to the apex.
- **A website test checks both files against Apple's and Google's documented schemas.** A second test checks that the app's declared link set matches the table.
- **BREAKING (for any existing preview or nightly pairing link):** preview and nightly builds stop emitting `https://waypoints.so/pair#…`. A link already shared from those builds opens the stable association, as it does today.

## Capabilities

This change lands in **`distribution`**, under a new area. Link association sits next to release delivery: the same site serves it, and it decides what an installed build does. It is not a sync concern. The `sync` domain specifies no pairing-link format, so nothing there changes.

### New Capabilities

- `distribution/app-links`: which hosts and paths open which installed app. Covers the per-channel host map, the single link table and its reserved paths, and what each host's association files may name. It also covers which builds are never associated, the per-channel fingerprint configuration, the rule that association files are served directly without a redirect or an access gate, the shared app-side declaration, the host a build writes into its links, and how the landing page behaves on a channel host.

### Modified Capabilities

None. `distribution/release-delivery` is untouched. The landing page links to its download surface but does not change it.

## Non-goals

- **Account features.** The reserved paths hold their names and nothing more. No binding or callback flow is designed, routed or handled here.
- **New deep-link flows.** Pairing stays the only associated path. This change does not change how the app handles a link once it has one.
- **Store listing work.** No Play Console or App Store Connect metadata, no track promotion and no submission. The Play app-signing fingerprint gets set when it exists. Creating it is not part of this change.
- **macOS Universal Links.** The desktop app registers only the `waypoints://` scheme. It has no associated-domains entitlement, and the deep-link plugin's desktop path handles schemes only. The Apple file's app IDs would already cover a Mac build with the same bundle identifier. Nothing here enables one.
- **Disambiguating the desktop scheme.** Every desktop flavor registers `waypoints://`, and which one the OS picks stays as it is.
- **A custom scheme on mobile.** No mobile build registers `waypoints://` (the plugin removes `CFBundleURLTypes` on iOS), so the landing page's "Open in Waypoints" button does nothing on a phone. That gets recorded here and stays unfixed.
- **Opening the site to the public.** Only the association files are exempted from Access.

## Impact

- `website/server/routes/.well-known/apple-app-site-association.get.ts`, `assetlinks.json.get.ts`: become thin handlers over pure builders keyed by the request host.
- `website/shared/utils/app-links.ts` (new) and `app-links.test.ts` (new): the link table, the channel map, the builders, schema-shape tests, and the consistency test against `src-tauri/Tauri.toml`.
- `website/nuxt.config.ts`: `runtimeConfig.appAssociation.android` becomes one fingerprint value per channel (`NUXT_APP_ASSOCIATION_ANDROID_{STABLE,PREVIEW,NIGHTLY}`), replacing `androidCertFingerprints`.
- `website/server/middleware/` (new): sends non-link paths on a channel host to the apex.
- `website/app/pages/pair.vue`: derives its channel and copy link from the host it is served on.
- `website/package.json`: `confbox` as a dev dependency (already in the lockfile transitively) for the consistency test.
- `src-tauri/Tauri.toml`: `[[plugins.deep-link.mobile]]` lists the three channel hosts, `https` only.
- `src-tauri/gen/android/app/src/main/AndroidManifest.xml`, `src-tauri/gen/apple/Waypoints_iOS/Waypoints_iOS.entitlements`: committed in the state the plugin writes, so a build leaves them clean.
- `crates/cartograph/src/sync/setup_code.rs`, `service.rs`: the pairing link takes its host from the build's channel, and classification accepts every channel host.
- `src-tauri/src/`: passes the build's channel (already stamped as `WAYPOINTS_CHANNEL`) into the sync service.
- Operations, outside the repo: DNS and Worker custom domains for `preview.waypoints.so` and `nightly.waypoints.so`; an Access bypass for `/.well-known/apple-app-site-association` and `/.well-known/assetlinks.json` on all three hosts; the Associated Domains capability on each shipped App ID, with profiles regenerated and resynced through `tooling/bin/sync-mobileprovision`; per-channel fingerprint values set on the Worker.
