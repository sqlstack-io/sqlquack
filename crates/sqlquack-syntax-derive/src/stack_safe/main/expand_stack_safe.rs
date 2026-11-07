//! `expand_stack_safe`.

use quote::format_ident;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Fields;

/// Derives `Clone`, `PartialEq` and `Debug` for an enum, each running one node on a grown stack.
pub(crate) fn expand_stack_safe(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let Data::Enum(data) = &input.data else {
        return quote!(compile_error!("StackSafe derives only for enums"););
    };
    let mut clones: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut comparisons: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut formats: Vec<proc_macro2::TokenStream> = Vec::new();
    for variant in &data.variants {
        let ident = &variant.ident;
        let label = ident.to_string();
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

        impl ::core::fmt::Debug for #name {
            fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                crate::expressions::tree::main::_on_grown_stack::on_grown_stack(|| match self {
                    #(#formats,)*
                })
            }
        }
    }
}
