use crate::keys;
use anyhow::Result;
use clap::Parser;
use ed25519_dalek::SigningKey;
use std::path::PathBuf;

/// Write a fresh Ed25519 seed to a file. The same bytes serve as a Station's
/// identity key (`station --key-file`) or a Fleet's signing key
/// (`station --fleet-key-file`); the flag decides the role, not the file.
/// Meant to run before first boot, so the file can become a Secret.
#[derive(Parser)]
pub struct KeygenArgs {
    /// Where to write the 32-byte seed. Created with mode 0600; an existing
    /// file is never overwritten.
    #[arg(long)]
    out: PathBuf,
}

pub fn run(args: KeygenArgs) -> Result<()> {
    let key = SigningKey::generate(&mut rand::rng());
    keys::write_new(&args.out, &key.to_bytes())?;
    tracing::info!(path = %args.out.display(), "wrote a new key");
    Ok(())
}
