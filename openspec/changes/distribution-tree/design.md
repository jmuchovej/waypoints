# Design

## Context

See proposal.md — Why. The committed tree at `ce4067aca` has:

- `deploy/justfile` — `lint`, `image`, `kind-up`, `kind-load`, `kind-apply`, `kind-down`; mounted at the root as `mod deploy`.
- `deploy/kubernetes/base/{configmap,kustomization,secrets.example,service,statefulset}.yaml` — the one-Station Fleet.
- `deploy/kubernetes/kind/{kind-config,kustomization}.yaml` — the kind overlay, `resources: [../base]`.
- `devenv.nix` — kubeconform, kubectl, kind in the mise lane `deploy`, projected to `.config/mise/conf.d/tools.deploy.toml`; `imports = [ ./nix/devenv ]`.
- `nix/overlay.nix`, `nix/channel.nix`, `nix/packages/{waypoints,outpost}/`, `nix/releases/` — the overlay, its channel reading, the recipes, the release index. The overlay builds a recorded release by fetching the tree at its `rev` and importing `nix/packages/waypoints/package.nix` from _that_ tree; `flake.nix` references the four by path; `tooling/bin/nix-pins` calls the recipe by path; `releases/index.rs` writes `nix/releases/`, and the index entry lands on `main` through the release bot's self-merging pull request.
- `nix/devenv/{default,android,mise}.nix`, `nix/devenv/bin/` — the dev shell's modules and the scripts devenv runs as tasks; `nix/AGENTS.md` documents both halves of `nix/`.

The release side fixes most of what a package manifest needs: artifacts are stored at `{channel}/{series}/{os}/{arch}/waypoints-{tag}.{ext}` (`tooling/xtask/src/releases/artifacts.rs`) and served at `https://waypoints.so/api/v1/releases/…`; each release channel's current release is the manifest `{channel}.json` — `version` (the tag, `v` included), `series`, `targets[{os, arch, extensions}]` — which `/api/v1/releases/{channel}/current` reduces to `{tag, title, date}` and the updater route reduces to a per-target feed. No route serves the manifest itself. Linux publishes `AppImage`, `deb`, `rpm` with a `.sig` each; Windows `exe` with its `.sig`; macOS `dmg` plus the updater's `app.tar.gz` and its `.sig` (`platform.rs`). A release channel and a flavor share a name (`Channel::flavor`); flavor identity is one TOML per flavor under `src-tauri/res/configs/flavors/` (`productName`, `identifier`, icons, updater endpoint); binaries are `waypoints`, `waypoints-preview`, `waypoints-nightly`. The Windows installer is not code-signed.

Every desktop install updates itself through the Tauri updater except a Nix-installed build, which the `nix-overlay` spec holds to notify-only.

xtask has no template engine; the release tooling's changelog is a `git-cliff` (tera) template.

## Goals / Non-Goals

**Goals:**

- One tree with one shape: `distribution/<channel>/` for every distribution channel, including the Station's deployment and the Nix overlay.
- One registry of which distribution channels each release channel reaches, and as what.
- Every release-specific value in a package manifest derives from the record; nothing under `distribution/` is edited at release time.
- Each of the four package channels is renderable and checkable today, so the distribute workflow that follows is rendering plus a push.

**Non-Goals:**

- The distribute workflow: when a render is submitted upstream, through what credential, under what cadence rules. It is the follow-up; this change gives it a registry to iterate and a renderer to call.
- Any change to what a flavor is or how it is built.
- Any change to the overlay's behaviour: what each channel attribute builds, how a recorded release is fetched, what the index records.
- Where CI runs a platform-bound validator. Each check names the tool it needs and skips with a reason without it.

## Decisions

### D1. One directory per channel; a channel's inner layout is its tooling's idiom (2026-11-06, revised 2026-11-08)

`deploy/kubernetes/{base,kind}` → `distribution/kubernetes/{base,kind}`; `deploy/justfile` → `distribution/kubernetes/justfile`, recipes unchanged. The first level under `distribution/` is the channel; below that, the channel lays itself out the way its own tooling expects — kustomize's base and overlays, Nix's `packages/` and `releases/` — and a new form of the same channel nests there rather than becoming a top-level peer. A channel's `justfile`, when it has one, sits at the channel root.

