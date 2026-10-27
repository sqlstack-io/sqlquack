//! Spans: the recorded source range of a node with a `span` field, through the expression enum.

use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Fields;
use syn::Type;

use crate::ast_node::_helpers::field_types::container_inner;
use crate::ast_node::_helpers::field_types::is_expression;
use crate::ast_node::_helpers::field_types::is_scalar;
use crate::constants::EXPRESSION_TYPE;
use crate::constants::SPAN_FIELD;

/// The `recorded_span` and `fill_span` methods, when the node holds or dispatches to a span.
pub(crate) fn span_methods(input: &DeriveInput) -> proc_macro2::TokenStream {
    match &input.data {
        Data::Struct(data) if has_span_field(&data.fields) => quote! {
            fn recorded_span(&self) -> Option<crate::tokens::models::Span> {
                crate::tokens::types::SpanField::recorded(&self.span)
            }

            fn fill_span(&mut self, span: crate::tokens::models::Span) {
                crate::tokens::types::SpanField::fill(&mut self.span, span);
            }
        },
        Data::Enum(data) if input.ident == EXPRESSION_TYPE => {
            let name = &input.ident;
            let mut payloads: Vec<(&syn::Ident, bool)> = Vec::new();
            for variant in &data.variants {
                if let Some(boxed) = payload(&variant.fields) {
                    payloads.push((&variant.ident, boxed));
                }
            }
            let recorded = payloads.iter().map(|(variant, boxed)| {
                let access = match boxed {
                    true => quote!(payload.as_ref()),
                    false => quote!(payload),
                };
                quote!(#name::#variant(payload) => crate::traversal::types::AstNode::recorded_span(#access))
            });
            let filled = payloads.iter().map(|(variant, boxed)| {
                let access = match boxed {
                    true => quote!(payload.as_mut()),
                    false => quote!(payload),
                };
                quote!(#name::#variant(payload) => crate::traversal::types::AstNode::fill_span(#access, span))
            });
            quote! {
                fn recorded_span(&self) -> Option<crate::tokens::models::Span> {
                    match self {
                        #(#recorded,)*
                        _ => None,
                    }
                }

                fn fill_span(&mut self, span: crate::tokens::models::Span) {
                    match self {
                        #(#filled,)*
                        _ => {}
                    }
                }
            }
        }
        _ => quote!(),
    }
}

fn has_span_field(fields: &Fields) -> bool {
    match fields {
        Fields::Named(fields) => fields.named.iter().any(|field| {
            field
                .ident
                .as_ref()
                .is_some_and(|ident| ident == SPAN_FIELD)
        }),
        _ => false,
    }
}

/// Whether a variant holds one node payload, and whether it is boxed.
fn payload(fields: &Fields) -> Option<bool> {
    let Fields::Unnamed(fields) = fields else {
        return None;
    };
    let [field] = fields.unnamed.iter().collect::<Vec<_>>()[..] else {
        return None;
    };
    match container_inner(&field.ty, "Box") {
        Some(inner) => is_node(inner).then_some(true),
        None => is_node(&field.ty).then_some(false),
    }
}

fn is_node(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    let container = path.path.segments.last().is_some_and(|segment| {
        matches!(segment.ident.to_string().as_str(), "Vec" | "Option" | "Box")
    });
    !container && !is_expression(ty) && !is_scalar(ty)
}
