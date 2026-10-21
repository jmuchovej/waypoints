//! Generates every app icon from the authored artwork in `src-tauri/res/`.
//!
//! Typst renders: `res/icon/icons.typ` writes a staging tree mirroring
//! `src-tauri/`, plus a `manifest.json` naming what to pack, which outputs this
//! task owns, and what to place. This task packs the containers Typst cannot
//! write (`.icns`, `.ico`, lossless WebP), then prunes and places. It knows no
//! sizes, colors, or artwork — all of that arrives in the manifest.
//!
//! `src-tauri/` is untouched until rendering and packing have both succeeded.

use std::{
    fs::{self, File},
    io::{self, BufReader, BufWriter},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context as _, Result, bail, ensure};
use glob::{MatchOptions, Pattern};
use serde::Deserialize;

use crate::{Flavor, workspace};

/// What `icons.typ` asks of this task. Every path is relative to both the
/// staging root and `src-tauri/`, which share a layout.
#[derive(Deserialize)]
struct Manifest {
    /// Glob patterns for every file the generator owns; pruned before placing.
    owned: Vec<String>,
    icns: Vec<Container<IcnsMember>>,
    ico: Vec<Container<String>>,
    webp: Vec<Webp>,
    /// Rendered outputs placed as they are.
    place: Vec<String>,
}

#[derive(Deserialize)]
struct Container<M> {
    file: String,
    members: Vec<M>,
}

#[derive(Deserialize)]
struct IcnsMember {
    os: String,
    png: String,
}

#[derive(Deserialize)]
struct Webp {
    from: String,
    to: String,
}

impl Manifest {
    fn outputs(&self) -> Vec<&str> {
        self.place
            .iter()
            .map(String::as_str)
            .chain(self.icns.iter().map(|icns| icns.file.as_str()))
            .chain(self.ico.iter().map(|ico| ico.file.as_str()))
            .chain(self.webp.iter().map(|webp| webp.to.as_str()))
            .collect()
    }
}

/// `*` stays within a path component; only `**` crosses one.
const MATCH: MatchOptions = MatchOptions {
    case_sensitive: true,
    require_literal_separator: true,
    require_literal_leading_dot: false,
};

/// What one run placed under `src-tauri/`.
pub struct Generated {
    /// How many outputs were placed.
    pub placed: usize,
    /// Owned files that were pruned and which no source describes any more.
    pub orphans: Vec<String>,
}

/// Renders every flavor in `flavors` — every flavor when empty — packs the
/// containers, and places the outputs.
pub fn generate(flavors: &[Flavor]) -> Result<Generated> {
    let metadata = workspace::load()?;
    let app = workspace::app_dir(&metadata)?;
    let staging = metadata
        .target_directory
        .join("icongen")
        .into_std_path_buf();

    if staging.exists() {
        fs::remove_dir_all(&staging)
            .with_context(|| format!("Failed to clear {}", staging.display()))?;
    }
    fs::create_dir_all(&staging)?;

    render(&app, &staging, flavors)?;
    let manifest: Manifest = serde_json::from_slice(&fs::read(staging.join("manifest.json"))?)
        .context("icons.typ wrote a manifest this task cannot read")?;
    pack(&staging, &manifest)?;

    let outputs = manifest.outputs();
    let owned = patterns(&manifest.owned)?;
    for output in &outputs {
        ensure!(
            staging.join(output).is_file(),
            "the render did not produce `{output}`"
        );
        ensure!(
            owned
                .iter()
                .any(|pattern| pattern.matches_with(output, MATCH)),
            "`{output}` matches no owned pattern, so a later run could never prune it"
        );
    }

    let pruned = prune(&app, &manifest.owned)?;
    for output in &outputs {
        let to = app.join(output);
        fs::create_dir_all(to.parent().expect("outputs are files under the app"))?;
        fs::copy(staging.join(output), &to)
            .with_context(|| format!("Failed to place {}", to.display()))?;
    }

    let orphans = pruned
        .into_iter()
        .filter(|path| !outputs.contains(&path.as_str()))
        .collect();
    Ok(Generated {
        placed: outputs.len(),
        orphans,
    })
}

