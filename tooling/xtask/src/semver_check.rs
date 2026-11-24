// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The API floor's measurement: `cargo-semver-checks`, as a library, over
//! each library member of a group, against the group's last tag, under the
//! stable flavor — the surface every series consumes (library-minting design
//! D5). The `semver` job runs it on a pull request; the cut reads it as
//! evidence.
//!
//! The checker runs in [`measure`], over rustdoc JSON this module builds for
//! each side; what to run it on, and what its answer means against a group's
//! header, is decided here where tests can reach it.

use crate::{
    constants::{APP, APP_GROUP, SEMVER_SCRATCH},
    release_groups::{GroupFile, tag_prefix},
    releases::{history::Commit as Walked, version::Version},
    tools::git,
    version_integrity::{Change, PullRequest},
};
use anyhow::{Context as _, Result, bail};
use cargo_metadata::{Metadata, TargetKind};
use cargo_semver_checks::{Check, GlobalConfig, ReleaseType, Rustdoc};
use git_conventional::Commit as Conventional;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// The features each side is documented with: the stable flavor, over the
/// crate's defaults as cargo leaves them. Never every feature at once —
/// every flavor at once, which no crate compiles — and never the flavor
/// alone, which would drop a crate's defaults from its surface.
pub const FEATURES: [&str; 2] = ["--features", "stable"];

/// A crate the checker can read: a library target that is not a procedural
/// macro.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    /// Relative to the workspace root.
    pub directory: String,
    pub package: String,
}

/// What the check requires of a member's next release, from its surface
/// against the baseline: a patch where nothing it lints changed, a minor
/// where the surface grew, a major where it broke. The bumper right-shifts
/// these before `1.0`, as it does a header's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Required {
    Patch,
    Minor,
    Major,
}

impl From<Option<ReleaseType>> for Required {
    fn from(required: Option<ReleaseType>) -> Self {
        match required {
            Some(ReleaseType::Major) => Self::Major,
            Some(ReleaseType::Minor) => Self::Minor,
            _ => Self::Patch,
        }
    }
}

/// What the check decides for one group. `breaks` are the members whose
/// surface broke; `grew` the ones whose surface grew, which breaks nothing
/// and which the next cut reads as a feature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// No tag to measure against: the first cut mints the baseline.
    Skipped { group: String, prefix: String },
    /// Nothing broke, or the group's header declared the break.
    Passed {
        group: String,
        breaks: Vec<String>,
        grew: Vec<String>,
    },
    /// The app's surface broke undeclared: reported, since its contract is
    /// the covenant and not a Rust API.
    Informational {
        group: String,
        breaks: Vec<String>,
        grew: Vec<String>,
    },
    /// A library's surface broke, and its header did not say so.
    Failed {
        group: String,
        breaks: Vec<String>,
        grew: Vec<String>,
    },
}

impl Verdict {
    pub fn failed(&self) -> bool {
        matches!(self, Verdict::Failed { .. })
    }

    /// One line for the job log, and for the failure it names.
    pub fn describe(&self) -> String {
        let growth = |grew: &[String]| {
            if grew.is_empty() {
                String::new()
            } else {
                format!(
                    "; the surface of `{}` grew, which the next cut reads as a feature",
                    grew.join("`, `")
                )
            }
        };
        match self {
            Verdict::Skipped { group, prefix } => format!(
                "`{group}` has no `{prefix}*` tag: no baseline until the first cut mints one; its declarations and the fixture floor still apply"
            ),
            Verdict::Passed {
                group,
                breaks,
                grew,
            } if breaks.is_empty() => {
                format!("`{group}`: no break against its last tag{}", growth(grew))
            }
            Verdict::Passed {
                group,
                breaks,
                grew,
            } => format!(
                "`{group}`: breaks in `{}`, declared by its header{}",
                breaks.join("`, `"),
                growth(grew)
            ),
            Verdict::Informational {
                group,
                breaks,
                grew,
            } => format!(
                "`{group}`: breaks in `{}`; information only, since the app's contract is the covenant{}",
                breaks.join("`, `"),
                growth(grew)
            ),
            Verdict::Failed {
                group,
                breaks,
                grew,
            } => format!(
                "`{group}`: breaks in `{}` that its header does not declare; add `!` to the group's header, or make the change compatible{}",
                breaks.join("`, `"),
                growth(grew)
            ),
        }
    }
}

