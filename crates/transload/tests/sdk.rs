//! The SDK on the host: one test importer through the `importer!` macro,
//! driven by the `Runtime` the exports wrap. The exports themselves exist
//! only on wasm32; the fixture plugin's tests exercise them through a module.
#![cfg(feature = "sdk")]

use domain::{Content, InlineMark, RichText, TextMarkKind, TextSegment};
use indexmap::IndexMap;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use transload::{
    abi::{ABI, Diagnostic, Envelope, FinalizeOutput, Init, Manifest, Output},
    fragment::{
        ContentFragment, FRAGMENT_FORMAT, FeatureFragment, FeatureKindFragment, LandmarkFragment,
        Protochart, ReadingFragment, SourceKey,
    },
    sdk::{Importer, Manifested, Runtime, panic},
};

/// Echoes each entry as a Landmark whose one paragraph is the entry's text
/// with a mention to a Mark nobody defines, and, when configured, binds it to
/// the protochart `finalize` emits with a reading on the named feature.
struct Echo {
    feature: Option<SourceKey>,
}

#[derive(Deserialize, JsonSchema)]
struct EchoConfig {
    #[serde(default)]
    feature: Option<String>,
}

fn key(key: &str) -> SourceKey {
    SourceKey::new(key).unwrap()
}

impl Importer for Echo {
    type Config = EchoConfig;

    fn init(config: EchoConfig) -> Result<Self, Diagnostic> {
        Ok(Self {
            feature: config.feature.as_deref().map(key),
        })
    }

    fn entry(&mut self, name: &str, bytes: &[u8]) -> Result<Output, Diagnostic> {
        let mut landmark = LandmarkFragment::new(key(&format!("page:{name}")), name.to_string());
        let text = RichText::new(vec![
            TextSegment {
                text: String::from_utf8_lossy(bytes).into_owned(),
                marks: Vec::new(),
            },
            TextSegment {
                text: "nowhere".to_string(),
                marks: vec![InlineMark {
                    kind: TextMarkKind::Mention,
                    data: Some(json!("page:nowhere")),
                }],
            },
        ]);
        landmark.body.push(ContentFragment::new(
            key(&format!("block:{name}:1")),
            Content::Paragraph {
                rich_text: text,
                theme: None,
            },
        ));
        if let Some(feature) = &self.feature {
            landmark.protochart = Some(key("shape"));
            landmark.readings.insert(
                feature.clone(),
                ReadingFragment::Text(name.to_string().into()),
            );
        }
        let mut output = Output::default();
        output.fragments.landmarks.push(landmark);
        output
            .diagnostics
            .push(Diagnostic::info("echoed").at(name, None));
        Ok(output)
    }

    fn finalize(self) -> Result<Output, Diagnostic> {
        let mut protochart = Protochart::new(key("shape"), "Shape");
        protochart.features.insert(
            key("title"),
            FeatureFragment {
                name: "Title".to_string(),
                description: None,
                required: false,
                kind: FeatureKindFragment::Text,
            },
        );
        let mut output = Output::default();
        output.fragments.protocharts.push(protochart);
        Ok(output)
    }
}

transload::importer!(Echo, manifest = "sdk/echo.toml");

fn ok<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> T {
    let envelope: Envelope<T> = serde_json::from_slice(bytes).unwrap();
    envelope.into_result().unwrap()
}

fn err<T: serde::de::DeserializeOwned + std::fmt::Debug>(bytes: &[u8]) -> Diagnostic {
    let envelope: Envelope<T> = serde_json::from_slice(bytes).unwrap();
    envelope.into_result().unwrap_err()
}

