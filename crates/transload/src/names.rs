//! What a mention's `[[…]]` holds: a Landmark's title, and for a block, an id
//! prefix within the file it is written in.
//!
//! Export and ingest build this from the same titles and the same files, so a
//! name spelled by one resolves in the other.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use domain::{
    MarkID,
    syntax::inline::{LinkName, Links},
};

/// The shortest id prefix a name carries.
const PREFIX: usize = 4;

#[derive(Debug, Default)]
pub struct Names {
    /// Landmark → its name and plain title.
    pages: BTreeMap<MarkID, (String, String)>,
    by_name: HashMap<String, MarkID>,
    /// Block → its home Landmark and prefix there.
    blocks: BTreeMap<MarkID, (MarkID, String)>,
    /// Landmark → every block written in its file.
    files: BTreeMap<MarkID, BTreeSet<MarkID>>,
}

impl Names {
    /// `titles` holds every Landmark's plain title; `files` every Landmark's
    /// blocks. A block written in several files is named in the first.
    pub fn new(titles: &BTreeMap<MarkID, String>, files: &BTreeMap<MarkID, Vec<MarkID>>) -> Self {
        let mut groups: BTreeMap<String, Vec<MarkID>> = BTreeMap::new();
        for (id, title) in titles {
            groups
                .entry(title.replace(['\n', '\r'], " "))
                .or_default()
                .push(*id);
        }

        // A title shared, empty, or shaped like an id gets a suffix on every
        // Landmark carrying it, never on all but one.
        let suffixed = |base: &str, ids: &[MarkID]| {
            ids.len() > 1 || base.is_empty() || base.parse::<uuid::Uuid>().is_ok()
        };
        let mut taken: BTreeSet<String> = groups
            .iter()
            .filter(|(base, ids)| !suffixed(base, ids))
            .map(|(base, _)| base.clone())
            .collect();

        let mut out = Self::default();
        for (base, ids) in &groups {
            let spelled: Vec<String> = if suffixed(base, ids) {
                (PREFIX..=32)
                    .map(|len| {
                        ids.iter()
                            .map(|id| with_suffix(base, &hex(id)[..len]))
                            .collect::<Vec<_>>()
                    })
                    .find(|names| {
                        names.iter().collect::<BTreeSet<_>>().len() == names.len()
                            && names.iter().all(|name| !taken.contains(name))
                    })
                    .unwrap_or_else(|| {
                        ids.iter()
                            .map(|id| with_suffix(base, &id.to_string()))
                            .collect()
                    })
            } else {
                vec![base.clone()]
            };
            for (id, name) in ids.iter().zip(spelled) {
                taken.insert(name.clone());
                out.by_name.insert(name.clone(), *id);
                out.pages.insert(*id, (name, titles[id].clone()));
            }
        }

        for (page, members) in files {
            let members: BTreeSet<MarkID> = members.iter().copied().collect();
            for block in &members {
                if out.blocks.contains_key(block) {
                    continue;
                }
                let own = hex(block);
                let prefix = (PREFIX..32)
                    .map(|len| &own[..len])
                    .find(|prefix| {
                        members
                            .iter()
                            .filter(|other| hex(other).starts_with(prefix))
                            .count()
                            == 1
                    })
                    .unwrap_or(&own);
                out.blocks.insert(*block, (*page, prefix.to_string()));
            }
            out.files.insert(*page, members);
        }
        out
    }
}

fn hex(id: &MarkID) -> String {
    id.0.simple().to_string()
}

fn with_suffix(base: &str, suffix: &str) -> String {
    if base.is_empty() {
        format!("({suffix})")
    } else {
        format!("{base} ({suffix})")
    }
}

impl Links for Names {
    fn name(&self, target: &str) -> LinkName {
        let verbatim = LinkName {
            page: target.to_string(),
            block: None,
            title: None,
        };
        let Ok(id) = target.parse::<uuid::Uuid>().map(MarkID) else {
            return verbatim;
        };
        if let Some((name, title)) = self.pages.get(&id) {
            return LinkName {
                page: name.clone(),
                block: None,
                title: Some(title.clone()),
            };
        }
        if let Some((page, prefix)) = self.blocks.get(&id) {
            return LinkName {
                page: self.pages[page].0.clone(),
                block: Some(prefix.clone()),
                title: None,
            };
        }
        verbatim
    }

