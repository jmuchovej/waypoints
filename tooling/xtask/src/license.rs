// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The project's licensing policy, and the validator that holds the tree to it.
//!
//! `cargo-deny` owns dependency allowlisting. This checks what it does not:
//! manifests against the policy, headers against manifests, layering, the
//! license texts against the FSF's, the third-party registry's consistency, and
//! the agreement CLA assistant displays against the repository's. Every check
//! reports all it finds, so one run names every failure.

pub mod notices;

use crate::tools::github;
use anyhow::{Context as _, Result, bail};
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use spdx::{Expression, Licensee};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

/// The license of every first-party crate and source file.
pub const POLICY: &str = "AGPL-3.0-or-later";

/// The copyright holder a first-party source file names.
pub const HOLDER: &str = "John Muchovej";

/// The tag a copyright line opens with, comment syntax aside.
pub const COPYRIGHT_TAG: &str = "SPDX-FileCopyrightText:";

/// The tag a license line opens with, comment syntax aside.
const IDENTIFIER_TAG: &str = "SPDX-License-Identifier:";

/// The only expression a crate may be born permissive under.
const PERMISSIVE: &str = "MIT OR Apache-2.0";

/// Extensions whose files carry a header. Data, configuration and documents do not.
const HEADER_EXTENSIONS: &[&str] = &[
    "rs", "ts", "vue", "js", "mjs", "css", "toml", "nix", "typ", "nu", "sh",
];

/// Extensionless scripts carry a header too, here.
const SCRIPT_DIR: &str = "tooling/bin";

/// First-party paths without headers: platform scaffolding, and the mise
/// fragments devenv rewrites on shell entry. Third-party paths come from the registry.
const EXEMPT: &[&str] = &["src-tauri/gen", ".config/mise/conf.d"];

/// The third-party registry, relative to the workspace root.
const REGISTRY: &str = ".config/third-party.toml";

/// The cargo-deny configuration holding the canonical allowlist.
const DENY: &str = ".config/deny.toml";

/// The frontend build configuration holding the bundle gate's allowlist.
const NUXT_CONFIG: &str = "src/nuxt.config.ts";

/// The constant in [`NUXT_CONFIG`] that names the allowlist.
const NUXT_ALLOWLIST: &str = "LICENSE_ALLOWLIST";

/// The FSF's plain-text AGPLv3, the only license file at the root.
const ROOT_LICENSE: (&str, &str) = (
    "LICENSE",
    "0d96a4ff68ad6d4b6f1f30f713b18d5184912ba8dd389f86aa7710db079abcb0",
);

/// The FSF's Markdown edition of the same text.
const MARKDOWN_LICENSE: (&str, &str) = (
    "docs/licensing/LICENSE.md",
    "00352ab19865e23bb0ab7e0f45332206aba6a66d75b3f3cc962fdd6508d63ea4",
);

/// The contributor agreement contributors sign.
const AGREEMENT: &str = "docs/licensing/CLA.md";

/// The signing form's custom-fields schema.
const AGREEMENT_METADATA: &str = "docs/licensing/cla-metadata.json";

/// The Gist CLA assistant displays. It is secret, not private: its id is public here.
pub const GIST_OWNER: &str = "jmuchovej";

/// See [`GIST_OWNER`].
pub const GIST_ID: &str = "f2903ad9aa3c2eb374faa28ebadab7f6";

/// The agreement's file name in the Gist.
pub const GIST_AGREEMENT: &str = "CLA.md";

/// The signing form's file name in the Gist.
pub const GIST_METADATA: &str = "metadata";

/// The public client id of the `wp-botswain` App, for the device flow.
pub const CLIENT_ID: &str = "Iv23liNfm1Q9EIKSExlK";

/// The year of the first copyright line in `text`, which a file keeps for life.
pub fn copyright_year(text: &str) -> Option<&str> {
    text.lines()
        .find_map(|line| line.split_once(COPYRIGHT_TAG))
        .and_then(|(_, rest)| rest.split_whitespace().next())
}

// ── The registry ─────────────────────────────────────────────────────────────

/// `.config/third-party.toml`: code in the tree this project did not write.
#[derive(Debug, Default, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub entry: Vec<Entry>,
}

/// One vendored crate, copied file or ported portion.
#[derive(Debug, Deserialize)]
pub struct Entry {
    pub path: String,
    pub kind: Kind,
    pub project: String,
    pub holder: String,
    pub license: String,
    #[serde(default)]
    pub texts: Vec<String>,
}

/// What an [`Entry`] covers.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A whole crate, carrying its own license and texts.
    Crate,
    /// Whole files, carrying their own license and texts.
    Files,
    /// A routine inside a first-party file, which keeps the project license.
    Portion,
}

impl Registry {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join(REGISTRY);
        let text = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("Failed to parse {}", path.display()))
    }

    fn parse(text: &str) -> Result<Self> {
        toml::from_str(text).map_err(Into::into)
    }

    /// The directories of crates that are not ours.
    fn crates(&self) -> impl Iterator<Item = &Path> {
        self.entry
            .iter()
            .filter(|entry| entry.kind == Kind::Crate)
            .map(|entry| Path::new(&entry.path))
    }

    /// Paths whose files carry their upstream's license and so no header of ours.
    fn exempt(&self) -> impl Iterator<Item = &Path> {
        self.entry
            .iter()
            .filter(|entry| entry.kind != Kind::Portion)
            .map(|entry| Path::new(&entry.path))
    }
}

// ── Licenses ─────────────────────────────────────────────────────────────────

/// The canonical allowlist from `.config/deny.toml`.
fn deny_allowlist(root: &Path) -> Result<Vec<String>> {
    let path = root.join(DENY);
    let text =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let table: toml::Table = text
        .parse()
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    table
        .get("licenses")
        .and_then(|licenses| licenses.get("allow"))
        .and_then(|allow| allow.as_array())
        .with_context(|| format!("{} has no [licenses].allow", path.display()))?
        .iter()
        .map(|id| {
            id.as_str()
                .map(str::to_owned)
                .with_context(|| format!("non-string allowlist entry in {}", path.display()))
        })
        .collect()
}

/// Whether some allowlisted identifier satisfies `expression`.
fn satisfied_by_allowlist(expression: &Expression, allowlist: &[String]) -> Result<bool> {
    let licensees = allowlist
        .iter()
        .map(|id| Licensee::parse(id).with_context(|| format!("unparsable allowlist entry {id}")))
        .collect::<Result<Vec<_>>>()?;
    Ok(expression.evaluate(|requirement| {
        licensees
            .iter()
            .any(|licensee| licensee.satisfies(requirement))
    }))
}

