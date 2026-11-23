// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};

/// What the app may do about a newer version: nothing, say so, or install it.
///
/// Ordered from least to most permissive so the effective policy is a `min`:
/// the user's stored setting against the ceiling the launching package set.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    specta::Type,
)]
#[serde(rename_all = "lowercase")]
pub enum SelfUpdatePolicy {
    /// Never contact the update endpoint.
    Off,
    /// Check, and announce a newer version without downloading or installing.
    Notify,
    /// Check, download, install, restart.
    #[default]
    On,
}

impl SelfUpdatePolicy {
    /// The ceiling a package-manager-owned install launches under.
    pub const CAPPED: Self = Self::Notify;
}

/// The policy as the settings page sees it: what the user chose, the ceiling
/// the launch set, and what the app does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct SelfUpdateStatus {
    /// The stored setting; never written by the launch flag.
    pub setting: SelfUpdatePolicy,
    /// `notify` when launched with `--no-self-update`, `on` otherwise.
    pub cap: SelfUpdatePolicy,
    /// `min(setting, cap)`.
    pub effective: SelfUpdatePolicy,
}

impl SelfUpdateStatus {
    pub fn new(setting: SelfUpdatePolicy, cap: SelfUpdatePolicy) -> Self {
        Self {
            setting,
            cap,
            effective: setting.min(cap),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use SelfUpdatePolicy::{Notify, Off, On};

    #[test]
    fn the_default_is_on() {
        assert_eq!(SelfUpdatePolicy::default(), On);
    }

    #[test]
    fn the_effective_policy_is_the_lesser_of_setting_and_cap() {
        let all = [Off, Notify, On];
        for setting in all {
            for cap in all {
                let status = SelfUpdateStatus::new(setting, cap);
                assert_eq!(status.effective, setting.min(cap));
                assert!(status.effective <= setting);
                assert!(status.effective <= cap);
            }
        }
        assert_eq!(SelfUpdateStatus::new(On, Notify).effective, Notify);
        assert_eq!(SelfUpdateStatus::new(Off, Notify).effective, Off);
        assert_eq!(SelfUpdateStatus::new(On, On).effective, On);
    }

    #[test]
    fn the_wire_form_is_lowercase() {
        assert_eq!(serde_json::to_string(&Notify).unwrap(), "\"notify\"");
        assert_eq!(
            serde_json::from_str::<SelfUpdatePolicy>("\"off\"").unwrap(),
            Off
        );
    }
}
