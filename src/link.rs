//! Import scanning, path resolution, and linking.

use std::collections::{HashMap, HashSet};

/// One `import "..."` expression found in a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSite {
    /// Byte offset of the `import` keyword.
    pub start: usize,
    /// Byte offset just past the closing quote.
    pub end: usize,
    /// The literal path as written.
    pub path: String,
}

/// Find every `import "path"` in `src`, skipping comments and string literals.
pub fn scan_imports(src: &str) -> Vec<ImportSite> {
    let bytes = src.as_bytes();
    let mut sites = Vec::new();
    let mut i = 0;
    let mut prev_is_ident = false;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'#' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            prev_is_ident = false;
            continue;
        }
        if b == b'"' {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            prev_is_ident = false;
            continue;
        }
        if is_ident_start(b) && !prev_is_ident {
            let start = i;
            while i < bytes.len() && is_ident_char(bytes[i]) {
                i += 1;
            }
            if &src[start..i] == "import" {
                let mut j = i;
                while j < bytes.len() && (bytes[j] as char).is_whitespace() {
                    j += 1;
                }
                if j < bytes.len() && bytes[j] == b'"' {
                    let path_start = j + 1;
                    let mut k = path_start;
                    while k < bytes.len() && bytes[k] != b'"' && bytes[k] != b'\n' {
                        k += 1;
                    }
                    if k < bytes.len() && bytes[k] == b'"' {
                        sites.push(ImportSite {
                            start,
                            end: k + 1,
                            path: src[path_start..k].to_string(),
                        });
                        i = k + 1;
                        prev_is_ident = false;
                        continue;
                    }
                }
            }
            prev_is_ident = true;
            continue;
        }
        prev_is_ident = is_ident_char(b);
        i += 1;
    }
    sites
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'\'' || b == b'-'
}

fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// Normalize a slash-separated path, resolving `.` and `..` components.
fn normalize(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.last().map_or(false, |p| *p != "..") {
                    parts.pop();
                } else if !absolute {
                    parts.push("..");
                }
            }
            p => parts.push(p),
        }
    }
    let joined = parts.join("/");
    if absolute {
        format!("/{joined}")
    } else {
        joined
    }
}

fn dirname(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
}

/// Resolve an import path against the file that imports it.
///
/// Absolute URLs are returned unchanged. Relative paths are joined onto the
/// importer's directory, whether that importer is a URL or a filesystem path.
pub fn resolve_path(base: &str, import: &str) -> String {
    if is_url(import) {
        return import.to_string();
    }
    if is_url(base) {
        let scheme_end = base.find("://").map(|i| i + 3).unwrap_or(0);
        let path_start = base[scheme_end..]
            .find('/')
            .map(|i| scheme_end + i)
            .unwrap_or(base.len());
        let host = &base[..path_start];
        let path = &base[path_start..];
        let joined = if import.starts_with('/') {
            import.to_string()
        } else {
            format!("{}/{}", dirname(path), import)
        };
        let joined = normalize(&joined);
        return format!("{host}{}", if joined.starts_with('/') { joined } else { format!("/{joined}") });
    }
    if import.starts_with('/') {
        return normalize(import);
    }
    let dir = dirname(base);
    if dir.is_empty() {
        if base.starts_with('/') {
            normalize(&format!("/{import}"))
        } else {
            normalize(import)
        }
    } else {
        normalize(&format!("{dir}/{import}"))
    }
}

/// A program with all imports hoisted into `let` bindings.
#[derive(Debug, Clone)]
pub struct Linked {
    /// The complete Nickel text to evaluate.
    pub text: String,
    /// Names of every file that contributed lines, in the order they were hoisted.
    pub files: Vec<String>,
    /// For each line of `text` (0-based), the `(file index, 1-based line)` it came from.
    /// Wrapper lines added by the linker map to `None`.
    pub line_map: Vec<Option<(usize, usize)>>,
    /// Byte offsets at which each line of `text` starts.
    pub line_starts: Vec<usize>,
}

impl Linked {
    /// Map a byte offset in `text` to `(file name, 1-based line, 1-based column)`.
    pub fn locate(&self, byte: usize) -> Option<(String, usize, usize)> {
        let line_idx = match self.line_starts.binary_search(&byte) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let (file_idx, file_line) = self.line_map.get(line_idx).copied().flatten()?;
        let line_start = self.line_starts[line_idx];
        let column = self.text[line_start..byte.min(self.text.len())].chars().count() + 1;
        Some((self.files[file_idx].clone(), file_line, column))
    }
}

