//! Goldens: one committed file per output, compared byte for byte.
//! `manifest.json`, `<entry name>.json` for each entry, and `finalize.json`.

use std::{fs, io, path::Path};

use super::{
    error::HarnessError,
    run::{Difference, Run},
};

/// Compare a run with the goldens under `dir`. Every mismatch is reported,
/// named by its output, with the first line that differs.
pub fn check(dir: &Path, run: &Run) -> Result<(), HarnessError> {
    let mut differences = Vec::new();
    for (name, actual) in run.outputs() {
        let path = dir.join(format!("{name}.json"));
        match fs::read(&path) {
            Ok(expected) if expected == actual => {}
            Ok(expected) => differences.push(Difference::new(name, &expected, actual)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => differences.push(Difference {
                name: name.to_string(),
                detail: format!("no golden at {}", path.display()),
            }),
            Err(error) => return Err(error.into()),
        }
    }
    if differences.is_empty() {
        Ok(())
    } else {
        Err(HarnessError::Mismatch {
            what: "goldens".to_string(),
            differences,
        })
    }
}

/// Write a run as the goldens under `dir`, replacing what is there.
pub fn write(dir: &Path, run: &Run) -> io::Result<()> {
    for (name, bytes) in run.outputs() {
        let path = dir.join(format!("{name}.json"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)?;
    }
    Ok(())
}