/// Copyleft means every way of satisfying the expression needs a copyleft
/// license, so `MIT OR GPL-3.0-or-later` is not copyleft.
fn is_copyleft(expression: &Expression) -> bool {
    !expression.evaluate(|requirement| {
        requirement
            .license
            .id()
            .is_some_and(|license| !license.is_copyleft())
    })
}

// ── Manifests ────────────────────────────────────────────────────────────────

/// A workspace crate, as the checks need it.
#[derive(Clone, Debug)]
struct Crate {
    id: String,
    name: String,
    /// Relative to the workspace root.
    dir: PathBuf,
    license: Option<String>,
}

fn workspace_crates(metadata: &Metadata) -> Vec<Crate> {
    let root = metadata.workspace_root.as_std_path();
    metadata
        .workspace_packages()
        .into_iter()
        .map(|package| Crate {
            id: package.id.repr.clone(),
            name: package.name.to_string(),
            dir: package
                .manifest_path
                .parent()
                .map(|dir| dir.as_std_path())
                .and_then(|dir| dir.strip_prefix(root).ok())
                .map(Path::to_path_buf)
                .unwrap_or_default(),
            license: package.license.clone(),
        })
        .collect()
}

/// Every workspace manifest's expression is valid, not deprecated, and allowed
/// for what the crate is: the policy for first-party code, its own compatible
/// license for registered third-party code, or the permissive birth tier.
fn check_manifests(
    root: &Path,
    crates: &[Crate],
    registry: &Registry,
    allowlist: &[String],
) -> Result<Vec<String>> {
    let third_party: BTreeSet<&Path> = registry.crates().collect();
    let mut findings = Vec::new();

    for krate in crates {
        let name = &krate.name;
        let Some(license) = &krate.license else {
            findings.push(format!("{name}: its manifest declares no license"));
            continue;
        };
        let expression = match Expression::parse(license) {
            Ok(expression) => expression,
            // A deprecated identifier lands here too: the parser refuses it, and
            // canonicalizing names its replacement.
            Err(error) => {
                let reason = &error.reason;
                let canonical = Expression::canonicalize(license).ok().flatten();
                findings.push(match canonical {
                    Some(canonical) => format!(
                        "{name}: `{license}` is not valid ({reason}); the canonical form is `{canonical}`"
                    ),
                    None => format!("{name}: `{license}` is not valid: {reason}"),
                });
                continue;
            }
        };

        if third_party.contains(krate.dir.as_path()) {
            if !satisfied_by_allowlist(&expression, allowlist)? {
                findings.push(format!(
                    "{name}: registered as third-party, but no allowlisted identifier satisfies `{license}`"
                ));
            }
        } else if expression.to_string() == PERMISSIVE {
            for text in ["LICENSE-MIT", "LICENSE-APACHE"] {
                if !root.join(&krate.dir).join(text).is_file() {
                    findings.push(format!(
                        "{name}: declared permissive, but {} is missing",
                        krate.dir.join(text).display()
                    ));
                }
            }
        } else if expression.to_string() != POLICY {
            findings.push(format!(
                "{name}: declares `{license}`, not `{POLICY}`; register it as third-party in {REGISTRY}, or inherit the policy with `license.workspace = true`"
            ));
        }
    }
    Ok(findings)
}

// ── Headers ──────────────────────────────────────────────────────────────────

/// How a file type writes comments.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Syntax {
    Hash,
    Slash,
    Css,
    Vue,
}

/// The syntax of a file that carries a header, or `None` for one that does not.
fn header_syntax(relative: &Path) -> Option<Syntax> {
    if relative.starts_with(SCRIPT_DIR) {
        return Some(Syntax::Hash);
    }
    if relative
        .to_str()
        .is_some_and(|path| path.ends_with(".d.ts"))
    {
        return None;
    }
    let extension = relative.extension()?.to_str()?;
    if !HEADER_EXTENSIONS.contains(&extension) {
        return None;
    }
    Some(match extension {
        "toml" | "nix" | "nu" | "sh" => Syntax::Hash,
        "css" => Syntax::Css,
        "vue" => Syntax::Vue,
        _ => Syntax::Slash,
    })
}

/// The text of a file's initial comment block, a line at a time and with comment
/// syntax removed: every comment before the first line of anything else, after
/// any shebang. A `#:tombi` directive is a comment, so it needs no special case.
fn initial_block(syntax: Syntax, text: &str) -> Vec<&str> {
    let mut lines = text.lines().peekable();
    if syntax == Syntax::Hash {
        while lines
            .peek()
            .is_some_and(|line| line.starts_with("#!") && !line.starts_with("#!["))
        {
            lines.next();
        }
    }

    let mut block = Vec::new();
    while let Some(line) = lines.next() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match syntax {
            Syntax::Hash if line.starts_with('#') => block.push(line.trim_start_matches('#')),
            Syntax::Slash if line.starts_with("//") => block.push(line.trim_start_matches('/')),
            Syntax::Slash | Syntax::Css if line.starts_with("/*") => {
                take_until(line, "*/", &mut lines, &mut block);
            }
            Syntax::Vue if line.starts_with("<!--") => {
                take_until(line, "-->", &mut lines, &mut block);
            }
            _ => break,
        }
    }
    block.into_iter().map(strip_comment).collect()
}

/// Pushes a block comment's lines, from `first` through the one holding `end`.
fn take_until<'a>(
    first: &'a str,
    end: &str,
    lines: &mut impl Iterator<Item = &'a str>,
    block: &mut Vec<&'a str>,
) {
    block.push(first);
    if first.contains(end) {
        return;
    }
    for line in lines {
        block.push(line);
        if line.contains(end) {
            return;
        }
    }
}

fn strip_comment(line: &str) -> &str {
    let line = line.trim();
    let line = line.strip_prefix("<!--").unwrap_or(line);
    let line = line.strip_prefix("/*").unwrap_or(line);
    let line = line.strip_suffix("-->").unwrap_or(line);
    let line = line.strip_suffix("*/").unwrap_or(line);
    line.trim_start_matches(['*', '!']).trim()
}

