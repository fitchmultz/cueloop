//! Purpose: Detect sensitive environment keys and provide cached sensitive
//! values for redaction.
//!
//! Responsibilities:
//! - Identify environment keys that should be treated as secret-bearing.
//! - Filter out path-like keys that must not be redacted by value.
//! - Materialize cached sensitive environment variable values for text
//!   redaction.
//!
//! Scope:
//! - Environment-derived redaction metadata only; string-pattern redaction and
//!   logging wrappers live in sibling modules.
//!
//! Usage:
//! - Called by `patterns.rs` during text redaction.
//!
//! Invariants/Assumptions:
//! - Production builds cache sensitive env values lazily behind a thread-safe
//!   `RwLock`.
//! - Test builds recompute values each call so env-mutation tests stay
//!   deterministic.

use std::collections::HashSet;
#[cfg(not(test))]
use std::sync::RwLock;

use crate::constants::limits::MIN_ENV_VALUE_LEN;

#[cfg(not(test))]
static SENSITIVE_ENV_CACHE: RwLock<Option<HashSet<String>>> = RwLock::new(None);

fn init_sensitive_env_cache() -> HashSet<String> {
    let mut sensitive_values = HashSet::new();
    for (key, value) in std::env::vars() {
        if !looks_sensitive_env_key(&key) {
            continue;
        }
        if is_path_like_env_key(&key) {
            continue;
        }
        let trimmed = value.trim();
        if trimmed.len() < MIN_ENV_VALUE_LEN {
            continue;
        }
        sensitive_values.insert(trimmed.to_string());
    }
    sensitive_values
}

#[cfg(test)]
pub(super) fn get_sensitive_env_values() -> HashSet<String> {
    init_sensitive_env_cache()
}

#[cfg(not(test))]
pub(super) fn get_sensitive_env_values() -> HashSet<String> {
    if let Ok(guard) = SENSITIVE_ENV_CACHE.read()
        && let Some(ref values) = *guard
    {
        return values.clone();
    }

    if let Ok(mut guard) = SENSITIVE_ENV_CACHE.write() {
        if guard.is_none() {
            *guard = Some(init_sensitive_env_cache());
        }
        guard.as_ref().cloned().unwrap_or_default()
    } else {
        init_sensitive_env_cache()
    }
}

pub fn looks_sensitive_env_key(key: &str) -> bool {
    let normalized = normalize_key(key);
    if normalized == "APIKEY" || normalized == "PRIVATEKEY" {
        return true;
    }
    for token in normalized.split(['_', '-']) {
        if token.is_empty() {
            continue;
        }
        if is_sensitive_token(token) {
            return true;
        }
    }
    false
}

pub fn is_path_like_env_key(key: &str) -> bool {
    matches!(
        normalize_key(key).as_str(),
        "CWD" | "HOME" | "OLDPWD" | "PATH" | "PWD" | "TEMP" | "TMP" | "TMPDIR"
    )
}

pub(super) fn looks_sensitive_label(key: &str) -> bool {
    let normalized = normalize_key(key);
    if normalized == "APIKEY" || normalized == "PRIVATEKEY" {
        return true;
    }
    if normalized == "API_KEY" || normalized == "API-KEY" {
        return true;
    }
    if normalized == "PRIVATE_KEY" || normalized == "PRIVATE-KEY" {
        return true;
    }
    looks_sensitive_env_key(&normalized)
}

fn is_sensitive_token(token: &str) -> bool {
    let token_upper = token.to_ascii_uppercase();
    for base in ["KEY", "SECRET", "TOKEN", "PASSWORD"] {
        if token_upper == base {
            return true;
        }
        if let Some(suffix) = token_upper.strip_prefix(base)
            && !suffix.is_empty()
            && suffix.chars().all(|c| c.is_ascii_digit())
        {
            return true;
        }
    }
    false
}

fn normalize_key(key: &str) -> String {
    key.trim().to_uppercase()
}
