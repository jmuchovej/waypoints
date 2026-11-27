// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The tooling's shared vocabulary — the paths, names, and identities it uses
//! as a whole — defined once. A constant private to one module's
//! implementation stays beside it. Paths are relative to the workspace root.

// The app.

/// The crate the shipped app is built from: the app group's own crate, whose
/// library target exists for the mobile bindings and whose contract is the
/// covenant rather than a Rust API.
pub const APP: &str = "Waypoints";
/// The app's group in the group file; [`APP`] is its crate. This is one
/// repository with one app, and what a group is — the app or a library —
/// decides how it is versioned.
pub const APP_GROUP: &str = "waypoints";
/// The manifest whose `version` is the app's and, by its suffix, the channel
/// the tree ships as. Tauri reads it when `Tauri.toml` leaves the version
/// unset.
pub const APP_MANIFEST: &str = "src-tauri/Cargo.toml";
/// The workspace's root manifest: its members, and the one place a library
/// dependency is declared.
pub const ROOT_MANIFEST: &str = "Cargo.toml";
/// Records every workspace crate's version, so a version write moves it too.
pub const LOCKFILE: &str = "Cargo.lock";

// The group file.

/// The group file.
pub const GROUP_FILE: &str = "tooling/release-groups.toml";
/// The group file's generated schema.
pub const SCHEMA_FILE: &str = "tooling/schemas/release-groups.schema.json";
/// The treewide formatting passes `git blame` skips, one full commit id per
/// line, at the name GitHub and the editors read it under.
pub const BLAME_IGNORE_FILE: &str = ".git-blame-ignore-revs";

// The repository and its bot.

/// Where every group's tag is minted.
pub const REPOSITORY: &str = "https://github.com/jmuchovej/waypoints";
/// The release bot's login: the only author who writes a version or a
/// bookkeeping subject, and whose every commit is bookkeeping to the planners.
pub const RELEASE_BOT: &str = "wp-botswain[bot]";
/// The address the bot's commits carry.
pub const RELEASE_BOT_EMAIL: &str = "3633092+wp-botswain[bot]@users.noreply.github.com";
/// The default branch: where a cut's bump lands and every group's tag is
/// minted.
pub const MAIN: &str = "main";
/// A release transition's pull requests come from here, one branch per
/// target, since a cut opens one against the series and one against `main`.
pub const RELEASE_PLAN_HEAD_PREFIX: &str = "botswain/release-plan/";
/// A release-index entry lands from here, one branch per channel, so two
/// channels publishing at once never overwrite each other's write.
pub const RELEASE_INDEX_HEAD_PREFIX: &str = "botswain/release-index/";
/// A withdrawn release's revert lands from here, one branch per target —
/// the series it undoes, or `main` for a cut's write there — which is also
/// how a bump finds the revert still open.
pub const RELEASE_UNDO_HEAD_PREFIX: &str = "botswain/release-undo/";
/// The weekly devenv update's branch, reused so a still-unmerged update is
/// refreshed in place rather than stacking a second pull request.
pub const DEVENV_UPDATE_BRANCH: &str = "devenv/update";

// The generated tree.

// A macro rather than a `const`, so `concat!` can build the glob below from
// the same spelling.
macro_rules! workflows_dir {
    () => {
        ".github/workflows"
    };
}

/// The tree GitHub reads workflows and actions from, which the workflow
/// linter audits whole.
pub const GITHUB_DIR: &str = ".github";
/// Where the generated workflows live.
pub const WORKFLOWS_DIR: &str = workflows_dir!();
macro_rules! actions_dir {
    () => {
        ".github/actions"
    };
}
/// Where the generated composite actions live, one directory per action.
pub const ACTIONS_DIR: &str = actions_dir!();
/// Where OpenTofu auto-loads `var.github`.
pub const TFVARS: &str = "tooling/infra/github.auto.tfvars.json";
/// The TypeScript bindings `generate bindings` writes.
pub const BINDINGS: [&str; 3] = [
    "crates/domain/typescript/types.g.ts",
    "crates/domain/typescript/schema.g.ts",
    "crates/cartograph/typescript/types.g.ts",
];
/// The roots every icon `generate icons` places falls under: wide enough that
/// a removed icon shows as a deletion.
pub const ICON_ROOTS: [&str; 4] = [
    "src-tauri/assets/icons/app/**",
    "src-tauri/gen/apple/Assets.xcassets/*.appiconset/**",
    "src-tauri/gen/android/app/src/*/res/mipmap-*/ic_launcher*",
    "src-tauri/gen/android/app/src/*/res/values/ic_launcher_background.xml",
];
/// Where the third-party notices are committed.
pub const NOTICES: &str = "docs/licensing/THIRD-PARTY-NOTICES.md";

// The licensing inputs the checks read.

/// The third-party registry.
pub const THIRD_PARTY_REGISTRY: &str = ".config/third-party.toml";
/// cargo-about's configuration.
pub const ABOUT_CONFIG: &str = ".config/about.toml";
/// cargo-about's template, which renders the notices' Rust section.
pub const ABOUT_TEMPLATE: &str = ".config/about.hbs";

// The release store.

/// The release index: one committed file per channel.
pub const INDEX_DIR: &str = "distribution/nix/releases";
/// The flat directory `release distributables rename` fills and
/// `release distributables upload --from` reads.
pub const STAGE_DIR: &str = "release";
/// Upload metadata the iOS build stages beside its ipa: TestFlight reads it
/// off macOS, and it never becomes a store object.
pub const APP_STORE_INFO: &str = "AppStoreInfo.plist";

// Scratch.

/// Where the API floor's measurement builds, under the workspace's `target/`:
/// the current tree's rustdoc, and a worktree per baseline tag with its own.
pub const SEMVER_SCRATCH: &str = "target/semver-checks";

// The ledger of generated outputs.

/// One generator: the sources it reads and the outputs it owns, as globs —
/// `*` within a component and `**` across, which GitHub's `paths:` filter,
/// git's pathspec, and jj's `glob:` fileset all read — so the lockstep
/// check's trigger, its diff, and `ls managed` derive from one list.
pub struct Generated {
    pub name: &'static str,
    pub sources: &'static [&'static str],
    pub outputs: &'static [&'static str],
}

/// The source of everything xtask itself generates.
const XTASK: &str = "tooling/xtask/**";

/// Every generator, in the order the check lists them. devenv's outputs come
/// from `devenv info` at run time, so its entry declares none.
pub const GENERATED: &[Generated] = &[
    Generated {
        name: "workflows",
        sources: &["devenv.nix", "devenv.yaml", "devenv.lock", XTASK],
        outputs: &[
            concat!(workflows_dir!(), "/**"),
            concat!(actions_dir!(), "/**"),
            TFVARS,
        ],
    },
    Generated {
        name: "bindings",
        sources: &["crates/domain/**", "crates/cartograph/**"],
        outputs: &BINDINGS,
    },
    Generated {
        name: "icons",
        sources: &["src-tauri/res/**"],
        outputs: &ICON_ROOTS,
    },
    Generated {
        name: "notices",
        sources: &[
            LOCKFILE,
            "pnpm-lock.yaml",
            THIRD_PARTY_REGISTRY,
            ABOUT_CONFIG,
            ABOUT_TEMPLATE,
        ],
        outputs: &[NOTICES],
    },
    Generated {
        name: "schema",
        sources: &[XTASK],
        outputs: &[SCHEMA_FILE],
    },
    Generated {
        name: "devenv",
        sources: &["devenv.nix", "devenv.yaml", "devenv.lock"],
        outputs: &[],
    },
];