/// What is wrong with one file's header, given the expression its crate declares.
fn check_header(relative: &Path, syntax: Syntax, text: &str, expected: &str) -> Vec<String> {
    let path = relative.display();
    let block = initial_block(syntax, text);
    let mut findings = Vec::new();

    let copyrights: Vec<&str> = block
        .iter()
        .filter_map(|line| line.strip_prefix(COPYRIGHT_TAG))
        .map(str::trim)
        .collect();
    if copyrights.is_empty() {
        findings.push(format!(
            "{path}: no {COPYRIGHT_TAG} line in its initial comment block"
        ));
    }
    for copyright in copyrights {
        if copyright.contains('[') {
            findings.push(format!(
                "{path}: `{copyright}` still holds a bracketed placeholder"
            ));
            continue;
        }
        let (year, holder) = copyright.split_once(' ').unwrap_or((copyright, ""));
        if year.len() != 4 || !year.bytes().all(|byte| byte.is_ascii_digit()) {
            findings.push(format!(
                "{path}: `{year}` is not a single four-digit year; the year records first authorship and never changes"
            ));
        }
        if holder.trim().is_empty() {
            findings.push(format!("{path}: `{copyright}` names no holder"));
        }
    }

    let identifiers: Vec<&str> = block
        .iter()
        .filter_map(|line| line.strip_prefix(IDENTIFIER_TAG))
        .map(str::trim)
        .collect();
    match identifiers.as_slice() {
        [] => findings.push(format!(
            "{path}: no {IDENTIFIER_TAG} line in its initial comment block"
        )),
        [identifier] => match Expression::parse(identifier) {
            Ok(expression) if expression.to_string() == expected => {}
            Ok(expression) => findings.push(format!(
                "{path}: its header says `{expression}`, its manifest says `{expected}`"
            )),
            Err(error) => findings.push(format!("{path}: `{identifier}` does not parse: {error}")),
        },
        _ => findings.push(format!(
            "{path}: more than one {IDENTIFIER_TAG} line in its initial comment block"
        )),
    }
    findings
}

/// The expression governing a file: the crate whose directory is the longest
/// prefix of its path, or the policy for a file outside every crate.
fn owning_expression(relative: &Path, crates: &[Crate]) -> String {
    crates
        .iter()
        .filter(|krate| !krate.dir.as_os_str().is_empty() && relative.starts_with(&krate.dir))
        .max_by_key(|krate| krate.dir.components().count())
        .and_then(|krate| krate.license.as_deref())
        .and_then(|license| Expression::parse(license).ok())
        .map(|expression| expression.to_string())
        .unwrap_or_else(|| POLICY.to_owned())
}

/// Walks the tree honouring `.gitignore`, so build output never enters. Symlinks
/// are not walked: what one points at is checked as itself.
fn check_headers(root: &Path, crates: &[Crate], registry: &Registry) -> Result<Vec<String>> {
    let exempt: Vec<&Path> = registry
        .exempt()
        .chain(EXEMPT.iter().map(Path::new))
        .collect();

    let walk = ignore::WalkBuilder::new(root)
        .hidden(false)
        .parents(false)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .filter_entry(|entry| !matches!(entry.file_name().to_str(), Some(".git" | ".jj")))
        .build();

    let mut findings = Vec::new();
    for entry in walk {
        let entry = entry.context("Failed to walk the tree")?;
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let relative = entry.path().strip_prefix(root).unwrap_or(entry.path());
        if exempt.iter().any(|path| relative.starts_with(path)) {
            continue;
        }
        let Some(syntax) = header_syntax(relative) else {
            continue;
        };
        let text = fs::read_to_string(entry.path())
            .with_context(|| format!("Failed to read {}", entry.path().display()))?;
        findings.extend(check_header(
            relative,
            syntax,
            &text,
            &owning_expression(relative, crates),
        ));
    }
    findings.sort();
    Ok(findings)
}

// ── Layering ─────────────────────────────────────────────────────────────────

/// Each package's normal-kind dependencies, by package id, from the resolve graph.
fn normal_edges(metadata: &Metadata) -> BTreeMap<String, Vec<String>> {
    metadata
        .resolve
        .iter()
        .flat_map(|resolve| &resolve.nodes)
        .map(|node| {
            let dependencies = node
                .deps
                .iter()
                .filter(|dependency| {
                    dependency
                        .dep_kinds
                        .iter()
                        .any(|kind| kind.kind == DependencyKind::Normal)
                })
                .map(|dependency| dependency.pkg.repr.clone())
                .collect();
            (node.id.repr.clone(), dependencies)
        })
        .collect()
}

/// No non-copyleft workspace crate reaches a copyleft one through normal edges,
/// so a future permissive crate cannot be born wrong. Vacuous while none exists.
fn check_layering(crates: &[Crate], edges: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    let copyleft: BTreeMap<&str, &Crate> = crates
        .iter()
        .filter(|krate| {
            krate
                .license
                .as_deref()
                .and_then(|license| Expression::parse(license).ok())
                .is_some_and(|expression| is_copyleft(&expression))
        })
        .map(|krate| (krate.id.as_str(), krate))
        .collect();

    let mut findings = Vec::new();
    for krate in crates {
        if copyleft.contains_key(krate.id.as_str()) {
            continue;
        }
        let mut seen = BTreeSet::new();
        let mut stack = vec![krate.id.as_str()];
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            if let Some(dependency) = copyleft.get(id) {
                findings.push(format!(
                    "{}: not copyleft, but depends on the copyleft workspace crate {}",
                    krate.name, dependency.name
                ));
                continue;
            }
            if let Some(dependencies) = edges.get(id) {
                stack.extend(dependencies.iter().map(String::as_str));
            }
        }
    }
    findings
}

// ── License texts and the registry ───────────────────────────────────────────

/// Both editions hash to the FSF's files, and the root holds no other license file.
fn check_texts(root: &Path) -> Result<Vec<String>> {
    let mut findings = Vec::new();
    for (path, digest) in [ROOT_LICENSE, MARKDOWN_LICENSE] {
        let full = root.join(path);
        match fs::symlink_metadata(&full) {
            Err(_) => findings.push(format!("{path}: missing")),
            Ok(metadata) if !metadata.is_file() => {
                findings.push(format!("{path}: must be a regular file, not a symlink"));
            }
            Ok(_) => {
                let bytes = fs::read(&full)
                    .with_context(|| format!("Failed to read {}", full.display()))?;
                if hex::encode(Sha256::digest(&bytes)) != digest {
                    findings.push(format!(
                        "{path}: differs from the FSF's published text; it may only be replaced with that text, verbatim"
                    ));
                }
            }
        }
    }

    for entry in fs::read_dir(root).with_context(|| format!("Failed to read {}", root.display()))? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let upper = name.to_ascii_uppercase();
        if name != ROOT_LICENSE.0
            && (upper.starts_with("LICENSE") || upper.starts_with("COPYING"))
            && entry.file_type()?.is_file()
        {
            findings.push(format!(
                "{name}: a second license file at the root; license detection reads the root and gives up on two"
            ));
        }
    }
    Ok(findings)
}

