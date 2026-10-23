//! Mints the release tag for a Nightly workflow run. The planning itself is
//! [`xtask::planners::nightly`]; this reads the checkout, decides, and
//! publishes the plan as the step's outputs.

use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use xtask::{
    planners::nightly::{self, Decision, Plan},
    releases::version::Version,
    tools::{git, github},
    workspace,
};

#[derive(Parser)]
pub struct Options {
    /// Build without publishing: no tag is minted and nothing is uploaded.
    ///
    /// The date doubles as the store build number, so a second publish on one
    /// day could not produce a distinct one. A dispatch exercises the pipeline
    /// instead, and the next scheduled run publishes. The artifacts still carry
    /// the version and build numbers that run would mint, so they can be
    /// uploaded by hand.
    #[arg(long)]
    build_only: bool,
}

pub fn run(args: Options) -> Result<()> {
    let target = Version::from(workspace::version()?).target();
    let tags = nightly::nightly_tags()?;
    let latest = nightly::latest_nightly(&target, &tags);
    let commits = nightly::commits_since(latest.as_ref())?;
    let stamp = nightly::date_stamp(Utc::now());

    let decision = nightly::decide(latest.as_ref(), &commits, &target, &stamp)?;

    if args.build_only {
        let version = decision.version();
        println!("Manual dispatch: building {version} without publishing.");
        return github::set_outputs(&Plan::build_only(version)?);
    }

    let plan = match decision {
        Decision::Current(_) => {
            println!("Nothing landed since the last nightly; nothing new to publish.");
            Plan::SKIP
        }
        Decision::New(version) => {
            let head = git::run(&["rev-parse", "HEAD"])?;
            let plan = Plan::publish(&version)?;
            github::create_tag(&plan.tag, &head)?;
            println!("Tagged nightly {tag} at {head}.", tag = plan.tag);
            plan
        }
    };

    github::set_outputs(&plan)
}
