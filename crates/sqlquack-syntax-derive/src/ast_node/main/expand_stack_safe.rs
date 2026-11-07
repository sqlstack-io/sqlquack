//! `expand_stack_safe`.

use quote::format_ident;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Fields;

use crate::ast_node::_helpers::naming::serde_snake_case;
use crate::constants::SNAKE_CASE;

/// Derives `Clone`, `PartialEq`, `Debug` and `Serialize` for an enum, each running one node on a grown stack.
pub(crate) fn expand_stack_safe(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let Data::Enum(data) = &input.data else {
        return quote!(compile_error!("StackSafe derives only for enums"););
    };
    if !has_only_snake_case_serde_attributes(input) {
        return quote!(compile_error!(
            "StackSafe serializes like serde's `rename_all = \"snake_case\"` with no other serde attribute"
        ););
    }
    let type_label = name.to_string();
    let mut serializations: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut clones: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut comparisons: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut formats: Vec<proc_macro2::TokenStream> = Vec::new();
    for (variant_index, variant) in (0_u32..).zip(&data.variants) {
        let ident = &variant.ident;
        let label = ident.to_string();
        let serialized_label = serde_snake_case(&label);
        serializations.push(match &variant.fields {
            Fields::Unit => quote! {
                Self::#ident => serializer.serialize_unit_variant(#type_label, #variant_index, #serialized_label)
            },
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => quote! {
                Self::#ident(own_0) => serializer.serialize_newtype_variant(#type_label, #variant_index, #serialized_label, own_0)
            },
            _ => return quote!(compile_error!("StackSafe serializes only unit and one-field variants");),
        });
        let (pattern, other_pattern, bindings, others, rebuild, format) = match &variant.fields {
            Fields::Unit => (
                quote!(Self::#ident),
                quote!(Self::#ident),
                Vec::new(),
                Vec::new(),
                quote!(Self::#ident),
                quote!(formatter.write_str(#label)),
            ),
            Fields::Unnamed(fields) => {
                let bindings: Vec<_> = (0..fields.unnamed.len())
                    .map(|index| format_ident!("own_{index}"))
                    .collect();
                let others: Vec<_> = (0..fields.unnamed.len())
                    .map(|index| format_ident!("other_{index}"))
                    .collect();
                (
                    quote!(Self::#ident(#(#bindings),*)),
                    quote!(Self::#ident(#(#others),*)),
                    bindings.clone(),
                    others,
                    quote!(Self::#ident(#(::core::clone::Clone::clone(#bindings)),*)),
                    quote! {{
                        let mut tuple = formatter.debug_tuple(#label);
                        #(tuple.field(#bindings);)*
                        tuple.finish()
                    }},
                )
            }
            Fields::Named(fields) => {
                let names: Vec<_> = fields
                    .named
                    .iter()
                    .filter_map(|field| field.ident.clone())
                    .collect();
                let labels: Vec<_> = names.iter().map(ToString::to_string).collect();
                let bindings: Vec<_> = names
                    .iter()
                    .map(|field| format_ident!("own_{field}"))
                    .collect();
                let others: Vec<_> = names
                    .iter()
                    .map(|field| format_ident!("other_{field}"))
                    .collect();
                (
                    quote!(Self::#ident { #(#names: #bindings),* }),
                    quote!(Self::#ident { #(#names: #others),* }),
                    bindings.clone(),
                    others,
                    quote!(Self::#ident { #(#names: ::core::clone::Clone::clone(#bindings)),* }),
                    quote! {{
                        let mut fields = formatter.debug_struct(#label);
                        #(fields.field(#labels, #bindings);)*
                        fields.finish()
                    }},
                )
            }
        };
        clones.push(quote!(#pattern => #rebuild));
        comparisons.push(quote!((#pattern, #other_pattern) => true #(&& #bindings == #others)*));
        formats.push(quote!(#pattern => #format));
    }

    quote! {
        impl ::core::clone::Clone for #name {
            fn clone(&self) -> Self {
                crate::expressions::tree::main::_on_grown_stack::on_grown_stack(|| match self {
                    #(#clones,)*
                })
            }
        }

        impl ::core::cmp::PartialEq for #name {
            #[allow(unreachable_patterns)]
            fn eq(&self, other: &Self) -> bool {
                crate::expressions::tree::main::_on_grown_stack::on_grown_stack(|| match (self, other) {
                    #(#comparisons,)*
                    _ => false,
                })
            }
        }

        impl ::serde::Serialize for #name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error> {
                crate::expressions::tree::main::_on_grown_stack::on_grown_stack(|| match self {
                    #(#serializations,)*
                })
            }
        }

        impl ::core::fmt::Debug for #name {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                crate::expressions::tree::main::_on_grown_stack::on_grown_stack(|| match self {
                    #(#formats,)*
                })
            }
        }
    }
}

/// Whether the only serde attribute on the enum is `rename_all = "snake_case"`, and none on its variants.
fn has_only_snake_case_serde_attributes(input: &DeriveInput) -> bool {
    let Data::Enum(data) = &input.data else {
        return false;
    };
    let variant_attributes = data.variants.iter().flat_map(|variant| &variant.attrs);
    let field_attributes = data
        .variants
        .iter()
        .flat_map(|variant| &variant.fields)
        .flat_map(|field| &field.attrs);
    if variant_attributes
        .chain(field_attributes)
        .any(|attribute| attribute.path().is_ident("serde"))
    {
        return false;
    }
    let mut snake_case = false;
    for attribute in input
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("serde"))
    {
        let parsed = attribute.parse_nested_meta(|meta| {
            let value: syn::LitStr = meta.value()?.parse()?;
            snake_case = meta.path.is_ident("rename_all") && value.value() == SNAKE_CASE;
            Ok(())
        });
        if parsed.is_err() || !snake_case {
            return false;
        }
    }
    snake_case
}