/// Every registered license is allowlisted, and every named text exists.
fn check_registry(root: &Path, registry: &Registry, allowlist: &[String]) -> Result<Vec<String>> {
    let mut findings = Vec::new();
    for entry in &registry.entry {
        let path = &entry.path;
        match Expression::parse(&entry.license) {
            Ok(expression) if satisfied_by_allowlist(&expression, allowlist)? => {}
            Ok(_) => findings.push(format!(
                "{REGISTRY}: {path} is `{}`, which no allowlisted identifier satisfies",
                entry.license
            )),
            Err(error) => findings.push(format!(
                "{REGISTRY}: {path} has an unparsable license `{}`: {error}",
                entry.license
            )),
        }

        let full = root.join(path);
        let dir = if full.is_dir() {
            full.as_path()
        } else {
            full.parent().unwrap_or(root)
        };
        for text in &entry.texts {
            if !dir.join(text).is_file() {
                findings.push(format!(
                    "{REGISTRY}: {path} names the license text {text}, which does not exist"
                ));
            }
        }
    }
    Ok(findings)
}

// ── Allowlists ───────────────────────────────────────────────────────────────

/// The string literals of the array assigned to `name` in TypeScript source.
fn typescript_string_array(source: &str, name: &str) -> Option<Vec<String>> {
    let after_name = &source[source.find(&format!("const {name}"))? + name.len()..];
    let body = &after_name[after_name.find('[')? + 1..];
    let body = &body[..body.find(']')?];
    Some(
        body.split('"')
            .skip(1)
            .step_by(2)
            .map(str::to_owned)
            .collect(),
    )
}

/// The frontend gate's allowlist and cargo-deny's are one list.
fn check_allowlists(root: &Path, allowlist: &[String]) -> Result<Vec<String>> {
    let path = root.join(NUXT_CONFIG);
    let source =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    let Some(frontend) = typescript_string_array(&source, NUXT_ALLOWLIST) else {
        return Ok(vec![format!(
            "{NUXT_CONFIG}: no `const {NUXT_ALLOWLIST} = [...]` to compare with {DENY}"
        )]);
    };
    Ok(compare_allowlists(&frontend, allowlist))
}

fn compare_allowlists(frontend: &[String], deny: &[String]) -> Vec<String> {
    let frontend: BTreeSet<&String> = frontend.iter().collect();
    let deny: BTreeSet<&String> = deny.iter().collect();
    frontend
        .difference(&deny)
        .map(|id| format!("{DENY}: lacks `{id}`, which {NUXT_CONFIG} allows"))
        .chain(
            deny.difference(&frontend)
                .map(|id| format!("{NUXT_CONFIG}: lacks `{id}`, which {DENY} allows")),
        )
        .collect()
}

// ── The published agreement ──────────────────────────────────────────────────

/// The agreement and its signing form, in the repository or in the Gist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Agreement {
    pub text: String,
    pub metadata: String,
}

impl Agreement {
    /// The repository's copy.
    pub fn from_repository(root: &Path) -> Result<Self> {
        let read = |path: &str| {
            fs::read_to_string(root.join(path)).with_context(|| format!("Failed to read {path}"))
        };
        Ok(Self {
            text: read(AGREEMENT)?,
            metadata: read(AGREEMENT_METADATA)?,
        })
    }

    /// The first line naming a version. A text change must change this line.
    fn version_line(&self) -> Option<&str> {
        self.text.lines().find(|line| line.contains(" version "))
    }

    /// Whether the text still holds a blank such as `[SUCCESSION_WINDOW]`.
    fn has_blank(&self) -> bool {
        self.text.split('[').skip(1).any(|rest| {
            rest.split_once(']').is_some_and(|(inner, _)| {
                !inner.is_empty()
                    && inner
                        .chars()
                        .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit())
            })
        })
    }
}

/// How the repository's agreement stands against the Gist's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Publication {
    /// The two are identical.
    Current,
    /// The repository's text holds a blank, so it cannot be published yet.
    UnpublishedDraft,
    /// The repository carries a version the Gist has not received.
    Pending,
    /// The text differs under the same version line, in one copy or the other.
    TextWithoutBump,
    /// The agreement is identical but the signing form is not.
    MetadataDrift,
}

/// Where the check runs, which decides how a pending publication is judged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Branch {
    /// A pull request or a local run: publication follows the merge.
    PullRequest,
    /// A push to `main`: the merge has happened, so publication is due.
    Main,
}

impl Branch {
    /// The runner's ref decides; anything but `main`, or no runner, is a pull request.
    pub fn from_environment() -> Self {
        match std::env::var("GITHUB_REF").as_deref() {
            Ok("refs/heads/main") => Self::Main,
            _ => Self::PullRequest,
        }
    }
}

/// Compares the two copies.
pub fn compare(repository: &Agreement, gist: &Agreement) -> Publication {
    if repository.has_blank() {
        Publication::UnpublishedDraft
    } else if repository == gist {
        Publication::Current
    } else if repository.text == gist.text {
        Publication::MetadataDrift
    } else if repository.version_line() == gist.version_line() {
        Publication::TextWithoutBump
    } else {
        Publication::Pending
    }
}

/// A finding, if `publication` should fail a run on `branch`.
fn judge(publication: Publication, branch: Branch) -> Option<String> {
    match publication {
        Publication::Current | Publication::UnpublishedDraft => None,
        Publication::Pending => (branch == Branch::Main).then(|| {
            format!(
                "{AGREEMENT}: its version has not been published; run `cargo xtask license publish`"
            )
        }),
        Publication::TextWithoutBump => Some(format!(
            "{AGREEMENT}: differs from the published copy under the same version line; bump the version, or restore whichever copy was edited"
        )),
        Publication::MetadataDrift => Some(format!(
            "{AGREEMENT_METADATA}: differs from the published signing form; run `cargo xtask license publish`"
        )),
    }
}

/// Reads and writes the Gist CLA assistant displays.
pub trait Gist {
    /// The published copy.
    fn read(&self) -> Result<Agreement>;
    /// Replaces the published copy, returning the revision created.
    fn write(&self, agreement: &Agreement) -> Result<String>;
}

/// The Gist as GitHub holds it. Reading needs nothing; writing runs the device flow.
pub struct GitHubGist;

impl Gist for GitHubGist {
    fn read(&self) -> Result<Agreement> {
        Ok(Agreement {
            text: github::raw_gist_file(GIST_OWNER, GIST_ID, GIST_AGREEMENT)?,
            metadata: github::raw_gist_file(GIST_OWNER, GIST_ID, GIST_METADATA)?,
        })
    }

    fn write(&self, agreement: &Agreement) -> Result<String> {
        github::update_gist_as_user(
            CLIENT_ID,
            GIST_ID,
            [
                (GIST_AGREEMENT, agreement.text.as_str()),
                (GIST_METADATA, agreement.metadata.as_str()),
            ],
        )
    }
}

/// What [`publish`] did.
#[derive(Debug, PartialEq, Eq)]
pub enum Published {
    /// The Gist already held this agreement.
    AlreadyCurrent,
    /// A new Gist revision, which is what makes CLA assistant ask signers again.
    Revision(String),
}