    fn resolve(&self, page: &str, block: Option<&str>) -> Result<String, String> {
        let landmark = match (self.by_name.get(page), block) {
            (Some(id), _) => *id,
            (None, None) if page.parse::<uuid::Uuid>().is_ok() => return Ok(page.to_string()),
            (None, _) => return Err(format!("no Landmark is named `{page}`")),
        };
        let Some(block) = block else {
            return Ok(landmark.to_string());
        };
        let matches: Vec<&MarkID> = self
            .files
            .get(&landmark)
            .into_iter()
            .flatten()
            .filter(|id| hex(id).starts_with(block))
            .collect();
        match matches.as_slice() {
            [only] => Ok(only.to_string()),
            [] => Err(format!("`{page}` holds no block `#{block}`")),
            _ => Err(format!("`#{block}` names more than one block in `{page}`")),
        }
    }
}

/// For Landmark titles, which keep every label: on ingest the names are built
/// from titles as written, so a title cannot lean on them.
pub struct KeepLabels<'a>(pub &'a dyn Links);

/// Reads any target as nothing, for the outline pass that builds the names.
pub struct Lenient;

impl Links for Lenient {
    fn name(&self, target: &str) -> LinkName {
        LinkName {
            page: target.to_string(),
            block: None,
            title: None,
        }
    }

    fn resolve(&self, _: &str, _: Option<&str>) -> Result<String, String> {
        Ok(String::new())
    }
}

impl Links for KeepLabels<'_> {
    fn name(&self, target: &str) -> LinkName {
        LinkName {
            title: None,
            ..self.0.name(target)
        }
    }

    fn resolve(&self, page: &str, block: Option<&str>) -> Result<String, String> {
        self.0.resolve(page, block)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u128) -> MarkID {
        MarkID(uuid::Uuid::from_u128(n))
    }

    fn names(titles: &[(u128, &str)], files: &[(u128, &[u128])]) -> Names {
        Names::new(
            &titles
                .iter()
                .map(|(n, title)| (id(*n), title.to_string()))
                .collect(),
            &files
                .iter()
                .map(|(page, blocks)| (id(*page), blocks.iter().map(|n| id(*n)).collect()))
                .collect(),
        )
    }

    #[test]
    fn a_shared_title_suffixes_every_holder() {
        let a = 0xa1b2_0000_0000_0000_0000_0000_0000_0000;
        let b = 0xc3d4_0000_0000_0000_0000_0000_0000_0000;
        let names = names(&[(a, "Cosmos"), (b, "Cosmos"), (3, "Solo"), (4, "")], &[]);
        assert_eq!(names.name(&id(a).to_string()).page, "Cosmos (a1b2)");
        assert_eq!(names.name(&id(b).to_string()).page, "Cosmos (c3d4)");
        assert_eq!(names.name(&id(3).to_string()).page, "Solo");
        assert_eq!(names.name(&id(4).to_string()).page, "(0000)");
        assert_eq!(names.resolve("Cosmos (c3d4)", None), Ok(id(b).to_string()));
        assert!(names.resolve("Cosmos", None).is_err());
    }

    #[test]
    fn a_suffix_grows_until_it_tells_holders_apart() {
        let a = 0xabcd_e000_0000_0000_0000_0000_0000_0000;
        let b = 0xabcd_f000_0000_0000_0000_0000_0000_0000;
        let names = names(&[(a, "X"), (b, "X")], &[]);
        assert_eq!(names.name(&id(a).to_string()).page, "X (abcde)");
        assert_eq!(names.name(&id(b).to_string()).page, "X (abcdf)");
    }

    #[test]
    fn a_block_is_named_in_its_first_file_by_a_unique_prefix() {
        let shared = 0x0164_b041_0000_0000_0000_0000_0000_0000;
        let near = 0x0164_b042_0000_0000_0000_0000_0000_0000;
        let names = names(
            &[(1, "Kuhn"), (2, "Later")],
            &[(1, &[shared, near]), (2, &[shared])],
        );
        let name = names.name(&id(shared).to_string());
        assert_eq!(
            (name.page.as_str(), name.block.as_deref()),
            ("Kuhn", Some("0164b041"))
        );
        assert_eq!(
            names.resolve("Kuhn", Some("0164b041")),
            Ok(id(shared).to_string())
        );
        assert!(names.resolve("Kuhn", Some("0164")).is_err());
    }

    #[test]
    fn a_target_outside_the_bundle_keeps_its_id() {
        let names = names(&[(1, "Kuhn")], &[]);
        let dangling = id(9).to_string();
        assert_eq!(names.name(&dangling).page, dangling);
        assert_eq!(names.resolve(&dangling, None), Ok(dangling.clone()));
    }
}
