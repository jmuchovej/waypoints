# Design

## Context

See proposal.md — Why, including the terms (release channel, flavor, distribution channel, package manifest, flavored package manifest, release manifest).

The tree has `deploy/justfile` (`lint`, `image`, `kind-up`, `kind-load`, `kind-apply`, `kind-down`, mounted at the root as `mod deploy`), `deploy/kubernetes/base/` (five manifests) and `deploy/kubernetes/kind/` (the overlay, `resources: [../base]`); `devenv.nix` puts kubeconform, kubectl, and kind in the mise lane `deploy` and imports `./nix/devenv`; `nix/` holds `overlay.nix`, `channel.nix`, `packages/{waypoints,outpost}/`, `releases/`, `devenv/`, and an `AGENTS.md` describing both halves. The overlay builds a recorded release by fetching the tree at the release index's `rev` and importing `nix/packages/waypoints/package.nix` from _that_ tree; `flake.nix` references the Nix files by path five times; `tooling/bin/nix-pins` calls the recipe by path; the index entry lands on `main` through the release bot's pull request, which merges itself once the required checks pass. `tooling/release-groups.toml` names `nix/releases/` as bookkeeping that counts toward no release group, and the planners and the `version-integrity` check read that file.

The release side fixes most of what a package manifest needs. Artifacts are stored at `{channel}/{series}/{os}/{arch}/waypoints-{tag}.{ext}` (`tooling/xtask/src/releases/artifacts.rs`) and served at `https://waypoints.so/api/v1/releases/…`. Each release channel's current release is the release manifest `{channel}.json` — `version` (the tag, `v` included), `series`, `targets[{os, arch, extensions}]` — which `/api/v1/releases/{channel}/current` reduces to `{tag, title, date}` and the updater route reduces to a per-target feed; no route serves the manifest itself. Linux publishes `AppImage`, `deb`, `rpm`, each with a `.sig`; Windows `exe` with its `.sig`; macOS `dmg` plus the updater's `app.tar.gz` and its `.sig` (`platform.rs`). Flavor identity is one TOML per flavor under `src-tauri/res/configs/flavors/` (`productName`, `identifier`, icons, updater endpoint); the binaries are `waypoints`, `waypoints-preview`, `waypoints-nightly` (`src-tauri/Cargo.toml` `[[bin]]`). On `main`, nightly tags are date-named (`v0.1.0-nightly.20261116`) and carry no build metadata. The Windows installer is not code-signed.

Every desktop install updates itself through the Tauri updater unless launched with `--no-self-update`, which caps it at notify-only (`self-update-policy`); the Nix package's wrapper passes it, and so does the AUR package (D8). xtask has no template engine; the release tooling's changelog is a `git-cliff` (tera) template. The `Tests` workflow has three Linux jobs (`test-linux`, `zizmor`, `license`) and no macOS one.

## Goals / Non-Goals

**Goals:**

- One tree with one shape: `distribution/<channel>/` for every distribution channel, including the Station's deployment and the Nix overlay.
- One registry of which distribution channels each release channel reaches, and as what.
- Every release-specific value in a flavored package manifest derives from the record; nothing under `distribution/` is edited at release time.
- Each of the four package channels is renderable and checkable in CI, so the distribute workflow that follows is rendering plus a push.

**Non-Goals:**

- The distribute workflow: when a render is submitted upstream, through what credential, under what cadence rules.
- The self-update setting and flag (`self-update-policy`).
- Any change to what a flavor is or how it is built, or to what the overlay builds and how the index is written.

## Decisions

### D1. One directory per channel; a channel's inner layout is its tooling's idiom (2026-11-08)

`deploy/kubernetes/{base,kind}` → `distribution/kubernetes/{base,kind}`; `deploy/justfile` → `distribution/kubernetes/justfile`, recipes unchanged. The first level under `distribution/` is the channel; below that, the channel lays itself out the way its own tooling expects — kustomize's base and overlays, Nix's `packages/` and `releases/` — and a new form of the same channel nests there rather than becoming a top-level peer. A channel's `justfile`, when it has one, sits at the channel root.

- _Rejected_: flattening to `distribution/{kubernetes,kind}` — makes an overlay of the manifests look like a channel, and cannot be the pattern beside a Nix channel that keeps two subdirectories.
- _Rejected_: `distribution/kubernetes/justfile` declaring `mod kind` — a module for six recipes that all concern the one cluster.
- _Rejected_: leaving `deploy/` alone — two homes for "how Waypoints gets onto a machine".

