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

//! `bedrock-core`: the schema vocabulary at the heart of bedrock.
//!
//! This crate is pure and engine-free. It answers one question — *how does a
//! Rust type map into storage?* — and nothing else. No turso, no SQL, no I/O.
//!
//! The load-bearing piece is the [`Field`] trait: every field type declares its
//! [`FieldKind`] (a promotable [`ColumnType`] scalar, or a `jsonb`
//! [`Document`](FieldKind::Document)) and round-trips through a [`StoredValue`].

mod collection;
mod field;
mod modify;
mod query;
mod spec;
mod value;
mod view;

pub use collection::{Collection, WriteOp};
pub use field::{DecodeError, EncodeError, Field, FieldKind};
pub use modify::{Assignment, Update};
pub use query::{CompareOp, Filter, Query, QueryError, ScalarValue, Sort};
pub use spec::{CollectionSpec, FieldSpec, IdSpec, IndexSpec, SpecError};
pub use value::{ColumnType, StoredValue};
pub use view::{Discriminant, ViewColumn, ViewScope, ViewSpec};
