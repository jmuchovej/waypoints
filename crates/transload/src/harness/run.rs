use std::fmt;

use domain::Json;

/// One entry a host feeds: its name and its complete bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Everything one run produced, as the bytes that crossed the boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    pub manifest: Vec<u8>,
    /// Each entry's output, in the order the entries were fed.
    pub entries: Vec<(String, Vec<u8>)>,
    pub finalize: Vec<u8>,
}

impl Run {
    /// Every output, named: `manifest`, each entry by its name, `finalize`.
    pub fn outputs(&self) -> impl Iterator<Item = (&str, &[u8])> {
        std::iter::once(("manifest", self.manifest.as_slice()))
            .chain(
                self.entries
                    .iter()
                    .map(|(name, bytes)| (name.as_str(), bytes.as_slice())),
            )
            .chain(std::iter::once(("finalize", self.finalize.as_slice())))
    }

    /// Where `other` departs from this run, output by output; empty when the
    /// two runs are byte-identical.
    pub fn differences(&self, other: &Run) -> Vec<Difference> {
        let mine: Vec<_> = self.outputs().collect();
        let theirs: Vec<_> = other.outputs().collect();
        let mut differences = Vec::new();
        for (index, (name, expected)) in mine.iter().enumerate() {
            match theirs.get(index) {
                Some((other_name, actual)) if other_name == name => {
                    if expected != actual {
                        differences.push(Difference::new(name, expected, actual));
                    }
                }
                Some((other_name, _)) => differences.push(Difference {
                    name: name.to_string(),
                    detail: format!("the other run has `{other_name}` in its place"),
                }),
                None => differences.push(Difference {
                    name: name.to_string(),
                    detail: "the other run has no such output".to_string(),
                }),
            }
        }
        for (name, _) in theirs.iter().skip(mine.len()) {
            differences.push(Difference {
                name: name.to_string(),
                detail: "this run has no such output".to_string(),
            });
        }
        differences
    }
}

/// One output that did not match, and where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Difference {
    pub name: String,
    pub detail: String,
}

impl Difference {
    /// The first line at which two outputs differ, both sides pretty-printed
    /// when they are JSON so the line is readable.
    pub fn new(name: &str, expected: &[u8], actual: &[u8]) -> Self {
        let expected = pretty(expected);
        let actual = pretty(actual);
        let expected: Vec<&str> = expected.lines().collect();
        let actual: Vec<&str> = actual.lines().collect();
        let detail = expected
            .iter()
            .zip(&actual)
            .position(|(e, a)| e != a)
            .map(|index| {
                format!(
                    "line {}:\n  expected: {}\n  actual:   {}",
                    index + 1,
                    expected[index],
                    actual[index]
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "expected {} lines, actual {} lines",
                    expected.len(),
                    actual.len()
                )
            });
        Self {
            name: name.to_string(),
            detail,
        }
    }
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` differs at {}", self.name, self.detail)
    }
}

fn pretty(bytes: &[u8]) -> String {
    serde_json::from_slice::<Json>(bytes)
        .ok()
        .and_then(|value| serde_json::to_string_pretty(&value).ok())
        .unwrap_or_else(|| String::from_utf8_lossy(bytes).into_owned())
}
