//! Visitors.

use quote::format_ident;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Fields;
use syn::Type;

use crate::ast_node::_helpers::field_types::container_inner;
use crate::ast_node::_helpers::field_types::is_expression;
use crate::ast_node::_helpers::field_types::is_scalar;
use crate::ast_node::_helpers::field_types::is_skipped;
use crate::constants::EXPRESSION_TYPE;

pub(crate) fn node_visitor(input: &DeriveInput, paths: bool) -> proc_macro2::TokenStream {
    let name = &input.ident;
    match &input.data {
        Data::Struct(data) => visit_fields(&data.fields, paths),
        Data::Enum(data) => {
            let arms = data.variants.iter().map(|variant| {
                let (pattern, body) =
                    visit_variant_fields(name, &variant.ident, &variant.fields, paths);
                quote!(#pattern => { #body })
            });
            quote!(match self { #(#arms),* })
        }
        Data::Union(_) => quote!(),
    }
}

fn visit_fields(fields: &Fields, paths: bool) -> proc_macro2::TokenStream {
    match fields {
        Fields::Named(fields) => {
            let visits = fields.named.iter().filter_map(|field| {
                if is_skipped(field) {
                    return None;
                }
                let ident = field.ident.as_ref()?;
                let field_name = ident.to_string();
                let immutable_access = quote!(&self.#ident);
                let visit = immutable_visit(&field.ty, immutable_access, paths);
                Some(if paths {
                    quote! {
                        path.push(crate::traversal::models::ChildPathSegment::Field(#field_name));
                        #visit
                        path.pop();
                    }
                } else {
                    visit
                })
            });
            quote!(#(#visits)*)
        }
        Fields::Unnamed(fields) => {
            let visits = fields
                .unnamed
                .iter()
                .enumerate()
                .filter_map(|(index, field)| {
                    if is_skipped(field) {
                        return None;
                    }
                    let index = syn::Index::from(index);
                    let immutable_access = quote!(&self.#index);
                    let visit = immutable_visit(&field.ty, immutable_access, paths);
                    Some(if paths {
                        quote! {
                            path.push(crate::traversal::models::ChildPathSegment::Index(#index));
                            #visit
                            path.pop();
                        }
                    } else {
                        visit
                    })
                });
            quote!(#(#visits)*)
        }
        Fields::Unit => quote!(),
    }
}

fn visit_variant_fields(
    enum_name: &syn::Ident,
    variant_name: &syn::Ident,
    fields: &Fields,
    paths: bool,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    match fields {
        Fields::Named(fields) => {
            let bindings: Vec<_> = fields
                .named
                .iter()
                .filter_map(|field| {
                    let ident = field.ident.as_ref()?;
                    Some(if is_skipped(field) || is_scalar(&field.ty) {
                        quote!(#ident: _)
                    } else {
                        quote!(#ident)
                    })
                })
                .collect();
            let visits = fields.named.iter().filter_map(|field| {
                if is_skipped(field) {
                    return None;
                }
                let ident = field.ident.as_ref()?;
                let field_name = ident.to_string();
                let visit = immutable_visit(&field.ty, quote!(#ident), paths);
                Some(if paths {
                    quote! {
                        path.push(crate::traversal::models::ChildPathSegment::Field(#field_name));
                        #visit
                        path.pop();
                    }
                } else {
                    visit
                })
            });
            (
                quote!(#enum_name::#variant_name { #(#bindings),* }),
                quote!(#(#visits)*),
            )
        }
        Fields::Unnamed(fields) => {
            let bindings: Vec<_> = (0..fields.unnamed.len())
                .map(|index| format_ident!("field_{index}"))
                .collect();
            let single_expression_payload =
                enum_name == EXPRESSION_TYPE && fields.unnamed.len() == 1;
            let visits = fields
                .unnamed
                .iter()
                .zip(bindings.iter())
                .enumerate()
                .filter_map(|(index, (field, binding))| {
                    if is_skipped(field) {
                        return None;
                    }
                    let visit = immutable_visit(&field.ty, quote!(#binding), paths);
                    Some(if single_expression_payload || !paths {
                        visit
                    } else {
                        quote! {
                            path.push(crate::traversal::models::ChildPathSegment::Index(#index));
                            #visit
                            path.pop();
                        }
                    })
                });
            (
                quote!(#enum_name::#variant_name(#(#bindings),*)),
                quote!(#(#visits)*),
            )
        }
        Fields::Unit => (quote!(#enum_name::#variant_name), quote!()),
    }
}

fn immutable_visit(
    ty: &Type,
    access: proc_macro2::TokenStream,
    paths: bool,
) -> proc_macro2::TokenStream {
    if is_expression(ty) {
        return if paths {
            quote!(visitor(path, #access);)
        } else {
            quote!(visitor(#access);)
        };
    }
    if let Some(inner) = container_inner(ty, "Option") {
        let visit = immutable_visit(inner, quote!(value), paths);
        return quote!(if let Some(value) = (#access).as_ref() { #visit });
    }
    if let Some(inner) = container_inner(ty, "Box") {
        return immutable_visit(inner, quote!((#access).as_ref()), paths);
    }
    if let Some(inner) = container_inner(ty, "Vec") {
        let visit = immutable_visit(inner, quote!(value), paths);
        if !paths {
            return quote!(for value in (#access).iter() { #visit });
        }
        return quote! {
            for (index, value) in (#access).iter().enumerate() {
                path.push(crate::traversal::models::ChildPathSegment::Index(index));
                #visit
                path.pop();
            }
        };
    }
    if let Type::Tuple(tuple) = ty {
        let visits = tuple.elems.iter().enumerate().map(|(index, element)| {
            let tuple_index = syn::Index::from(index);
            let visit = immutable_visit(element, quote!(&(#access).#tuple_index), paths);
            if paths {
                quote! {
                    path.push(crate::traversal::models::ChildPathSegment::Index(#index));
                    #visit
                    path.pop();
                }
            } else {
                visit
            }
        });
        return quote!(#(#visits)*);
    }
    if is_scalar(ty) {
        return quote!();
    }
    if paths {
        quote!(crate::traversal::types::AstNode::visit_expressions(#access, path, visitor);)
    } else {
        let visit_type = if matches!(ty, Type::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == "DataType"))
        {
            quote!(type_visitor(#access);)
        } else {
            quote!()
        };
        quote! {
            #visit_type
            crate::traversal::types::AstNode::visit_syntax_untracked(#access, visitor, type_visitor);
        }
    }
}
