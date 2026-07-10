//! Derive macros for bedrock's schema vocabulary.
//!
//! Three derives turn a Rust type into something bedrock can store. They all
//! generate code against `::bedrock_core`, so a consumer depends on both this
//! crate and `bedrock-core`.
//!
//! - [`Scalar`](macro@Scalar) — a newtype over a scalar, or a fieldless enum,
//!   that promotes to a single indexable column.
//! - [`Document`](macro@Document) — a composite type stored as a `jsonb`
//!   document, round-tripped through serde.
//! - [`Collection`](macro@Collection) — an entity mapped to a whole collection
//!   (table), producing its [`CollectionSpec`] and row split/merge.
//!
//! [`CollectionSpec`]: bedrock_core::CollectionSpec

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod attr;
mod collection;
mod document;
mod scalar;

/// Derive [`Field`](bedrock_core::Field) for a scalar type.
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

/// Derive [`Field`](bedrock_core::Field) for a composite type stored as a
/// `jsonb` document.
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

/// Derive [`Collection`](bedrock_core::Collection) for an entity struct.
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