/// The outcome of trying to link a program from the sources available so far.
#[derive(Debug, Clone)]
pub enum Plan {
    /// These files are imported but not yet available. Fetch them and try again.
    NeedSources(Vec<String>),
    /// Every import is available; here is the linked program.
    Ready(Linked),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkError {
    Cycle(Vec<String>),
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkError::Cycle(path) => write!(f, "import cycle: {}", path.join(" -> ")),
        }
    }
}

impl std::error::Error for LinkError {}

/// Link `root` using `sources` (a map from resolved path to file text).
///
/// If any transitively imported file is missing from `sources`, returns
/// [`Plan::NeedSources`] listing the missing resolved paths. Callers fetch
/// those, add them to `sources`, and call `plan` again.
pub fn plan(root: &str, sources: &HashMap<String, String>) -> Result<Plan, LinkError> {
    let mut order: Vec<String> = Vec::new();
    let mut done: HashSet<String> = HashSet::new();
    let mut on_stack: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    fn visit(
        path: &str,
        sources: &HashMap<String, String>,
        order: &mut Vec<String>,
        done: &mut HashSet<String>,
        on_stack: &mut Vec<String>,
        missing: &mut Vec<String>,
    ) -> Result<(), LinkError> {
        if done.contains(path) {
            return Ok(());
        }
        if on_stack.iter().any(|p| p == path) {
            let mut cycle = on_stack.clone();
            cycle.push(path.to_string());
            return Err(LinkError::Cycle(cycle));
        }
        let Some(src) = sources.get(path) else {
            if !missing.iter().any(|m| m == path) {
                missing.push(path.to_string());
            }
            return Ok(());
        };
        on_stack.push(path.to_string());
        for site in scan_imports(src) {
            let resolved = resolve_path(path, &site.path);
            visit(&resolved, sources, order, done, on_stack, missing)?;
        }
        on_stack.pop();
        done.insert(path.to_string());
        order.push(path.to_string());
        Ok(())
    }

    visit(root, sources, &mut order, &mut done, &mut on_stack, &mut missing)?;
    if !missing.is_empty() {
        return Ok(Plan::NeedSources(missing));
    }
    Ok(Plan::Ready(assemble(root, &order, sources)))
}

fn hoisted_name(index: usize) -> String {
    format!("__nb_import_{index}")
}

/// Rewrite each `import "..."` in `src` to the hoisted binding for its target.
fn rewrite_imports(path: &str, src: &str, names: &HashMap<String, String>) -> String {
    let mut out = src.to_string();
    let mut sites = scan_imports(src);
    sites.sort_by_key(|s| std::cmp::Reverse(s.start));
    for site in sites {
        let resolved = resolve_path(path, &site.path);
        if let Some(name) = names.get(&resolved) {
            out.replace_range(site.start..site.end, name);
        }
    }
    out
}

