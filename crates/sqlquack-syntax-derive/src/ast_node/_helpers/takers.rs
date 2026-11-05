//! Takers: move a node's direct expression children out so a tree can be freed without recursion.

use quote::format_ident;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Field;
use syn::Fields;
use syn::Type;
use syn::Variant;

use crate::ast_node::_helpers::field_types::container_inner;
use crate::ast_node::_helpers::field_types::is_expression;
use crate::ast_node::_helpers::field_types::is_scalar;
use crate::ast_node::_helpers::field_types::is_skipped;

/// The body of `take_expressions`, which moves every direct expression child into `out`.
pub(crate) fn node_taker(input: &DeriveInput) -> proc_macro2::TokenStream {
    match &input.data {
        Data::Struct(data) => struct_takes(&data.fields),
        Data::Enum(data) => {
            let mut arms: Vec<proc_macro2::TokenStream> = Vec::new();
            for variant in &data.variants {
                arms.push(variant_arm(&input.ident, variant));
            }
            quote!(match self { #(#arms),* })
        }
        Data::Union(_) => quote!(),
    }
}

/// Takes from every traversed field of a struct.
fn struct_takes(fields: &Fields) -> proc_macro2::TokenStream {
    let mut takes: Vec<proc_macro2::TokenStream> = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        if is_skipped(field) {
            continue;
        }
        let access = match &field.ident {
            Some(ident) => quote!(&mut self.#ident),
            None => {
                let index = syn::Index::from(index);
                quote!(&mut self.#index)
            }
        };
        takes.push(take(&field.ty, access));
    }
    quote!(#(#takes)*)
}

/// Whether a field can hold expressions and is traversed.
fn is_kept(field: &Field) -> bool {
    !is_skipped(field) && !is_scalar(&field.ty)
}

/// One match arm that binds an enum variant's kept fields and takes from them.
fn variant_arm(enum_name: &syn::Ident, variant: &Variant) -> proc_macro2::TokenStream {
    let variant_name = &variant.ident;
    let mut bindings: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut takes: Vec<proc_macro2::TokenStream> = Vec::new();
    for (index, field) in variant.fields.iter().enumerate() {
        let binding = match &field.ident {
            Some(ident) => ident.clone(),
            None => format_ident!("field_{index}"),
        };
        let kept = is_kept(field);
        bindings.push(match (&field.ident, kept) {
            (Some(ident), true) => quote!(#ident),
            (Some(ident), false) => quote!(#ident: _),
            (None, true) => quote!(#binding),
            (None, false) => quote!(_),
        });
        if kept {
            takes.push(take(&field.ty, quote!(#binding)));
        }
    }
    match &variant.fields {
        Fields::Named(_) => quote!(#enum_name::#variant_name { #(#bindings),* } => { #(#takes)* }),
        Fields::Unnamed(_) => quote!(#enum_name::#variant_name(#(#bindings),*) => { #(#takes)* }),
        Fields::Unit => quote!(#enum_name::#variant_name => {}),
    }
}

/// Moves the expressions in a field reached through the `&mut` place `access` into `out`.
fn take(ty: &Type, access: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    if is_expression(ty) {
        return quote! {
            {
                let slot: &mut crate::expressions::tree::models::Expression = #access;
                if !matches!(slot, crate::expressions::tree::models::Expression::Null(_)) {
                    out.push(::core::mem::replace(
                        slot,
                        crate::expressions::tree::models::Expression::Null(
                            crate::expressions::names::models::Null,
                        ),
                    ));
                }
            }
        };
    }
    if let Some(inner) = container_inner(ty, "Option") {
        let take = take(inner, quote!(value));
        return quote!(if let Some(value) = (#access).as_mut() { #take });
    }
    if let Some(inner) = container_inner(ty, "Box") {
        return take(inner, quote!(&mut **(#access)));
    }
    if let Some(inner) = container_inner(ty, "Vec") {
        let take = take(inner, quote!(value));
        return quote!(for value in (#access).iter_mut() { #take });
    }
    if let Type::Tuple(tuple) = ty {
        let mut takes: Vec<proc_macro2::TokenStream> = Vec::new();
        for (index, element) in tuple.elems.iter().enumerate() {
            let index = syn::Index::from(index);
            takes.push(take(element, quote!(&mut (#access).#index)));
        }
        return quote!(#(#takes)*);
    }
    if is_scalar(ty) {
        return quote!();
    }
    quote!(crate::traversal::types::AstNode::take_expressions(#access, out);)
}
