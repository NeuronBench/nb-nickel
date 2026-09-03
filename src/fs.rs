//! Filesystem-backed linking for native tools and tests.
//!
//! The browser cannot use this; nb-sim drives [`crate::plan`] with its own
//! fetch loop instead. Here every import is read from disk relative to the
//! importing file.

use std::collections::HashMap;
use std::path::Path;

use crate::eval::Diagnostic;
use crate::link::{plan, Linked, Plan};

/// Make `root` absolute so relative imports resolve deterministically.
pub fn absolute_root(root: impl AsRef<Path>) -> String {
    let root = root.as_ref();
    let abs = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir().map(|cwd| cwd.join(root)).unwrap_or_else(|_| root.to_path_buf())
    };
    abs.to_string_lossy().to_string()
}

/// Link `root` by reading it and every transitive import from disk.
pub fn link(root: impl AsRef<Path>) -> Result<Linked, Vec<Diagnostic>> {
    let root = absolute_root(root);
    let mut sources: HashMap<String, String> = HashMap::new();
    loop {
        match plan(&root, &sources) {
            Ok(Plan::Ready(linked)) => return Ok(linked),
            Ok(Plan::NeedSources(missing)) => {
                for path in missing {
                    let text = std::fs::read_to_string(&path)
                        .map_err(|e| vec![Diagnostic::plain(format!("could not read {path}: {e}"))])?;
                    sources.insert(path, text);
                }
            }
            Err(e) => return Err(vec![Diagnostic::plain(e.to_string())]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::evaluate_json;

    #[test]
    fn links_and_evaluates_files_from_disk() {
        let dir = std::env::temp_dir().join(format!("nb-nickel-fs-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("lib")).unwrap();
        std::fs::write(dir.join("main.ncl"), "let l = import \"lib/x.ncl\" in l.x * 2").unwrap();
        std::fs::write(dir.join("lib/x.ncl"), "{ x = 21 }").unwrap();
        let linked = link(dir.join("main.ncl")).unwrap();
        assert_eq!(evaluate_json(&linked, &[], None).unwrap().trim(), "42");
        std::fs::remove_dir_all(dir).ok();
    }
}
