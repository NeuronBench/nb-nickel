# nb-nickel

[Nickel](https://nickel-lang.org) evaluation for NeuronBench. This crate is
embedded in the nb-sim simulator (native and WASM) and in the nb-site server,
so the editor preview and the server agree byte-for-byte on how a scene file
is evaluated.

What it adds on top of `nickel-lang-core`:

- **Linking without a filesystem.** `plan(root, sources)` reports which
  imports are still missing; once the caller has fetched them (over HTTP in
  the browser, from disk with `fs::link`), every `import "x.ncl"` is hoisted
  into a `let` binding and the program evaluates as one text. A line map
  turns Nickel's byte offsets back into the user's file, line, and column.
- **The `nb` prelude.** Every program sees `nb.Scene`, `nb.Neuron`,
  `nb.Membrane`, `nb.Channel`, `nb.Synapse`, ... as contracts, generated from
  the simulator's serde types, plus helpers: `nb.Slider { min, max }`,
  `nb.Nullable C`, and `nb.Tagged { A = {...}, B = {...} }` for
  `{ type = 'A, ... }` records.
- **Structured diagnostics.** Parse, type, contract, and evaluation errors
  come back as `Diagnostic` values with locations in user files.
- **Parameter discovery.** `read_params` lists the numeric fields of a
  top-level `params` record with their doc, default, and slider range;
  `Override::number` re-evaluates with a new value.

```rust
let linked = nb_nickel::fs::link("scene.ncl")?;
let json = nb_nickel::evaluate_json(&linked, &[], None)?;
let sliders = nb_nickel::read_params(&linked)?;
```

## Schema

`prelude/schema.ncl` is generated. The source of truth is `src/serialize.rs`
in nb-sim; run `cargo run --bin gen_nickel_schema` there with this repository
checked out as a sibling, then commit the result here. nb-sim's tests fail
when its types and this file disagree.

## Gotchas

Nickel record fields are recursively scoped: inside `{ neuron = neuron }` the
right-hand `neuron` is the field itself. Bind with a different name. Nickel is
also lazy, so a contract on a library entry that nothing uses is never
checked; export the whole file to force it.
