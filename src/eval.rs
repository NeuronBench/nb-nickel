//! Evaluate a linked program, with structured diagnostics and parameter discovery.

use nickel_lang_core::error::{Error as NickelError, IntoDiagnostics};
use nickel_lang_core::eval::cache::CacheImpl;
use nickel_lang_core::program::{Program, ProgramBuilder};
use nickel_lang_core::serialize::{to_string, ExportFormat};
use nickel_lang_core::term::MergePriority;
use serde::{Deserialize, Serialize};

use crate::link::Linked;

/// The name the linked program is registered under inside Nickel's file database.
const MAIN_NAME: &str = "<nb-main>";

/// A field override applied on top of the program, `path = value`, where
/// `value` is Nickel source text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Override {
    pub path: String,
    pub value: String,
}

impl Override {
    /// Override a numeric parameter such as `params.g_na`.
    pub fn number(path: impl Into<String>, value: f64) -> Self {
        let value = if value.is_finite() { value } else { 0.0 };
        Override { path: path.into(), value: format!("{value}") }
    }
}

/// One source location attached to a diagnostic.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticLabel {
    pub primary: bool,
    /// The original file name (URL or path) as the user wrote it, or a
    /// Nickel-internal name such as `<stdlib>`.
    pub file: String,
    /// 1-based line, 0 when unknown.
    pub line: usize,
    /// 1-based column, 0 when unknown.
    pub column: usize,
    pub message: String,
}

/// A parse, type, contract, or evaluation error, mapped back to source files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: String,
    pub message: String,
    pub notes: Vec<String>,
    pub labels: Vec<DiagnosticLabel>,
}

impl Diagnostic {
    /// A diagnostic that has no source position, e.g. a fetch failure.
    pub fn plain(message: impl Into<String>) -> Self {
        Diagnostic {
            severity: "error".to_string(),
            message: message.into(),
            notes: Vec::new(),
            labels: Vec::new(),
        }
    }

    /// The most useful location, as `(file, line, column)`: the first label
    /// in one of the user's files (preferring the primary one), falling back
    /// to internal files such as the prelude or the standard library.
    pub fn primary_location(&self) -> Option<(&str, usize, usize)> {
        let is_user = |l: &&DiagnosticLabel| !l.file.starts_with('<');
        self.labels
            .iter()
            .filter(is_user)
            .find(|l| l.primary)
            .or_else(|| self.labels.iter().find(is_user))
            .or_else(|| self.labels.iter().find(|l| l.primary))
            .or(self.labels.first())
            .map(|l| (l.file.as_str(), l.line, l.column))
    }

    /// A compact single-string rendering, `file:line:col: message`.
    pub fn render(&self) -> String {
        let mut out = String::new();
        match self.primary_location() {
            Some((file, line, col)) if line > 0 => out.push_str(&format!("{file}:{line}:{col}: ")),
            Some((file, _, _)) => out.push_str(&format!("{file}: ")),
            None => {}
        }
        out.push_str(&self.message);
        for label in &self.labels {
            if !label.message.is_empty() {
                out.push_str(&format!("\n  {}:{}:{}: {}", label.file, label.line, label.column, label.message));
            }
        }
        for note in &self.notes {
            out.push_str(&format!("\n  = {note}"));
        }
        out
    }
}

fn build_program(linked: &Linked, overrides: &[Override]) -> Result<Program<CacheImpl>, Vec<Diagnostic>> {
    let mut program = ProgramBuilder::new()
        .add_source_string(linked.text.clone(), MAIN_NAME)
        .build::<CacheImpl>()
        .map_err(|e| vec![Diagnostic::plain(format!("could not build program: {e:?}"))])?;
    let mut parsed = Vec::with_capacity(overrides.len());
    for o in overrides {
        let assignment = format!("{}={}", o.path, o.value);
        let fo = program
            .parse_override(assignment.clone(), MergePriority::Top)
            .map_err(|e| vec![Diagnostic::plain(format!("bad override `{assignment}`: {e:?}"))])?;
        parsed.push(fo);
    }
    program.add_overrides(parsed);
    Ok(program)
}

