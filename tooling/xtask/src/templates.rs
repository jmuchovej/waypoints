// SPDX-FileCopyrightText: 2026 John Muchovej
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Every message xtask writes on the forge — commit subjects, a summary's
//! headers, pull-request bodies, and what a dry run renders — lives in
//! `templates.toml` as a Tera template, named by its table path. Rendering is
//! the one way to produce one, so the file is the one place to read them.

use anyhow::{Context as _, Result, anyhow};
use serde::Serialize;
use std::{collections::BTreeMap, sync::LazyLock};
use tera::{Context, Tera};

const SOURCE: &str = include_str!("templates.toml");

static TEMPLATES: LazyLock<std::result::Result<Tera, String>> = LazyLock::new(|| {
    let table: toml::Table = SOURCE
        .parse()
        .map_err(|error| format!("templates.toml: {error}"))?;
    let mut named = BTreeMap::new();
    flatten(&table, String::new(), &mut named);

    let mut tera = Tera::default();
    tera.autoescape_on(Vec::new());
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

/// Renders the template `name` with `values`.
pub fn render<T: Serialize>(name: &str, values: &T) -> Result<String> {
    let tera = TEMPLATES.as_ref().map_err(|error| anyhow!("{error}"))?;
    // `()` is a template with nothing to fill in, not a refusal.
    let context = match serde_json::to_value(values).context("the template's values serialize")? {
        serde_json::Value::Null => Context::new(),
        value => Context::from_value(value).context("the template's values are an object")?,
    };
    tera.render(name, &context)
        .with_context(|| format!("rendering the template `{name}`"))
}

/// [`render`] where the values are the program's own — a plan's fields, the
/// workflow generator's constants — so a failure is a defect in
/// `templates.toml` rather than in any input, and is reported as one.
pub fn must<T: Serialize>(name: &str, values: &T) -> String {
    render(name, values).unwrap_or_else(|error| panic!("{error:#}"))
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

    #[test]
    fn every_template_in_the_file_is_loaded() {
        let tera = TEMPLATES.as_ref().unwrap();
        for name in names() {
            assert!(tera.get_template(&name).is_ok(), "{name} did not load");
        }
        assert!(names().contains(&"commit.series".to_owned()));
        assert!(names().contains(&"dry-run.refs".to_owned()));
    }

    #[test]
    fn a_subject_renders_from_its_values() {
        #[derive(Serialize)]
        struct Series<'a> {
            branch: &'a str,
            channel: &'a str,
            version: &'a str,
        }
        let subject = render(
            "commit.series",
            &Series {
                branch: "v0.2.x",
                channel: "preview",
                version: "0.2.0-pre.1",
            },
        )
        .unwrap();
        assert_eq!(subject, "chore(release): v0.2.x preview 0.2.0-pre.1");
        assert!(render("commit.nothing", &()).is_err());
    }
}
