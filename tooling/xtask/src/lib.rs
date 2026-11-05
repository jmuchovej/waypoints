// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of Waypoints.
//
// Waypoints is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Waypoints is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with Waypoints. If not, see <https://www.gnu.org/licenses/>.
//
// Additional permission under GNU AGPL version 3 section 7: this Program
// may be conveyed through application stores on the terms set out in
// docs/licensing/COPYING.exceptions.

//! `xtask` — the Rust-based dev-toolchain for Waypoints.
//!
//! This crate is the library: everything the `cargo xtask` tasks are built
//! from — workflow generation, release planning and publishing, CI test
//! planning, TypeScript binding and icon generation, and the browser-dev HTTP
//! server. The binary under `src/bin/xtask/` is the CLI over it: argument
//! parsing, orchestration, and printing, nothing more. Nothing in this crate
//! ships in a release artifact, so a module can depend on app crates (e.g.
//! `cartograph`) without gating one.

#![allow(clippy::disallowed_macros)]

mod channel;
mod platform;
pub use channel::{Channel, Flavor};
pub use platform::{Arch, Platform, PublishedTarget};

/// The browser-dev HTTP backend: the runtime, its routes, and its errors.
pub mod dev {
    pub mod commands;
    mod error;
    mod instance;
    mod registry;

    pub use error::ApiError;
    pub use instance::{AppState, Instance, Instances};
    pub use registry::SettingsRegistryStore;
}

/// The decisions a CI run reads back as step outputs: what to test, and
/// which version to mint.
pub mod planners {
    pub mod nightly;
    pub mod release;
    pub mod test;
}

/// Staging, publishing, and recording a release, and the versions it is
/// all keyed by.
pub mod releases {
    pub mod artifacts;
    pub mod index;
    pub mod manifest;
    pub mod store;
    pub mod version;
}

pub mod icongen;
pub mod license {
    struct FrozenDocument {
        path: &'static str,
        checksum: &'static str,
    }

    impl FrozenDocument {
        const fn new(path: &'static str, checksum: &'static str) -> Self {
            Self { path, checksum }
        }
    }

    /// The license of every first-party crate and source file.
    pub const POLICY: &str = "AGPL-3.0-or-later";
    /// The copyright holder a first-party source file names.
    pub const HOLDER: &str = "John Muchovej";

    /// The tag a copyright line opens with, comment syntax aside.
    pub const SPDX_COPYRIGHT_TAG: &str = "SPDX-FileCopyrightText:";
    /// The tag a license line opens with, comment syntax aside.
    const SPDX_IDENTIFIER_TAG: &str = "SPDX-License-Identifier:";

    const LICENSE: FrozenDocument = FrozenDocument::new(
        "LICENSE",
        "0d96a4ff68ad6d4b6f1f30f713b18d5184912ba8dd389f86aa7710db079abcb0",
    );
    const LICENSE_MD: FrozenDocument = FrozenDocument::new(
        "docs/licensing/LICENSE.md",
        "00352ab19865e23bb0ab7e0f45332206aba6a66d75b3f3cc962fdd6508d63ea4",
    );

    #[cfg(test)]
    const CLA_PATH: &str = "docs/licensing/CLA.md";

    /// Where the notices are committed, relative to the workspace root.
    pub const NOTICES_PATH: &str = "docs/licensing/THIRD-PARTY-NOTICES.md";

    mod headers;
    mod notices;

    pub use headers::{Entry, Kind, Registry};
    pub use headers::{check as check_headers, copyright_year};
    pub use notices::generate as generate_notices;
}
pub mod tools;
pub mod typegen;
pub mod workflows;
pub mod workspace;
