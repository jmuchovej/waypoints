//! Step constructors shared across workflows.
//!
//! Each returns a fully-labelled step, so a workflow reads as a list of
//! intentions rather than a pile of `uses:`/`run:` noise.

use gh_workflow::{Run, Step, Use};

use super::actions;

pub(crate) fn checkout() -> Step<Use> {
    actions::CHECKOUT.step("Checkout").id("checkout")
}

pub(crate) fn install_nix() -> Step<Use> {
    actions::INSTALL_NIX.step("Install Nix").id("setup-nix")
}

/// Reads (and, given a token, writes) the named Cachix binary cache.
pub(crate) fn cachix(cache: &str) -> Step<Use> {
    actions::CACHIX
        .step(format!("Use the {cache} binary cache"))
        .id(format!("setup-{cache}-cachix"))
        .add_with(("name", cache))
}

/// Installs flake outputs into the ambient profile, for tools needed *before*
/// the devenv shell can be entered.
pub(crate) fn nix_profile_install(packages: &[&str]) -> Step<Run> {
    let refs = packages
        .iter()
        .map(|package| format!("nixpkgs#{package}"))
        .collect::<Vec<_>>()
        .join(" ");

    Step::new(format!("Install {}", packages.join(" and ")))
        .run(format!("nix profile install {refs}"))
        .id("setup-nix-profile")
}

/// Runs a `just` recipe. The single entry point for real work — CI should ask
/// the justfile for things, never inline its own copy of a command.
pub(crate) fn just(recipe: &str) -> Step<Run> {
    Step::new(format!("just {recipe}"))
        .run(format!("just {recipe}"))
        .id(recipe)
}
