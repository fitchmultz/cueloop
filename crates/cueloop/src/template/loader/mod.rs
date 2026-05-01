//! Purpose: Provide the public template-loading API surface for built-in and
//! custom task templates.
//!
//! Responsibilities:
//! - Declare the `template::loader` child modules.
//! - Re-export the stable public API used by CLI surfaces and callers.
//!
//! Scope:
//! - Thin facade only; implementation lives in sibling files under
//!   `template/loader/`.
//!
//! Usage:
//! - Import public template-loading helpers through `crate::template` or
//!   `crate::template::loader`.
//!
//! Invariants/Assumptions:
//! - The public API surface remains stable across the split.
//! - Custom-template precedence, validation, and substitution behavior remain
//!   unchanged.

mod list;
mod load;
mod types;

#[cfg(test)]
mod tests;

pub use list::{list_templates, template_exists};
pub use load::{
    get_template_context, load_template, load_template_with_context,
    load_template_with_context_legacy,
};
pub use types::{LoadedTemplate, TemplateError, TemplateInfo, TemplateSource};
