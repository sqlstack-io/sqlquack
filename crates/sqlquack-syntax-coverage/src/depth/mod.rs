//! The deep-nesting probe: native parsers must refuse deep input, never crash or hang on it.

pub(crate) mod _helpers;
pub mod constants;
pub mod main;
pub mod models;

#[cfg(test)]
mod tests;
