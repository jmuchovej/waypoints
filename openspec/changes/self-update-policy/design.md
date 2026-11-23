# Design

## Context

See proposal.md — Why. `src-tauri/src/builders/menus.rs` has `check_for_updates`, whose two `Ok(Some(update))` arms are selected by `#[cfg(feature = "self-update")]`: with the feature, download, install, restart; without it, a dialog naming the available version and saying the build updates through its package manager. `src-tauri/Cargo.toml` lists `self-update` in `default`. `distribution/nix/packages/waypoints/package.nix` builds with `buildNoDefaultFeatures = true` and `buildFeatures = ["desktop", <flavor>]`, which is how the Nix build loses the feature. Each flavor's binary (`src-tauri/src/bin/*.rs`) already inspects argv before starting Tauri: `--version`/`-V` is answered only as the sole argument, because deep links arrive in argv on Linux and Windows, the single-instance plugin forwards a second instance's argv to the first, and macOS passes process serial numbers on some launch paths. Settings live in a JSON store (`src-tauri/src/settings.rs`: `settings.json`), read from the frontend's settings pages under `src/app/pages/settings/`.

## Goals / Non-Goals

**Goals:**

- One binary per release; the install decides at launch whether it may rewrite itself.
- The user owns the setting; a package can only lower its ceiling.
- A capped user learns why from the app, and the details from one page that can grow.

**Non-Goals:**

- Changing the check's schedule, the endpoints, or the updater's accepted payloads.
- Any flag beyond the boolean pair.

## Decisions

### D1. A three-valued setting with a ceiling, not a value, from the launcher (2026-11-20)

`self_update ∈ {off, notify, on}`, default `on`, stored under its own key in `settings.json`. The effective policy is `min(setting, cap)` where the cap is `notify` when `--no-self-update` was passed and `on` otherwise. `off` therefore survives a cap — a user who wants no check keeps that — and `on` under a cap becomes `notify`. The stored value is never written by the flag, so a profile moved to an unmanaged install gets the user's choice back.

- _Rejected_: the flag setting the value (`--self-update=notify`) — overwrites a user's `off`, and leaves the setting meaning different things on different machines.
- _Rejected_: a marker file the package installs — launch arguments are already how every manager here configures a program; a file is a second mechanism and a second thing to clean up on uninstall.

### D2. `--[no-]self-update`, parsed as a literal anywhere in argv (2026-11-20)

The bins strip every argument equal to `--no-self-update` or `--self-update` from argv before anything else looks at it, remember the last one seen (so a wrapper's `--no-self-update` can be overridden by a later `--self-update` for debugging), and hand the remainder on unchanged. A deep link (`waypoints://…`) is never equal to either literal, so the existing rule — interpret nothing else — holds; `--version` keeps its sole-argument rule. The policy is recorded once in the primary instance; argv forwarded by the single-instance plugin is used for its deep links and nothing else.

- _Rejected_: `tauri-plugin-cli` or clap — a parser for two literals, and either would want to own `--version` and reject the deep-link argument it does not recognize.
- _Rejected_: an environment variable — equally easy for a wrapper, harder to see in a process listing or a desktop entry; the flag can gain an env-var alias later if a packaging format needs one.

### D3. The `self-update` feature is removed; `check_for_updates` branches at runtime (2026-11-20)

The two `cfg` arms become one `match` on the effective policy: `off` returns before `updater.check()`; `notify` shows the existing dialog; `on` downloads, installs, and restarts. `Cargo.toml` drops the feature from `[features]` and `default`; `package.nix` keeps only the flavor in `buildFeatures` (and so the Nix build is the same code as the published artifact), and adds `--add-flags --no-self-update` to the wrapper it already uses to put the binary on `PATH`.

- _Rejected_: keeping the feature as a belt-and-braces guarantee for Nix — two binaries for one release, which is the problem; the wrapper is the package, and the `nix-overlay` spec is amended to say so.

### D4. The notice is generic; the page is specific (2026-11-20)

The settings page renders the `on` option disabled with: _Self-updating is disabled because this copy of Waypoints is managed by a package manager, which delivers updates itself. Learn more._ The link is a stable website URL whose page lists the channels that launch this way. The app never names a distribution or manager, so adding a channel is a page edit, not a release.

- _Rejected_: an optional reason on the flag (`--no-self-update=nix`) — possible later without breaking the boolean form; not needed to ship.

### D5. What the frontend sees (2026-11-20)

One command returns `{ setting, cap, effective }`; one sets `setting`. The settings control binds to `setting`, disables values above `cap`, and shows the notice when `cap < on`. Nothing else in the frontend reads the policy.

## Risks / Trade-offs

- **The guarantee moves from the compiler to the launcher** → a Nix user who runs the store binary directly, bypassing the wrapper, gets the full policy. Accepted: the wrapper is the package's entry point on `PATH` and in the desktop entry, and the user who digs past it has chosen to.
- **A future packaging format may not be able to pass arguments** → the env-var alias in D2 is the fallback; nothing here precludes it.
- **The notice text ships in the binary** → wording changes are a release; the page carries anything that changes often.

## Open Questions

- The docs page's slug. It is fixed once, since the binary links to it; a redirect covers a later move.
