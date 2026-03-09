//! Queue import unit tests grouped by concern.
//!
//! Responsibilities:
//! - Share the extracted queue import unit suite across parser, normalization, and merge helpers.
//! - Keep the production import facade free of large inline scenario blocks.

use super::*;

mod merge_tests;
mod normalize_tests;
mod parse_tests;
