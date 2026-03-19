//! Purpose: Apply secret redaction patterns to arbitrary text.
//!
//! Responsibilities:
//! - Redact known secret-shaped substrings such as key-value pairs, bearer
//!   tokens, AWS tokens, SSH blocks, long hex strings, and sensitive env
//!   values.
//! - Preserve non-secret text, including non-ASCII content.
//!
//! Scope:
//! - Text transformation only; environment-key detection and logging wrappers
//!   live in sibling modules.
//!
//! Usage:
//! - Called anywhere Ralph must render user-visible output safely.
//!
//! Invariants/Assumptions:
//! - Empty and whitespace-only strings round-trip unchanged.
//! - Redaction order remains key/value → bearer → AWS → SSH → hex → env-value.

use crate::constants::defaults::REDACTED;

use super::env::{get_sensitive_env_values, looks_sensitive_label};

pub fn redact_text(value: &str) -> String {
    if value.trim().is_empty() {
        return value.to_string();
    }

    let with_pairs = redact_key_value_pairs(value);
    let with_bearer = redact_bearer_tokens(&with_pairs);
    let with_aws = redact_aws_keys(&with_bearer);
    let with_ssh = redact_ssh_keys(&with_aws);
    let with_hex = redact_hex_tokens(&with_ssh);
    redact_sensitive_env_values(&with_hex)
}

fn push_next_char(out: &mut String, text: &str, index: &mut usize) {
    debug_assert!(text.is_char_boundary(*index));
    if let Some(ch) = text[*index..].chars().next() {
        out.push(ch);
        *index += ch.len_utf8();
    } else {
        *index += 1;
    }
}

fn redact_aws_keys(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if i + 20 <= bytes.len() && &bytes[i..i + 4] == b"AKIA" {
            let mut all_caps_alphanum = true;
            for j in 0..16 {
                let b = bytes[i + 4 + j];
                if !(b.is_ascii_uppercase() || b.is_ascii_digit()) {
                    all_caps_alphanum = false;
                    break;
                }
            }
            if all_caps_alphanum {
                let word_boundary_start = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
                let word_boundary_end =
                    i + 20 == bytes.len() || !bytes[i + 20].is_ascii_alphanumeric();

                if word_boundary_start && word_boundary_end {
                    out.push_str(REDACTED);
                    i += 20;
                    continue;
                }
            }
        }

        if i + 40 <= bytes.len() {
            let mut is_secret = true;
            for j in 0..40 {
                let b = bytes[i + j];
                if !(b.is_ascii_alphanumeric() || b == b'/' || b == b'+' || b == b'=') {
                    is_secret = false;
                    break;
                }
            }
            if is_secret {
                let word_boundary_start = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
                let word_boundary_end =
                    i + 40 == bytes.len() || !bytes[i + 40].is_ascii_alphanumeric();

                if word_boundary_start && word_boundary_end {
                    out.push_str(REDACTED);
                    i += 40;
                    continue;
                }
            }
        }

        push_next_char(&mut out, text, &mut i);
    }
    out
}

fn redact_ssh_keys(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;

    while i < text.len() {
        if text[i..].starts_with("-----BEGIN")
            && let Some(end_marker_pos) = text[i..].find("-----END")
            && let Some(final_dash_pos) = text[i + end_marker_pos + 8..].find("-----")
        {
            let total_end = i + end_marker_pos + 8 + final_dash_pos + 5;
            out.push_str(REDACTED);
            i = total_end;
            continue;
        }
        push_next_char(&mut out, text, &mut i);
    }
    out
}

fn redact_hex_tokens(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i].is_ascii_hexdigit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
                i += 1;
            }
            let len = i - start;
            if len >= 32 {
                let word_boundary_start = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
                let word_boundary_end = i == bytes.len() || !bytes[i].is_ascii_alphanumeric();

                if word_boundary_start && word_boundary_end {
                    out.push_str(REDACTED);
                    continue;
                }
            }
            out.push_str(&text[start..i]);
        } else {
            push_next_char(&mut out, text, &mut i);
        }
    }
    out
}

fn redact_key_value_pairs(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        if !is_key_char(ch) {
            out.push(ch);
            i += 1;
            continue;
        }

        let start = i;
        let mut end = i;
        while end < chars.len() && is_key_char(chars[end]) {
            end += 1;
        }

        let key: String = chars[start..end].iter().collect();
        if looks_sensitive_label(&key) {
            let mut cursor = end;
            while cursor < chars.len() && chars[cursor].is_whitespace() && chars[cursor] != '\n' {
                cursor += 1;
            }
            if cursor < chars.len() && (chars[cursor] == ':' || chars[cursor] == '=') {
                cursor += 1;
                while cursor < chars.len() && chars[cursor].is_whitespace() && chars[cursor] != '\n'
                {
                    cursor += 1;
                }

                let value_start = cursor;
                let mut value_end = value_start;
                if value_start < chars.len()
                    && (chars[value_start] == '"' || chars[value_start] == '\'')
                {
                    let quote = chars[value_start];
                    value_end += 1;
                    while value_end < chars.len() && chars[value_end] != quote {
                        value_end += 1;
                    }
                    if value_end < chars.len() {
                        value_end += 1;
                    }
                } else {
                    while value_end < chars.len() && !chars[value_end].is_whitespace() {
                        value_end += 1;
                    }
                }

                out.extend(chars[i..value_start].iter());
                out.push_str(REDACTED);
                i = value_end;
                continue;
            }
        }

        out.extend(chars[i..end].iter());
        i = end;
    }

    out
}

fn redact_bearer_tokens(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let needle = "bearer ";
    let mut out = String::with_capacity(text.len());
    let mut index = 0;

    while let Some(pos) = lower[index..].find(needle) {
        let abs = index + pos;
        if abs > 0 {
            let prev = text.as_bytes()[abs - 1];
            if prev.is_ascii_alphanumeric() {
                let next_index = abs + 1;
                out.push_str(&text[index..next_index]);
                index = next_index;
                continue;
            }
        }

        let start = abs + needle.len();
        let bytes = text.as_bytes();
        let mut end = start;
        while end < bytes.len() && !bytes[end].is_ascii_whitespace() {
            end += 1;
        }

        out.push_str(&text[index..start]);
        out.push_str(REDACTED);
        index = end;
    }

    out.push_str(&text[index..]);
    out
}

fn redact_sensitive_env_values(text: &str) -> String {
    let sensitive_values = get_sensitive_env_values();
    if sensitive_values.is_empty() {
        return text.to_string();
    }
    let mut redacted = text.to_string();
    for value in &sensitive_values {
        redacted = redacted.replace(value.as_str(), REDACTED);
    }
    redacted
}

fn is_key_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}