### D2. The root module is `dist`; the mise lane follows; the Nix pins recipes move with their channel (2026-11-06)

`mod dist "distribution/justfile"`; `distribution/justfile` declares a `mod` per channel that has recipes (`kubernetes`, `nix`, and each package channel), so `just deploy::kind-up` becomes `just dist::kubernetes::kind-up` and the root `nix-pins`/`nix-pins-check` become `just dist::nix::pins`/`pins-check`. The mise lane `deploy` becomes `dist`.

- _Rejected_: `mod distribution` — every recipe is typed; `dist` is unambiguous here.
- _Rejected_: keeping `mod deploy` pointed at the new path — a name on the task-runner surface that no longer exists in the tree.
- _Rejected_: leaving the pins recipes at the root — the one channel recipe outside the module, for no reason beyond history.

### D3. The Nix channel moves to `distribution/nix/`; the devenv modules to `tooling/devenv/`; `nix/` goes away (2026-11-07)

`nix/overlay.nix`, `nix/channel.nix`, `nix/packages/`, `nix/releases/` → `distribution/nix/` with their internal structure kept; `overlay.nix`'s `tree` becomes `../..` and the recipe path it imports from a fetched tree becomes `/distribution/nix/packages/waypoints/package.nix`. `flake.nix` stays at the root — Nix requires it — and repoints five references. `nix/devenv/` → `tooling/devenv/`: the modules describe how the repository builds itself, which is what `tooling/` holds. `nix/AGENTS.md` moves to `tooling/devenv/AGENTS.md`, keeping its "who runs the script" rule (`tooling/devenv/bin` for devenv tasks, `tooling/bin` for people) and dropping the package half.

The cost is one evaluation gap. On `main`, `waypoints.nightly` is `recorded "nightly"`: it fetches the tree at the index's recorded rev and imports the recipe from the new path, which a pre-move tree lacks. From the merge until the next nightly's index pull request records a post-move rev, `nix build .#nightly` on `main` fails to evaluate. `stable` and `preview` throw already (`{}` entries); `checks.waypoints-in-tree` and `outpost` use the in-tree recipe and are unaffected; no CI step evaluates the recorded packages; no consumer applies the overlay yet. The gap lasts one nightly.

- _Rejected_: leaving `nix/packages` and `nix/releases` put — protects consumers who do not exist and leaves `nix/` meaning "the overlay and also the dev shell".
- _Rejected_: an overlay that tries both recipe paths — permanent complexity for a one-nightly gap.
- _Rejected_: re-recording `nightly.json` by hand — rewrites a release record for a layout change.
- _Rejected_: `devenv/` at the root for the modules — a third top-level home for build tooling when `tooling/` already is one.
- _Rejected_: flattening `distribution/nix/{packages,releases}` — the index is not a package; two subdirectories say so.

### D4. `distribution/channels.toml` is the registry: release channel → distribution channel → identity (2026-11-09)

```toml
[stable]
brew = "waypoints"
aur = "waypoints-bin"
winget = "Waypoints.Waypoints"
chocolatey = "waypoints"

[preview]
brew = "waypoints@preview"
aur = "waypoints-preview-bin"
winget = "Waypoints.Waypoints.Preview"
chocolatey = "waypoints-preview"

[nightly]
brew = "waypoints@nightly"
aur = "waypoints-nightly-bin"
winget = "Waypoints.Waypoints.Nightly"
chocolatey = "waypoints-nightly"
```

One table per release channel; each key is a distribution channel directory and each value the package identity that release channel takes there, in that manager's convention. The renderer resolves `package.id` from `[<flavor>].<channel>` and refuses a pair the registry omits. Keyed by release channel because that is how everything downstream is organized — one release workflow per channel (`release-{stable,preview,nightly}.yaml`) today, one distribute workflow per channel later — and because the file then doubles as the statement of which distribution channels serve which release channel. A value may become a table (`brew = { id = "…", … }`) when a distribution channel needs more than a name; nothing moves. Package manifests have no control flow (D5), so this file is the only place per-channel identity lives.

- _Rejected_: a declaration file per distribution channel directory — the same facts inverted and scattered across four files.
- _Rejected_: deriving identities by filter from the flavor name — hides four naming conventions in Rust and gives no way to make a release channel ineligible for a distribution channel.
- _Rejected_: one package manifest per release channel — three files differing only in values, which is the drift this change exists to end.

