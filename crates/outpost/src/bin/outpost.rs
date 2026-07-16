//! The `outpost` self-host binary — a thin wrapper over [`outpost::cli`].

fn main() -> anyhow::Result<()> {
    outpost::cli::main()
}