/// The verdict for `group` from what the check `measured` of each member,
/// whether its header `declared` a break, and whether it is the app, whose
/// Rust API is nobody's contract. Only a break fails: a surface that grew
/// is information the cut reads.
pub fn verdict(
    group: &str,
    measured: Vec<(String, Required)>,
    declared: bool,
    is_app: bool,
) -> Verdict {
    let group = group.to_owned();
    let mut breaks = Vec::new();
    let mut grew = Vec::new();
    for (directory, required) in measured {
        match required {
            Required::Major => breaks.push(directory),
            Required::Minor => grew.push(directory),
            Required::Patch => {}
        }
    }
    if breaks.is_empty() || declared {
        Verdict::Passed {
            group,
            breaks,
            grew,
        }
    } else if is_app {
        Verdict::Informational {
            group,
            breaks,
            grew,
        }
    } else {
        Verdict::Failed {
            group,
            breaks,
            grew,
        }
    }
}

/// The tag `group` is measured against: the newest of its `tags` that is a
/// release — a library's every tag, the app's stable ones, never a nightly
/// or a preview.
pub fn baseline(prefix: &str, tags: &[String]) -> Option<String> {
    tags.iter()
        .filter_map(|tag| tag.strip_prefix(prefix))
        .filter_map(|rest| Version::parse(rest).ok())
        .filter(Version::is_stable)
        .max()
        .map(|version| format!("{prefix}{version}"))
}

/// The members of `group` the checker can read: each with a library target
/// and no procedural macro, which has no surface to measure and is
/// declared-only. The app's own crate is left out of its group: its library
/// target exists for the mobile bindings, its contract is the covenant, and
/// its build drags the GUI toolchain in.
pub fn library_members(
    metadata: &Metadata,
    groups: &GroupFile,
    group: &str,
) -> Result<Vec<Member>> {
    let root = metadata.workspace_root.as_std_path();
    let named = groups
        .group
        .get(group)
        .with_context(|| format!("no group named `{group}`"))?;
    let is_app_group = group == APP_GROUP;

    let mut members = Vec::new();
    for package in metadata.workspace_packages() {
        let Some(directory) = package
            .manifest_path
            .as_std_path()
            .parent()
            .and_then(|dir| dir.strip_prefix(root).ok())
            .map(|dir| dir.to_string_lossy().into_owned())
        else {
            continue;
        };
        let is_app = is_app_group && package.name.as_str() == APP;
        if !named.members.contains(&directory) || is_app {
            continue;
        }
        let kinds: Vec<&TargetKind> = package
            .targets
            .iter()
            .flat_map(|target| &target.kind)
            .collect();
        let readable =
            kinds.contains(&&TargetKind::Lib) && !kinds.contains(&&TargetKind::ProcMacro);
        if readable {
            members.push(Member {
                directory,
                package: package.name.to_string(),
            });
        }
    }
    // By directory, as the group file lists them: cargo's order is its own.
    members.sort_by(|a, b| a.directory.cmp(&b.directory));
    Ok(members)
}

