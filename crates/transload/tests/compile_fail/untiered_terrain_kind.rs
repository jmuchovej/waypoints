//! A new `TerrainKind` must not be able to ship without a tier assignment.
//!
//! The exporter's `tier` is a non-wildcard match, so adding a variant breaks
//! it and the workspace fails to build — which is the guarantee the transload
//! spec asks for. A test cannot add a variant to someone else's enum, so what
//! this pins is the property that guarantee rests on: a match over
//! `TerrainKind` is *required* to be exhaustive.
//!
//! If `TerrainKind` ever gained `#[non_exhaustive]`, this file would start
//! compiling — and the exporter would be forced into a wildcard arm, turning a
//! compile error into an edge that silently never reaches a Bundle. That is
//! exactly the regression worth a test.

use domain::TerrainKind;

fn main() {
    let kind = TerrainKind::Child;
    // `TerrainKind::Custom` is deliberately absent.
    let _tier = match kind {
        TerrainKind::Child => 0,
        TerrainKind::Reference => 1,
        TerrainKind::Reading(_, _) => 2,
    };
}
