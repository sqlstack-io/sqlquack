//! Derives exhaustive AST traversal: fields in order, containers walked, `#[ast(skip)]` skipped.

#![forbid(unsafe_code)]

mod ast_node;
mod constants;

use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::DeriveInput;

use crate::ast_node::main::expand_ast_node::expand_ast_node;

#[proc_macro_derive(AstNode, attributes(ast))]
/// Derive the expression-child visitors for an AST payload.
pub fn derive_ast_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_ast_node(&input).into()
}