fn assemble(root: &str, order: &[String], sources: &HashMap<String, String>) -> Linked {
    let mut text = String::new();
    let mut files: Vec<String> = Vec::new();
    let mut line_map: Vec<Option<(usize, usize)>> = Vec::new();

    fn push_wrapper(text: &mut String, line_map: &mut Vec<Option<(usize, usize)>>, s: &str) {
        text.push_str(s);
        text.push('\n');
        line_map.push(None);
    }

    fn push_file(
        text: &mut String,
        files: &mut Vec<String>,
        line_map: &mut Vec<Option<(usize, usize)>>,
        name: &str,
        body: &str,
    ) {
        let idx = files.len();
        files.push(name.to_string());
        for (i, line) in body.lines().enumerate() {
            text.push_str(line);
            text.push('\n');
            line_map.push(Some((idx, i + 1)));
        }
        if body.lines().count() == 0 {
            // An empty file still needs a line so the parens have something to wrap.
            text.push_str("null\n");
            line_map.push(Some((idx, 1)));
        }
    }

    push_wrapper(&mut text, &mut line_map, "let helpers = (");
    push_file(&mut text, &mut files, &mut line_map, "<nb helpers>", crate::PRELUDE_HELPERS);
    push_wrapper(&mut text, &mut line_map, ") in let schema = (");
    push_file(&mut text, &mut files, &mut line_map, "<nb schema>", crate::PRELUDE_SCHEMA);
    push_wrapper(&mut text, &mut line_map, ") helpers in let nb = helpers & schema in");

    let mut names: HashMap<String, String> = HashMap::new();
    for (i, path) in order.iter().enumerate() {
        if path != root {
            names.insert(path.clone(), hoisted_name(i));
        }
    }
    for path in order {
        let body = rewrite_imports(path, &sources[path], &names);
        if path == root {
            push_file(&mut text, &mut files, &mut line_map, path, &body);
        } else {
            push_wrapper(&mut text, &mut line_map, &format!("let {} = (", names[path]));
            push_file(&mut text, &mut files, &mut line_map, path, &body);
            push_wrapper(&mut text, &mut line_map, ") in");
        }
    }

    let mut line_starts = vec![0];
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' && i + 1 < text.len() {
            line_starts.push(i + 1);
        }
    }
    Linked { text, files, line_map, line_starts }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_imports_and_skips_comments_and_strings() {
        let src = r#"
# import "not_me.ncl"
let a = import "a.ncl" in
let s = "import \"also_not_me.ncl\"" in
let b = import   "dir/b.ncl" in
{ x = a, y = b, z = s }
"#;
        let paths: Vec<_> = scan_imports(src).into_iter().map(|s| s.path).collect();
        assert_eq!(paths, vec!["a.ncl", "dir/b.ncl"]);
    }

    #[test]
    fn resolves_relative_and_url_paths() {
        assert_eq!(resolve_path("/p/scene.ncl", "neurons/n.ncl"), "/p/neurons/n.ncl");
        assert_eq!(resolve_path("/p/neurons/n.ncl", "../membranes.ncl"), "/p/membranes.ncl");
        assert_eq!(resolve_path("lib/scene.ncl", "channels.ncl"), "lib/channels.ncl");
        assert_eq!(resolve_path("/scene.ncl", "channels.ncl"), "/channels.ncl");
        assert_eq!(resolve_path("scene.ncl", "channels.ncl"), "channels.ncl");
        assert_eq!(
            resolve_path("https://h.test/acct/proj/scene.ncl", "channels.ncl"),
            "https://h.test/acct/proj/channels.ncl"
        );
        assert_eq!(
            resolve_path("https://h.test/acct/proj/neurons/n.ncl", "../lib.ncl"),
            "https://h.test/acct/proj/lib.ncl"
        );
        assert_eq!(
            resolve_path("/local/scene.ncl", "https://h.test/x.ncl"),
            "https://h.test/x.ncl"
        );
    }

    #[test]
    fn reports_missing_sources_then_links() {
        let mut sources = HashMap::new();
        sources.insert("/p/scene.ncl".to_string(), "let a = import \"a.ncl\" in a + 1".to_string());
        match plan("/p/scene.ncl", &sources).unwrap() {
            Plan::NeedSources(missing) => assert_eq!(missing, vec!["/p/a.ncl"]),
            Plan::Ready(_) => panic!("should need a.ncl"),
        }
        sources.insert("/p/a.ncl".to_string(), "41".to_string());
        let linked = match plan("/p/scene.ncl", &sources).unwrap() {
            Plan::Ready(l) => l,
            Plan::NeedSources(m) => panic!("still missing {m:?}"),
        };
        assert!(linked.text.contains("let __nb_import_0 = ("));
        assert!(linked.text.ends_with("let a = __nb_import_0 in a + 1\n"));
        let root_idx = linked.files.iter().position(|f| f == "/p/scene.ncl").unwrap();
        assert_eq!(linked.line_map.last().copied().flatten(), Some((root_idx, 1)));
    }

    #[test]
    fn detects_cycles() {
        let mut sources = HashMap::new();
        sources.insert("/a.ncl".to_string(), "import \"b.ncl\"".to_string());
        sources.insert("/b.ncl".to_string(), "import \"a.ncl\"".to_string());
        assert!(matches!(plan("/a.ncl", &sources), Err(LinkError::Cycle(_))));
    }

    #[test]
    fn locate_maps_back_to_original_lines() {
        let mut sources = HashMap::new();
        sources.insert("/p/scene.ncl".to_string(), "let a = import \"a.ncl\" in\n{ x = a }".to_string());
        sources.insert("/p/a.ncl".to_string(), "1\n+\n2".to_string());
        let Plan::Ready(linked) = plan("/p/scene.ncl", &sources).unwrap() else { panic!() };
        let plus = linked.text.find("\n+\n").unwrap() + 1;
        assert_eq!(linked.locate(plus), Some(("/p/a.ncl".to_string(), 2, 1)));
        let x = linked.text.rfind("{ x = a }").unwrap();
        assert_eq!(linked.locate(x), Some(("/p/scene.ncl".to_string(), 2, 1)));
    }
}
