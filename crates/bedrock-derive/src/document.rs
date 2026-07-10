use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

/// A document is stored as a `jsonb` blob, round-tripped through serde. The type
/// is assumed to be `Serialize` + `Deserialize`; if it is not, the generated
/// `serde_json` calls fail to compile — which is the intended feedback.
pub(crate) fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    let name = &input.ident;
    let name_str = name.to_string();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::bedrock::Field for #name #ty_generics #where_clause {
            const KIND: ::bedrock::FieldKind = ::bedrock::FieldKind::Document;

            fn encode(&self) -> ::core::result::Result<::bedrock::StoredValue, ::bedrock::EncodeError> {
                match ::serde_json::to_value(self) {
                    ::core::result::Result::Ok(value) =>
                        ::core::result::Result::Ok(::bedrock::StoredValue::Json(value)),
                    ::core::result::Result::Err(e) =>
                        ::core::result::Result::Err(::bedrock::EncodeError::Serialize {
                            target: #name_str,
                            message: ::std::string::ToString::to_string(&e),
                        }),
                }
            }

            fn decode(value: ::bedrock::StoredValue) -> ::core::result::Result<Self, ::bedrock::DecodeError> {
                match value {
                    ::bedrock::StoredValue::Json(v) => ::serde_json::from_value(v).map_err(|e| {
                        ::bedrock::DecodeError::Document {
                            target: #name_str,
                            message: ::std::string::ToString::to_string(&e),
                        }
                    }),
                    other => ::core::result::Result::Err(::bedrock::DecodeError::TypeMismatch {
                        expected: #name_str,
                        found: other.type_name(),
                    }),
                }
            }
        }
    })
}
