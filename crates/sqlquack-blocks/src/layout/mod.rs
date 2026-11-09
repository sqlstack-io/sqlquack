//! A simple layout of DuckDB SQL over lines, changing only the whitespace between tokens.

pub(crate) mod _helpers;
pub mod constants;
pub mod main;
pub(crate) mod models;

#[cfg(test)]
mod tests;