- _Rejected_: flattening to `distribution/{kubernetes,kind}`, the layout the uncommitted working tree carries — makes an overlay of the manifests look like a channel, and cannot be the pattern beside a Nix channel that keeps two subdirectories.
- _Rejected_: `distribution/kubernetes/justfile` declaring `mod kind` — a module for six recipes that all concern the one cluster.
- _Rejected_: leaving `deploy/` alone — two homes for "how Waypoints gets onto a machine".

### D2. The root module is `dist`; the mise lane follows (2026-11-06)

`mod dist "distribution/justfile"`; `distribution/justfile` declares `mod kubernetes "kubernetes/justfile"` and a `mod` per channel that has recipes, so `just deploy::kind-up` becomes `just dist::kubernetes::kind-up`. The mise lane `deploy` becomes `dist`.

- _Rejected_: `mod distribution` — every recipe is typed; `dist` is unambiguous here.
- _Rejected_: keeping `mod deploy` pointed at the new path — a name on the task-runner surface that no longer exists in the tree.

### D3. The Nix channel moves to `distribution/nix/`; the devenv modules to `tooling/devenv/`; `nix/` goes away (2026-11-07)

`nix/overlay.nix`, `nix/channel.nix`, `nix/packages/`, `nix/releases/` → `distribution/nix/` with their internal structure kept; `overlay.nix`'s `tree` becomes `../..` and the recipe path it imports from a fetched tree becomes `/distribution/nix/packages/waypoints/package.nix`. `flake.nix` stays at the root — Nix requires it — and repoints five references. `nix/devenv/` → `tooling/devenv/`: the modules describe how the repository builds itself, which is what `tooling/` holds. `nix/AGENTS.md` moves to `tooling/devenv/AGENTS.md`, keeping its "who runs the script" rule (`tooling/devenv/bin` for devenv tasks, `tooling/bin` for people) and dropping the package half.

The cost is one evaluation gap. On `main`, `waypoints.nightly` is `recorded "nightly"`: it fetches the tree at the index's recorded rev and imports the recipe from the new path, which a pre-move tree lacks. From the merge until the next nightly's index pull request merges a post-move rev into `distribution/nix/releases/nightly.json`, `nix build .#nightly` on `main` fails to evaluate. `stable` and `preview` throw already (`{}` entries); `checks.waypoints-in-tree` and `outpost` use the in-tree recipe and are unaffected; no CI step evaluates the recorded packages; no consumer applies the overlay yet. The index write path through a pull request is live (the 2026-11-08 nightly landed `#158` on protected `main`), so the gap is one nightly long.

- _Rejected_: leaving `nix/packages` and `nix/releases` put — protects consumers who do not exist and leaves `nix/` meaning "the overlay and also the dev shell".
- _Rejected_: an overlay that tries both recipe paths — permanent complexity for a one-nightly gap.
- _Rejected_: re-recording `nightly.json` by hand — rewrites a release record for a layout change.
- _Rejected_: `devenv/` at the root for the modules — a third top-level home for build tooling when `tooling/` already is one.
- _Rejected_: flattening `distribution/nix/{packages,releases}` — the index is not a package; two subdirectories say so.

### D4. `distribution/channels.toml` is the registry: release channel → distribution channel → identity (2026-11-09)

```toml
[stable]
brew       = "waypoints"
aur        = "waypoints-bin"
winget     = "Waypoints.Waypoints"
chocolatey = "waypoints"

[preview]
brew       = "waypoints@preview"
aur        = "waypoints-preview-bin"
winget     = "Waypoints.Waypoints.Preview"
chocolatey = "waypoints-preview"

[nightly]
brew       = "waypoints@nightly"
aur        = "waypoints-nightly-bin"
winget     = "Waypoints.Waypoints.Nightly"
chocolatey = "waypoints-nightly"
```

One table per release channel; each key is a distribution channel directory and each value the package identity that release channel takes there, in that manager's convention. The renderer resolves `flavor.id` from `[<flavor>].<channel>` and refuses a pair the registry omits. Keyed by release channel because that is how everything downstream is organised — one builder per release channel today, one distribute workflow per release channel later — and because the file then doubles as the statement of which distribution channels serve which release channel. A value may become a table (`brew = { id = "…", … }`) when a distribution channel needs more than a name; nothing moves.

Templates have no control flow (D5), so this file is the only place per-channel identity lives.

- _Rejected_: a `channel.toml` per distribution channel directory listing its release channels — the same facts inverted, scattered across four files, and it invited putting submission cadence (a distribute-workflow concern) into the render tree.
- _Rejected_: deriving identities by filter from the flavor name — hides four naming conventions in Rust and gives no way to make a release channel ineligible for a distribution channel.
- _Rejected_: one template file per release channel — the three-cask drift this change exists to end.

