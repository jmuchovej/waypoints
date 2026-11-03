use std::{fs, io, path::Path};

use super::run::Entry;
use crate::abi::Source;

/// The entries of a fixture directory in the ABI's order: the manifest's
/// markers first, in the order it lists them, then every other file the
/// plugin's extensions admit, in byte-wise sorted path order. Names are
/// `/`-separated paths relative to `dir`.
pub fn entries(dir: &Path, source: &Source) -> io::Result<Vec<Entry>> {
    let mut names = Vec::new();
    walk(dir, "", &mut names)?;
    names.sort();

    let is_marker = |name: &str| source.markers.iter().any(|marker| marker == name);
    let admitted = |name: &str| {
        source.extensions.is_empty()
            || source
                .extensions
                .iter()
                .any(|extension| name.ends_with(extension))
    };

    let mut ordered: Vec<&String> = source
        .markers
        .iter()
        .filter_map(|marker| names.iter().find(|name| *name == marker))
        .collect();
    ordered.extend(
        names
            .iter()
            .filter(|name| !is_marker(name) && admitted(name)),
    );

    ordered
        .into_iter()
        .map(|name| {
            Ok(Entry {
                name: name.clone(),
                bytes: fs::read(dir.join(name))?,
            })
        })
        .collect()
}

fn walk(root: &Path, prefix: &str, names: &mut Vec<String>) -> io::Result<()> {
    for entry in fs::read_dir(root.join(prefix))? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        let name = if prefix.is_empty() {
            file_name.into_owned()
        } else {
            format!("{prefix}/{file_name}")
        };
        if entry.file_type()?.is_dir() {
            walk(root, &name, names)?;
        } else {
            names.push(name);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abi::SourceKind;

    fn source(markers: &[&str], extensions: &[&str]) -> Source {
        Source {
            kinds: vec![SourceKind::Directory],
            markers: markers.iter().map(ToString::to_string).collect(),
            extensions: extensions.iter().map(ToString::to_string).collect(),
        }
    }

    fn names(dir: &Path, source: &Source) -> Vec<String> {
        entries(dir, source)
            .unwrap()
            .into_iter()
            .map(|entry| entry.name)
            .collect()
    }

    #[test]
    fn markers_lead_in_manifest_order_and_the_rest_follow_sorted() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("logseq")).unwrap();
        for name in [
            "pages/b.md",
            "pages/a.md",
            "logseq/config.edn",
            "z.md",
            "notes.txt",
        ] {
            let path = dir.path().join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, name).unwrap();
        }
        assert_eq!(
            names(
                dir.path(),
                &source(&["z.md", "logseq/config.edn"], &[".md"])
            ),
            ["z.md", "logseq/config.edn", "pages/a.md", "pages/b.md"]
        );
        assert_eq!(
            names(dir.path(), &source(&[], &[])),
            [
                "logseq/config.edn",
                "notes.txt",
                "pages/a.md",
                "pages/b.md",
                "z.md"
            ]
        );
    }

    #[test]
    fn an_absent_marker_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.md"), "a").unwrap();
        assert_eq!(
            names(dir.path(), &source(&["missing.toml"], &[".md"])),
            ["a.md"]
        );
    }
}
