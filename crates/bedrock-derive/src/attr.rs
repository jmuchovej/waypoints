//! Attribute parsing shared by the derives: enough of serde's `rename` grammar
//! to keep stored names in sync with the virtual document, plus bedrock's own
//! `#[bedrock(id)]`.

use syn::{Attribute, LitStr};

/// The `rename_all` rule from `#[serde(rename_all = "...")]`, if present.
pub(crate) fn serde_rename_all(attrs: &[Attribute]) -> syn::Result<Option<String>> {
    extract_serde_string(attrs, "rename_all")
}

/// The `rename` value from `#[serde(rename = "...")]`, if present.
pub(crate) fn serde_rename(attrs: &[Attribute]) -> syn::Result<Option<String>> {
    extract_serde_string(attrs, "rename")
}

/// Bedrock's field-level attributes: `#[bedrock(id)]`, `#[bedrock(index)]`,
/// `#[bedrock(unique)]`.
#[derive(Default)]
pub(crate) struct FieldAttrs {
    pub is_id: bool,
    /// `Some(unique)` if the field is singly indexed; `None` otherwise.
    pub index: Option<bool>,
}

pub(crate) fn field_attrs(attrs: &[Attribute]) -> syn::Result<FieldAttrs> {
    let mut out = FieldAttrs::default();
    for attr in attrs {
        if !attr.path().is_ident("bedrock") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("id") {
                out.is_id = true;
                Ok(())
            } else if meta.path.is_ident("index") {
                out.index.get_or_insert(false);
                Ok(())
            } else if meta.path.is_ident("unique") {
                out.index = Some(true);
                Ok(())
            } else {
                Err(meta
                    .error("unknown bedrock field attribute; expected `id`, `index`, or `unique`"))
            }
        })?;
    }
    Ok(out)
}

/// Bedrock's container-level attributes, parsed in one pass (the parser rejects
/// unknown keys, so every recognized key must be handled here together):
/// composite indexes `#[bedrock(index("a", "b"))]` / `#[bedrock(unique("a", "b"))]`
/// and the write hook `#[bedrock(before_save = path)]`.
#[derive(Default)]
pub(crate) struct ContainerAttrs {
    /// `(columns, unique)` per composite index.
    pub indexes: Vec<(Vec<String>, bool)>,
    /// The `#[bedrock(before_save = path)]` function, if declared.
    pub before_save: Option<syn::Path>,
}

pub(crate) fn container_attrs(attrs: &[Attribute]) -> syn::Result<ContainerAttrs> {
    let mut out = ContainerAttrs::default();
    for attr in attrs {
        if !attr.path().is_ident("bedrock") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("before_save") {
                if out.before_save.is_some() {
                    return Err(meta.error("duplicate #[bedrock(before_save = ...)]"));
                }
                out.before_save = Some(meta.value()?.parse()?);
                Ok(())
            } else if meta.path.is_ident("index") || meta.path.is_ident("unique") {
                let unique = meta.path.is_ident("unique");
                let content;
                syn::parenthesized!(content in meta.input);
                let columns: syn::punctuated::Punctuated<LitStr, syn::Token![,]> =
                    syn::punctuated::Punctuated::parse_terminated(&content)?;
                if columns.is_empty() {
                    return Err(meta.error("a composite index needs at least one column"));
                }
                out.indexes
                    .push((columns.iter().map(LitStr::value).collect(), unique));
                Ok(())
            } else {
                Err(meta.error(
                    "unknown bedrock container attribute; expected `index(...)`, \
                     `unique(...)`, or `before_save = path`",
                ))
            }
        })?;
    }
    Ok(out)
}

/// Apply a serde `rename_all` rule to an identifier.
pub(crate) fn apply_rename_all(ident: &str, rule: &str) -> String {
    use heck::{
        ToKebabCase, ToLowerCamelCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase,
        ToUpperCamelCase,
    };
    match rule {
        "lowercase" => ident.to_lowercase(),
        "UPPERCASE" => ident.to_uppercase(),
        "PascalCase" => ident.to_upper_camel_case(),
        "camelCase" => ident.to_lower_camel_case(),
        "snake_case" => ident.to_snake_case(),
        "SCREAMING_SNAKE_CASE" => ident.to_shouty_snake_case(),
        "kebab-case" => ident.to_kebab_case(),
        "SCREAMING-KEBAB-CASE" => ident.to_shouty_kebab_case(),
        // Unknown rule: leave the identifier untouched rather than guess.
        _ => ident.to_string(),
    }
}

fn extract_serde_string(attrs: &[Attribute], key: &str) -> syn::Result<Option<String>> {
    let mut found = None;
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(key) {
                let lit: LitStr = meta.value()?.parse()?;
                found = Some(lit.value());
                Ok(())
            } else {
                // Skip any other serde entry (`flag`, `key = expr`, or `key(..)`)
                // without choking on grammar we don't care about.
                skip_serde_meta(&meta)
            }
        })?;
    }
    Ok(found)
}

fn skip_serde_meta(meta: &syn::meta::ParseNestedMeta) -> syn::Result<()> {
    if meta.input.peek(syn::Token![=]) {
        let value = meta.value()?;
        let _: syn::Expr = value.parse()?;
    } else if meta.input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in meta.input);
        let _: proc_macro2::TokenStream = content.parse()?;
    }
    Ok(())
}
