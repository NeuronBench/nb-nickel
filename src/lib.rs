//! Nickel evaluation for NeuronBench.
//!
//! The browser has no filesystem, and `nickel-lang-core` resolves `import`
//! statements through one. This crate therefore *links* a program before
//! evaluating it: every `import "path"` is hoisted into a `let` binding whose
//! body is the imported file's text, in dependency order, and a line map is
//! kept so that diagnostics point back at the original file and line.
//!
//! The same linked text is then evaluated in-process, natively or in WASM,
//! with optional field overrides (used for slider parameters).
//!
//! The `nb` prelude (schema contracts plus helpers such as `nb.Slider`) is
//! prepended to every program, so user code can write `{...} | nb.Channel`
//! without importing anything.
//!
//! Who fetches sources is up to the caller: [`plan`] reports which imports
//! are still missing, nb-sim fetches them over HTTP, nb-site with reqwest,
//! and [`fs::link`] reads them from disk for native tools and tests.

pub mod eval;
pub mod link;

#[cfg(not(target_arch = "wasm32"))]
pub mod fs;

pub use eval::{evaluate_check, evaluate_json, read_params, Diagnostic, DiagnosticLabel, Override, ParamSpec};
pub use link::{plan, resolve_path, scan_imports, ImportSite, LinkError, Linked, Plan};

/// Hand-written helpers available as `nb.Nullable`, `nb.Tagged`, `nb.Slider`, ...
pub const PRELUDE_HELPERS: &str = include_str!("../prelude/helpers.ncl");

/// Generated schema contracts (`nb.Scene`, `nb.Channel`, ...).
///
/// The source of truth is `src/serialize.rs` in nb-sim. Regenerate with
/// `cargo run --bin gen_nickel_schema` there, which writes this file in a
/// sibling checkout; nb-sim's test suite fails when the two disagree.
pub const PRELUDE_SCHEMA: &str = include_str!("../prelude/schema.ncl");
