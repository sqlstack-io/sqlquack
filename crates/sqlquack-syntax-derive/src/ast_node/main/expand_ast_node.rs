//! `expand_ast_node`.

use quote::quote;
use syn::Data;
use syn::DeriveInput;

use crate::ast_node::_helpers::naming::serde_snake_case;
use crate::ast_node::_helpers::visitors::node_visitor;
use crate::constants::EXPRESSION_TYPE;

pub(crate) fn expand_ast_node(input: &DeriveInput) -> proc_macro2::TokenStream {
    let name = &input.ident;
    let immutable = node_visitor(input, true);
    let untracked = node_visitor(input, false);
    let serialized_variant_names = if name == EXPRESSION_TYPE {
        if let Data::Enum(data) = &input.data {
            let names = data.variants.iter().map(|variant| {
                let name = serde_snake_case(&variant.ident.to_string());
                syn::LitStr::new(&name, variant.ident.span())
            });
            quote! {
                impl #name {
                    pub(crate) const SERIALIZED_VARIANT_NAMES: &'static [&'static str] = &[
                        #(#names),*
                    ];
                }
            }
        } else {
            quote!()
        }
    } else {
        quote!()
    };

    quote! {
        impl crate::traversal::types::AstNode for #name {
            fn visit_expressions<'ast, F>(
                &'ast self,
                path: &mut Vec<crate::traversal::models::ChildPathSegment>,
                visitor: &mut F,
            )
            where
                F: FnMut(
                    &[crate::traversal::models::ChildPathSegment],
                    &'ast crate::expressions::tree::models::Expression,
                ),
            {
                #immutable
            }

            fn visit_syntax_untracked<'ast, F, T>(&'ast self, visitor: &mut F, type_visitor: &mut T)
            where
                F: FnMut(&'ast crate::expressions::tree::models::Expression),
                T: FnMut(&'ast crate::expressions::types::models::DataType),
            {
                #untracked
            }
        }

        #serialized_variant_names
    }
}
