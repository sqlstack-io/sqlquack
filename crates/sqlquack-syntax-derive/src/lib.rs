//! Derives exhaustive AST traversal, and stack-safe `Clone`, `PartialEq` and `Debug` for recursive enums.

#![forbid(unsafe_code)]

mod ast_node;
mod constants;
mod stack_safe;

use proc_macro::TokenStream;
use syn::parse_macro_input;
use syn::DeriveInput;

use crate::ast_node::main::expand_ast_node::expand_ast_node;
use crate::stack_safe::main::expand_stack_safe::expand_stack_safe;

#[proc_macro_derive(AstNode, attributes(ast))]
/// Derive the expression-child visitors for an AST payload.
pub fn derive_ast_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_ast_node(&input).into()
}

#[proc_macro_derive(StackSafe)]
/// Derive `Clone`, `PartialEq` and `Debug` for a recursive enum, each on a grown stack when it runs low.
pub fn derive_stack_safe(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_stack_safe(&input).into()
}
