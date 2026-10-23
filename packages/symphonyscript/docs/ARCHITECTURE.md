# SymphonyScript Architecture

This document describes how the implementation realizes `SPEC.md`. The SPEC defines what programs mean; this document defines the crates, data structures and interfaces that compute it. A conflict between the two is a defect the owner resolves.

## 1. Crates

```
packages/symphonyscript/
  CLAUDE.md
  Cargo.toml
  docs/
  crates/
    core/        symphonyscript-core
    compiler/    symphonyscript
    tool/        symphonyscript-tool
    kernel/      symphonyscript-kernel
    fixtures/    symphonyscript-fixtures
  editors/
    vscode/
```

| Crate | Environment | Contents | Depends on |
|---|---|---|---|
| `symphonyscript-core` | `no_std` with `alloc` | values and types, the vocabulary and its builder, the Program, the runtime, the `Consumer` trait | `libm` |
| `symphonyscript` | `std` | the compiler, `compile`, `eval`, diagnostics, Rust code generation | core |
| `symphonyscript-tool` | `std` | the framework tool: CLI, watch loop, language server | compiler |
| `symphonyscript-kernel` | as synaptic-kernel | the `Consumer` for synaptic-kernel | core, synaptic-kernel |
| `symphonyscript-fixtures` | `std`, tests only | the example vocabulary of SPEC §1.3, sample projects | core |

Only `symphonyscript-core` runs on devices. Everything in it builds without `std`, and its float functions come from `libm` on every target, so desktop and device compute bit-identical values (SPEC §12.5).

## 2. The vocabulary

A framework author builds a vocabulary in Rust. The builder returns typed handles, which the framework's consumer keeps and matches against the Program.

```rust
let mut v = Vocabulary::builder("music");

let beats = v.unit_family("beats").base("beats").unit("bars", 4.0);
let wave = v.enumeration("Wave", ["Sine", "Saw", "Square"]);

let note = v.node("Note")
    .doc("A single pitched event inside a clip.")
    .attr::<i32>("pitch").default(60).default_attr()
    .attr::<f32>("velocity").default(1.0).doc("Loudness, 0 to 1.")
    .attr::<f64>("length").default(0.25).family(beats, "beats")
    .attr::<bool>("accent").default(false).flag('\'')
    .handle();

let clip = v.node("Clip")
    .top_level(Count::Any)
    .children(note, Count::Any)
    .attr::<f32>("gain").default(1.0)
    .handle();

let plays = v.wire("Plays")
    .connects(clip, clip)
    .cyclic()
    .attr::<i32>("transpose").default(0)
    .handle();

v.node("Session").top_level(Count::AtMost(1)).entry();
v.prelude("music.sym", include_str!("music.sym"));

let vocab = v.build()?;
```

- `attr::<T>` takes `i32`, `i64`, `f32`, `f64`, `bool`, fixed-length arrays of these (`[bool; 8]`) and `&'static str` for string attributes. `attr_enum(name, enum, default_variant)` declares an enum attribute. The default is checked against `T` by the Rust compiler.
- `Count` is `Exactly(n)`, `AtLeast(n)`, `AtMost(n)`, `Between(n, m)` or `Any`.
- `.doc(text)` attaches documentation to kinds, attributes, wires, enums and units. The language server shows it.
- `build()` validates the whole vocabulary at once and returns every error: duplicate names, a second default attribute on a kind, two attributes with one flag character, a reference to a kind of another vocabulary, a count whose minimum exceeds its maximum, a default that does not fit its unit family.
- A built vocabulary carries a stamp: its name and a 64-bit FNV-1a hash of its canonical content, identical on every target.

The prelude is SymphonyScript source. The compiler compiles it with the program; prelude diagnostics name the prelude file.

## 3. The Program

