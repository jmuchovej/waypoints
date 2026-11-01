use domain::{Icon, Json};
use serde::{Deserialize, Serialize};

/// What shape of source a plugin reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    /// A directory tree; every file is an entry named by its relative path.
    Directory,
    /// An archive (a zip, say); every member is an entry named by its path.
    Archive,
    /// One payload — the share-sheet case.
    File,
}

/// The source shape a plugin declares: what it reads, and how a host may
/// recognize one without asking.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub kinds: Vec<SourceKind>,
    /// Paths whose presence marks a source as this plugin's. Fed first, in
    /// this order, before every other entry.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<String>,
    /// Extensions of the entries the plugin wants; empty means all.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<String>,
}

/// The fields a plugin author writes, as a TOML document beside the crate.
/// Every field is required except `source.markers` and `source.extensions`;
/// a field the manifest does not define is refused, since an author-written
/// file is where a misspelling hides.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Authored {
    /// A stable identifier, `kebab-case`.
    pub id: String,
    /// The name a person sees.
    pub name: String,
    pub icon: Icon,
    /// The reverse-DNS key that scopes every source key and `extra` entry
    /// the plugin writes.
    pub namespace: String,
    pub source: Source,
}

/// What `wp_manifest` returns: the authored fields plus what the SDK stamps.
///
/// A reader ignores fields it does not know, per the additive-evolution rule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// The ABI major. Stamped.
    pub abi: u32,
    pub id: String,
    pub name: String,
    pub icon: Icon,
    pub namespace: String,
    pub source: Source,
    /// The plugin crate's version. Stamped from `CARGO_PKG_VERSION`.
    pub version: String,
    /// The Fragment envelope version the plugin emits. Stamped.
    pub fragment_format: String,
    /// The data-model grammar version the plugin was compiled against. Stamped.
    pub domain_version: String,
    /// JSON Schema Draft-07 for the plugin's configuration. Generated.
    pub options: Json,
}

impl Manifest {
    /// Merge the authored fields with the stamped ones.
    pub fn stamp(authored: Authored, version: impl Into<String>, options: Json) -> Self {
        Self {
            abi: super::ABI,
            id: authored.id,
            name: authored.name,
            icon: authored.icon,
            namespace: authored.namespace,
            source: authored.source,
            version: version.into(),
            fragment_format: crate::fragment::FRAGMENT_FORMAT.to_string(),
            domain_version: domain::VERSION.to_string(),
            options,
        }
    }
}
