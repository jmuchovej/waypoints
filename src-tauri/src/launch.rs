// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! What the launching package tells the app: the self-update flags, read off
//! argv before Tauri starts and recorded once for the life of the instance.

use cartograph::SelfUpdatePolicy;
use std::sync::OnceLock;

pub const NO_SELF_UPDATE: &str = "--no-self-update";
pub const SELF_UPDATE: &str = "--self-update";

/// The arguments a binary received, with the self-update flags taken out.
///
/// Only these two literals are interpreted. Everything else — deep links, the
/// process serial numbers macOS passes on some launch paths, `--version` — is
/// handed on untouched in its original order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchArgs {
    /// `notify` when `--no-self-update` was the last flag seen, `on` otherwise.
    pub cap: SelfUpdatePolicy,
    /// Every other argument.
    pub rest: Vec<String>,
}

impl LaunchArgs {
    pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Self {
        let mut cap = SelfUpdatePolicy::On;
        let mut rest = Vec::new();
        for arg in args {
            match arg.as_str() {
                NO_SELF_UPDATE => cap = SelfUpdatePolicy::CAPPED,
                SELF_UPDATE => cap = SelfUpdatePolicy::On,
                _ => rest.push(arg),
            }
        }
        Self { cap, rest }
    }

    /// `--version` as the sole remaining argument. Anywhere else it is one more
    /// argument the app must not interpret.
    pub fn wants_version(&self) -> bool {
        matches!(self.rest.as_slice(), [flag] if flag == "--version" || flag == "-V")
    }
}

/// The running instance's launch policy. Recorded when the primary instance
/// starts and never again: arguments a second instance forwards cannot move it.
#[derive(Debug, Default)]
pub struct Instance {
    cap: OnceLock<SelfUpdatePolicy>,
}

impl Instance {
    pub fn launched(cap: SelfUpdatePolicy) -> Self {
        let instance = Self::default();
        let _ = instance.record(cap);
        instance
    }

    /// Records the cap; refused, returning the rejected value, once one is set.
    pub fn record(&self, cap: SelfUpdatePolicy) -> Result<(), SelfUpdatePolicy> {
        self.cap.set(cap)
    }

    pub fn cap(&self) -> SelfUpdatePolicy {
        self.cap.get().copied().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use SelfUpdatePolicy::{Notify, On};

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_flag_alone_caps_and_leaves_nothing() {
        let parsed = LaunchArgs::parse(args(&["--no-self-update"]));
        assert_eq!(parsed.cap, Notify);
        assert!(parsed.rest.is_empty());
    }

    #[test]
    fn a_deep_link_passes_through_beside_the_flag() {
        let parsed = LaunchArgs::parse(args(&["--no-self-update", "waypoints://pair/x"]));
        assert_eq!(parsed.cap, Notify);
        assert_eq!(parsed.rest, args(&["waypoints://pair/x"]));
    }

    #[test]
    fn the_last_flag_wins() {
        let parsed = LaunchArgs::parse(args(&["--no-self-update", "--self-update"]));
        assert_eq!(parsed.cap, On);
        assert!(parsed.rest.is_empty());

        let parsed = LaunchArgs::parse(args(&["--self-update", "--no-self-update"]));
        assert_eq!(parsed.cap, Notify);
    }

    #[test]
    fn version_keeps_its_sole_argument_rule() {
        let parsed = LaunchArgs::parse(args(&["--version"]));
        assert_eq!(parsed.cap, On);
        assert!(parsed.wants_version());

        let parsed = LaunchArgs::parse(args(&["--no-self-update", "--version"]));
        assert_eq!(parsed.cap, Notify);
        assert!(parsed.wants_version());

        let parsed = LaunchArgs::parse(args(&["waypoints://pair/x", "--version"]));
        assert!(!parsed.wants_version());
    }

    #[test]
    fn nothing_passed_means_no_cap() {
        let parsed = LaunchArgs::parse(args(&[]));
        assert_eq!(parsed.cap, On);
        assert!(parsed.rest.is_empty());
    }

    #[test]
    fn a_forwarded_argument_list_does_not_lift_the_cap() {
        let instance = Instance::launched(Notify);
        // A second instance started without the flag forwards `[]`; its parse
        // says `on`, and the running instance refuses it.
        let forwarded = LaunchArgs::parse(args(&[]));
        assert_eq!(instance.record(forwarded.cap), Err(On));
        assert_eq!(instance.cap(), Notify);
    }

    #[test]
    fn an_unrecorded_instance_is_uncapped() {
        assert_eq!(Instance::default().cap(), On);
    }
}
