//! Generates every app icon from the authored artwork in `src-tauri/res/`.
//! See [`xtask::icongen`].

use anyhow::Result;
use clap::Parser;
use xtask::{Flavor, icongen};

#[derive(Parser)]
pub struct Options {
    /// Generate only this flavor; repeat for several. Every flavor by default.
    #[arg(long = "flavor", value_enum, value_name = "FLAVOR")]
    flavors: Vec<Flavor>,
}

pub fn run(args: Options) -> Result<()> {
    let generated = icongen::generate(&args.flavors)?;
    println!("icongen: placed {} files", generated.placed);
    for orphan in generated.orphans {
        println!("icongen: removed {orphan}, which no source describes any more");
    }
    Ok(())
}
