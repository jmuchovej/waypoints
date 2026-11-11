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

//! Derive macros for bedrock's schema vocabulary.
//!
//! Three derives turn a Rust type into something bedrock can store. They emit
//! code against `::bedrock` (the façade crate), which re-exports both these
//! derives and the core vocabulary — so a consumer depends only on `bedrock`.
//!
//! - [`Scalar`](macro@Scalar) — a newtype over a scalar, or a fieldless enum,
//!   that promotes to a single indexable column.
//! - [`Document`](macro@Document) — a composite type stored as a `jsonb`
//!   document, round-tripped through serde.
//! - [`Collection`](macro@Collection) — an entity mapped to a whole collection
//!   (table), producing its `CollectionSpec` and row split/merge.

// Features are additive and flavors are not: one shipped flavor per build.
#[cfg(all(feature = "nightly", feature = "preview"))]
compile_error!(
    "bedrock-derive: the `nightly` and `preview` flavors are both enabled; a build takes exactly one"
);
#[cfg(all(feature = "nightly", feature = "stable"))]
compile_error!(
    "bedrock-derive: the `nightly` and `stable` flavors are both enabled; a build takes exactly one"
);
#[cfg(all(feature = "preview", feature = "stable"))]
compile_error!(
    "bedrock-derive: the `preview` and `stable` flavors are both enabled; a build takes exactly one"
);

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod attr;
mod collection;
mod document;
mod scalar;

/// Derive `bedrock::Field` for a scalar type.
///
/// Applies to a newtype struct (one unnamed field), forwarding to the inner
/// type's `Field` impl, or to a fieldless enum, which maps to a `TEXT` column
/// via each variant's serde name (`#[serde(rename_all)]` / `#[serde(rename)]`
/// are honored).
#[proc_macro_derive(Scalar)]
pub fn derive_scalar(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    scalar::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derive `bedrock::Field` for a composite type stored as a `jsonb` document.
///
/// The type must also be `serde::Serialize` + `serde::Deserialize`; encoding and
/// decoding round-trip through `serde_json`.
#[proc_macro_derive(Document)]
pub fn derive_document(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    document::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derive `bedrock::Collection` for an entity struct.
///
/// Exactly one field must be marked `#[bedrock(id)]` — there is no implicit id.
/// The id field must be a scalar column (enforced at compile time). Field names
/// honor `#[serde(rename_all)]` / `#[serde(rename)]`.
#[proc_macro_derive(Collection, attributes(bedrock))]
pub fn derive_collection(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    collection::expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
