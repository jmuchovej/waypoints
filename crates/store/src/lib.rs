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

pub(crate) mod error;
pub use error::{StoreError, StoreResult};

pub mod collections;
mod store;
pub use store::Store;

pub(crate) mod tools;
pub use tools::{checkpoint, open, reset};

pub mod query {
    pub mod chart;
    pub mod mark;
    pub mod sync;
    pub mod terrain;
}

#[cfg(feature = "test-utils")]
pub mod test_utils;

/// Built-in [`Chart`][C] types that provide common schemas out-of-the-box for
/// Waypoints.
///
/// These built-ins serve as:
/// - Ready-to-use types for common use-cases.
/// - Templates that users can build upon to create their own custom charts.
/// - Defaults that can be reset if users break their schemas.
///
/// The built-in system follows these principles:
/// - Each built-in chart has a fixed [`ChartID`][id] for consistency.
/// - Uses can reset to these original schemas at any time.
/// - Built-ins cover common content management and productivity needs.
///
/// [C]: domain::Chart
/// [id]: domain::ChartID
pub mod builtin {
    mod _utils;
    use _utils::{status_multi_select, tags_feature};

    pub mod book;
    pub mod bookmark;
    pub mod campaign;
    pub mod journal;
    pub mod meeting;
    pub mod note;
    pub mod objective;
    pub mod page;
    pub mod person;
}
