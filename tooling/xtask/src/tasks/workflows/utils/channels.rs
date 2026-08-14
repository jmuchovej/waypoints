//! The release channels, and everything that varies between them.
//!
//! The hand-written YAML threaded these as six `workflow_call` inputs through
//! three near-identical caller workflows. They are one enum here.

use derive_more::Display;
use gh_workflow::Expression;

use super::Platform;

#[derive(Clone, Copy, PartialEq, Eq, Display)]
#[display(rename_all = "lowercase")]
pub enum Channel {
    Stable,
    Preview,
    Nightly,
}

impl Channel {
    pub const ALL: [Channel; 3] = [Channel::Stable, Channel::Preview, Channel::Nightly];

    /// Names the workflow, the `just icons::<flavor>` recipe, and the cargo
    /// feature — they have deliberately been kept the same string.
    pub fn flavor(self) -> String {
        self.to_string()
    }

    pub fn prerelease(self) -> bool {
        self != Channel::Stable
    }

    /// Nightly has no iOS build: TestFlight rejects more than a handful of
    /// builds a day against the same base version.
    pub fn ships_ios(self) -> bool {
        self != Channel::Nightly
    }

    /// The bare expression yielding this channel's release tag, for use in an
    /// `if:` — which is already an expression context and must not be wrapped.
    ///
    /// Stable and preview are tag-driven, so a `push` already knows its tag and
    /// a manual dispatch supplies one (empty means build-only). Nightly derives
    /// its tag in the `prepare` job.
    fn tag_expr(self) -> &'static str {
        match self {
            Channel::Nightly => "needs.prepare.outputs.tag",
            _ => "github.event_name == 'push' && github.ref_name || inputs.tag",
        }
    }

    /// The same tag, wrapped for use as a `with:` value.
    pub fn tag(self) -> String {
        format!("${{{{ {} }}}}", self.tag_expr())
    }

    /// True when this run should publish. A build-only run has an empty tag.
    pub fn tag_condition(self) -> Expression {
        Expression::new(match self {
            Channel::Nightly => format!("{} != ''", self.tag_expr()),
            _ => format!("({}) != ''", self.tag_expr()),
        })
    }

    /// The tag pattern a push must match to trigger this channel.
    pub fn tag_pattern(self) -> &'static str {
        use Channel::*;
        match self {
            Stable => "v[0-9]+.[0-9]+.[0-9]+",
            Preview => "v[0-9]+.[0-9]+.[0-9]+-pre.[0-9]+",
            Nightly => "v[0-9]+.[0-9]+.[0-9]+-nightly.[0-9]{8}",
        }
    }

    /// The full set of choices offered by the `platforms` dispatch input.
    pub fn dispatch_platforms(self) -> Vec<&'static str> {
        let mut options = vec!["All", "Desktop", "macOS", "Linux", "Windows", "Android"];
        if self.ships_ios() {
            options.push("iOS");
        }
        options
    }

    /// Guards a job on the `platforms` dispatch input.
    ///
    /// `labels` are the dropdown choices that should select this job — a
    /// desktop job answers to both `Desktop` and its own OS. Nightly also waits
    /// on `prepare`, which decides whether anything changed since last night.
    fn guard(self, labels: &[&str]) -> Expression {
        let choices = labels
            .iter()
            .map(|label| format!("\"{label}\""))
            .collect::<Vec<_>>()
            .join(",");
        let selected = format!("contains(fromJson('[{choices}]'), inputs.platforms)");

        Expression::new(match self {
            Channel::Nightly => format!(
                "needs.prepare.outputs.should_build == 'true' && \
                 (github.event_name != 'workflow_dispatch' || {selected})"
            ),
            _ => format!("github.event_name == 'push' || {selected}"),
        })
    }

    pub fn desktop_guard(self, platform: Platform) -> Expression {
        self.guard(&["All", "Desktop", platform.dispatch_label().as_str()])
    }

    pub fn android_guard(self) -> Expression {
        self.guard(&["All", Platform::Android.dispatch_label().as_str()])
    }

    pub fn ios_guard(self) -> Expression {
        self.guard(&["All", Platform::iOS.dispatch_label().as_str()])
    }
}

/// The `upload-artifact` name for one build: `waypoints-{platform}-{channel}`.
///
/// A newtype because `impl Display for (Channel, Platform)` cannot compile —
/// the orphan rule treats tuples as foreign however local their elements, so a
/// foreign trait needs a local type to hang off. This is that type.
#[derive(Display)]
#[display("waypoints-{}-{}", _1.slug(), _0)]
pub struct Artifact(pub Channel, pub Platform);
