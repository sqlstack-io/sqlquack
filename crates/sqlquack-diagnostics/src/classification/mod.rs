//! Classification of every function, operator, type, cast, join and clause in a statement.

pub(crate) mod _helpers;
pub mod constants;
pub mod main;
pub mod models;

#[cfg(test)]
mod tests;
