use crate::attr;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Fields, Type, Variant};

pub(crate) fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Unnamed(fields),
            ..
        }) if fields.unnamed.len() == 1 => expand_newtype(&input, &fields.unnamed[0].ty),
        Data::Enum(data) => expand_fieldless_enum(&input, data),
        _ => Err(syn::Error::new_spanned(
            &input.ident,
            "Scalar can only be derived for a newtype struct (one unnamed field) or a fieldless enum",
        )),
    }
}

/// A newtype forwards every part of `Field` to its inner type.
fn expand_newtype(input: &DeriveInput, inner: &Type) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::bedrock::Field for #name #ty_generics #where_clause {
            const KIND: ::bedrock::FieldKind = <#inner as ::bedrock::Field>::KIND;
            const NULLABLE: bool = <#inner as ::bedrock::Field>::NULLABLE;

            fn encode(&self) -> ::core::result::Result<::bedrock::StoredValue, ::bedrock::EncodeError> {
                <#inner as ::bedrock::Field>::encode(&self.0)
            }

            fn decode(value: ::bedrock::StoredValue) -> ::core::result::Result<Self, ::bedrock::DecodeError> {
                <#inner as ::bedrock::Field>::decode(value).map(Self)
            }
        }
    })
}

/// A fieldless enum maps to a `TEXT` column, one string per variant.
fn expand_fieldless_enum(input: &DeriveInput, data: &DataEnum) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let name_str = name.to_string();
    let rename_all = attr::serde_rename_all(&input.attrs)?;

    let mut encode_arms = Vec::new();
    let mut decode_arms = Vec::new();
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                variant,
                "Scalar enums must have only fieldless (unit) variants; use Document for data-carrying enums",
            ));
        }
        let ident = &variant.ident;
        let serialized = variant_name(variant, rename_all.as_deref())?;
        encode_arms.push(quote! { Self::#ident => #serialized });
        decode_arms.push(quote! { #serialized => ::core::result::Result::Ok(Self::#ident) });
    }

    Ok(quote! {
        impl ::bedrock::Field for #name {
            const KIND: ::bedrock::FieldKind =
                ::bedrock::FieldKind::Column(::bedrock::ColumnType::Text);

            fn encode(&self) -> ::core::result::Result<::bedrock::StoredValue, ::bedrock::EncodeError> {
                let name: &'static str = match self { #(#encode_arms),* };
                ::core::result::Result::Ok(::bedrock::StoredValue::Text(::std::string::String::from(name)))
            }

            fn decode(value: ::bedrock::StoredValue) -> ::core::result::Result<Self, ::bedrock::DecodeError> {
                match value {
                    ::bedrock::StoredValue::Text(s) => match s.as_str() {
                        #(#decode_arms,)*
                        _ => ::core::result::Result::Err(::bedrock::DecodeError::Parse {
                            target: #name_str,
                            input: s,
                            message: ::std::string::String::from("unknown variant"),
                        }),
                    },
                    other => ::core::result::Result::Err(::bedrock::DecodeError::TypeMismatch {
                        expected: #name_str,
                        found: other.type_name(),
                    }),
                }
            }
        }
    })
}

fn variant_name(variant: &Variant, rename_all: Option<&str>) -> syn::Result<String> {
    if let Some(rename) = attr::serde_rename(&variant.attrs)? {
        return Ok(rename);
    }
    let base = variant.ident.to_string();
    Ok(match rename_all {
        Some(rule) => attr::apply_rename_all(&base, rule),
        None => base,
    })
}