fn render(app: &Path, staging: &Path, flavors: &[Flavor]) -> Result<()> {
    // The marketing pipeline's flags: `pt` is a pixel at 72 ppi, and the app
    // directory is the root, so `/res/…` resolves.
    let mut typst = Command::new("typst");
    typst.arg("compile").arg("--root").arg(app).args([
        "--features",
        "bundle",
        "--format",
        "bundle",
        "--ppi",
        "72",
    ]);
    if !flavors.is_empty() {
        let names = flavors.iter().map(Flavor::to_string).collect::<Vec<_>>();
        typst
            .arg("--input")
            .arg(format!("flavors={}", names.join(",")));
    }
    typst.arg(app.join("res/icon/icons.typ")).arg(staging);

    let output = match typst.output() {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            bail!("`typst` is not on PATH; run this inside the devenv shell")
        }
        result => result.context("Failed to run `typst`")?,
    };
    if !output.status.success() {
        bail!(
            "`typst compile` failed:\n{}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn pack(staging: &Path, manifest: &Manifest) -> Result<()> {
    for icns in &manifest.icns {
        let mut family = icns::IconFamily::new();
        for member in &icns.members {
            let ostype = member
                .os
                .parse::<icns::OSType>()
                .map_err(anyhow::Error::msg)?;
            let icon_type = icns::IconType::from_ostype(ostype)
                .with_context(|| format!("`{}` is not an icon OSType", member.os))?;
            let image = icns::Image::read_png(open(staging, &member.png)?)?;
            family
                .add_icon_with_type(&image, icon_type)
                .with_context(|| format!("{} as `{}`", member.png, member.os))?;
        }
        family.write(create(staging, &icns.file)?)?;
    }

    for ico in &manifest.ico {
        let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
        for member in &ico.members {
            let image = ico::IconImage::read_png(open(staging, member)?)?;
            dir.add_entry(ico::IconDirEntry::encode_as_png(&image)?);
        }
        dir.write(create(staging, &ico.file)?)?;
    }

    for webp in &manifest.webp {
        let image = image::open(staging.join(&webp.from))
            .with_context(|| format!("Failed to read {}", webp.from))?
            .into_rgba8();
        image::codecs::webp::WebPEncoder::new_lossless(create(staging, &webp.to)?).encode(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )?;
    }
    Ok(())
}

/// Removes every existing file under `app` matching an owned pattern, and
/// returns them relative to `app`.
fn prune(app: &Path, owned: &[String]) -> Result<Vec<String>> {
    let base = Pattern::escape(app.to_str().context("the app path is not UTF-8")?);
    let mut removed = Vec::new();
    for pattern in owned {
        for path in glob::glob_with(&format!("{base}/{pattern}"), MATCH)? {
            let path = path?;
            if !path.is_file() {
                continue;
            }
            fs::remove_file(&path)
                .with_context(|| format!("Failed to prune {}", path.display()))?;
            removed.push(relative(app, &path)?);
        }
    }
    Ok(removed)
}

fn patterns(owned: &[String]) -> Result<Vec<Pattern>> {
    owned
        .iter()
        .map(|pattern| {
            Pattern::new(pattern).with_context(|| format!("bad owned pattern `{pattern}`"))
        })
        .collect()
}

fn relative(app: &Path, path: &Path) -> Result<String> {
    Ok(path
        .strip_prefix(app)?
        .to_str()
        .context("a pruned path is not UTF-8")?
        .to_owned())
}

fn open(staging: &Path, file: &str) -> Result<BufReader<File>> {
    let path = staging.join(file);
    Ok(BufReader::new(File::open(&path).with_context(|| {
        format!("Failed to open {}", path.display())
    })?))
}

fn create(staging: &Path, file: &str) -> Result<BufWriter<File>> {
    let path: PathBuf = staging.join(file);
    fs::create_dir_all(path.parent().expect("outputs are files under the root"))?;
    Ok(BufWriter::new(File::create(&path).with_context(|| {
        format!("Failed to create {}", path.display())
    })?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owns(pattern: &str, path: &str) -> bool {
        Pattern::new(pattern).unwrap().matches_with(path, MATCH)
    }

    #[test]
    fn the_desktop_directory_is_owned_recursively() {
        assert!(owns(
            "assets/icons/app/**/*",
            "assets/icons/app/stable-032.png"
        ));
        assert!(!owns(
            "assets/icons/app/**/*",
            "assets/icons/menu/stable.png"
        ));
    }

    /// Pattern matching and the filesystem walk disagree on a bare `**`, which
    /// yields only directories when walking — so prune against a real tree.
    #[test]
    fn pruning_removes_owned_files_and_nothing_else() {
        let app = tempfile::tempdir().unwrap();
        for file in [
            "assets/icons/app/stable-032.png",
            "assets/icons/app/orphan.ico",
            "assets/icons/menu/stable.png",
            "gen/android/res/mipmap-hdpi/ic_launcher_foreground.webp",
            "gen/android/res/mipmap-hdpi/splash.png",
        ] {
            let path = app.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"").unwrap();
        }

        let mut removed = prune(
            app.path(),
            &[
                "assets/icons/app/**/*".to_owned(),
                "gen/android/res/mipmap-hdpi/ic_launcher_*".to_owned(),
            ],
        )
        .unwrap();
        removed.sort();

        assert_eq!(
            removed,
            [
                "assets/icons/app/orphan.ico",
                "assets/icons/app/stable-032.png",
                "gen/android/res/mipmap-hdpi/ic_launcher_foreground.webp",
            ]
        );
        assert!(app.path().join("assets/icons/menu/stable.png").exists());
        assert!(
            app.path()
                .join("gen/android/res/mipmap-hdpi/splash.png")
                .exists()
        );
    }

    #[test]
    fn a_flavor_pattern_owns_its_files_and_no_other_flavors() {
        let dev = "assets/icons/app/dev[-.]*";
        assert!(owns(dev, "assets/icons/app/dev-032.png"));
        assert!(owns(dev, "assets/icons/app/dev.icns"));
        assert!(!owns(dev, "assets/icons/app/development.ico"));
    }

    #[test]
    fn android_ownership_stops_at_the_launcher_icons() {
        let mipmap = "gen/android/app/src/stable/res/mipmap-hdpi/ic_launcher_*";
        assert!(owns(
            mipmap,
            "gen/android/app/src/stable/res/mipmap-hdpi/ic_launcher_foreground.webp"
        ));
        assert!(!owns(
            mipmap,
            "gen/android/app/src/stable/res/mipmap-hdpi/splash.png"
        ));
        assert!(!owns(
            mipmap,
            "gen/android/app/src/stable/res/mipmap-hdpi/nested/ic_launcher_x.webp"
        ));
    }

    #[test]
    fn outputs_cover_every_placed_and_packed_file() {
        let manifest: Manifest = serde_json::from_str(
            r#"{
                "owned": [],
                "icns": [{ "file": "a.icns", "members": [{ "os": "ic07", "png": "m/128.png" }] }],
                "ico": [{ "file": "a.ico", "members": ["m/16.png"] }],
                "webp": [{ "from": "m/f.png", "to": "f.webp" }],
                "place": ["a.png"]
            }"#,
        )
        .unwrap();
        assert_eq!(manifest.outputs(), ["a.png", "a.icns", "a.ico", "f.webp"]);
    }
}