/// The command that documents `member`, in the tree at `root`, as rustdoc
/// JSON under `target_dir` — one side of a measurement, under [`FEATURES`].
///
/// Built in the crate's own workspace, with its lockfile and its `[patch]`
/// entries. The tool's own build is a throwaway project depending on the
/// crate, which resolves every dependency afresh and fails on what the
/// lockfile pins.
pub fn rustdoc_command(root: &Path, member: &Member, target_dir: &Path) -> Vec<String> {
    let manifest = root.join(&member.directory).join("Cargo.toml");
    let mut args: Vec<String> = [
        "cargo",
        "rustdoc",
        "--manifest-path",
        &manifest.to_string_lossy(),
        "--lib",
        "--target-dir",
        &target_dir.to_string_lossy(),
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    args.extend(FEATURES.iter().map(|flag| (*flag).to_owned()));
    args.extend(
        [
            "--",
            "-Zunstable-options",
            "--output-format",
            "json",
            "--cap-lints",
            "allow",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    args
}

/// Documents `member` in the tree at `root` into `target_dir`, and names the
/// JSON it wrote.
fn rustdoc_json(root: &Path, member: &Member, target_dir: &Path) -> Result<PathBuf> {
    let args = rustdoc_command(root, member, target_dir);
    let status = Command::new(&args[0])
        .args(&args[1..])
        // The stable toolchain emits rustdoc JSON only as an unstable option;
        // this is how the tool itself asks for it.
        .env("RUSTC_BOOTSTRAP", "1")
        .status()
        .context("running `cargo rustdoc`")?;
    if !status.success() {
        bail!(
            "`cargo rustdoc` failed on {} in {} ({status})",
            member.directory,
            root.display()
        );
    }
    let json = target_dir
        .join("doc")
        .join(format!("{}.json", member.package.replace('-', "_")));
    if !json.exists() {
        bail!("`cargo rustdoc` wrote no {}", json.display());
    }
    Ok(json)
}

/// The tree at `tag`, as a detached worktree under the scratch directory,
/// made afresh.
fn baseline_tree(root: &Path, tag: &str) -> Result<PathBuf> {
    let path = root.join(SEMVER_SCRATCH).join("baseline").join(tag);
    let shown = path.to_string_lossy().into_owned();
    if path.exists() {
        // A registered worktree, or a leftover directory: either way, gone.
        let _ = git::run(&["worktree", "remove", "--force", &shown]);
        if path.exists() {
            fs::remove_dir_all(&path).with_context(|| format!("removing {shown}"))?;
        }
    }
    git::run(&["worktree", "prune"])?;
    git::run(&["worktree", "add", "--detach", &shown, tag])
        .with_context(|| format!("checking out {tag} as a worktree"))?;
    Ok(path)
}

/// Whether `pull_request` declares a break for `group`: a `!` on the header
/// the group reads — the subject of a single-group change, or in a summary
/// the body header scoped to the group — as the planners read a landed
/// commit.
pub fn declares_break(
    pull_request: &PullRequest,
    changes: &[Change],
    group: &str,
    groups: &GroupFile,
) -> bool {
    let paths: Vec<String> = changes.iter().map(|change| change.path.clone()).collect();
    let landed = Walked {
        sha: String::new(),
        subject: pull_request.title.trim().to_owned(),
        body: pull_request.description(),
        covenant_break: pull_request.covenant_break,
        groups: groups.touched(paths.iter().map(String::as_str)),
        paths,
    };
    landed
        .header_for(group, groups)
        .is_some_and(|header| Conventional::parse(&header).is_ok_and(|header| header.breaking()))
}

/// The tag prefix of `group`, for its baseline and its skip message.
pub fn prefix<'a>(groups: &'a GroupFile, group: &str) -> Result<&'a str> {
    let named = groups
        .group
        .get(group)
        .with_context(|| format!("no group named `{group}`"))?;
    tag_prefix(&named.tag)
}

/// Measures every readable member of `group` against `tag`: each directory
/// with what its surface requires of the next release. The tag's tree is
/// checked out once for the group.
pub fn measure_group(
    metadata: &Metadata,
    groups: &GroupFile,
    group: &str,
    tag: &str,
) -> Result<Vec<(String, Required)>> {
    let root = metadata.workspace_root.as_std_path();
    let members = library_members(metadata, groups, group)?;
    if members.is_empty() {
        return Ok(Vec::new());
    }
    let baseline = baseline_tree(root, tag)?;
    let mut measured = Vec::new();
    for member in members {
        println!("Measuring {} against {tag}", member.directory);
        let required = measure(root, &baseline, &member)?;
        measured.push((member.directory, required));
    }
    Ok(measured)
}

/// Documents `member` in the tree at `root` and in the `baseline` tree and
/// compares the two through `cargo-semver-checks`, which prints its report
/// as it goes, and says what the surface requires of the next release. A
/// member the baseline does not carry yet has no surface to break. The
/// checker's own failure to measure is an error, not a finding.
pub fn measure(root: &Path, baseline: &Path, member: &Member) -> Result<Required> {
    if !baseline.join(&member.directory).join("Cargo.toml").exists() {
        println!(
            "  {} is new since the baseline; nothing to break",
            member.directory
        );
        return Ok(Required::Patch);
    }
    let current = rustdoc_json(root, member, &root.join(SEMVER_SCRATCH).join("current"))?;
    let before = rustdoc_json(baseline, member, &baseline.join("target"))?;

    // Two rustdoc files, so the checker builds nothing and asks no rustc.
    let mut check = Check::new(Rustdoc::from_path(current));
    check.set_baseline(Rustdoc::from_path(before));
    let mut config = GlobalConfig::new();
    config.set_log_level(Some(log::Level::Info));
    let report = check
        .check_release(&mut config)
        .with_context(|| format!("measuring `{}`", member.directory))?;
    let (_, verdict) =
        report.crate_reports().iter().next().with_context(|| {
            format!("`{}` was measured, but reported nothing", member.directory)
        })?;
    Ok(Required::from(verdict.required_bump()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        release_groups,
        version_integrity::{Status, User},
        workspace,
    };

    fn groups() -> GroupFile {
        release_groups::load(&workspace::load().unwrap()).unwrap()
    }

    fn pull_request(title: &str, body: &str) -> PullRequest {
        let mut pull_request = PullRequest {
            title: title.to_owned(),
            body: Some(body.to_owned()),
            user: User {
                login: "someone".to_owned(),
            },
            covenant_break: false,
        };
        pull_request.covenant_break =
            crate::version_integrity::declares_covenant_break(&pull_request.message()).unwrap();
        pull_request
    }

    fn changes(paths: &[&str]) -> Vec<Change> {
        paths
            .iter()
            .map(|path| Change {
                status: Status::Modified,
                path: (*path).to_owned(),
            })
            .collect()
    }

    fn tags(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    /// Scenario _Procedural macro member_, on this workspace: `bedrock-derive`
    /// has no surface to measure and is left out; every other library member
    /// is read; the app's own crate is left out of its group.
    #[test]
    fn the_readable_members_are_the_library_targets() {
        let metadata = workspace::load().unwrap();
        let groups = groups();
        let directories = |group: &str| {
            library_members(&metadata, &groups, group)
                .unwrap()
                .into_iter()
                .map(|member| member.directory)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            directories("bedrock"),
            ["crates/bedrock", "crates/bedrock-core"]
        );
        assert_eq!(
            directories("outpost"),
            [
                "crates/outpost",
                "crates/outpost-fabric",
                "crates/outpost-hold",
                "crates/outpost-protocol",
            ]
        );
        assert_eq!(directories("domain"), ["crates/domain"]);
        assert_eq!(directories("waypoints"), ["crates/cartograph"]);
    }

    /// A library's baseline is its newest tag; the app's is its newest
    /// stable one, never a nightly.
    #[test]
    fn the_baseline_is_the_newest_release_tag() {
        assert_eq!(
            baseline(
                "bedrock-v",
                &tags(&["bedrock-v0.2.0", "bedrock-v0.3.0", "outpost-v9.0.0"])
            ),
            Some("bedrock-v0.3.0".to_owned())
        );
        assert_eq!(
            baseline(
                "v",
                &tags(&["v0.1.0-nightly.20261108", "v0.1.0", "v0.1.1-pre.1"])
            ),
            Some("v0.1.0".to_owned())
        );
        assert_eq!(baseline("v", &tags(&["v0.1.0-nightly.20261108"])), None);
        assert_eq!(baseline("transload-v", &[]), None);
    }

    /// Scenario _A nightly-gated break is not evidence_, at the commands:
    /// each side is documented as rustdoc JSON under the stable flavor over
    /// the crate's defaults, with nothing enabling every feature, and the
    /// tool compares the two JSONs rather than building anything itself.
    #[test]
    fn the_check_measures_the_stable_surface() {
        let member = Member {
            directory: "crates/bedrock".to_owned(),
            package: "bedrock".to_owned(),
        };
        let args = rustdoc_command(
            Path::new("/tree"),
            &member,
            Path::new("/tree/target/semver-checks/current"),
        );
        assert_eq!(args[..2], ["cargo", "rustdoc"].map(str::to_owned));
        assert!(args.contains(&"/tree/crates/bedrock/Cargo.toml".to_owned()));
        assert!(args.contains(&"--lib".to_owned()));
        let features: Vec<&str> = args
            .windows(2)
            .filter(|pair| pair[0] == "--features")
            .map(|pair| pair[1].as_str())
            .collect();
        assert_eq!(features, ["stable"]);
        assert!(!args.iter().any(|arg| arg == "--all-features"));
        assert!(!args.iter().any(|arg| arg == "--no-default-features"));
        assert!(!args.iter().any(|arg| arg.contains("nightly")));
        let rustdoc = args.iter().position(|arg| arg == "--").unwrap();
        assert!(args[rustdoc..].contains(&"--output-format".to_owned()));
        assert!(args[rustdoc..].contains(&"json".to_owned()));
    }

    /// The checker's answer, read as what the next release requires: a
    /// break is a major, growth a minor, anything else — nothing found, or
    /// a kind of bump this reading does not know — a patch.
    #[test]
    fn the_checker_s_answer_is_read_as_a_required_bump() {
        assert_eq!(Required::from(Some(ReleaseType::Major)), Required::Major);
        assert_eq!(Required::from(Some(ReleaseType::Minor)), Required::Minor);
        assert_eq!(Required::from(Some(ReleaseType::Patch)), Required::Patch);
        assert_eq!(Required::from(None), Required::Patch);
        assert!(Required::Major > Required::Minor && Required::Minor > Required::Patch);
    }

    /// Scenario _Break in an undeclared group_: the summary declares a break
    /// for `bedrock` alone, so a detected break in `outpost` fails, naming
    /// it.
    #[test]
    fn a_break_in_an_undeclared_group_fails() {
        let groups = groups();
        let pull_request = pull_request(
            "summary: rename the filter API",
            "feat(bedrock)!: rename Filter::eq\nrefactor(outpost): follow the rename",
        );
        let changes = changes(&["crates/bedrock/src/lib.rs", "crates/outpost/src/lib.rs"]);
        assert!(declares_break(&pull_request, &changes, "bedrock", &groups));
        assert!(!declares_break(&pull_request, &changes, "outpost", &groups));

        let broke = |directory: &str| vec![(directory.to_owned(), Required::Major)];
        let outpost = verdict("outpost", broke("crates/outpost"), false, false);
        assert!(outpost.failed());
        let line = outpost.describe();
        assert!(
            line.starts_with("`outpost`: breaks in `crates/outpost`"),
            "{line}"
        );
        assert!(line.contains("does not declare"), "{line}");

        let bedrock = verdict("bedrock", broke("crates/bedrock"), true, false);
        assert!(!bedrock.failed());
        assert!(bedrock.describe().contains("declared by its header"));
    }

    /// A surface that grew is not a break: the check passes without a
    /// declaration and says what grew, and a member at a patch says nothing.
    #[test]
    fn growth_passes_and_is_named() {
        let measured = vec![
            ("crates/bedrock".to_owned(), Required::Minor),
            ("crates/bedrock-core".to_owned(), Required::Patch),
        ];
        let verdict = verdict("bedrock", measured, false, false);
        assert!(!verdict.failed());
        assert_eq!(
            verdict,
            Verdict::Passed {
                group: "bedrock".to_owned(),
                breaks: Vec::new(),
                grew: vec!["crates/bedrock".to_owned()],
            }
        );
        let line = verdict.describe();
        assert!(line.starts_with("`bedrock`: no break"), "{line}");
        assert!(line.contains("`crates/bedrock` grew"), "{line}");
        assert!(!line.contains("bedrock-core"), "{line}");
    }

    /// Scenario _Over-declared break_: a declared break with nothing detected
    /// passes — behaviour changes escape the checker, and the header is
    /// trusted.
    #[test]
    fn an_over_declared_break_passes() {
        let groups = groups();
        let pull_request = pull_request("fix(bedrock)!: tighten the query", "");
        let changes = changes(&["crates/bedrock/src/query.rs"]);
        assert!(declares_break(&pull_request, &changes, "bedrock", &groups));
        let verdict = verdict("bedrock", Vec::new(), true, false);
        assert_eq!(
            verdict,
            Verdict::Passed {
                group: "bedrock".to_owned(),
                breaks: Vec::new(),
                grew: Vec::new(),
            }
        );
        assert_eq!(
            verdict.describe(),
            "`bedrock`: no break against its last tag"
        );
    }

    /// Scenario _App API break is information only_.
    #[test]
    fn an_app_api_break_is_information_only() {
        let groups = groups();
        let pull_request = pull_request("feat(cartograph): widen the chart API", "");
        let changes = changes(&["crates/cartograph/src/lib.rs"]);
        assert!(!declares_break(
            &pull_request,
            &changes,
            "waypoints",
            &groups
        ));
        let verdict = verdict(
            "waypoints",
            vec![("crates/cartograph".to_owned(), Required::Major)],
            false,
            true,
        );
        assert!(!verdict.failed());
        assert!(
            verdict.describe().contains("information only"),
            "{}",
            verdict.describe()
        );
    }

    /// Scenario _No tag to measure against_: skipped, naming the group and
    /// the cut that mints its baseline.
    #[test]
    fn a_group_with_no_tag_is_skipped_by_name() {
        let groups = groups();
        assert_eq!(prefix(&groups, "transload").unwrap(), "transload-v");
        let verdict = Verdict::Skipped {
            group: "transload".to_owned(),
            prefix: "transload-v".to_owned(),
        };
        assert!(!verdict.failed());
        let line = verdict.describe();
        assert!(
            line.starts_with("`transload` has no `transload-v*` tag"),
            "{line}"
        );
        assert!(line.contains("first cut"), "{line}");
    }
}
