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

/// Whether a field carries `#[bedrock(id)]`. Errors on any other `#[bedrock(..)]`.
pub(crate) fn has_bedrock_id(attrs: &[Attribute]) -> syn::Result<bool> {
    let mut is_id = false;
    for attr in attrs {
        if !attr.path().is_ident("bedrock") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("id") {
                is_id = true;
                Ok(())
            } else {
                Err(meta.error("unknown bedrock attribute; the only field attribute is `id`"))
            }
        })?;
    }
    Ok(is_id)
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
