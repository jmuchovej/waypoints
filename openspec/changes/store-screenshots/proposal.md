## Why

Every phone screenshot in the store listing is a drawing of the app, not the app. `src-tauri/fastlane/lib/screen.typ`'s `device()` lays out a rounded slab and `en-US.typ` fills it by hand — `block-row(accent.at(0), "Heading", "structures the page")`, a mocked `Book` Landmark, a mocked Terrain edge list. Both stores require the UI shown to be the UI that ships, and the mock is already a claim nobody verifies: it drifts the moment the editor's chrome changes, and nothing fails when it does.

What is wanted is what `frameit` and its siblings exist for — a realistic picture of the running app. The framing around it is ours either way, and is better done where the rest of the listing's layout already lives.

fastlane ships the capture half: `snapshot` drives XCUITest on the iOS Simulator, `screengrab` drives Espresso on an Android emulator. Neither project is wired for them — `src-tauri/gen/apple/project.yml` declares no UI test target, and `src-tauri/gen/android/app/src/` has flavor source sets but no `androidTest`.

The content a capture shows needs to be deterministic, and nearly all of that already exists: `store/mock` seeds the `getting-started` Bundle through `transload` — the same ingest path an import takes. Today it runs inside a `development` build when the store opens; here the same code produces the territory on the host, and the harness places it on the device before the release build first launches — a returning user's data, already there. The app that is captured is the app that ships.

This change is the controls and the minting process, nothing after it: a way to put the app into whatever state marketing imagery needs — a route, a pose on that route, a corpus underneath — and a command a person runs that boots a device, installs a build, places the corpus, opens the link and writes a capture, which the Typst pipeline composes into the store screenshots under `metadata/<platform>/`. Deciding what that state should be, whether captures are tracked, and whether the pipeline takes them are all left open — see Non-goals.

## What Changes

- **Capture harnesses.** A UI test target in `src-tauri/gen/apple/project.yml` and an `androidTest` source set under `src-tauri/gen/android/app/src/`, each launching the installed release build and capturing.
- **The demo territory is placed, the app is not changed.** `cargo xtask demo-territory` produces the corpus on the host through the same ingest an import takes — the `getting-started` Bundle by default, any Bundle by argument — and the harness copies it into the installed app's data directory before first launch — a returning user whose data is already on the device. No capture build enables `mock`; a release build still seeds nothing. On Android the capture is a release apk, which is universal and carries an x86_64 slice; on iOS it is a simulator build of the same commit and flavor, because an ipa runs nowhere we control — see design.md D3c.
- **Screens are reached by deep link, and a link names a route.** `waypoints://library`, `waypoints://marks/<id>` — the routes under `src/app/pages/` as they already exist, with no parallel vocabulary of capture scenes. This is partly the point: it is the first use of the custom scheme on mobile, and the first second kind of link the frontend dispatch has had to carry — see design.md D3.
- **A link can pose the page it names.** Presentation state — which item is selected, which panel is open, which tab — travels in the link's query. The handler forwards the whole URL to the router, and one convention lets a page declare and read the keys it honours; undeclared keys are ignored and no key mutates anything. Today no page reads the URL at all. Proven with one pose on the demo screen — see design.md D3d.
- **The scheme carries the flavor.** `waypoints`, `waypoints-preview`, `waypoints-nightly`. Today the base config registers one scheme for every flavor, so with two installed side by side the operating system decides which receives a link — a latent bug this change fixes.
- **A link navigates and never acts**, generalising the rule the pairing path already follows. A pose is navigation.
- **Minting is a manual command.** `xfastlane capture <platform>` boots the simulator or emulator, installs the build, places the territory, runs the harness, and writes captures to `src-tauri/fastlane/captures/<platform>/<locale>/`. It runs on a developer's machine, by hand; nothing in CI calls it — see design.md D4.
- **Typst keeps the composition.** It writes the headline and supporting line, paints the background, lays the page out, and embeds the capture inside a device frame. `frameit` is not used — see design.md D1.
- **Frames are sourced, in the tree and out of history.** Mockuuups Studio renders of the iPhone 17 Pro and Pixel 10 Pro, exported under a paid plan, keyed so the screen is transparent, and kept under `src-tauri/fastlane/res/` where `**/*.png` already ignores them. The licence allows the use and forbids redistributing the render itself, which a committed PNG would be — see design.md D2.
- **Device profiles.** A `lib/devices.typ` naming each frame's image, screen rect, corner radius and capture scale. This change ships `iphone-17-pro` and `pixel-10-pro`.
- **A working demo on both devices.** The change is done when one screen renders end to end from a real capture on an iPhone 17 Pro and a Pixel 10 Pro — the library over the demo territory, through a posed link, is enough.