fn to_diagnostics(linked: &Linked, program: &Program<CacheImpl>, err: NickelError) -> Vec<Diagnostic> {
    let mut files = program.files();
    err.into_diagnostics(&mut files)
        .into_iter()
        .map(|d| {
            let labels = d
                .labels
                .iter()
                .map(|l| {
                    let name = files.name(l.file_id).to_string_lossy().to_string();
                    let (file, line, column) = if name == MAIN_NAME {
                        linked
                            .locate(l.range.start)
                            .unwrap_or_else(|| ("<nb>".to_string(), 0, 0))
                    } else {
                        let loc = files.location(l.file_id, l.range.start as u32).ok();
                        (
                            name,
                            loc.map(|x| x.line.to_usize() + 1).unwrap_or(0),
                            loc.map(|x| x.column.to_usize() + 1).unwrap_or(0),
                        )
                    };
                    DiagnosticLabel {
                        primary: matches!(l.style, nickel_lang_core::error::LabelStyle::Primary),
                        file,
                        line,
                        column,
                        message: l.message.clone(),
                    }
                })
                .collect();
            Diagnostic {
                severity: format!("{:?}", d.severity).to_lowercase(),
                message: d.message.clone(),
                notes: d.notes.clone(),
                labels,
            }
        })
        .collect()
}

/// Fully evaluate the program (or one of its fields, when `field` is given)
/// and export the result as JSON text.
pub fn evaluate_json(linked: &Linked, overrides: &[Override], field: Option<&str>) -> Result<String, Vec<Diagnostic>> {
    let mut program = build_program(linked, overrides)?;
    if let Some(field) = field {
        program.field = program
            .parse_field_path(field.to_string())
            .map_err(|e| vec![Diagnostic::plain(format!("bad field path `{field}`: {e:?}"))])?;
    }
    let value = match program.eval_full_for_export() {
        Ok(v) => v,
        Err(e) => return Err(to_diagnostics(linked, &program, e)),
    };
    to_string(ExportFormat::Json, &value)
        .map_err(|e| vec![Diagnostic::plain(format!("could not export result as JSON: {e:?}"))])
}

/// Fully evaluate the program without exporting it. Use this to validate a
/// file that may evaluate to a function or another non-serializable value:
/// every field and contract is still forced.
pub fn evaluate_check(linked: &Linked, overrides: &[Override]) -> Result<(), Vec<Diagnostic>> {
    let mut program = build_program(linked, overrides)?;
    match program.eval_full() {
        Ok(_) => Ok(()),
        Err(e) => Err(to_diagnostics(linked, &program, e)),
    }
}

/// A tunable parameter declared under the program's `params` field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamSpec {
    /// Field name inside `params`.
    pub name: String,
    /// Full override path, `params.<name>`.
    pub path: String,
    pub default: f64,
    pub doc: Option<String>,
    /// Present when the field carries an `nb.Slider { min, max }` contract.
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// Pretty-printed contract annotations, for display.
    pub contracts: Vec<String>,
}

fn parse_named_number(s: &str, key: &str) -> Option<f64> {
    let mut rest = s;
    while let Some(i) = rest.find(key) {
        let after = &rest[i + key.len()..];
        let after = after.trim_start();
        if let Some(after) = after.strip_prefix('=') {
            let after = after.trim_start();
            let end = after
                .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E'))
                .unwrap_or(after.len());
            if let Ok(v) = after[..end].parse::<f64>() {
                return Some(v);
            }
        }
        rest = &rest[i + key.len()..];
    }
    None
}

