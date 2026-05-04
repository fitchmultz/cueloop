//! Cursor SDK runner configuration.
//!
//! Purpose:
//! - Define CueLoop-owned configuration for Cursor's SDK-backed runner.
//!
//! Responsibilities:
//! - Model Cursor SDK model parameter pass-through in a user-friendly map shape.
//! - Model local Cursor setting-source selection.
//! - Provide merge behavior for config/task/phase layering.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Cursor SDK runner configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct CursorRunnerConfig {
    /// Cursor SDK model parameters keyed by SDK parameter id.
    ///
    /// CueLoop serializes values to the SDK's `{ id, value }[]` string shape. Use
    /// `cueloop runner models cursor --model <id>` to discover valid ids/values when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_params: Option<BTreeMap<String, CursorModelParamValue>>,

    /// Ambient Cursor settings layers to load for the local SDK runner.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setting_sources: Option<Vec<CursorSettingSource>>,
}

impl CursorRunnerConfig {
    /// Leaf-wise merge: set fields override, unset fields inherit.
    pub fn merge_from(&mut self, other: Self) {
        if let Some(other_params) = other.model_params {
            match &mut self.model_params {
                Some(existing) => existing.extend(other_params),
                None => self.model_params = Some(other_params),
            }
        }
        if other.setting_sources.is_some() {
            self.setting_sources = other.setting_sources;
        }
    }

    pub fn is_effectively_empty(&self) -> bool {
        self.model_params.as_ref().is_none_or(BTreeMap::is_empty)
            && self.setting_sources.as_ref().is_none_or(Vec::is_empty)
    }
}

/// Cursor SDK model parameter value.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(untagged)]
pub enum CursorModelParamValue {
    String(String),
    Bool(bool),
}

impl CursorModelParamValue {
    pub fn as_sdk_value(&self) -> String {
        match self {
            CursorModelParamValue::String(value) => value.clone(),
            CursorModelParamValue::Bool(value) => value.to_string(),
        }
    }
}

/// Cursor local setting source.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CursorSettingSource {
    Project,
    User,
    Team,
    Mdm,
    Plugins,
    All,
}

impl CursorSettingSource {
    pub fn as_sdk_value(self) -> &'static str {
        match self {
            CursorSettingSource::Project => "project",
            CursorSettingSource::User => "user",
            CursorSettingSource::Team => "team",
            CursorSettingSource::Mdm => "mdm",
            CursorSettingSource::Plugins => "plugins",
            CursorSettingSource::All => "all",
        }
    }
}
