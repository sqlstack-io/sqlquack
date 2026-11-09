//! The text of raises and how their IDs are hashed.

/// The text opening every raise, before its message.
pub const RAISE_OPENING: &str = "CAST(CAST('";
/// The site argument a raising helper call takes first, until its expression identifies it.
pub const UNIDENTIFIED_SITE: &str = "'#?'";
/// FNV-1a's 64-bit offset basis, which raise IDs start hashing from.
pub(crate) const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a's 64-bit prime.
pub(crate) const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
