// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

use cartograph::SelfUpdatePolicy;
use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

pub const SETTINGS_FILE: &str = "settings.json";
pub const KEY_TERRITORIES: &str = "territories";
pub const KEY_SELF_UPDATE: &str = "self_update";

pub type SettingsResult<T> = Result<T, SettingsError>;

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("settings store: {0}")]
    Store(#[from] tauri_plugin_store::Error),
    #[error("settings value: {0}")]
    Encode(#[from] serde_json::Error),
}

impl Serialize for SettingsError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// The user's stored self-update setting; the default when none is stored.
pub fn self_update(app: &AppHandle) -> SettingsResult<SelfUpdatePolicy> {
    let store = app.store(SETTINGS_FILE)?;
    let policy = decode_self_update(store.get(KEY_SELF_UPDATE));
    store.close_resource();
    Ok(policy)
}

pub fn set_self_update(app: &AppHandle, policy: SelfUpdatePolicy) -> SettingsResult<()> {
    let store = app.store(SETTINGS_FILE)?;
    store.set(KEY_SELF_UPDATE, serde_json::to_value(policy)?);
    store.save()?;
    store.close_resource();
    Ok(())
}

/// An absent or unreadable value is the default, not an error: a setting
/// written by a newer build must not keep an older one from checking.
fn decode_self_update(value: Option<serde_json::Value>) -> SelfUpdatePolicy {
    match value {
        None => SelfUpdatePolicy::default(),
        Some(value) => serde_json::from_value(value.clone()).unwrap_or_else(|error| {
            tracing::warn!("ignoring unreadable `{KEY_SELF_UPDATE}` setting {value}: {error}");
            SelfUpdatePolicy::default()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_absent_key_is_on() {
        assert_eq!(decode_self_update(None), SelfUpdatePolicy::On);
    }

    #[test]
    fn a_stored_value_is_read_back() {
        assert_eq!(
            decode_self_update(Some(json!("notify"))),
            SelfUpdatePolicy::Notify
        );
        assert_eq!(
            decode_self_update(Some(json!("off"))),
            SelfUpdatePolicy::Off
        );
    }

    #[test]
    fn an_unreadable_value_is_the_default() {
        assert_eq!(decode_self_update(Some(json!(3))), SelfUpdatePolicy::On);
        assert_eq!(
            decode_self_update(Some(json!("sometimes"))),
            SelfUpdatePolicy::On
        );
    }
}
