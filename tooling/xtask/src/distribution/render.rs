// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Renders a distribution channel's package manifests — names and contents —
//! into flavored package manifests for one release channel.
//!
//! A channel's `justfile` holds its validation recipes and is not a package
//! manifest, so it is never rendered. Rendering is all or nothing: every file
//! renders in memory before any is written.

use crate::{
    Channel,
    distribution::{context, registry::Registry},
    releases::manifest::Manifest,
};
use anyhow::{Context as _, Result, anyhow, bail};
use std::path::{Path, PathBuf};

/// The tree package manifests live in, from the repository root.
pub const DIST_DIR: &str = "distribution";

/// Distribution channels whose files are the artifact itself, not templates.
pub const NOTHING_TO_RENDER: [&str; 2] = ["kubernetes", "nix"];

/// Where `channel`'s renders for `flavor` are written, from the repository root.
pub fn out_dir(channel: &str, flavor: Channel) -> PathBuf {
    Path::new("target/dist").join(channel).join(flavor.flavor())
}

/// A flavored package manifest: its path under the output directory, and its
/// contents.
#[derive(Debug, PartialEq, Eq)]
pub struct Rendered {
    pub path: PathBuf,
    pub contents: String,
}

/// Every package manifest of `channel` rendered for `flavor` from `manifest`.
pub fn render(
    root: &Path,
    channel: &str,
    flavor: Channel,
    manifest: &Manifest,
) -> Result<Vec<Rendered>> {
    if NOTHING_TO_RENDER.contains(&channel) {
        bail!("{channel} keeps no package manifests; it has nothing to render");
    }
    let registry = Registry::read(root)?;
    let package_id = registry.identity(flavor, channel).ok_or_else(|| {
        let listed = registry.release_channels(channel);
        if listed.is_empty() {
            anyhow!("no release channel is published to {channel}")
        } else {
            anyhow!(
                "{flavor} is not published to {channel}; the registry lists {}",
                listed
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    })?;
    let context = context::build(root, flavor, package_id, manifest)?;

    let source = root.join(DIST_DIR).join(channel);
    let mut tera = tera::Tera::default();
    tera.autoescape_on(Vec::<&str>::new());
    tera.register_filter("pkgver", context::pkgver_filter);

    let mut rendered = Vec::new();
    for path in templates(&source)? {
        let relative = path
            .strip_prefix(&source)
            .expect("walked under the source")
            .to_string_lossy()
            .into_owned();
        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;

        let name = format!("name:{relative}");
        tera.add_raw_template(&name, &relative)
            .with_context(|| format!("parsing the name {relative}"))?;
        tera.add_raw_template(&relative, &contents)
            .with_context(|| format!("parsing {relative}"))?;
        rendered.push(Rendered {
            path: PathBuf::from(
                tera.render(&name, &context)
                    .with_context(|| format!("rendering the name {relative}"))?,
            ),
            contents: tera
                .render(&relative, &context)
                .with_context(|| format!("rendering {relative}"))?,
        });
    }
    if rendered.is_empty() {
        bail!("{} holds no package manifests", source.display());
    }
    Ok(rendered)
}

/// Writes `rendered` under `out`, replacing whatever an earlier render left.
pub fn write(out: &Path, rendered: &[Rendered]) -> Result<()> {
    if out.exists() {
        std::fs::remove_dir_all(out).with_context(|| format!("clearing {}", out.display()))?;
    }
    for file in rendered {
        let path = out.join(&file.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&path, &file.contents)
            .with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(())
}

/// Every file under `source` but its `justfile`, sorted, dotfiles included
/// (the AUR's `.SRCINFO`).
fn templates(source: &Path) -> Result<Vec<PathBuf>> {
    if !source.is_dir() {
        bail!("{} does not exist", source.display());
    }
    let mut files = Vec::new();
    for entry in ignore::WalkBuilder::new(source).hidden(false).build() {
        let entry = entry?;
        let is_file = entry.file_type().is_some_and(|kind| kind.is_file());
        if is_file && entry.path() != source.join("justfile") {
            files.push(entry.into_path());
        }
    }
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Manifest {
        serde_json::from_str(include_str!("../../tests/fixtures/release-manifest.json")).unwrap()
    }

    /// A scratch root with the committed registry and flavor configs, and one
    /// package channel holding `files`.
    fn scratch(files: &[(&str, &str)]) -> tempfile::TempDir {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let root = tempfile::tempdir().unwrap();
        let flavors = root.path().join("src-tauri/res/configs/flavors");
        std::fs::create_dir_all(&flavors).unwrap();
        for flavor in ["stable", "preview", "nightly"] {
            std::fs::copy(
                repo.join(format!("src-tauri/res/configs/flavors/{flavor}.toml")),
                flavors.join(format!("{flavor}.toml")),
            )
            .unwrap();
        }
        let channel = root.path().join("distribution/brew");
        std::fs::create_dir_all(&channel).unwrap();
        std::fs::copy(
            repo.join("distribution/channels.toml"),
            root.path().join("distribution/channels.toml"),
        )
        .unwrap();
        for (name, contents) in files {
            std::fs::write(channel.join(name), contents).unwrap();
        }
        root
    }

    const CASK: &str = r#"cask "{{ package.id }}" do
  version "{{ release.tag }}"
  sha256 "{{ artifact.macos_arm64.dmg.sha256 }}"
  url "{{ artifact.macos_arm64.dmg.url }}"
end
"#;

    #[test]
    fn a_file_name_renders_through_the_context() {
        let root = scratch(&[("{{ package.id }}.rb", CASK), ("justfile", "audit:\n")]);
        let rendered = render(root.path(), "brew", Channel::Nightly, &fixture()).unwrap();
        assert_eq!(rendered.len(), 1, "the justfile is not a package manifest");
        assert_eq!(rendered[0].path, Path::new("waypoints@nightly.rb"));
        assert!(
            rendered[0]
                .contents
                .starts_with("cask \"waypoints@nightly\" do")
        );
        assert!(
            rendered[0]
                .contents
                .contains("version \"v0.1.0-nightly.20261116\"")
        );
    }

    #[test]
    fn rendering_twice_is_byte_identical() {
        let root = scratch(&[("{{ package.id }}.rb", CASK)]);
        let first = render(root.path(), "brew", Channel::Nightly, &fixture()).unwrap();
        let second = render(root.path(), "brew", Channel::Nightly, &fixture()).unwrap();
        assert_eq!(first, second);

        let out = root.path().join("out");
        write(&out, &first).unwrap();
        let written = std::fs::read(out.join("waypoints@nightly.rb")).unwrap();
        write(&out, &second).unwrap();
        assert_eq!(
            std::fs::read(out.join("waypoints@nightly.rb")).unwrap(),
            written
        );
    }

    #[test]
    fn a_placeholder_the_context_lacks_fails_the_render() {
        let root = scratch(&[("{{ package.id }}.rb", "version \"{{ release.nope }}\"\n")]);
        let error = render(root.path(), "brew", Channel::Nightly, &fixture())
            .expect_err("an unknown placeholder is refused");
        assert!(format!("{error:#}").contains("release.nope"), "{error:#}");
    }

    #[test]
    fn a_release_channel_the_registry_omits_is_refused_naming_the_listed_ones() {
        let root = scratch(&[("{{ package.id }}.rb", CASK)]);
        std::fs::write(
            root.path().join("distribution/channels.toml"),
            "[stable]\nbrew = \"waypoints\"\n[preview]\nbrew = \"waypoints@preview\"\n",
        )
        .unwrap();
        let error = render(root.path(), "brew", Channel::Nightly, &fixture())
            .expect_err("an unlisted release channel is refused");
        assert!(
            format!("{error:#}").contains("the registry lists stable, preview"),
            "{error:#}"
        );
    }

    #[test]
    fn nix_and_kubernetes_have_nothing_to_render() {
        let root = scratch(&[]);
        for channel in NOTHING_TO_RENDER {
            let error =
                render(root.path(), channel, Channel::Stable, &fixture()).expect_err("refused");
            assert!(
                format!("{error:#}").contains("nothing to render"),
                "{error:#}"
            );
        }
    }
}