```rust
pub struct Program {
    pub vocabulary: VocabularyStamp,
    pub top_level: Vec<InstanceId>,
    pub instances: Vec<Instance>,
    pub wires: Vec<Wire>,
    pub inputs: Vec<Input>,
    pub cells: Vec<Cell>,
}

pub struct Instance {
    pub kind: NodeKind,
    pub path: Path,
    pub span: Span,
    pub attrs: Vec<(AttrId, Binding)>,
    pub gate: Option<Binding>,
    pub sequence: Vec<Entry>,
}

pub enum Entry {
    Child(InstanceId),
    Bundle(Vec<InstanceId>),
    Wire(WireId),
}

pub struct Wire {
    pub kind: WireKind,
    pub path: Path,
    pub span: Span,
    pub source: InstanceId,
    pub target: Target,
    pub attrs: Vec<(AttrId, Binding)>,
    pub gate: Option<Binding>,
}

pub enum Target {
    Fixed(InstanceId),
    Select { candidates: Vec<InstanceId>, index: Binding },
}

pub enum Binding {
    Const(Value),
    Input(InputSlot),
    Cell(CellId),
    Array(Vec<Binding>),
}

pub struct Input {
    pub name: String,
    pub ty: Ty,
    pub slots: Range<InputSlot>,
    pub initial: Vec<Value>,
    pub span: Span,
}

pub struct Cell {
    pub ty: Ty,
    pub expr: Expr,
    pub span: Span,
}
```

- `instances[i]` has `InstanceId(i)`; the same holds for wires, cells and input slots.
- An array input occupies one slot per element, so element writes address single slots. An array attribute binds `Binding::Array`.
- `cells` is in dependency order: a cell reads only inputs and cells with lower indices.
- `Expr` is a tree of constants, input and cell reads, typed unary and binary operators, `if`, `match`, `Pick` (indexing with a runtime index, clamped), conversions and built-in calls. Every operator node carries its operand type. Functions, patterns, units and compile-time values are already resolved; nothing in `Expr` names a source declaration.
- `Value` is `I32`, `I64`, `F32`, `F64`, `Bool`, `Enum(EnumId, u16)` or `Str` (string attributes only).
- `Span` is a file and byte range, used in diagnostics and tooling.

`Program` implements structural equality, so two Programs compiled from the same sources compare equal.

## 4. The runtime

```rust
let mut rt = Runtime::new(program);
let cutoff = rt.input("cutoff").unwrap();
let steps = rt.input("steps").unwrap();

rt.write(cutoff, Value::F32(0.7))?;
rt.write_element(steps, 3, Value::Bool(true))?;

for change in rt.commit() {
    match change {
        Change::Attr { instance, attr, element } => { let v = rt.attr(instance, attr, element); }
        Change::WireAttr { wire, attr, element } => { let v = rt.wire_attr(wire, attr, element); }
        Change::Active(instance) => { let on = rt.active(instance); }
        Change::WireActive(wire) => { let on = rt.wire_active(wire); }
        Change::Target(wire) => { let to = rt.target(wire); }
    }
}
```

- `Runtime::new` evaluates every cell and computes every activation and target. It allocates every buffer the runtime ever uses.
- `commit` never allocates. It walks the cells in index order, evaluating only cells marked dirty, and marks readers dirty when a value changes bit-for-bit. The iterator it returns borrows the runtime's change buffer, whose capacity is the number of reportable items.
- Each input, cell and gate has a precomputed list of the attributes, gates and targets that read it, so changes are reported without searching.
- Effective activation is computed parents first, so a gate change reaches every descendant in one pass.
- `Expr` is evaluated by a tree-walking interpreter.
- `write` and `write_element` return an error for a value of the wrong type and for a dead handle. Nothing in the runtime panics.
- The runtime is `Send` and is used from one thread. Consumers move changes to other threads themselves.

## 5. Reload

```rust
match rt.reload(new_program) {
    Ok(report) => {
        for item in report.structure() {
            match item {
                Structural::Kept { old, new } => {}
                Structural::Added(new) => {}
                Structural::Removed(old) => {}
                Structural::Reordered(parent) => {}
            }
        }
        for change in report.changes() {}
    }
    Err(Refused::Vocabulary) => {}
}
```

`structure()` covers instances and wires (`Item::Instance` and `Item::Wire`). `reload`:

1. Refuses a Program with a different vocabulary stamp.
2. Carries input values over by name, following SPEC §13.2. Each input records whether the host has written it.
3. Matches instances and wires following SPEC §13.3: a path map for named items, then, per parent sequence, a longest common subsequence over anonymous entries keyed by kind and constant attributes, then in-gap pairing.
4. Builds the new state and evaluates every cell.
5. Reports the structure and the attribute, activation and target differences of matched items.

Reload may allocate. Input handles are stable: the runtime keeps a registry from input names to handles across reloads, adds handles for new names, and marks handles of removed inputs dead.

## 6. The Consumer trait

```rust
pub trait Consumer {
    fn load(&mut self, program: &Program, runtime: &Runtime);
    fn apply(&mut self, changes: Changes<'_>, runtime: &Runtime);
    fn reload(&mut self, report: &ReloadReport, runtime: &Runtime);
}
```

A consumer turns a Program into something: a synaptic-kernel graph, a MIDI file, a test trace, a printed dump. The core helpers (`eval`, the tool's watch loop) drive any consumer through this trait. A consumer checks the Program's vocabulary stamp against the vocabulary it was built for in `load` and `reload`.

## 7. The compiler

```rust
pub fn compile(sources: &[Source], vocab: &Vocabulary) -> Result<Compiled, Failed>;
pub fn eval(source: &str, vocab: &Vocabulary) -> Result<Runtime, Failed>;

pub struct Source { pub path: String, pub text: String }
pub struct Compiled { pub program: Program, pub warnings: Vec<Diagnostic> }
pub struct Failed { pub diagnostics: Vec<Diagnostic> }
```

The compiler performs no file I/O; the caller supplies every source. This keeps the compiler usable in editors, build scripts and WebAssembly. Stages:

1. **Lex.** Layout tokens (SPEC §2.2), literals, keywords, unit suffixes looked up in the vocabulary. Every byte belongs to a token or to trivia (whitespace and comments).
2. **Parse.** A lossless syntax tree that reproduces the source byte for byte, with a typed view over it. The parser recovers from errors: it records a diagnostic, skips to a synchronization point (the next line at the same or lower indentation) and continues, so a broken file still yields a tree. A placement whose kind is a wire kind of the vocabulary parses as a wire placement.
3. **Collect.** The top-level namespace across all files, the prelude and the vocabulary, with duplicate detection.
4. **Expand.** Calls to non-recursive functions are inlined, calls to recursive functions are evaluated to values (SPEC §6.3), patterns are expanded and `for` loops unrolled. Structural values are computed as expansion reaches them, in the types SPEC §4.6 fixes. Every expanded item keeps the spans of its use and of its definition.
5. **Resolve.** Names, paths, `to`, enum variants; each value is classified as compile-time or reactive by what it reads.
6. **Infer.** Types and unit families, from uses back to sources (SPEC §4).
7. **Check.** The vocabulary checks (SPEC §10.2), cycles among cells and wires, fit checks, and the reachability warning.
8. **Lower.** Constant folding, the cell graph in dependency order, paths, filled defaults, the Program.

Diagnostics carry a stable code in kebab-case (`unknown-attribute`, `count-exceeded`), a primary span, secondary spans (for a pattern use: the use and the line in the pattern), a plain-language message, and a hint when one exists. The compiler collects every diagnostic of a run instead of stopping at the first.

Each change recompiles the whole project.

## 8. Delivery

- **Library.** A host calls `compile` and `Runtime::new` in its own process. Edits reload live. `eval` compiles and loads a single source in one call.
- **Generated Rust.** A framework's `build.rs` calls `symphonyscript::codegen::generate(src_dir, &vocab, out_dir)`, which writes `program.rs` containing `pub fn program() -> Program`. The firmware or application includes it and never contains the compiler. Generated builds do not reload.

A differential test compiles every fixture project both ways and requires equal Programs and equal commit traces.

## 9. Tooling

A framework ships one binary, built from its vocabulary:

```rust
fn main() {
    symphonyscript_tool::Tool::new(music::vocabulary())
        .command("play", music::play)
        .command("export-midi", music::export_midi)
        .run()
}
```

| Subcommand | Provided by | Does |
|---|---|---|
| `new <name>` | tool | creates a project: `symphony.toml` and `src/` with a starter file |
| `check` | tool | compiles and prints diagnostics |
| `dump` | tool | prints the Program |
| `lsp` | tool | runs the language server |
| any other | framework | playing, rendering, exporting |

- **Single files.** Every subcommand accepts a lone `.sym` file in place of a project.
- **Watch.** `watch(project, vocab, consumer)` recompiles on save, reloads the runtime and drives the consumer. Frameworks call it from their own commands.
- **`symphony.toml`** holds `tool`, the name of the framework's binary, and `source`, the source root (default `src`).
- **Editor.** One VS Code extension: a static syntax grammar plus a language client that reads `symphony.toml` and starts `<tool> lsp`. Other editors use their own language-client configuration.
- **Language server.** Diagnostics; completion of attributes after `|`, accepted child kinds in a body, valid destinations after a wire's `:`, enum variants where an enum is expected; hover with documentation and inferred types with their origin; go to definition; find references; rename of instances, patterns, functions and inputs; an outline of top-level items; semantic tokens.

## 10. The synaptic-kernel adapter

`symphonyscript-kernel` implements `Consumer` for synaptic-kernel. It is generic over vocabularies: it maps structure and values, and the engine on the kernel's consumer thread interprets kinds.

### 10.1 Structure

| Program | Kernel |
|---|---|
| Instance | a node; `kind` is the vocabulary's node-kind index |
| Wire | a synapse; `kind` is the wire-kind index |
| An instance's sequence | a sub-chain of its children in sequence order, reached from the instance through a `contains` synapse |
| Bundle | a bundle node in the parent's chain; its members form a sub-chain behind another `contains` synapse |
| Wire's position in a sequence | a wire-point node at that position in the chain; the wire's synapse goes from it to the destination |
| Destination chosen at runtime | one synapse per candidate, each with an active flag |
| Top-level instances | a sub-chain after a root sentinel node, in top-level order |
| Entry | the sentinel's slot, written to `mem_metadata` once at load and never changed |

`kind` is 8 bits. The adapter reserves three node kinds (sentinel, bundle and wire point) and one synapse kind (`contains`), so a vocabulary used with the adapter has at most 253 node kinds and 255 wire kinds. `load` refuses a larger vocabulary.

The engine enters at the sentinel on every cycle, walks the top-level chain, and follows `contains` synapses into sequences.

### 10.2 Values

Each node and synapse holds the adapter's own cells, followed by attribute cells:

- **Adapter cells**, always in meta: the active flag and a serial number unique to the instance's lifetime. The engine keys its per-instance state (playheads, sounding notes, phases) by serial, because a freed slot is reused by another instance.
- **Attribute cells**, placed by type:

```rust
fn plane(ty: &Ty) -> Plane {
    match ty {
        Ty::Array(..) => Plane::Meta,
        Ty::I64 | Ty::F64 if !CELL64 => Plane::Meta,
        _ => Plane::Attr,
    }
}
```

A value that fits one cell is untorn in the attribute plane and needs no publish. A value spanning several cells goes to meta, where `publish()` makes it visible all at once. The adapter computes a layout table from the vocabulary once, giving every attribute of every kind a plane and an offset. The engine reads through the same table, so the two never disagree.

`apply` writes each change to its location and calls `publish()` only when a change landed in meta. The guarantee to engines: every single value is consistent; meta-plane values are consistent with each other at every publish; independent attribute-plane values changed by one commit may be seen mixed for one engine cycle.

Strides are uniform per store: the meta stride and the attribute stride are the largest any kind needs.

### 10.3 Capacity and reload

`load` sizes the kernel from the Program's exact counts plus headroom for live editing. `reload` inserts added nodes, removes removed ones, writes changes and publishes. A reload that outgrows capacity calls `grow`. Reordered and kept nodes move to their new chain positions through the kernel's move operations; where the kernel has none, a move is a remove and an insert that keeps the instance's serial. Kernel move operations are an approved addition on the condition that they preserve the kernel's invariants.
