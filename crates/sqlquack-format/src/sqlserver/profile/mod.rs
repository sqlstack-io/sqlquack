//! SQL Server's Transact-SQL's format profile: its tokenizer, parser and layout rules.

pub(crate) mod _helpers;
pub(crate) mod constants;
pub(crate) mod main;

#[cfg(test)]
mod tests;