### D5. Templates keep their host syntax; placeholders live inside string literals; file names are templates too (2026-11-06, extended 2026-11-08)

A channel's files are tera templates whose placeholders (`{{ release.tag }}`, `{{ artifact.macos_arm64.dmg.sha256 }}`) appear only where the host language has a string: `version "{{ release.tag }}"` is valid Ruby, `sha256sums=("{{ … }}")` valid bash, `InstallerSha256: "{{ … }}"` valid YAML, `$checksum64 = '{{ … }}'` valid PowerShell. Files keep their native extension so rufo/rubocop and editors treat them as what they are.

No `{% %}` control flow. A `{% if %}` on its own line is not valid Ruby, bash, YAML, or XML, so one tag would take the template out of every formatter and linter, leaving only the render checkable. The ban holds because of a rule about the packages themselves: **a distribution channel's recipe is structurally identical across release channels**. Stable, preview, and nightly differ in _values_ — identity, version, digests, and, should a flavor ever need one, a dependency list — and every value comes from the context (D4; a registry entry grows into a table when a channel needs more than a name, and a list renders with `join`). If a flavor ships something the others do not (a sidecar, an extra artifact), it ships on every release channel, as a no-op where unneeded, so the recipe does not branch. Changing that takes a compelling case recorded here, not a conditional in a template.

A template's file name is rendered through the same context: `{{ flavor.id }}.rb` → `waypoints@nightly.rb`, `{{ flavor.id }}.installer.yaml` → `Waypoints.Waypoints.Nightly.installer.yaml`. Homebrew derives a cask's token from its file name and winget derives the package identifier from its, so a fixed name would render files their own validators reject.

- _Rejected_: control flow inside host-language comments (`# {% if %}` … `# {% endif %}`) — keeps the template parseable, but it is a conditional recipe by another route, and the structural rule above makes it unnecessary.
- _Rejected_: `.tera` suffix — loses every native formatter and linter on the one place the manifests are edited.
- _Rejected_: a shared Ruby library the casks `require_relative` — Homebrew's loader accepts it (tested 2026-11-06), but only Homebrew can use it, the helper runs on users' machines, and an `uninstall_*flight` block would switch the installed caskfile to `.rb` and break uninstall when the helper is absent.
- _Rejected_: generating manifests from Rust with no templates — every stanza becomes a string literal in xtask, and the channel's own tooling can no longer read the source form.
- _Rejected_: a per-channel rename step after rendering — a second place that knows the naming convention.

### D6. Rendering is `cargo xtask dist render`, into `target/dist/`; renders are never committed (2026-11-06)

`cargo xtask dist render <channel> --flavor <flavor> [--manifest <path|url>]` builds a context and renders every file under `distribution/<channel>/` to `target/dist/<channel>/<flavor>/`. The context:

