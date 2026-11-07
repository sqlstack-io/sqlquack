//! SQLite 3.53's grammar (`parse.y`), by recursive descent, building sqlquack-syntax trees.

pub(crate) mod _helpers;
pub(crate) mod constants;
pub mod errors;
pub mod main;

#[cfg(test)]
mod tests;