/// Writes the repository's agreement to the Gist, refusing anything a signer
/// should not be asked to sign.
pub fn publish(repository: &Agreement, gist: &impl Gist) -> Result<Published> {
    if repository.has_blank() {
        bail!("{AGREEMENT} still holds a bracketed blank; fill it before publishing");
    }
    let published = gist.read()?;
    match compare(repository, &published) {
        Publication::Current => Ok(Published::AlreadyCurrent),
        Publication::TextWithoutBump => bail!(
            "{AGREEMENT} differs from the published copy under the same version line; every signer would be asked again for a change nobody can name, so bump the version first"
        ),
        Publication::UnpublishedDraft | Publication::Pending | Publication::MetadataDrift => {
            gist.write(repository).map(Published::Revision)
        }
    }
}

// ── The check ────────────────────────────────────────────────────────────────

/// The outcome of [`check`].
#[derive(Debug, Default)]
pub struct Report {
    /// Everything that must be fixed. Empty means the run passes.
    pub findings: Vec<String>,
    /// What a reader should know without failing the run.
    pub notes: Vec<String>,
    /// How many workspace crates were judged.
    pub crates: usize,
}

/// Runs every check against the workspace at `root`, reporting them all.
pub fn check(root: &Path, branch: Branch, gist: &impl Gist) -> Result<Report> {
    let metadata = MetadataCommand::new()
        .current_dir(root)
        .exec()
        .context("Failed to load cargo metadata")?;
    let crates = workspace_crates(&metadata);
    let registry = Registry::load(root)?;
    let allowlist = deny_allowlist(root)?;

    let mut report = Report {
        crates: crates.len(),
        ..Report::default()
    };
    report
        .findings
        .extend(check_manifests(root, &crates, &registry, &allowlist)?);
    report
        .findings
        .extend(check_headers(root, &crates, &registry)?);
    report
        .findings
        .extend(check_layering(&crates, &normal_edges(&metadata)));
    report.findings.extend(check_texts(root)?);
    report
        .findings
        .extend(check_registry(root, &registry, &allowlist)?);
    report.findings.extend(check_allowlists(root, &allowlist)?);

    let repository = Agreement::from_repository(root)?;
    match gist.read() {
        Ok(published) => {
            let publication = compare(&repository, &published);
            report.findings.extend(judge(publication, branch));
            match publication {
                Publication::UnpublishedDraft => report.notes.push(format!(
                    "{AGREEMENT} is a draft with blanks; it cannot be published until they are filled"
                )),
                Publication::Pending if branch == Branch::PullRequest => report.notes.push(format!(
                    "{AGREEMENT} carries a version not yet published; publish it after this merges"
                )),
                _ => {}
            }
        }
        Err(error) => report.notes.push(format!(
            "could not compare {AGREEMENT} with the published copy, so it was not checked: {error:#}"
        )),
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn allowlist() -> Vec<String> {
        ["MIT", "Apache-2.0", "MPL-2.0"].map(str::to_owned).to_vec()
    }

    fn krate(name: &str, dir: &str, license: Option<&str>) -> Crate {
        Crate {
            id: name.to_owned(),
            name: name.to_owned(),
            dir: PathBuf::from(dir),
            license: license.map(str::to_owned),
        }
    }

    fn expression(text: &str) -> Expression {
        Expression::parse(text).unwrap()
    }

    fn registry(text: &str) -> Registry {
        Registry::parse(text).unwrap()
    }

    const REGISTERED: &str = r#"
        [[entry]]
        path = "tooling/vendored"
        kind = "crate"
        project = "someone/vendored"
        holder = "Someone"
        license = "Apache-2.0"
        texts = ["LICENSE"]

        [[entry]]
        path = "src/icons"
        kind = "files"
        project = "someone/icons"
        holder = "Someone"
        license = "MIT"

        [[entry]]
        path = "src/app/ported.rs"
        kind = "portion"
        project = "someone/routine"
        holder = "Someone"
        license = "MIT"
    "#;

    fn manifests(krate: &Crate) -> Vec<String> {
        let root = tempfile::tempdir().unwrap();
        check_manifests(
            root.path(),
            std::slice::from_ref(krate),
            &registry(REGISTERED),
            &allowlist(),
        )
        .unwrap()
    }

    // ── Registry ─────────────────────────────────────────────────────────────

    #[test]
    fn the_registry_yields_crates_and_exempt_paths() {
        let registry = registry(REGISTERED);
        assert_eq!(
            registry.crates().collect::<Vec<_>>(),
            [Path::new("tooling/vendored")]
        );
        assert_eq!(
            registry.exempt().collect::<Vec<_>>(),
            [Path::new("tooling/vendored"), Path::new("src/icons")],
            "a portion's file stays first-party, so it stays under the header check"
        );
    }

    #[test]
    fn a_missing_text_and_an_unlisted_license_are_named() {
        let root = tempfile::tempdir().unwrap();
        let registry = registry(
            r#"
            [[entry]]
            path = "tooling/vendored"
            kind = "crate"
            project = "someone/vendored"
            holder = "Someone"
            license = "GPL-3.0-only"
            texts = ["LICENSE"]
            "#,
        );
        let findings = check_registry(root.path(), &registry, &allowlist()).unwrap();
        assert_eq!(findings.len(), 2, "{findings:?}");
        assert!(findings[0].contains("GPL-3.0-only"), "{findings:?}");
        assert!(findings[1].contains("LICENSE"), "{findings:?}");
    }

    // ── Manifests ────────────────────────────────────────────────────────────

    #[test]
    fn a_crate_at_the_policy_passes() {
        assert_eq!(manifests(&krate("a", "crates/a", Some(POLICY))), [""; 0]);
    }

    #[test]
    fn a_deprecated_identifier_is_named() {
        let findings = manifests(&krate("a", "crates/a", Some("AGPL-3.0")));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("deprecated"), "{findings:?}");
        assert!(findings[0].contains("AGPL-3.0-only"), "{findings:?}");
    }

    #[test]
    fn a_registered_third_party_crate_passes_under_its_own_license() {
        let findings = manifests(&krate("v", "tooling/vendored", Some("Apache-2.0")));
        assert!(findings.is_empty(), "{findings:?}");
    }

    #[test]
    fn an_unregistered_crate_is_told_both_remedies() {
        let findings = manifests(&krate("a", "crates/a", Some("MIT")));
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains(REGISTRY), "{findings:?}");
        assert!(findings[0].contains("license.workspace"), "{findings:?}");
    }

    #[test]
    fn a_permissive_crate_needs_both_texts() {
        let findings = manifests(&krate("a", "crates/a", Some(PERMISSIVE)));
        assert_eq!(findings.len(), 2, "{findings:?}");
    }

    #[test]
    fn a_missing_license_is_named() {
        let findings = manifests(&krate("a", "crates/a", None));
        assert!(findings[0].contains("declares no license"), "{findings:?}");
    }

    // ── Headers ──────────────────────────────────────────────────────────────

    const TAGS: &str =
        "SPDX-FileCopyrightText: 2026 John Muchovej\nSPDX-License-Identifier: AGPL-3.0-or-later";

    fn commented(prefix: &str) -> String {
        TAGS.lines()
            .map(|line| format!("{prefix}{line}\n"))
            .collect()
    }

    fn header(path: &str, text: &str) -> Vec<String> {
        let path = Path::new(path);
        check_header(path, header_syntax(path).unwrap(), text, POLICY)
    }

    #[test]
    fn a_shebang_script_passes() {
        let text = format!("#!/usr/bin/env nu\n{}\ndef main [] {{}}\n", commented("# "));
        assert_eq!(header("tooling/bin/xtauri", &text), [""; 0]);
    }

    #[test]
    fn a_tombi_directive_may_come_first() {
        let text = format!(
            "#:tombi toml-version = \"v1.1.0\"\n#:schema https://example.com\n{}\n[a]\n",
            commented("# ")
        );
        assert_eq!(header("Cargo.toml", &text), [""; 0]);
    }

    #[test]
    fn a_vue_file_carries_an_html_comment() {
        let text = format!("<!--\n{}-->\n\n<template></template>\n", commented("  "));
        assert_eq!(header("src/app/app.vue", &text), [""; 0]);
    }

    #[test]
    fn a_css_file_carries_a_block_comment() {
        let text = format!("/*\n{} */\n\nbody {{}}\n", commented(" * "));
        assert_eq!(header("src/app/main.css", &text), [""; 0]);
    }

    #[test]
    fn a_generated_marker_may_come_first() {
        let text = format!(
            "// DO NOT EDIT.\n{}\nexport type A = 1;\n",
            commented("// ")
        );
        assert_eq!(header("crates/a/types.g.ts", &text), [""; 0]);
    }

    /// `#![…]` is an inner attribute, not a shebang, and comes after the header.
    #[test]
    fn an_inner_attribute_is_not_a_shebang() {
        let text = format!("{}\n#![allow(unused)]\n", commented("// "));
        assert_eq!(header("crates/a/src/lib.rs", &text), [""; 0]);
    }

    #[test]
    fn a_missing_tag_is_named() {
        let text = "// SPDX-FileCopyrightText: 2026 John Muchovej\nfn main() {}\n";
        let findings = header("crates/a/src/lib.rs", text);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains(IDENTIFIER_TAG), "{findings:?}");
        assert!(findings[0].contains("crates/a/src/lib.rs"), "{findings:?}");
    }

    #[test]
    fn a_tag_after_the_code_does_not_count() {
        let text = format!("fn main() {{}}\n{}", commented("// "));
        assert_eq!(header("crates/a/src/lib.rs", &text).len(), 2);
    }

    #[test]
    fn a_second_holder_is_accepted() {
        let text = format!(
            "// SPDX-FileCopyrightText: 2028 A Contributor\n{}\nfn main() {{}}\n",
            commented("// ")
        );
        assert_eq!(header("crates/a/src/lib.rs", &text), [""; 0]);
    }

    #[test]
    fn a_year_range_is_rejected() {
        let text = "// SPDX-FileCopyrightText: 2026-2029 John Muchovej\n// SPDX-License-Identifier: AGPL-3.0-or-later\n";
        let findings = header("crates/a/src/lib.rs", text);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].contains("single four-digit year"),
            "{findings:?}"
        );
    }

    #[test]
    fn a_header_disagreeing_with_its_crate_names_both() {
        let text =
            "// SPDX-FileCopyrightText: 2026 John Muchovej\n// SPDX-License-Identifier: MIT\n";
        let findings = header("crates/a/src/lib.rs", text);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("MIT"), "{findings:?}");
        assert!(findings[0].contains(POLICY), "{findings:?}");
    }

    #[test]
    fn an_unparsable_expression_is_named() {
        let text = "// SPDX-FileCopyrightText: 2026 John Muchovej\n// SPDX-License-Identifier: not a license\n";
        let findings = header("crates/a/src/lib.rs", text);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("does not parse"), "{findings:?}");
    }

    #[test]
    fn a_placeholder_holder_is_named() {
        let text = "// SPDX-FileCopyrightText: 2026 [HOLDER]\n// SPDX-License-Identifier: AGPL-3.0-or-later\n";
        let findings = header("crates/a/src/lib.rs", text);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("placeholder"), "{findings:?}");
    }

    #[test]
    fn two_identifiers_are_rejected() {
        let text = format!("{}// SPDX-License-Identifier: MIT\n", commented("// "));
        let findings = header("crates/a/src/lib.rs", &text);
        assert!(findings[0].contains("more than one"), "{findings:?}");
    }

    #[test]
    fn data_and_declarations_carry_no_header() {
        for path in ["a.json", "a.yaml", "a.md", "src/a.d.ts", "Cargo.lock"] {
            assert!(header_syntax(Path::new(path)).is_none(), "{path}");
        }
        assert!(header_syntax(Path::new("tooling/bin/xtauri")).is_some());
    }

    #[test]
    fn the_longest_crate_prefix_owns_a_file() {
        let crates = [
            krate("outer", "tooling", Some(POLICY)),
            krate("inner", "tooling/vendored", Some("Apache-2.0")),
        ];
        assert_eq!(
            owning_expression(Path::new("tooling/vendored/src/lib.rs"), &crates),
            "Apache-2.0"
        );
        assert_eq!(
            owning_expression(Path::new("tooling/x.rs"), &crates),
            POLICY
        );
        assert_eq!(owning_expression(Path::new("flake.nix"), &crates), POLICY);
    }

    /// The walk over a real tree: gitignored output, third-party code, scaffolding
    /// and symlinks stay out, and an unheadered first-party file is caught.
    #[test]
    fn the_walk_skips_what_it_should() {
        let root = tempfile::tempdir().unwrap();
        let write = |path: &str, text: &str| {
            let full = root.path().join(path);
            fs::create_dir_all(full.parent().unwrap()).unwrap();
            fs::write(full, text).unwrap();
        };
        write(".gitignore", "target\n");
        write("target/debug/build.rs", "fn main() {}\n");
        write("tooling/vendored/src/lib.rs", "fn main() {}\n");
        write("src/icons/icon.ts", "export {};\n");
        write("src-tauri/gen/apple/a.rs", "fn main() {}\n");
        write(".config/mise/conf.d/tools.toml", "[tools]\n");
        write("src/app/ported.rs", "fn main() {}\n");
        write("crates/a/src/lib.rs", &commented("// "));
        write("data.json", "{}\n");
        #[cfg(unix)]
        std::os::unix::fs::symlink("crates/a/src/lib.rs", root.path().join("linked.rs")).unwrap();

        let findings = check_headers(
            root.path(),
            &[krate("a", "crates/a", Some(POLICY))],
            &registry(REGISTERED),
        )
        .unwrap();
        assert_eq!(findings.len(), 2, "{findings:?}");
        assert!(
            findings.iter().all(|f| f.starts_with("src/app/ported.rs")),
            "{findings:?}"
        );
    }

    // ── Layering ─────────────────────────────────────────────────────────────

    #[test]
    fn copyleft_means_every_path_needs_copyleft() {
        assert!(is_copyleft(&expression(POLICY)));
        assert!(!is_copyleft(&expression("MIT OR GPL-3.0-or-later")));
        assert!(!is_copyleft(&expression(PERMISSIVE)));
    }

    #[test]
    fn a_permissive_crate_reaching_a_copyleft_one_names_both() {
        let crates = [
            krate("sdk", "crates/sdk", Some(PERMISSIVE)),
            krate("engine", "crates/engine", Some(POLICY)),
        ];
        let edges = BTreeMap::from([("sdk".to_owned(), vec!["engine".to_owned()])]);
        let findings = check_layering(&crates, &edges);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(findings[0].contains("sdk"), "{findings:?}");
        assert!(findings[0].contains("engine"), "{findings:?}");
    }

    #[test]
    fn a_path_through_an_external_crate_counts() {
        let crates = [
            krate("sdk", "crates/sdk", Some(PERMISSIVE)),
            krate("engine", "crates/engine", Some(POLICY)),
        ];
        let edges = BTreeMap::from([
            ("sdk".to_owned(), vec!["external".to_owned()]),
            ("external".to_owned(), vec!["engine".to_owned()]),
        ]);
        assert_eq!(check_layering(&crates, &edges).len(), 1);
    }

    /// [`normal_edges`] leaves dev edges out, which is how a dev dependency passes.
    #[test]
    fn a_tree_without_a_permissive_crate_passes_vacuously() {
        let crates = [krate("engine", "crates/engine", Some(POLICY))];
        assert!(check_layering(&crates, &BTreeMap::new()).is_empty());
    }

    #[test]
    fn dev_edges_are_not_normal_edges() {
        let metadata: Metadata = serde_json::from_value(serde_json::json!({
            "packages": [],
            "workspace_members": [],
            "workspace_default_members": [],
            "target_directory": "/t",
            "version": 1,
            "workspace_root": "/w",
            "resolve": {
                "root": null,
                "nodes": [{
                    "id": "sdk",
                    "dependencies": [],
                    "features": [],
                    "deps": [
                        {"name": "dev", "pkg": "engine-dev", "dep_kinds": [{"kind": "dev", "target": null}]},
                        {"name": "build", "pkg": "engine-build", "dep_kinds": [{"kind": "build", "target": null}]},
                        {"name": "normal", "pkg": "normal", "dep_kinds": [{"kind": null, "target": null}]}
                    ]
                }]
            }
        }))
        .unwrap();
        assert_eq!(normal_edges(&metadata)["sdk"], ["normal"]);
    }

    // ── Texts ────────────────────────────────────────────────────────────────

    /// A temp root holding what the real tree's texts hold.
    fn texts_root() -> tempfile::TempDir {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("docs/licensing")).unwrap();
        for (path, _) in [ROOT_LICENSE, MARKDOWN_LICENSE] {
            fs::copy(repo.join(path), root.path().join(path)).unwrap();
        }
        root
    }

    #[test]
    fn the_real_texts_pass() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(check_texts(&repo).unwrap(), [""; 0]);
    }

    #[test]
    fn a_one_byte_edit_to_either_text_fails() {
        for (path, _) in [ROOT_LICENSE, MARKDOWN_LICENSE] {
            let root = texts_root();
            let mut bytes = fs::read(root.path().join(path)).unwrap();
            bytes[0] ^= 1;
            fs::write(root.path().join(path), bytes).unwrap();
            let findings = check_texts(root.path()).unwrap();
            assert_eq!(findings.len(), 1, "{findings:?}");
            assert!(findings[0].starts_with(path), "{findings:?}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_root_symlink_fails() {
        let root = texts_root();
        fs::remove_file(root.path().join("LICENSE")).unwrap();
        std::os::unix::fs::symlink("docs/licensing/LICENSE.md", root.path().join("LICENSE"))
            .unwrap();
        let findings = check_texts(root.path()).unwrap();
        assert!(
            findings.iter().any(|f| f.contains("regular file")),
            "{findings:?}"
        );
    }

    #[test]
    fn a_missing_text_fails() {
        let root = texts_root();
        fs::remove_file(root.path().join("docs/licensing/LICENSE.md")).unwrap();
        let findings = check_texts(root.path()).unwrap();
        assert!(findings[0].contains("missing"), "{findings:?}");
    }

    #[test]
    fn a_second_root_license_file_fails() {
        let root = texts_root();
        fs::write(root.path().join("COPYING"), "text").unwrap();
        let findings = check_texts(root.path()).unwrap();
        assert!(findings[0].starts_with("COPYING"), "{findings:?}");
    }

    /// The pledge is the agreement's clause, not a promise beside it.
    #[test]
    fn the_pledge_quotes_the_agreement() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let agreement = fs::read_to_string(repo.join(AGREEMENT)).unwrap();
        let overview = fs::read_to_string(repo.join("docs/licensing/README.md")).unwrap();
        let clauses: Vec<&str> = agreement
            .lines()
            .filter(|line| {
                [
                    "(a) **Avail",
                    "(b) **Open",
                    "(c) **Commercial",
                    "(d) **Condition",
                ]
                .iter()
                .any(|start| line.starts_with(start))
            })
            .collect();
        assert_eq!(clauses.len(), 4);
        for clause in clauses {
            assert!(overview.contains(&format!("> {clause}")), "{clause}");
        }
    }

    /// The agreement's pointer resolves: the section it names exists by that name.
    #[test]
    fn the_non_owner_pointer_resolves() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let agreement = fs::read_to_string(repo.join(AGREEMENT)).unwrap();
        let overview = fs::read_to_string(repo.join("docs/licensing/README.md")).unwrap();
        let section = "Contributions containing material you do not own";
        assert!(agreement.contains(&format!("\"{section}\"")));
        assert!(overview.contains(&format!("## {section}\n")));
    }

    // ── Allowlists ───────────────────────────────────────────────────────────

    #[test]
    fn the_frontend_list_is_read_from_its_constant() {
        let source = "const OTHER = [\"x\"];\nconst LICENSE_ALLOWLIST = [\n  \"MIT\",\n  \"Apache-2.0 WITH LLVM-exception\",\n];\n";
        assert_eq!(
            typescript_string_array(source, NUXT_ALLOWLIST).unwrap(),
            ["MIT", "Apache-2.0 WITH LLVM-exception"]
        );
        assert!(typescript_string_array("const X = 1;", NUXT_ALLOWLIST).is_none());
    }

    #[test]
    fn lists_differing_by_one_id_name_it_and_the_side() {
        let deny = ["MIT", "ISC"].map(str::to_owned);
        let frontend = ["MIT", "Zlib"].map(str::to_owned);
        let findings = compare_allowlists(&frontend, &deny);
        assert_eq!(findings.len(), 2, "{findings:?}");
        assert!(
            findings[0].starts_with(DENY) && findings[0].contains("Zlib"),
            "{findings:?}"
        );
        assert!(
            findings[1].starts_with(NUXT_CONFIG) && findings[1].contains("ISC"),
            "{findings:?}"
        );
    }

    #[test]
    fn identical_lists_pass() {
        let list = allowlist();
        assert!(compare_allowlists(&list, &list).is_empty());
    }

    // ── Publication ──────────────────────────────────────────────────────────

    fn agreement(text: &str, metadata: &str) -> Agreement {
        Agreement {
            text: text.to_owned(),
            metadata: metadata.to_owned(),
        }
    }

    const V1: &str = "**Agreement — version 1.0**\n\nThe text.\n";
    const V1_EDITED: &str = "**Agreement — version 1.0**\n\nThe edited text.\n";
    const V2: &str = "**Agreement — version 2.0**\n\nThe text.\n";
    const DRAFT: &str = "**Agreement — version 1.0-draft**\n\nWithin [SUCCESSION_WINDOW].\n";

    #[test]
    fn identical_copies_are_current_everywhere() {
        let publication = compare(&agreement(V1, "{}"), &agreement(V1, "{}"));
        assert_eq!(publication, Publication::Current);
        assert_eq!(judge(publication, Branch::PullRequest), None);
        assert_eq!(judge(publication, Branch::Main), None);
    }

    #[test]
    fn a_newer_version_is_pending_until_the_merge() {
        let publication = compare(&agreement(V2, "{}"), &agreement(V1, "{}"));
        assert_eq!(publication, Publication::Pending);
        assert_eq!(judge(publication, Branch::PullRequest), None);
        assert!(
            judge(publication, Branch::Main)
                .unwrap()
                .contains("publish")
        );
    }

    #[test]
    fn a_text_change_without_a_bump_fails_everywhere() {
        let publication = compare(&agreement(V1_EDITED, "{}"), &agreement(V1, "{}"));
        assert_eq!(publication, Publication::TextWithoutBump);
        assert!(judge(publication, Branch::PullRequest).is_some());
        assert!(judge(publication, Branch::Main).is_some());
    }

    #[test]
    fn metadata_drift_beside_an_identical_text_fails_everywhere() {
        let publication = compare(&agreement(V1, "{\"a\":1}"), &agreement(V1, "{}"));
        assert_eq!(publication, Publication::MetadataDrift);
        assert!(judge(publication, Branch::PullRequest).is_some());
        assert!(judge(publication, Branch::Main).is_some());
    }

    /// A publication writes both files, so a new form beside a new version is pending.
    #[test]
    fn a_pending_publication_covers_the_form_too() {
        let publication = compare(&agreement(V2, "{\"a\":1}"), &agreement(V1, "{}"));
        assert_eq!(publication, Publication::Pending);
    }

    /// The state from the merge until the first publication: the Gist holds a
    /// placeholder and an older form, and the repository's draft cannot go there.
    #[test]
    fn a_draft_with_blanks_passes_everywhere() {
        let placeholder = agreement("Placeholder until the CLA's written.", "{\"old\":1}");
        let publication = compare(&agreement(DRAFT, "{}"), &placeholder);
        assert_eq!(publication, Publication::UnpublishedDraft);
        assert_eq!(judge(publication, Branch::PullRequest), None);
        assert_eq!(judge(publication, Branch::Main), None);
    }

    #[test]
    fn a_markdown_link_is_not_a_blank() {
        assert!(!agreement("See [the page](x) and [Lowercase].", "").has_blank());
        assert!(agreement("Within [FALLBACK_ORGANIZATION].", "").has_blank());
    }

    /// The Gist, in memory.
    struct FakeGist {
        published: RefCell<Agreement>,
        writes: RefCell<usize>,
    }

    impl FakeGist {
        fn holding(agreement: Agreement) -> Self {
            Self {
                published: RefCell::new(agreement),
                writes: RefCell::new(0),
            }
        }
    }

    impl Gist for FakeGist {
        fn read(&self) -> Result<Agreement> {
            Ok(self.published.borrow().clone())
        }

        fn write(&self, agreement: &Agreement) -> Result<String> {
            *self.published.borrow_mut() = agreement.clone();
            *self.writes.borrow_mut() += 1;
            Ok("revision-sha".to_owned())
        }
    }

    #[test]
    fn publishing_a_new_version_writes_both_files() {
        let gist = FakeGist::holding(agreement(V1, "{}"));
        let repository = agreement(V2, "{\"a\":1}");
        assert_eq!(
            publish(&repository, &gist).unwrap(),
            Published::Revision("revision-sha".to_owned())
        );
        assert_eq!(*gist.published.borrow(), repository);
    }

    #[test]
    fn publishing_what_is_there_writes_nothing() {
        let gist = FakeGist::holding(agreement(V1, "{}"));
        assert_eq!(
            publish(&agreement(V1, "{}"), &gist).unwrap(),
            Published::AlreadyCurrent
        );
        assert_eq!(*gist.writes.borrow(), 0);
    }

    #[test]
    fn publishing_refuses_a_blank() {
        let gist = FakeGist::holding(agreement(V1, "{}"));
        let error = publish(&agreement(DRAFT, "{}"), &gist).unwrap_err();
        assert!(error.to_string().contains("bracketed blank"), "{error}");
        assert_eq!(*gist.writes.borrow(), 0);
    }

    #[test]
    fn publishing_refuses_a_text_change_without_a_bump() {
        let gist = FakeGist::holding(agreement(V1, "{}"));
        let error = publish(&agreement(V1_EDITED, "{}"), &gist).unwrap_err();
        assert!(error.to_string().contains("bump the version"), "{error}");
        assert_eq!(*gist.writes.borrow(), 0);
    }

    /// A signing form edited in the Gist is restored from the repository.
    #[test]
    fn publishing_restores_a_drifted_form() {
        let gist = FakeGist::holding(agreement(V1, "{\"hand\":1}"));
        assert_eq!(
            publish(&agreement(V1, "{}"), &gist).unwrap(),
            Published::Revision("revision-sha".to_owned())
        );
    }
}
