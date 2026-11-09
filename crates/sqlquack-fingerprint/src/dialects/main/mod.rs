//! Dialect entries.

#[cfg(feature = "generic-keywords")]
pub(crate) mod is_generic_reserved;
#[cfg(feature = "keyword-lists")]
pub(crate) mod keyword_set;
pub(crate) mod profile;
#[cfg(feature = "shared-tokenizer")]
pub(crate) mod shared_texts;
