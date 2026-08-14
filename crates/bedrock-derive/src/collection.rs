use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataStruct, DeriveInput, Field, Fields, Ident, Type};

use crate::attr;

struct FieldInfo<'a> {
    ident: &'a Ident,
    ty: &'a Type,
    name: String,
    /// `Some(unique)` if the field carries `#[bedrock(index)]`/`#[bedrock(unique)]`.
    index: Option<bool>,
    /// Set when the field carries `#[bedrock(discriminant)]`.
    discriminant: Option<attr::DiscriminantAttr>,
}

pub(crate) fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "Collection does not support generic entities",
        ));
    }

    let fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "Collection can only be derived for a struct with named fields",
            ));
        }
    };
    if fields.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "Collection requires at least one field",
        ));
    }

    let struct_rename_all = attr::serde_rename_all(&input.attrs)?;

    let mut infos: Vec<FieldInfo> = Vec::with_capacity(fields.len());
    let mut id: Option<usize> = None;
    for (position, field) in fields.iter().enumerate() {
        let ident = field.ident.as_ref().expect("named field has an ident");
        let field_attrs = attr::field_attrs(&field.attrs)?;
        if field_attrs.is_id {
            if id.is_some() {
                return Err(syn::Error::new_spanned(
                    field,
                    "duplicate #[bedrock(id)]; a collection has exactly one id",
                ));
            }
            id = Some(position);
        }
        infos.push(FieldInfo {
            ident,
            ty: &field.ty,
            name: field_name(field, ident, struct_rename_all.as_deref())?,
            index: field_attrs.index,
            discriminant: field_attrs.discriminant,
        });
    }

    let id = id.ok_or_else(|| {
        syn::Error::new_spanned(
            &input.ident,
            "Collection requires exactly one field marked #[bedrock(id)]",
        )
    })?;
    let id_ty = infos[id].ty;
    let id_name = &infos[id].name;

    let name = &input.ident;
    let table = input.ident.to_string().to_snake_case();
    let column_count = infos.len();

    let container = attr::container_attrs(&input.attrs)?;

    let mut index_defs: Vec<(Vec<String>, bool)> = Vec::new();
    for info in &infos {
        if let Some(unique) = info.index {
            index_defs.push((vec![info.name.clone()], unique));
        }
    }
    index_defs.extend(container.indexes);
    let index_specs = index_defs.iter().map(|(columns, unique)| {
        let index_name = format!("{}_{}_idx", table, columns.join("_"));
        let column_list = columns
            .iter()
            .map(|c| quote! { ::std::string::String::from(#c) });
        quote! {
            ::bedrock::IndexSpec {
                name: ::std::string::String::from(#index_name),
                columns: ::std::vec![ #(#column_list),* ],
                unique: #unique,
            }
        }
    });

    let field_specs = infos.iter().map(|f| {
        let fname = &f.name;
        let fty = f.ty;
        quote! {
            ::bedrock::FieldSpec {
                name: ::std::string::String::from(#fname),
                kind: <#fty as ::bedrock::Field>::KIND,
                nullable: <#fty as ::bedrock::Field>::NULLABLE,
            }
        }
    });

    let encode_fields = infos.iter().map(|f| {
        let ident = f.ident;
        quote! { ::bedrock::Field::encode(&self.#ident)? }
    });

    let decode_fields = infos.iter().map(|f| {
        let ident = f.ident;
        let fty = f.ty;
        quote! {
            #ident: <#fty as ::bedrock::Field>::decode(
                ::core::iter::Iterator::next(&mut columns).expect("row length checked above"),
            )?
        }
    });

    // View-scope helpers for the `#[bedrock(discriminant)]` field, if any (at
    // most one). The field must be a document column; the scope's path is the
    // field type's serde tag (inferred) or the explicit `= "…"` value.
    let mut discriminant_field: Option<&FieldInfo> = None;
    for info in &infos {
        if info.discriminant.is_some() {
            if discriminant_field.is_some() {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "a collection has at most one #[bedrock(discriminant)] field",
                ));
            }
            discriminant_field = Some(info);
        }
    }
    let discriminant_impl = match discriminant_field {
        Some(info) => {
            let field_name = &info.name;
            let field_ty = info.ty;
            let tag_expr = match info.discriminant.as_ref().expect("checked Some above") {
                attr::DiscriminantAttr::Explicit(path) => {
                    quote! { ::std::string::String::from(#path) }
                }
                attr::DiscriminantAttr::Inferred => quote! {
                    ::std::string::String::from(<#field_ty as ::bedrock::Discriminant>::TAG)
                },
            };
            quote! {
                // The discriminant must be a document column — a view scopes into
                // its JSON, so a scalar/id field is a build error.
                #[allow(clippy::assertions_on_constants)]
                const _: () = ::core::assert!(
                    ::core::matches!(
                        <#field_ty as ::bedrock::Field>::KIND,
                        ::bedrock::FieldKind::Document
                    ),
                    "the #[bedrock(discriminant)] field must be a document column",
                );

                impl #name {
                    /// The document column carrying this collection's discriminant.
                    pub const DISCRIMINANT_DOCUMENT: &'static str = #field_name;

                    /// A view scope selecting rows whose discriminant (the
                    /// document's serde tag) equals `value`.
                    pub fn discriminant_scope(
                        value: impl ::core::convert::Into<::std::string::String>,
                    ) -> ::bedrock::ViewScope {
                        ::bedrock::ViewScope {
                            document: ::std::string::String::from(#field_name),
                            path: #tag_expr,
                            equals: ::core::convert::Into::into(value),
                        }
                    }

                    /// A view scope on an arbitrary `path` within the discriminant
                    /// document — finer granularity than the top-level tag.
                    pub fn scope(
                        path: impl ::core::convert::Into<::std::string::String>,
                        value: impl ::core::convert::Into<::std::string::String>,
                    ) -> ::bedrock::ViewScope {
                        ::bedrock::ViewScope {
                            document: ::std::string::String::from(#field_name),
                            path: ::core::convert::Into::into(path),
                            equals: ::core::convert::Into::into(value),
                        }
                    }
                }
            }
        }
        None => quote! {},
    };

    // The write hook, wired to `#[bedrock(before_save = path)]` when present. The
    // `let f: fn(...) = #path;` binding enforces the hook's signature — a wrong
    // one is a compile error, not a silent mismatch.
    let before_save_impl = match container.before_save {
        Some(path) => quote! {
            fn before_save(self, op: ::bedrock::WriteOp) -> Self {
                let f: fn(Self, ::bedrock::WriteOp) -> Self = #path;
                f(self, op)
            }
        },
        None => quote! {},
    };

    Ok(quote! {
        // The id must be a scalar column, not a document — enforced at compile time
        // so a bad id is a build error rather than a runtime `validate()` failure.
        #[allow(clippy::assertions_on_constants)]
        const _: () = ::core::assert!(
            ::core::matches!(
                <#id_ty as ::bedrock::Field>::KIND,
                ::bedrock::FieldKind::Column(_)
            ),
            "the #[bedrock(id)] field must be a scalar column, not a document",
        );

        impl ::bedrock::Collection for #name {
            type Id = #id_ty;

            fn spec() -> ::bedrock::CollectionSpec {
                ::bedrock::CollectionSpec {
                    name: ::std::string::String::from(#table),
                    id: ::bedrock::IdSpec {
                        field: ::std::string::String::from(#id_name),
                        column_type: match <#id_ty as ::bedrock::Field>::KIND {
                            ::bedrock::FieldKind::Column(ct) => ct,
                            // Unreachable: the const assertion above rejects a document id.
                            ::bedrock::FieldKind::Document => ::core::unreachable!(),
                        },
                    },
                    fields: ::std::vec![ #(#field_specs),* ],
                    indexes: ::std::vec![ #(#index_specs),* ],
                }
            }

            fn to_row(&self) -> ::core::result::Result<
                ::std::vec::Vec<::bedrock::StoredValue>,
                ::bedrock::EncodeError,
            > {
                ::core::result::Result::Ok(::std::vec![ #(#encode_fields),* ])
            }

            fn from_row(
                row: ::std::vec::Vec<::bedrock::StoredValue>,
            ) -> ::core::result::Result<Self, ::bedrock::DecodeError> {
                if row.len() != #column_count {
                    return ::core::result::Result::Err(::bedrock::DecodeError::ColumnCount {
                        collection: #table,
                        expected: #column_count,
                        found: row.len(),
                    });
                }
                let mut columns = ::core::iter::IntoIterator::into_iter(row);
                ::core::result::Result::Ok(Self { #(#decode_fields),* })
            }

            #before_save_impl
        }

        #discriminant_impl
    })
}

fn field_name(field: &Field, ident: &Ident, rename_all: Option<&str>) -> syn::Result<String> {
    if let Some(rename) = attr::serde_rename(&field.attrs)? {
        return Ok(rename);
    }
    let base = ident.to_string();
    Ok(match rename_all {
        Some(rule) => attr::apply_rename_all(&base, rule),
        None => base,
    })
}
