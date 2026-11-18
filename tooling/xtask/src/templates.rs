// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Every message xtask writes on the forge — commit subjects, a summary's
//! headers, pull-request bodies, and what a dry run renders — lives in
//! `templates.toml` as a Tera template, named by its table path. Rendering is
//! the one way to produce one, so the file is the one place to read them.
//!
//! A caller builds its context with [`tera::context!`]: a flat template takes
//! `key => &value` pairs, and a nested table is a [`TeraValue`] map inserted
//! like any other value, since the macro inserts whatever serializes.

use anyhow::{Context as _, Result, anyhow};
use std::{collections::BTreeMap, sync::LazyLock};
use tera::{Context, Tera, Value as TeraValue};

const SOURCE: &str = include_str!("templates.toml");

static TEMPLATES: LazyLock<std::result::Result<Tera, String>> = LazyLock::new(|| {
    let table: toml::Table = SOURCE
        .parse()
        .map_err(|error| format!("templates.toml: {error}"))?;
    let mut named = BTreeMap::new();
    flatten(&table, String::new(), &mut named);

    let mut tera = Tera::default();
    tera.autoescape_on(Vec::<&str>::new());
    tera.add_raw_templates(
        named
            .iter()
            .map(|(name, text)| (name.as_str(), text.as_str())),
    )
    .map_err(|error| format!("templates.toml: {error}"))?;
    Ok(tera)
});

/// `commit.series`, `dry-run.refs`: a table path names a template.
fn flatten(table: &toml::Table, prefix: String, into: &mut BTreeMap<String, String>) {
    for (key, value) in table {
        let name = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            toml::Value::Table(nested) => flatten(nested, name, into),
            toml::Value::String(text) => {
                into.insert(name, text.clone());
            }
            other => {
                into.insert(name, other.to_string());
            }
        }
    }
}

/// Renders the template `name` with `context`; an empty context is a template
/// with nothing to fill in.
pub fn render(name: &str, context: Context) -> Result<String> {
    let tera = TEMPLATES.as_ref().map_err(|error| anyhow!("{error}"))?;
    tera.render(name, &context)
        .with_context(|| format!("rendering the template `{name}`"))
}

/// [`render`] where the values are the program's own — a plan's fields, the
/// workflow generator's constants — so a failure is a defect in
/// `templates.toml` rather than in any input, and is reported as one.
pub fn must(name: &str, context: Context) -> String {
    render(name, context).unwrap_or_else(|error| panic!("{error:#}"))
}

/// The names the file defines, for the test that holds every one to a render.
#[cfg(test)]
fn names() -> Vec<String> {
    let table: toml::Table = SOURCE.parse().unwrap();
    let mut named = BTreeMap::new();
    flatten(&table, String::new(), &mut named);
    named.into_keys().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tera::context;

    #[test]
    fn every_template_in_the_file_is_loaded() {
        let tera = TEMPLATES.as_ref().unwrap();
        for name in names() {
            assert!(tera.contains_template(&name), "{name} did not load");
        }
        assert!(names().contains(&"commit.series".to_owned()));
        assert!(names().contains(&"dry-run.refs".to_owned()));
    }

    #[test]
    fn a_subject_renders_from_its_values() {
        let subject = render(
            "commit.series",
            context! {
                branch => "v0.2.x",
                channel => "preview",
                version => "0.2.0-pre.1",
            },
        )
        .unwrap();
        assert_eq!(subject, "chore(release): v0.2.x preview 0.2.0-pre.1");
        assert!(render("commit.nothing", Context::new()).is_err());
    }

    /// A nested table reaches the template as a `Value` map, read with the
    /// dotted path the template already uses.
    #[test]
    fn a_nested_table_renders_from_a_value_map() {
        let main = TeraValue::from(BTreeMap::from([
            ("next", TeraValue::from(Some("0.3.0-nightly".to_owned()))),
            ("landing", TeraValue::from("a pull request")),
            ("headers", TeraValue::from(vec!["chore(bedrock): 0.2.0"])),
        ]));
        let refs = render(
            "dry-run.refs",
            context! {
                transition => "cut",
                body => "the body",
                main => &main,
                groups => &Vec::<String>::new(),
            },
        )
        .unwrap();
        assert!(refs.contains("- main moves to `0.3.0-nightly`\n"), "{refs}");
        assert!(refs.contains("    chore(bedrock): 0.2.0\n"), "{refs}");
        assert!(!refs.contains("series"), "{refs}");
    }
}
