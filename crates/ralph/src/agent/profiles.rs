//! Built-in configuration profiles and helpers for resolving effective profile patches.
//!
//! Responsibilities:
//! - Provide built-in profile definitions (quick/thorough).
//! - Provide helpers to list and resolve profiles from config + built-ins.
//!
//! Not handled here:
//! - CLI parsing (see `crate::cli` / `crate::agent::args`).
//! - Applying profiles to resolved config (see `crate::config`).
//!
//! Invariants/assumptions:
//! - Profile values are `AgentConfig` patches: only `Some(...)` fields override.

use crate::contracts::{AgentConfig, Model, Runner};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const BUILTIN_QUICK: &str = "quick";
pub(crate) const BUILTIN_THOROUGH: &str = "thorough";

pub(crate) fn builtin_profiles() -> BTreeMap<&'static str, AgentConfig> {
    BTreeMap::from([
        (
            BUILTIN_QUICK,
            AgentConfig {
                runner: Some(Runner::Kimi),
                model: Some(Model::Custom("kimi-for-coding".to_string())),
                phases: Some(1),
                ..Default::default()
            },
        ),
        (
            BUILTIN_THOROUGH,
            AgentConfig {
                runner: Some(Runner::Claude),
                model: Some(Model::Custom("opus".to_string())),
                phases: Some(3),
                ..Default::default()
            },
        ),
    ])
}

pub(crate) fn all_profile_names(
    config_profiles: Option<&BTreeMap<String, AgentConfig>>,
) -> BTreeSet<String> {
    let mut names: BTreeSet<String> = builtin_profiles()
        .keys()
        .map(|k| (*k).to_string())
        .collect();
    if let Some(map) = config_profiles {
        for name in map.keys() {
            names.insert(name.clone());
        }
    }
    names
}

pub(crate) fn resolve_profile_patch(
    name: &str,
    config_profiles: Option<&BTreeMap<String, AgentConfig>>,
) -> Option<AgentConfig> {
    if let Some(map) = config_profiles
        && let Some(patch) = map.get(name)
    {
        return Some(patch.clone());
    }
    builtin_profiles().get(name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_profiles_includes_quick_and_thorough() {
        let profiles = builtin_profiles();
        assert!(profiles.contains_key(BUILTIN_QUICK));
        assert!(profiles.contains_key(BUILTIN_THOROUGH));
    }

    #[test]
    fn builtin_quick_has_expected_values() {
        let profiles = builtin_profiles();
        let quick = profiles.get(BUILTIN_QUICK).unwrap();
        assert_eq!(quick.runner, Some(Runner::Kimi));
        assert_eq!(
            quick.model,
            Some(Model::Custom("kimi-for-coding".to_string()))
        );
        assert_eq!(quick.phases, Some(1));
    }

    #[test]
    fn builtin_thorough_has_expected_values() {
        let profiles = builtin_profiles();
        let thorough = profiles.get(BUILTIN_THOROUGH).unwrap();
        assert_eq!(thorough.runner, Some(Runner::Claude));
        assert_eq!(thorough.model, Some(Model::Custom("opus".to_string())));
        assert_eq!(thorough.phases, Some(3));
    }

    #[test]
    fn all_profile_names_includes_builtins() {
        let names = all_profile_names(None);
        assert!(names.contains(BUILTIN_QUICK));
        assert!(names.contains(BUILTIN_THOROUGH));
    }

    #[test]
    fn all_profile_names_includes_config_profiles() {
        let mut config_profiles = BTreeMap::new();
        config_profiles.insert(
            "custom".to_string(),
            AgentConfig {
                runner: Some(Runner::Codex),
                ..Default::default()
            },
        );
        let names = all_profile_names(Some(&config_profiles));
        assert!(names.contains(BUILTIN_QUICK));
        assert!(names.contains(BUILTIN_THOROUGH));
        assert!(names.contains("custom"));
    }

    #[test]
    fn resolve_profile_patch_returns_config_profile_first() {
        let mut config_profiles = BTreeMap::new();
        let custom_quick = AgentConfig {
            runner: Some(Runner::Codex),
            model: Some(Model::Gpt53),
            phases: Some(2),
            ..Default::default()
        };
        config_profiles.insert(BUILTIN_QUICK.to_string(), custom_quick.clone());

        let resolved = resolve_profile_patch(BUILTIN_QUICK, Some(&config_profiles)).unwrap();
        assert_eq!(resolved.runner, Some(Runner::Codex));
        assert_eq!(resolved.model, Some(Model::Gpt53));
        assert_eq!(resolved.phases, Some(2));
    }

    #[test]
    fn resolve_profile_patch_falls_back_to_builtin() {
        let resolved = resolve_profile_patch(BUILTIN_QUICK, None).unwrap();
        assert_eq!(resolved.runner, Some(Runner::Kimi));
    }

    #[test]
    fn resolve_profile_patch_returns_none_for_unknown() {
        let resolved = resolve_profile_patch("unknown_profile", None);
        assert!(resolved.is_none());
    }
}