#[test]
fn the_manifest_is_the_toml_plus_the_stamped_fields() {
    assert_eq!(Echo::AUTHORED, include_str!("sdk/echo.toml"));
    let manifest = Echo::manifest().unwrap();
    assert_eq!(manifest.id, "echo");
    assert_eq!(manifest.name, "Echo");
    assert_eq!(manifest.namespace, "so.waypoints.importer.echo");
    assert_eq!(manifest.source.markers, ["echo.toml"]);
    assert_eq!(manifest.abi, ABI);
    assert_eq!(manifest.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(manifest.fragment_format, FRAGMENT_FORMAT);
    assert_eq!(manifest.domain_version, domain::VERSION);
    assert_eq!(manifest.options["$schema"], transload::sdk::DRAFT_07);
    assert_eq!(
        manifest.options["properties"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["feature"]
    );

    let served: Manifest = serde_json::from_slice(&Runtime::<Echo>::new().manifest()).unwrap();
    assert_eq!(served, manifest);
}

#[test]
fn the_lifecycle_runs_over_bytes() {
    let mut runtime = Runtime::<Echo>::new();

    let init: Init = ok(&runtime.init(b"{}"));
    assert_eq!(init.state, 1);

    let output: Output = ok(&runtime.entry(init.state, "a.txt", b"hello"));
    assert_eq!(output.fragments.landmarks.len(), 1);
    assert_eq!(output.fragments.landmarks[0].src.as_str(), "page:a.txt");
    assert_eq!(
        output.fragments.landmarks[0].body[0].src.as_str(),
        "block:a.txt:1"
    );
    assert_eq!(output.diagnostics[0].at.as_ref().unwrap().entry, "a.txt");

    let finalize: FinalizeOutput = ok(&runtime.finalize(init.state));
    assert_eq!(finalize.fragments.protocharts[0].src.as_str(), "shape");
    assert_eq!(finalize.fragment_format, FRAGMENT_FORMAT);
    assert_eq!(finalize.summary.landmarks, 1);
    assert_eq!(finalize.summary.content, 1);
    assert_eq!(finalize.summary.protocharts, 1);
    assert_eq!(
        finalize.census.content.iter().collect::<Vec<_>>(),
        ["paragraph"]
    );
    assert_eq!(
        finalize.census.feature_kinds.iter().collect::<Vec<_>>(),
        ["text"]
    );

    let stale = err::<FinalizeOutput>(&runtime.finalize(init.state));
    assert!(stale.msg.contains("no live instance"), "{stale}");
}

#[test]
fn configuration_the_type_rejects_is_an_err_envelope() {
    let mut runtime = Runtime::<Echo>::new();
    let diagnostic = err::<Init>(&runtime.init(b"{\"feature\": 3}"));
    assert!(
        diagnostic.msg.starts_with("configuration is not valid"),
        "{diagnostic}"
    );
}

#[test]
fn a_reading_on_an_undefined_feature_fails_finalize_and_a_forward_mention_does_not() {
    let mut runtime = Runtime::<Echo>::new();
    let init: Init = ok(&runtime.init(br#"{"feature": "title"}"#));
    ok::<Output>(&runtime.entry(init.state, "a.txt", b"x"));
    ok::<FinalizeOutput>(&runtime.finalize(init.state));

    let init: Init = ok(&runtime.init(br#"{"feature": "colour"}"#));
    ok::<Output>(&runtime.entry(init.state, "a.txt", b"x"));
    let diagnostic = err::<FinalizeOutput>(&runtime.finalize(init.state));
    assert!(diagnostic.msg.contains("`page:a.txt`"), "{diagnostic}");
    assert!(diagnostic.msg.contains("`colour`"), "{diagnostic}");
}

#[test]
fn state_handles_are_independent_and_reused_after_finalize() {
    let mut runtime = Runtime::<Echo>::new();
    let first: Init = ok(&runtime.init(b"{}"));
    let second: Init = ok(&runtime.init(b"{}"));
    assert_ne!(first.state, second.state);
    ok::<FinalizeOutput>(&runtime.finalize(first.state));
    let third: Init = ok(&runtime.init(b"{}"));
    assert_eq!(third.state, first.state);
    ok::<Output>(&runtime.entry(second.state, "b.txt", b"y"));
}

#[test]
fn the_hook_keeps_a_panics_message_and_location() {
    panic::install();
    panic::clear();
    assert_eq!(panic::last(), None);
    let caught = std::panic::catch_unwind(|| panic!("asked to fail on `a.txt`"));
    assert!(caught.is_err());
    let last = panic::last().unwrap();
    assert!(last.contains("asked to fail on `a.txt`"), "{last}");
    assert!(last.contains("tests/sdk.rs"), "{last}");
    panic::clear();
}

#[test]
fn an_empty_key_is_refused_by_the_runtime_too() {
    let _ = IndexMap::<SourceKey, ReadingFragment>::new();
    assert!(SourceKey::new("").is_err());
}