### D5. Package manifests keep their host syntax; placeholders live inside string literals; file names are templates too (2026-11-08)

A channel's package manifests are tera templates whose placeholders (`{{ release.tag }}`, `{{ artifact.macos_arm64.dmg.sha256 }}`) appear only where the host language has a string: `version "{{ release.tag }}"` is valid Ruby, `sha256sums=("{{ … }}")` valid bash, `InstallerSha256: "{{ … }}"` valid YAML, `$checksum64 = '{{ … }}'` valid PowerShell. Files keep their native extension so the repository's formatters (rubocop's layout cops for Ruby, oxfmt for YAML, the XML formatter for nuspec) and editors treat them as what they are.

No `{% %}` control flow. A `{% if %}` on its own line is not valid Ruby, bash, YAML, or XML, so one tag would take the file out of every formatter, leaving only the render checkable. The ban holds because of a rule about the packages themselves: **a distribution channel's recipe is structurally identical across release channels**. Stable, preview, and nightly differ in _values_ — identity, version, digests, and, should a flavor ever need one, a dependency list — and every value comes from the context (D4; a registry entry grows into a table when a channel needs more than a name, and a list renders with `join`). If a flavor ships something the others do not (a sidecar, an extra artifact), it ships on every release channel, as a no-op where unneeded, so the recipe does not branch. Changing that takes a compelling case recorded here, not a conditional in a template.

A package manifest's file name is rendered through the same context: `{{ package.id }}.rb` → `waypoints@nightly.rb`, `{{ package.id }}.installer.yaml` → `Waypoints.Waypoints.Nightly.installer.yaml`. Homebrew derives a cask's token from its file name and winget derives the package identifier from its, so a fixed name would render files their own validators reject.

- _Rejected_: control flow inside host-language comments (`# {% if %}` … `# {% endif %}`) — keeps the file parseable, but it is a conditional recipe by another route, and the structural rule above makes it unnecessary.
- _Rejected_: `.tera` suffix — loses every native formatter on the one place the manifests are edited.
- _Rejected_: a shared Ruby library the casks `require_relative` — only Homebrew could use it, and it would run on users' machines.
- _Rejected_: generating manifests from Rust with no templates — every stanza becomes a string literal in xtask, and the channel's own tooling can no longer read the source form.
- _Rejected_: a per-channel rename step after rendering — a second place that knows the naming convention.

### D6. Rendering is `cargo xtask dist render`, into `target/dist/`; renders are never committed (2026-11-06)

`cargo xtask dist render <channel> --flavor <flavor> [--manifest <path|url>]` builds a context and renders every file under `distribution/<channel>/` to `target/dist/<channel>/<flavor>/`. The context:

- `package` — `id` (D4).
- `flavor` — `product_name` and `identifier` (the flavor TOML), `executable` (`Flavor::executable()` in xtask, pinned by test to `src-tauri/Cargo.toml`'s `[[bin]]` names).
- `release` — `tag` (the release manifest's `version`), `version` (tag without `v`, which is also winget's and Chocolatey's spelling now that nightlies carry no build metadata), `series`, and the filter `pkgver` (Arch forbids the hyphen: `v0.1.0-pre.2` → `0.1.0pre.2`, `v0.1.0-nightly.20261116` → `0.1.0nightly.20261116`).
- `artifact.<os>_<arch>.<ext>` — `url`, `file_name`, `sha256`, from the release manifest and the release-delivery URL layout. `file_name` is the stored object name; see Risks for the readable-name scenario.

The default manifest source is `https://waypoints.so/api/v1/releases/<flavor>/manifest` (D7); `--manifest` takes a local file for tests and for a publishing job, which holds the manifest it just wrote. A placeholder the context lacks fails the render (tera strict mode); nothing partial is written. `target/` is already ignored. `nix` and `kubernetes` have nothing to render and are refused by name.

- _Rejected_: minijinja — equivalent; tera is what `git-cliff` already evaluates in this repo.
- _Rejected_: committing flavored package manifests — the renderer plus a fixture release manifest already validates every channel and release channel in CI; a committed render is exactly what D9 refuses; and the external repositories are the renders' home with their own history.

### D7. The release manifest carries a digest per uploaded file and is served as stored (2026-11-08)

`Manifest.targets[]` gains `digests: { "<ext>": "<sha256 hex>" }` covering every file the run uploads for that target — installers, updater payloads, and `.sig`s alike — computed from the staged file as it uploads, so a manifest cannot describe bytes the run did not place. `extensions` stays for existing readers; the website's `ReleaseManifest` gains the optional field. A new route, `/api/v1/releases/{channel}/manifest`, streams the stored object through `readObject`/`sendR2Object`, bytes unchanged: the existing routes reduce the manifest (`current` to `{tag, title, date}`, `update` to a per-target feed) and nothing exposes `targets` or `digests`, which the renderer needs. The release-delivery spec forbids clients reaching the store, so the route is the only way a render outside the publishing job can read the record.

- _Rejected_: the renderer downloads and hashes — slow, and it would describe whatever the CDN served at render time rather than what was published.
- _Rejected_: the renderer holds the store credential — moves an integrity credential out of the one step that needs it.
- _Rejected_: returning `readManifest`'s parsed object — h3 re-serializes it compact while xtask writes it pretty-printed, so the served bytes would not be the stored ones.
- _Rejected_: a separate `SHA256SUMS` object — a second file to keep in step with the manifest.

### D8. Every release channel is registered on every package channel; package-manager-owned installs launch notify-only (2026-11-20)

A package manager's listing is a seed: it puts a first install on the machine, and where the app updates itself, the Tauri updater carries the install forward from there. So nightly belongs in the registry for winget and Chocolatey even though their review queues cannot take a daily submission — a listing that lags the current nightly by days still delivers the current nightly within one updater check. How often each upstream receives a render is the distribute workflow's rule set, not the registry's.

Where the package manager owns the installed files, the app must not rewrite them. The AUR package therefore launches the app with `--no-self-update` (defined by `self-update-policy`: the setting is capped at notify, the user's stored preference untouched), through a `/usr/bin/<executable>` wrapper over the real binary in `/usr/lib/<package>/` and through the deb's desktop entry with its `Exec` rewritten to that wrapper. The Nix overlay does the same through its wrapper (that change amends `nix-overlay`). Homebrew, winget, and Chocolatey pass nothing; the setting defaults to on.

- _Rejected_: leaving nightly out of the registry for reviewed channels — treats the listing as the update path when the app has one.
- _Rejected_: a cadence field in the registry — the registry says where a release channel goes; when is the distribute workflow's.
- _Rejected_: a second Linux artifact built without self-updating for the AUR — a new release artifact and a second Linux build, to express one launch flag.

### D9. Package manifests are refused if they are flavored (2026-11-06)

`cargo xtask dist check` scans `distribution/` for a concrete tag (`v\d+\.\d+\.\d+`), a 64-hex digest, or a versioned `/api/v1/releases/` URL, and fails naming the file — excluding `distribution/nix/releases/`, whose job is to carry exactly those. Wired as `just dist::check` and run by the `dist` job (D10).

### D10. Channel shapes and the `dist` job (2026-11-20)

| Channel      | Files under `distribution/<channel>/`                              | Source        |
| ------------ | ------------------------------------------------------------------ | ------------- |
| `brew`       | `{{ package.id }}.rb`                                              | dmg, AppImage |
| `aur`        | `PKGBUILD`, `.SRCINFO`                                             | deb           |
| `winget`     | `{{ package.id }}{,.installer,.locale.en-US}.yaml`                 | exe           |
| `chocolatey` | `{{ package.id }}.nuspec`, `tools/chocolateyinstall.ps1`           | exe           |
| `nix`        | `overlay.nix`, `channel.nix`, `packages/`, `releases/`, `justfile` | source build  |
| `kubernetes` | `base/`, `kind/`, `justfile`                                       | —             |

All three release channels are registered on each of the four package channels (D4).

**Homebrew** — top level: `arch arm:`, `os macos: linux:`, one `url` with the extension from `on_system_conditional`, `sha256 arm:`/`x86_64_linux:`, `livecheck` with `strategy :json` reading `tag` from `/current`, `auto_updates true`. `on_macos`: `depends_on macos: :sequoia` (Tauri `minimumSystemVersion = "15"`), `arch: :arm64`, `app`, `uninstall quit:`, `zap` on `~/Library/Application Support/<identifier>` and `Caches`. `on_linux`: `depends_on arch: :x86_64`, `app_image` with `target:` the executable, `zap` on `~/.cache`, `~/.config`, `~/.local/share` for the identifier. No `license` stanza (not in the cask DSL). Validator: `brew style`, then `brew audit --cask` for the host and with `--os linux --arch intel`, all by name from a trusted throwaway tap (current Homebrew loads casks neither by path nor from an untrusted tap); `BREW` points the recipe at a writable Homebrew where the installed one is read-only.

**AUR** — `pkgname` from the registry; the published deb extracted with bsdtar for binary, desktop entry, and icons; the wrapper and `Exec` rewrite from D8; `pkgver` from the filter; `sha256sums` from the deb digest. `.SRCINFO` is templated rather than generated so it renders anywhere. Validator: `makepkg --printsrcinfo` diffed against the render, `namcap`, on Arch.

**winget** — `PackageIdentifier` from the registry; `PackageVersion` is `release.version`; `InstallerType: nsis`, `Scope: user`, `InstallerSha256` from the exe digest; `AppsAndFeaturesEntries` with `DisplayName` from the flavor's product name and `DisplayVersion` from `release.version`, since Tauri's NSIS registers its uninstall key under the product name (not the bundle identifier) and writes `DisplayVersion` as the Cargo version verbatim. Validator: `winget validate` on Windows.

**Chocolatey** — `<id>` from the registry; `<version>` is `release.version`; `url64` on the versioned exe, `checksum64` from the digest, `checksumType64 sha256`, `silentArgs /S`, `validExitCodes @(0)`; no uninstall script, relying on Chocolatey's Auto Uninstaller (unverified; see Risks). Validator: `choco pack` on Windows.

**The `dist` job** — the one job of a `Distribution` workflow (`workflows/distribution.rs`) on `runners::MACOS`, triggered by changes under `distribution/`, the flavor configs, or `tooling/xtask/`, running `just dist::check-all`: the stamp scan, every render against the fixture, and every channel's `check`. `brew audit` runs there; the AUR, winget, and Chocolatey checks report skipped with their reasons. It is not a required check. It has its own workflow because every job in `Tests` is one: `render_tfvars` names each of them in `var.github`'s required checks. Making it required is adding it to that list.

### D11. The Linux cask installs the AppImage through `app_image` (2026-11-08)

Homebrew's cask DSL has a Linux-only `app_image` artifact (`Library/Homebrew/cask/artifact/appimage.rb`, in `ORDINARY_ARTIFACT_CLASSES`; Cask Cookbook: "`app_image` is Linux-only, `app` and `pkg` are macOS-only, and `binary` works on either"). Since Homebrew/brew#24343, first released in Homebrew 7.0.12 (2026-11-02), it installs the AppImage's embedded desktop entry into `${XDG_DATA_HOME:-~/.local/share}/applications`, with `Exec` rewritten to the installed path and the icon copied into the Caskroom, and removes them on uninstall. Tauri's AppImage embeds that desktop entry and icon. `auto_updates true` covers both OSes: the Tauri updater replaces an AppImage in place, the installed path and so the desktop entry's `Exec` do not change, and `brew upgrade` skips auto-updating casks without `--greedy`.

- _Rejected_: a `binary` stanza plus a tarball artifact — a launcher-less install, and a new release artifact to serve it.
- _Accepted limitation_: a Linuxbrew older than 7.0.12 installs the AppImage without the launcher entry until its next auto-update (daily by default).

## Risks / Trade-offs

- **The `deploy::` and `nix-pins` renames break muscle memory and open changes** → the open changes are edited here; `just --list` shows `dist`; the kustomization comments name the new recipes.
- **`waypoints.nightly` on `main` does not evaluate for one nightly after the merge** → accepted, per D3.
- **The `nix/releases/` pathspec lives in `tooling/release-groups.toml` and in test fixtures, not only in `releases/index.rs`** → if one copy is missed, index-only commits count as product work and the required `version-integrity` check reads the wrong set; task 2.4 names every copy and ends on a search that must come back empty.
- **`artifact.file_name` assumes the download serves the stored name** → the release-delivery spec's scenario "A nightly saves under a readable name" is met by date-named tags, and the versioned route sets no Content-Disposition today. If a rename is ever added there, `file_name` must become "the name the download surface serves" or the cask's `app_image` path breaks.
- **`.SRCINFO` is templated rather than generated** → the AUR check diffs it against `makepkg --printsrcinfo`, and the AUR rejects a mismatch at push time anyway.
- **Claims about external tools** — verified 2026-11-18 against official docs or source: `brew audit --os`/`--arch`; `livecheck`'s `:json` strategy; Homebrew 7.0.12 as the first release with #24343; winget comparing version parts numerically (`Versions.cpp`), so date-named nightlies sort; Tauri's NSIS writing `DisplayVersion` verbatim and keying the uninstall entry by product name. Unverified, and each a verify-before-submit item for the distribute workflow: winget leaving an installed version newer than its manifest alone; Chocolatey comparing its own install record rather than Add/Remove Programs (so a refreshed listing may reinstall over a newer self-updated build until the next updater check); the Chocolatey community repository accepting SemVer 2 prerelease identifiers (the CLI does since 2.0.0); the Auto Uninstaller removing an NSIS-registered program without a script. Verified 2026-11-06 against Homebrew/brew `main` `dsl.rb`: `os`, `arch`, `on_system_conditional`, `sha256 arm:`/`x86_64_linux:`, no `license` stanza, a second `url` raises.
- **Digests widen the wire format** → additive, optional in the website type.
- **Package manifests with no control flow push logic into the context** → the context grows a registry lookup and one filter; testable in Rust.
- **`brew audit`, `makepkg`, `winget validate`, `choco pack` are platform-bound** → recipes report skipped with a reason; the `dist` job runs `brew audit` and records the skips.

## Open Questions

- Whether a nightly seed on winget and Chocolatey needs a refresh floor independent of the preview cadence — for the distribute workflow, once a preview cadence exists.

## Appendix A — Channel notes (non-normative)

Recorded 2026-11-06–08 for reference; not requirements, and not a sequencing decision.

| Channel              | Setup  | Per release        | Review           | Status   |
| -------------------- | ------ | ------------------ | ---------------- | -------- |
| Direct download      | —      | —                  | ours             | exists   |
| Nix overlay          | —      | index write        | ours             | exists   |
| Homebrew             | low    | one commit         | ours             | chosen   |
| AUR `-bin`           | low    | one push/flavor    | ours             | chosen   |
| winget               | medium | one PR/version     | days, theirs     | chosen   |
| Chocolatey           | medium | one push/version   | days–weeks       | chosen   |
| apt/dnf repos        | medium | re-sign, re-index  | ours             | deferred |
| Snap                 | medium | one upload/flavor  | ours             | deferred |
| Flathub              | high   | one PR/version     | days–weeks       | deferred |
| Debian/Fedora source | high   | one upload/release | hours (COPR/OBS) | deferred |
| scoop                | low    | one commit         | ours             | deferred |

**Direct download** — download, drag or run; the Tauri updater handles every update after. The deb and rpm stay in this set for Debian/Ubuntu and Fedora users.

**Nix overlay** — a store-installed build notifies about updates and never self-updates (`nix-overlay` spec).

**Homebrew** — `brew install --cask`, then `brew upgrade` or the in-app updater. Setup is a tap repository, the package manifest, and `brew audit` in CI.

**AUR `-bin`** — `yay -S`; updates with the system, notify-only in the app. Setup is three AUR packages and an SSH key; self-service, so a daily nightly is normal for a `-bin` package.

**winget** — `winget install`; one PR per version, bot-validated, human-merged over days. The real prerequisite is a Windows signing identity: an unsigned installer trips SmartScreen and may draw reviewer pushback.

**Chocolatey** — `choco install`; a package's first versions are human-moderated (days to weeks), later ones pass automated validation. Same signing gate as winget.

**Self-hosted apt/dnf repositories** (deferred) — the deb and rpm already exist, so this is the cheapest way to make them update with the system; needs repository metadata, hosting beside the store, and a GPG key the vault does not hold. The direct downloads and the updater serve those users meanwhile.

**Snap** (deferred) — one snap with release channels as risk tracks means they are not co-installable; strict confinement of WebKitGTK needs debugging; a sandboxed install cannot self-update, so the package would launch with `--no-self-update` once that exists.

**Flathub** (deferred) — AppStream metainfo with screenshots, permission review, a Flathub repository, likely pressure to build from source (offline cargo plus pnpm sources). Days to weeks of review per version; nightly impossible; the same `--no-self-update` prerequisite as snap.

**Debian / Fedora source packaging** (deferred) — needs a vendored-source tarball because COPR/PPA/OBS builders have no network, and two build systems. Official Debian inclusion is out of reach while every pnpm dependency would need packaging; Fedora proper needs a sponsor and bundled-crate exceptions. If pursued, COPR/OBS is the venue.

**scoop** (deferred) — already served by winget and Chocolatey; wants a portable archive, which Windows does not publish.
