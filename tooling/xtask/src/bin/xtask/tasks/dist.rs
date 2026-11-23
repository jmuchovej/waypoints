// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Renders package manifests for a release, and refuses stamped ones. See
//! [`xtask::distribution`].

use anyhow::{Context as _, Result, bail};
use clap::{Parser, Subcommand};
use std::path::Path;
use xtask::{
    Channel,
    distribution::{check, context::DOWNLOADS, render},
    releases::manifest::Manifest,
};

#[derive(Parser)]
pub struct Options {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Renders a distribution channel's package manifests for one release channel into `target/dist/`.
    Render(RenderOptions),
    /// Fails if any file under `distribution/` carries a concrete tag, digest, or versioned URL.
    Check,
}

#[derive(Parser)]
struct RenderOptions {
    /// The distribution channel: a directory under `distribution/`.
    channel: String,

    /// The release channel to render for.
    #[arg(long)]
    flavor: Channel,

    /// The release manifest to render from: a file, or an `http(s)` URL.
    /// Defaults to the download surface's copy of the flavor's manifest.
    #[arg(long)]
    manifest: Option<String>,
}

pub fn run(args: Options) -> Result<()> {
    match args.command {
        Command::Render(args) => render(args),
        Command::Check => check(),
    }
}

fn render(args: RenderOptions) -> Result<()> {
    let source = args
        .manifest
        .unwrap_or_else(|| format!("{DOWNLOADS}/{}/manifest", args.flavor.flavor()));
    let manifest = read_manifest(&source)?;
    let root = Path::new(".");
    let rendered = render::render(root, &args.channel, args.flavor, &manifest)?;
    let out = render::out_dir(&args.channel, args.flavor);
    render::write(&out, &rendered)?;
    for file in &rendered {
        println!("{}", out.join(&file.path).display());
    }
    Ok(())
}

fn check() -> Result<()> {
    let stamps = check::scan(Path::new("."))?;
    if stamps.is_empty() {
        println!("no package manifest under distribution/ is stamped for a release");
        return Ok(());
    }
    for stamp in &stamps {
        eprintln!("{}:{}: {}", stamp.path.display(), stamp.line, stamp.kind);
    }
    bail!(
        "{} stamped value(s) under distribution/; package manifests take release values from the context",
        stamps.len()
    )
}

fn read_manifest(source: &str) -> Result<Manifest> {
    let body = if source.starts_with("https://") || source.starts_with("http://") {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("building the tokio runtime")?;
        runtime
            .block_on(async { reqwest::get(source).await?.error_for_status()?.text().await })
            .with_context(|| format!("fetching {source}"))?
    } else {
        std::fs::read_to_string(source).with_context(|| format!("reading {source}"))?
    };
    serde_json::from_str(&body)
        .with_context(|| format!("parsing the release manifest from {source}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_takes_a_channel_a_flavor_and_an_optional_manifest() {
        let parsed = Options::try_parse_from([
            "dist",
            "render",
            "brew",
            "--flavor",
            "nightly",
            "--manifest",
            "m.json",
        ])
        .unwrap();
        let Command::Render(render) = parsed.command else {
            panic!("render");
        };
        assert_eq!(render.channel, "brew");
        assert_eq!(render.flavor, Channel::Nightly);
        assert_eq!(render.manifest.as_deref(), Some("m.json"));
        assert!(
            Options::try_parse_from(["dist", "render", "brew", "--flavor", "development"]).is_err()
        );
    }
}
