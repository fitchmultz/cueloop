//! Shared enum-token parsing for contract types.
//!
//! Purpose:
//! - Shared enum-token parsing for contract types.
//!
//! Responsibilities:
//! - Provide the `snake_case_from_str!` macro for simple snake_case enums.
//! - Own the `normalize_enum_token` helper shared by macro users and hand-written `FromStr` impls.
//!
//! Not handled here:
//! - Enums with catch-all variants (`Runner`, `Model`) — those have hand-written `FromStr`.
//! - Contract type definitions themselves.
//!
//! Usage:
//! - Internal to the contracts module; invoked by runner and config submodules.

/// Normalize a CLI/config token to lowercase snake_case.
///
/// Trims whitespace, lowercases, and replaces hyphens with underscores.
pub(crate) fn normalize_enum_token(value: &str) -> String {
    value.trim().to_lowercase().replace('-', "_")
}

/// Generate a `FromStr` impl for a simple snake_case enum.
///
/// Normalizes input via `normalize_enum_token`, then matches the listed
/// variant-token pairs. Returns a static error string on mismatch.
///
/// Use this only for enums without fallback/catch-all variants.
/// `Runner` and `Model` have their own hand-written `FromStr` for that reason.
macro_rules! snake_case_from_str {
    ($ty:ty { $($variant:ident => $token:literal),+ $(,)? } $err_msg:literal) => {
        impl std::str::FromStr for $ty {
            type Err = &'static str;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match $crate::contracts::enum_parse::normalize_enum_token(value).as_str() {
                    $($token => Ok(<$ty>::$variant),)+
                    _ => Err($err_msg),
                }
            }
        }
    };
}

pub(crate) use snake_case_from_str;