- `flavor` — `id` (D4), `product_name` and `identifier` (the flavor TOML), `executable` (xtask's `Channel`).
- `release` — `tag` (the manifest's `version`), `version` (tag without `v`), `series`, and the filters `pkgver` (Arch: `v0.1.0-pre.2` → `0.1.0pre.2`), `winget_version`, `choco_version` (both strip build metadata).
- `artifact.<os>_<arch>.<ext>` — `url`, `file_name`, `sha256`, from the manifest and the release-delivery URL layout. `file_name` is the stored object name; see Risks for the readable-name clause.

The default manifest source is `https://waypoints.so/api/v1/releases/<flavor>/manifest` (D7); `--manifest` takes a local file for tests and for a publishing job, which holds the manifest it just wrote. A placeholder the context lacks fails the render (tera strict mode); nothing partial is written. `target/` is already ignored. `nix` and `kubernetes` have nothing to render and are refused by name.

- _Rejected_: minijinja — equivalent; tera is what `git-cliff` already evaluates in this repo.
- _Rejected_: committing the rendered manifests — the renderer plus a fixture manifest already validates every channel and release channel in CI; a committed render is exactly the stamped manifest `dist check` refuses; and the external repositories are the renders' home with their own history.

### D7. The channel manifest carries a digest per uploaded file and is served as such (2026-11-06, route added 2026-11-08)

`Manifest.targets[]` gains `digests: { "<ext>": "<sha256 hex>" }` covering every file the run uploads for that target — installers, updater payloads, and `.sig`s alike — computed from the staged file as it uploads, so a manifest cannot describe bytes the run did not place. `extensions` stays for existing readers; the website's `ReleaseManifest` gains the optional field. A new route, `/api/v1/releases/{channel}/manifest`, returns the stored manifest verbatim: the existing routes reduce it (`current` to `{tag, title, date}`, `update` to a per-target feed) and nothing exposes `targets` or `digests`, which the renderer needs. The release-delivery spec already forbids clients reaching the store, so the route is the only way a render outside the publishing job can read the record.

- _Rejected_: the renderer downloads and hashes — slow, and it would describe whatever the CDN served at render time rather than what was published.
- _Rejected_: the renderer holds the store credential — moves an integrity credential out of the one step that needs it.
- _Rejected_: a separate `SHA256SUMS` object — a second file to keep in step with the manifest.

### D8. Nightly is registered on every package channel because the app updates itself (2026-11-08, scope trimmed 2026-11-09)

A package manager's listing is a seed: it puts a first install on the machine, and the in-app updater — which checks the flavor's endpoint on every platform the four channels serve — carries the install forward. So nightly belongs in the registry for winget and Chocolatey even though their review queues cannot take a daily submission: a listing that lags the current nightly by days still delivers the current nightly within one updater check. How often each upstream actually receives a render — daily for the self-service tap and AUR, throttled for the reviewed queues — is the distribute workflow's rule set, not the registry's. Two facts that workflow will need are recorded here: winget compares the Add/Remove Programs version with its manifest, so a self-updated install reads as newer and is left alone; Chocolatey compares its own install record, so a refreshed listing reinstalls over a newer self-updated build — a regression of at most the listing's age, healed at the next updater check. Both unverified (see Risks). The Nix overlay is unaffected: it records all three channels and never self-updates.

- _Rejected_: leaving nightly out of the registry for reviewed channels — treats the listing as the update path when the app has one.
- _Rejected_: a cadence field in the registry — the registry says where a release channel goes; when is the distribute workflow's.

### D9. Templates are refused if they are stamped (2026-11-06)

`cargo xtask dist check` scans `distribution/` for a concrete tag (`v\d+\.\d+\.\d+`), a 64-hex digest, or a versioned `/api/v1/releases/` URL, and fails naming the file — excluding `distribution/nix/releases/`, whose job is to carry exactly those. Wired as `just dist::check` and into the pull-request workflow's check job.

### D10. Channel shapes (2026-11-08)

| Channel      | Files under `distribution/<channel>/`                   | Source        |
| ------------ | ------------------------------------------------------- | ------------- |
| `brew`       | `{{ flavor.id }}.rb`                                    | dmg, AppImage |
| `aur`        | `PKGBUILD`, `.SRCINFO`                                  | deb           |
| `winget`     | `{{ flavor.id }}{,.installer,.locale.en-US}.yaml`       | exe           |
| `chocolatey` | `{{ flavor.id }}.nuspec`, `tools/chocolateyinstall.ps1` | exe           |
| `nix`        | `overlay.nix`, `channel.nix`, `packages/`, `releases/`  | source build  |
| `kubernetes` | `base/`, `kind/`, `justfile`                            | —             |

All three release channels are registered on each of the four package channels (D4).

**Homebrew** — top level: `arch arm:`, `os macos: linux:`, one `url` with the extension from `on_system_conditional`, `sha256 arm:`/`x86_64_linux:`, `livecheck` with `strategy :json` reading `tag` from `/current`, `auto_updates true`. `on_macos`: `depends_on macos: :sequoia` (Tauri `minimumSystemVersion = "15"`), `arch: :arm64`, `app`, `uninstall quit:`, `zap` on `~/Library/Application Support/<identifier>` and `Caches`. `on_linux`: `depends_on arch: :x86_64`, `app_image` with `target:` the executable, `zap` on `~/.cache`, `~/.config`, `~/.local/share` for the identifier. No `license` stanza (not in the cask DSL). Validator: `brew style`, `brew audit --cask` for the host and under Linux simulation.

**AUR** — `pkgname` from the registry; the published deb extracted with bsdtar for binary, desktop entry, and icons; `pkgver` from the filter; `sha256sums` from the deb digest. `.SRCINFO` is templated rather than generated so it renders anywhere. Validator: `makepkg --printsrcinfo` diffed against the render, `namcap`, on Arch.

**winget** — `PackageIdentifier` from the registry; `InstallerType: nsis`, `Scope: user`, `InstallerSha256` from the exe digest, `ProductCode` from the bundle identifier (unverified; see Risks). Validator: `winget validate` on Windows.

**Chocolatey** — `<id>` from the registry; `url64` on the versioned exe, `checksum64` from the digest, `checksumType64 sha256`, `silentArgs /S`, `validExitCodes @(0)`; the NSIS uninstaller is registered, so the auto-uninstaller suffices. Validator: `choco pack` on Windows.

### D11. The Linux cask installs the AppImage through `app_image` (2026-11-06, reversed 2026-11-08)

Homebrew's cask DSL has a Linux-only `app_image` artifact (`Library/Homebrew/cask/artifact/appimage.rb`, in `ORDINARY_ARTIFACT_CLASSES`; Cask Cookbook: "`app_image` is Linux-only, `app` and `pkg` are macOS-only, and `binary` works on either"). Since Homebrew/brew#24343 (merged 2026-10-27) it installs the AppImage's embedded desktop entry into `${XDG_DATA_HOME:-~/.local/share}/applications`, with `Exec` rewritten to the installed path and the icon copied into the Caskroom, and removes them on uninstall. Tauri's AppImage embeds that desktop entry and icon. `auto_updates true` covers both OSes: the Tauri updater replaces an AppImage in place, the installed path and so the desktop entry's `Exec` do not change, and `brew upgrade` skips auto-updating casks without `--greedy`.

- _Why reversed_: the 2026-11-06 reading that the DSL could not install a desktop entry came from `dsl.rb`'s stanza list without opening the artifact classes.
- _Accepted limitation_: a Linuxbrew older than brew#24343's release installs the AppImage without the launcher entry until its next auto-update (daily by default).

## Risks / Trade-offs

- **The `deploy::` rename breaks muscle memory and three open changes** → the open changes are edited here; `just --list` shows `dist`; the kustomization comments name the new recipes.
- **`waypoints.nightly` on `main` does not evaluate between the merge and the next nightly** → accepted: no consumers, no CI evaluation, and the nightly's index pull request closes it.
- **Three more open changes cite `nix/` paths** → edited here; `release-groups-guards`' `paths = [ "nix/releases/" ]` ruleset would silently stop matching if missed.
- **`station-replication` rewrites the same requirement this change modifies** → its delta is edited to the new path here, so whichever archives second carries the other's text.
- **`artifact.file_name` assumes the download serves the stored name** → the release-delivery spec permits a readable Content-Disposition name for nightlies, which Homebrew honours; the versioned route sets none today. If that clause is ever implemented, the cask's `app_image` path breaks. Settle it there: the versioned URL serves the stored name, or `file_name` becomes "the name the download surface serves".
- **`.SRCINFO` is templated rather than generated** → the AUR check diffs it against `makepkg --printsrcinfo`, and the AUR rejects a mismatch at push time anyway.
- **Claims about external tools taken from general knowledge, not verified** → each is a render-and-validate task here and a verify-before-submit task for the distribute workflow:
  - winget compares the Add/Remove Programs version with the manifest and treats a newer installed version as current; what Tauri's NSIS writes as `DisplayVersion` for a prerelease tag.
  - winget's `ProductCode` equals Tauri's NSIS uninstall key (the bundle identifier).
  - Chocolatey's community feed accepts SemVer 2 prerelease identifiers; its auto-uninstaller handles an NSIS uninstaller without a script; `choco upgrade` reinstalls over a newer self-updated build.
  - `brew audit`'s OS/architecture simulation flags; `livecheck`'s `:json` strategy.
  - Verified 2026-11-06 against `dsl.rb` on `main`: `os`, `arch`, `on_system_conditional`, `sha256 arm:`/`x86_64_linux:`, no `license` stanza, a second `url` raises.
- **Digests widen the wire format** → additive, optional in the website type.
- **Templates with no control flow push logic into the context** → the context grows a registry lookup and three filters; testable in Rust.
- **`brew audit`, `makepkg`, `winget validate`, `choco pack` are platform-bound** → recipes report skipped with a reason; in CI only `brew audit` runs (macOS runner), and the check-all log says which ran.

## Migration Plan

1. Restore the working tree's `distribution/` sketches and `nix/justfile` to `ce4067aca`.
2. Move the deployment tree (one commit): `deploy/**` → `distribution/kubernetes/**`, root `mod dist`, `distribution/justfile`, comment updates, `devenv.nix` lane rename and its regenerated mise projection.
3. Move the Nix channel and the devenv modules (one commit): `nix/{overlay,channel}.nix`, `nix/{packages,releases}/` → `distribution/nix/`; `nix/devenv/` → `tooling/devenv/`; `flake.nix`, `devenv.nix`, `nix-pins`, `index.rs`, the release-index workflow, `openspec/config.yaml`. `nix build .#nightly` is expected to fail until the next nightly.
4. Amend the spec paths and the six open changes.
5. Manifest digests and the manifest route.
6. Template engine, the registry, `dist render`, `dist check`, treefmt coverage, then the four channel templates with their validation recipes, one channel per commit.
7. Rollback: steps 2–3 are pure moves; steps 5–6 are additive.

## Appendix A — Channel notes (non-normative)

Judgments recorded 2026-11-06–08 for reference, not requirements. Reach is relative to likely early adopters (developers and technical knowledge workers on macOS, Arch, Ubuntu/Fedora, Windows). Claims about external tools here are unverified unless the Risks section says otherwise.

| Channel              | Reach    | Setup  | Per release        | Review           | Status   |
| -------------------- | -------- | ------ | ------------------ | ---------------- | -------- |
| Direct download      | everyone | —      | —                  | ours             | exists   |
| Nix overlay          | small    | —      | index write        | ours             | exists   |
| Homebrew             | high     | low    | one commit         | ours             | chosen   |
| AUR `-bin`           | high     | low    | one push/flavor    | ours             | chosen   |
| winget               | highest  | medium | one PR/version     | days, theirs     | chosen   |
| Chocolatey           | medium   | medium | one push/version   | days–weeks       | chosen   |
| apt/dnf repos        | medium   | medium | re-sign, re-index  | ours             | deferred |
| Snap                 | medium   | medium | one upload/flavor  | ours             | deferred |
| Flathub              | broadest | high   | one PR/version     | days–weeks       | deferred |
| Debian/Fedora source | medium   | high   | one upload/release | hours (COPR/OBS) | deferred |
| scoop                | small    | low    | one commit         | ours             | deferred |

**Direct download** — download, drag or run; the Tauri updater handles every update after. The deb and rpm stay in this set for Debian/Ubuntu and Fedora users.

**Nix overlay** — a hard requirement; a store-installed build notifies about updates and never self-updates.

**Homebrew** — `brew install --cask`, then `brew upgrade` or the in-app updater. Setup is a tap repository, the template, and `brew audit` in CI. Reach is high among the primary audience; Linuxbrew adds the Linux users who already run it.

**AUR `-bin`** — `yay -S`; updates with the system. Setup is three AUR packages and an SSH key; self-service, so a daily nightly is normal for a `-bin` package. Arch users expect it.

**winget** — `winget install`; one PR per version, bot-validated, human-merged over days. The real prerequisite is a Windows signing identity: an unsigned installer trips SmartScreen and may draw reviewer pushback. Highest reach by OS count.

**Chocolatey** — `choco install`; a package's first versions are human-moderated (days to weeks), later ones pass automated validation. Same signing gate as winget. Reach is Windows developers and IT.

**Self-hosted apt/dnf repositories** (deferred) — the deb and rpm already exist, so this is the cheapest way to make them update with the system; needs repository metadata, hosting beside the store, and a GPG key the vault does not hold. Deferred until it is clear which Linux package managers users want; the direct downloads and the updater serve them meanwhile.

**Snap** (deferred) — Ubuntu by default, contested elsewhere. One snap with release channels as risk tracks means they are not co-installable; strict confinement of WebKitGTK needs debugging; a sandboxed install cannot self-update, so the build would first need the notify-only updater behaviour the Nix build has.

**Flathub** (deferred) — the broadest Linux reach. High setup: AppStream metainfo with screenshots, permission review, a Flathub repository, likely pressure to build from source (offline cargo plus pnpm sources). Days to weeks of review per version; nightly impossible; the same notify-only prerequisite as snap.

**Debian / Fedora source packaging** (deferred) — needs a vendored-source tarball because COPR/PPA/OBS builders have no network, and two build systems. Official Debian inclusion is out of reach while every pnpm dependency would need packaging; Fedora proper needs a sponsor and bundled-crate exceptions. If pursued, COPR/OBS is the venue.

**scoop** (deferred) — a small Windows subset already served by winget and Chocolatey; wants a portable archive, which Windows does not publish.