/// Discover numeric parameters under the program's top-level `params` record.
///
/// Returns an empty list when the program is not a record or has no `params`
/// field. Non-numeric fields of `params` are ignored.
pub fn read_params(linked: &Linked) -> Result<Vec<ParamSpec>, Vec<Diagnostic>> {
    // Metadata (doc, contracts) comes from the record spine, which keeps
    // field annotations intact without forcing values.
    let mut program = build_program(linked, &[])?;
    let spine = match program.eval_record_spine() {
        Ok(v) => v,
        Err(e) => return Err(to_diagnostics(linked, &program, e)),
    };
    let Some(root) = spine.as_record().and_then(|c| c.into_opt()) else { return Ok(Vec::new()) };
    let Some(params_field) = root.fields.iter().find(|(id, _)| id.label() == "params") else {
        return Ok(Vec::new());
    };
    let Some(params_value) = params_field.1.value.as_ref() else { return Ok(Vec::new()) };
    let Some(params) = params_value.as_record().and_then(|c| c.into_opt()) else { return Ok(Vec::new()) };

    let mut specs: Vec<ParamSpec> = params
        .fields
        .iter()
        .map(|(id, field)| {
            let name = id.label().to_string();
            let (doc, contracts) = match field.metadata.0.as_ref() {
                Some(m) => (
                    m.doc.as_ref().map(|d| d.to_string()),
                    m.annotation
                        .contracts
                        .iter()
                        .map(|c| format!("{}", c.typ))
                        .collect::<Vec<_>>(),
                ),
                None => (None, Vec::new()),
            };
            let slider = contracts.iter().find(|c| c.contains("Slider"));
            ParamSpec {
                path: format!("params.{name}"),
                name,
                default: 0.0,
                doc,
                min: slider.and_then(|c| parse_named_number(c, "min")),
                max: slider.and_then(|c| parse_named_number(c, "max")),
                contracts,
            }
        })
        .collect();

    // Values come from a full evaluation of the `params` field.
    let json = evaluate_json(linked, &[], Some("params"))?;
    let values: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| vec![Diagnostic::plain(format!("params did not export as JSON: {e}"))])?;
    specs.retain_mut(|spec| match values.get(&spec.name).and_then(|v| v.as_f64()) {
        Some(v) => {
            spec.default = v;
            true
        }
        None => false,
    });
    Ok(specs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::{plan, Plan};
    use std::collections::HashMap;

    fn link(files: &[(&str, &str)]) -> Linked {
        let sources: HashMap<String, String> =
            files.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        match plan(files[0].0, &sources).unwrap() {
            Plan::Ready(l) => l,
            Plan::NeedSources(m) => panic!("missing {m:?}"),
        }
    }

    #[test]
    fn evaluates_with_prelude_and_imports() {
        let linked = link(&[
            ("/p/main.ncl", "let lib = import \"lib.ncl\" in { total = lib.x + 1, ok = nb.Nullable Number }"),
            ("/p/lib.ncl", "{ x = 41 }"),
        ]);
        let json = evaluate_json(&linked, &[], Some("total")).unwrap();
        assert_eq!(json.trim(), "42");
    }

    #[test]
    fn params_are_discovered_and_overridable() {
        let linked = link(&[(
            "/p/main.ncl",
            r#"{
  params = {
    g | nb.Slider { min = 0, max = 2 } | doc "gain" | default = 0.5,
    n | Number | default = 3,
    label | String | default = "not a slider",
  },
  out = params.g * params.n,
}"#,
        )]);
        let specs = read_params(&linked).unwrap();
        assert_eq!(specs.len(), 2);
        let g = specs.iter().find(|s| s.name == "g").unwrap();
        assert_eq!((g.default, g.min, g.max, g.doc.as_deref()), (0.5, Some(0.0), Some(2.0), Some("gain")));
        let n = specs.iter().find(|s| s.name == "n").unwrap();
        assert_eq!((n.default, n.min, n.max), (3.0, None, None));

        let json = evaluate_json(&linked, &[Override::number("params.g", 2.0)], Some("out")).unwrap();
        assert_eq!(json.trim(), "6");

        let err = evaluate_json(&linked, &[Override::number("params.g", 7.0)], Some("out")).unwrap_err();
        assert!(err[0].message.contains("contract broken"), "{:?}", err);
    }

    #[test]
    fn diagnostics_point_into_imported_file() {
        let linked = link(&[
            ("/p/main.ncl", "let lib = import \"lib.ncl\" in\nlib.x"),
            ("/p/lib.ncl", "{\n  x = 1 + \"boom\",\n}"),
        ]);
        let diags = evaluate_json(&linked, &[], None).unwrap_err();
        let (file, line, _col) = diags[0].primary_location().unwrap();
        assert_eq!((file, line), ("/p/lib.ncl", 2));
    }

    #[test]
    fn tagged_records_export_like_serde() {
        let linked = link(&[(
            "/p/main.ncl",
            r#"let TC = nb.Tagged { Gaussian = { type | Dyn, sigma | Number }, Instantaneous = { type | Dyn } } in
{ a | TC = { type = 'Gaussian, sigma = 2 }, b | TC = { type = "Instantaneous" } }"#,
        )]);
        let json: serde_json::Value = serde_json::from_str(&evaluate_json(&linked, &[], None).unwrap()).unwrap();
        assert_eq!(json["a"]["type"], "Gaussian");
        assert_eq!(json["b"]["type"], "Instantaneous");
    }
}
