use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataStruct, DeriveInput, Field, Fields, Ident, Type};

use crate::attr;

struct FieldInfo<'a> {
    ident: &'a Ident,
    ty: &'a Type,
    name: String,
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
    for (index, field) in fields.iter().enumerate() {
        let ident = field.ident.as_ref().expect("named field has an ident");
        if attr::has_bedrock_id(&field.attrs)? {
            if id.is_some() {
                return Err(syn::Error::new_spanned(
                    field,
                    "duplicate #[bedrock(id)]; a collection has exactly one id",
                ));
            }
            id = Some(index);
        }
        infos.push(FieldInfo {
            ident,
            ty: &field.ty,
            name: field_name(field, ident, struct_rename_all.as_deref())?,
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

    let field_specs = infos.iter().map(|f| {
        let fname = &f.name;
        let fty = f.ty;
        quote! {
            ::bedrock_core::FieldSpec {
                name: ::std::string::String::from(#fname),
                kind: <#fty as ::bedrock_core::Field>::KIND,
                nullable: <#fty as ::bedrock_core::Field>::NULLABLE,
            }
        }
    });

    let encode_fields = infos.iter().map(|f| {
        let ident = f.ident;
        quote! { ::bedrock_core::Field::encode(&self.#ident)? }
    });

    let decode_fields = infos.iter().map(|f| {
        let ident = f.ident;
        let fty = f.ty;
        quote! {
            #ident: <#fty as ::bedrock_core::Field>::decode(
                ::core::iter::Iterator::next(&mut columns).expect("row length checked above"),
            )?
        }
    });

    Ok(quote! {
        // The id must be a scalar column, not a document — enforced at compile time
        // so a bad id is a build error rather than a runtime `validate()` failure.
        #[allow(clippy::assertions_on_constants)]
        const _: () = ::core::assert!(
            ::core::matches!(
                <#id_ty as ::bedrock_core::Field>::KIND,
                ::bedrock_core::FieldKind::Column(_)
            ),
            "the #[bedrock(id)] field must be a scalar column, not a document",
        );

        impl ::bedrock_core::Collection for #name {
            fn spec() -> ::bedrock_core::CollectionSpec {
                ::bedrock_core::CollectionSpec {
                    name: ::std::string::String::from(#table),
                    id: ::bedrock_core::IdSpec {
                        field: ::std::string::String::from(#id_name),
                        column_type: match <#id_ty as ::bedrock_core::Field>::KIND {
                            ::bedrock_core::FieldKind::Column(ct) => ct,
                            // Unreachable: the const assertion above rejects a document id.
                            ::bedrock_core::FieldKind::Document => ::core::unreachable!(),
                        },
                    },
                    fields: ::std::vec![ #(#field_specs),* ],
                }
            }

            fn to_row(&self) -> ::core::result::Result<
                ::std::vec::Vec<::bedrock_core::StoredValue>,
                ::bedrock_core::EncodeError,
            > {
                ::core::result::Result::Ok(::std::vec![ #(#encode_fields),* ])
            }

            fn from_row(
                row: ::std::vec::Vec<::bedrock_core::StoredValue>,
            ) -> ::core::result::Result<Self, ::bedrock_core::DecodeError> {
                if row.len() != #column_count {
                    return ::core::result::Result::Err(::bedrock_core::DecodeError::ColumnCount {
                        collection: #table,
                        expected: #column_count,
                        found: row.len(),
                    });
                }
                let mut columns = ::core::iter::IntoIterator::into_iter(row);
                ::core::result::Result::Ok(Self { #(#decode_fields),* })
            }
        }
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