## Capabilities

This change stays inside the `distribution` domain. Listing assets are what a release presents to a store, alongside the metadata `release-planning` already governs; the capture harness exists to make that presentation verifiable rather than drawn.

### New Capabilities

- `distribution/store-screenshots`: what a listing screenshot is allowed to be (a capture of the shipping build, not a drawing), what content a capture is taken against and how it reaches the build, how a screen is reached and posed, and the rule that a capture is composed onto the store's canvas rather than uploaded at the device's own size.

### Modified Capabilities

None. The Typst pipeline, the listing copy, and the fastlane lanes are untouched as capabilities; `device()`'s signature and the locale file's screen bodies change, which is implementation.

## Impact

- **New files:** `src-tauri/fastlane/lib/devices.typ` (image path, screen rect, corner radius and capture scale per device); `src-tauri/fastlane/res/.gitkeep` beside the untracked frames and their un-keyed exports; `src-tauri/fastlane/captures/{ios,android}/en-US/`; a UI test target source under `src-tauri/gen/apple/`; an `androidTest` source set under `src-tauri/gen/android/app/src/androidTest/`; one composable under `src/app/` through which a page reads presentation state from its route.
- **`tooling/bin/xfastlane`:** a `capture` subcommand (boot, install, place the territory, run harness, collect).
- **`tooling/xtask`:** a `demo-territory` task producing the corpus on the host through `crates/store` with `mock` on, taking the Bundle to ingest and defaulting to `getting-started`.
- **`src-tauri/fastlane/lib/screen.typ`:** `device()` takes a capture and a device profile instead of `body`.
- **`src-tauri/fastlane/en-US.typ`:** the demo screen's body becomes a capture reference; the headline and supporting line stay.
- **`src-tauri/gen/apple/project.yml`** and the Android Gradle config: a test target and the `screengrab` dependency; both are generated, so their generators change.
- **Deep links:** the scheme moved from `src-tauri/Tauri.toml` into `src-tauri/res/configs/flavors/*.toml` and made flavor-specific; that scheme registered for mobile (iOS `CFBundleURLTypes`, Android intent filter — `[[plugins.deep-link.mobile]]` carries only the `waypoints.so` host and `/pair` today); `src/app/plugins/deep-link.client.ts` dispatching on link kind instead of funnelling every URL through `classifySetupCode`, and forwarding a route link's path, params, query and hash to the router unchanged.
- **`src/app/pages/library`:** honours one pose through the convention, so the demo capture can show what the bare route cannot.
- **CI:** nothing. No workflow gains a job or a step.

## Non-goals

- **No choice of what marketing imagery shows.** Which route each listing screen opens, which pose it carries, which Bundle the territory holds, and the headline and supporting line beside each — the next change. This one lands the controls: any route, any pose the convention can express, any Bundle, and proves them together on one screen.
- **No catalogue of poses.** The convention and one pose on the library land here. Which keys each further page honours is added page by page when a screen needs it, because a key no screen uses is scaffolding.
- **No marketing Bundle.** The territory placed here is the `getting-started` Bundle. A corpus written for the listing goes through the same `demo-territory` argument later, as content.
- **No pipeline integration.** No capture job, no artifact handoff, no listing in the release store. The release pipeline keeps rendering from whatever captures are in the tree when `xfastlane prepare` runs, exactly as it does today. Automating minting is a later change, once the manual process has been run enough times to know what it costs.
- **No tracking policy.** Whether captures are committed, and for which channels, is undecided and is not decided here. The leaning is that nightly's churn makes committing them pointless, while preview and stable change rarely enough to commit — or that captures are minted when a release group is cut and refreshed with it. That is a structure question for a change of its own; until then captures stay generated output under the existing ignore rule.
- **No synced demo territory.** Pairing a release build to a Station holding the territory would exercise onboarding too, and is the intended successor to placement; it waits on content sync, which does not exist yet — see design.md D3c.
- **No `frameit`.** See design.md D1; its Android frames stop at the Pixel 5.
- **No tablet surfaces.** `lib/surfaces.typ` grows no rows here.
- **No second locale.** The capture path is keyed by locale from the start, but only `en-US` is produced.
- **No app-preview videos.**
- **No change to the lanes.** `upload_to_play_store` and `upload_to_testflight` already read what the Typst pipeline writes under `metadata/<platform>/`.
