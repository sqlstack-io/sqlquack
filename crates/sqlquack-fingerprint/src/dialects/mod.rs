//! Dialect profiles: how each dialect's SQL is split into tokens and which words it reserves, and the one registry.

#[cfg(feature = "generic-keywords")]
pub(crate) mod _helpers;
pub(crate) mod main;
pub(crate) mod models;
pub(crate) mod types;
