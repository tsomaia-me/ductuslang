# SymphonyScript Architecture

This document describes how the implementation realizes `SPEC.md`. The SPEC defines what programs mean; this document defines the crates, data structures and interfaces that compute it. A conflict between the two is a defect the owner resolves.

## 1. Crates

```text
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
    jetbrains/
```

| Crate | Environment | Contents | Depends on |
|---|---|---|---|
| `symphonyscript-core` | `no_std` with `alloc` | values and types, the vocabulary and its builder, the Program, the runtime, the `Consumer` trait | `libm` |
| `symphonyscript` | `std` | the compiler, `compile`, `eval`, diagnostics, Rust code generation | core |
| `symphonyscript-tool` | `std` | the framework tool: CLI, watch loop, language server | compiler |
| `symphonyscript-kernel` | `std` | the `Consumer` for synaptic-kernel | core, synaptic-kernel |
| `symphonyscript-fixtures` | `std`, tests only | the example vocabulary of SPEC §1.3, the fixture projects (PLAN M1 and M3) | core |

Only `symphonyscript-core` runs on devices. Everything in it builds without `std`, and its float functions come from `libm` on every target, so desktop and device compute bit-identical values (SPEC §12.5).

## 2. The vocabulary

A framework author builds a vocabulary in Rust. `v.node(name)` and `v.wire(name)` return a kind's typed handle at once, and `v.kind(handle)` and `v.wire_kind(handle)` configure it, so a kind can accept itself or a kind declared after it. The framework's consumer keeps the handles and matches them against the Program. A handle compares, hashes and looks up by its kind's index alone, so two builds of one vocabulary give equal handles; the vocabulary stamp tells vocabularies apart, and only `build()` rejects a handle from another builder (SPEC §10.1). A part of the SPEC §1.3 vocabulary:

```rust
let mut v = Vocabulary::builder("music");

let beats = v.unit_family("beats").base("beats").unit("bars", 4.0);
let wave = v.enumeration("Wave", ["Sine", "Saw", "Square"]);

let note = v.node("Note");
let clip = v.node("Clip");
let track = v.node("Track");
let osc = v.node("Osc");
let session = v.node("Session");
let plays = v.wire("Plays");
let starts = v.wire("Starts");

v.kind(note)
    .doc("A single pitched event inside a clip.")
    .attr::<i32>("pitch").default(60).default_attr()
    .attr::<f32>("velocity").default(1.0).doc("Loudness, 0 to 1.")
    .attr::<f32>("brightness").default(0.5)
    .attr::<f64>("length").default(0.25).family(beats, "beats")
    .attr::<bool>("accent").default(false).flag('\'');

v.kind(clip)
    .top_level(Count::Any)
    .children(note, Count::Any)
    .attr::<f32>("gain").default(1.0)
    .attr::<f32>("level").default(1.0);

v.kind(track)
    .top_level(Count::Any)
    .attr::<&'static str>("label").default("");

v.kind(osc)
    .top_level(Count::Any)
    .attr::<f64>("freq").default(440.0).default_attr()
    .attr_enum("wave", wave, "Sine")
    .attr::<f32>("rate").default(1.0);

v.kind(session)
    .top_level(Count::AtMost(1))
    .entry()
    .children(track, Count::Any)
    .attr::<f64>("tempo").default(120.0).default_attr();

v.wire_kind(plays)
    .connects(clip, clip)
    .cyclic()
    .attr::<f32>("gain").default(1.0).default_attr()
    .attr::<i32>("transpose").default(0);

v.wire_kind(starts)
    .connects(clip, clip)
    .connects(track, clip)
    .cyclic();

v.prelude("music.sym", include_str!("music.sym"));

let vocab = v.build()?;
```

- `attr::<T>` takes `u8`, `i32`, `i64`, `f32`, `f64`, `bool`, fixed-length arrays of these nested to any depth (`[bool; 8]`, `[[i32; 3]; 2]`), core's bounded list `Bounded<T, N>` for a variable array of capacity N (SPEC §3.7), whose elements may be fixed arrays and which no array holds, and `&'static str` for string attributes. The Rust compiler checks the default against `T`. `attr_enum(name, enum, default_variant)` declares an enum attribute, and `attr_enum_array(name, enum, length, default_variant)` an array of one.
- `v.native(name, signature, function)` declares a native function (SPEC §10.1). `function` reads its arguments and writes its result through views of core that borrow the runtime's buffers, so a call allocates nothing. The stamp's serialization holds each native function's name and signature, never its code.
- `Count` is `Exactly(n)`, `AtLeast(n)`, `AtMost(n)`, `Between(n, m)` or `Any`.
- `.children(kind, count)` accepts a child kind. Each `.connects(from, to)` adds a source and destination pair to a wire kind. `.incoming(wire, count)` and `.outgoing(wire, count)` bound how many wires of one kind a node kind takes; a kind that states no bound takes any number.
- `.doc(text)` attaches documentation to kinds, attributes, wires, enums and units. The language server shows it.
- `build()` validates the whole vocabulary at once and returns every error SPEC §10.1 lists, except the prelude's, which the compiler reports (§7).
- A built vocabulary carries a stamp: its name and a 64-bit FNV-1a hash of a canonical serialization of its content, identical on every target. The serialization holds the kinds and wire kinds in declaration order, which fixes their indices, with their attributes (names, types, defaults, families, default units, flags and default-attribute marks), counts, connections, entry and cyclic marks; the enums and their variants; the unit families and scales; and the prelude's text with its line endings normalized to LF. Documentation is not part of it, so a `.doc` edit leaves the stamp unchanged.

The prelude's source is SymphonyScript. `build()` cannot compile it, because the compiler is not in core; the compiler handles it (§7).

## 3. The Program

```rust
pub struct Program {
    pub vocabulary: VocabularyStamp,
    pub top_level: Vec<InstanceId>,
    pub instances: Vec<Instance>,
    pub wires: Vec<Wire>,
    pub inputs: Vec<Input>,
    pub cells: Vec<Cell>,
    pub strings: Vec<String>,
}

pub struct Instance {
    pub kind: NodeKind,
    pub path: Path,
    pub span: Span,
    pub expanded_from: Vec<Span>,
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
    pub expanded_from: Vec<Span>,
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
    pub family: Option<FamilyId>,
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

- `instances[i]` has `InstanceId(i)`; the same holds for wires, cells and input slots. Instances, wires and inputs are in the order of SPEC §11.
- An array input occupies one slot per element, numbered row by row (SPEC §12.1), so element writes address single slots. An array attribute binds `Binding::Array`.
- `cells` is in dependency order: a cell reads only inputs and cells with lower indices.
- An `AttrId` indexes its kind's attribute list.
- `gate` combines the item's own `when` with the `when` arm conditions around it in one binding (SPEC §11).
- `Select` keeps the candidates in the order of SPEC §8.2; `index` is an integer binding, clamped to them.
- `Expr` is a list of nodes: constants, input and cell reads, typed unary and binary operators, `if`, `match`, `Pick` (indexing with a runtime index, clamped), conversions and built-in calls. A node's operands come before it in the list, so one node may be the operand of several, and equality, hashing, printing and dropping go over the list once. Every operator node carries its operand type, and every read of a cell that its use reads in another type carries an explicit conversion (SPEC §4.4). Functions, patterns, units and compile-time values are already resolved; nothing in `Expr` names a source declaration.
- `Value` is `U8`, `I32`, `I64`, `F32`, `F64`, `Bool`, `Enum(EnumId, u16)` or `Str(StrId)`. A `StrId` indexes `strings`, which holds each string attribute value once.
- `Span` is a file, named by its `Source.path`, and a byte range, used in diagnostics and tooling. `span` is where an item is written; `expanded_from` holds the pattern uses that produced it, innermost first, and is empty outside patterns.

`Program` implements structural equality that compares floats by their bits, which the canonical NaN of SPEC §12.5 keeps stable, so two Programs compiled from the same sources compare equal.

## 4. The runtime

```rust
let mut rt = Runtime::new(program)?;
let cutoff = rt.input("cutoff").unwrap();
let steps = rt.input("steps").unwrap();

rt.write(cutoff, Value::F32(0.7))?;
rt.write_element(steps, 3, Value::Bool(true))?;

rt.commit();
for change in rt.changes() {
    match change {
        Change::Attr { instance, attr, element } => { let v = rt.attr(instance, attr, element); }
        Change::WireAttr { wire, attr, element } => { let v = rt.wire_attr(wire, attr, element); }
        Change::Active(instance) => { let on = rt.active(instance); }
        Change::WireActive(wire) => { let on = rt.wire_active(wire); }
        Change::Target(wire) => { let to = rt.target(wire); }
    }
}
```

- `Runtime::new` validates the Program (ids in range, cells in dependency order, operand types consistent) and returns an error for one that fails. A Program that passes it evaluates, commits and reloads without a panic. It then evaluates every cell, computes every activation and target, and allocates every buffer the runtime ever uses.
- `commit` never allocates. It walks the cells in index order, evaluating only cells marked dirty, and marks readers dirty when a value changes bit-for-bit. It fills the change buffer, whose capacity is the number of reportable items, in the order of SPEC §12.3. `changes` reads that buffer until the next commit.
- Each input, cell and gate has a precomputed list of the attributes, gates and targets that read it, so changes are reported without searching.
- Effective activation is computed parents first, so a gate change reaches every descendant in one pass.
- `Expr` is evaluated without recursion, each node at most once per evaluation, into buffers sized in `Runtime::new` for the largest expression, so no Program overflows the native stack and a shared node is computed once.
- `write`, `write_element` and `write_array` return an error for a write SPEC §12.1 rejects and for a dead handle. Nothing in the runtime panics.
- `program` returns the loaded Program.
- The runtime is `Send` and is used from one thread. Consumers move changes to other threads themselves.

## 5. Reload

```rust
let snapshot = rt.reload_snapshot();
match snapshot.prepare(new_program) {
    Ok(pending) => {
        let report = pending.report();
        for item in report.structure() {
            match item {
                Structural::Kept { old, new } => {}
                Structural::Added(new) => {}
                Structural::Removed(old) => {}
                Structural::Reordered(parent) => {}
            }
        }
        for write in report.dropped() {}
        for change in report.changes() {}
        rt.install(pending);
    }
    Err(Refused::Vocabulary | Refused::Invalid) => {}
}
```

`structure()` covers instances, bundles and wires (`Item::Instance`, `Item::Bundle` and `Item::Wire`); `Reordered` names a parent instance, a bundle or `Parent::Top`. `reload_snapshot` copies what preparing reads, the old Program by reference and the input values with their written marks, on the runtime's thread between commits; the snapshot is `Send`, so `prepare` runs on any thread while the runtime keeps committing. `prepare`:

1. Refuses a Program with a different vocabulary stamp (`Refused::Vocabulary`), or one that fails `Runtime::new`'s validation (`Refused::Invalid`).
2. Carries input values over by name, following SPEC §13.2. Each input element records whether the host has written it.
3. Matches instances, bundles and wires following SPEC §13.3, from the top level down: paths below matched parents for named items; then, per parent sequence, the common prefix and suffix, and Myers' difference algorithm over the keys of the remaining anonymous entries; then per-kind pairing in each gap.
4. Builds the new state and evaluates every cell.
5. Reports the structure, the staged writes it dropped, and the attribute, activation and target differences of matched items.

`install` replaces the running state with the pending one, between two commits, after staging again against the new Program, by input name, the writes staged and committed since the snapshot, as SPEC §13.2 does for writes staged before a reload, so the next commit applies them. `Runtime::with_limits(program, limits)` sets the reload search bound of SPEC §13.3, and `Runtime::new` uses the default. Dropping a `PendingReload` instead leaves the runtime on the old Program. Preparing may allocate; installing swaps buffers and does not. Input handles are stable: the runtime keeps a registry from input names to handles across reloads, adds handles for new names, marks handles of removed inputs dead, and revives a dead handle when an input of its name returns.

## 6. The Consumer trait

```rust
pub trait Consumer {
    type Error;
    fn load(&mut self, runtime: &Runtime) -> Result<(), Self::Error>;
    fn apply(&mut self, runtime: &Runtime);
    fn reload(&mut self, pending: &PendingReload) -> Result<(), Self::Error>;
}
```

A consumer turns a Program into something: a synaptic-kernel graph, a MIDI file, a test trace, a printed dump. It reads the Program through `runtime.program()` and each commit's changes through `runtime.changes()`. The tool's watch loop drives any consumer through this trait. `load` refuses a Program the consumer cannot take, such as one whose vocabulary stamp differs from the vocabulary the consumer was built for. `reload` reads the new Program and the report from the pending reload and either applies all of it or changes nothing and returns an error. The watch loop installs a pending reload only after the consumer's `reload` succeeds, so a refusal leaves the runtime and the consumer on the old Program, and the loop reports the error.

## 7. The compiler

```rust
pub struct Compiler { /* the vocabulary and its parsed, resolved prelude */ }

impl Compiler {
    pub fn new(vocab: &Vocabulary) -> Result<Compiler, Failed>;
    pub fn with_limits(vocab: &Vocabulary, limits: Limits) -> Result<Compiler, Failed>;
    pub fn compile(&self, sources: &[Source]) -> Result<Compiled, Failed>;
}

pub fn compile(sources: &[Source], vocab: &Vocabulary) -> Result<Compiled, Failed>;
pub fn eval(source: &str, vocab: &Vocabulary) -> Result<Runtime, Failed>;

pub struct Source { pub path: String, pub text: String }
pub struct Compiled { pub program: Program, pub warnings: Vec<Diagnostic> }
pub struct Failed { pub diagnostics: Vec<Diagnostic> }
```

`Compiler::new` parses and resolves the prelude once and reports its errors as vocabulary errors (SPEC §10.1). The tool and the language server keep one `Compiler`; `compile` and `eval` build one for a single call. `Source.path` is relative to the project's source root, with `/` separators. `Limits`, in `symphonyscript-core`, holds every resource limit of SPEC §4.6, §6.3 and §13.3: recursive calls, nesting depth, inlined calls, expansion steps, expanded items and the reload search bound. `Limits::default()` takes each default from one file of constants in core; `Compiler::new`, `compile` and `eval` use the defaults, and `Compiler::with_limits` takes others.

The compiler performs no file I/O; the caller supplies every source. This keeps the compiler usable in editors, build scripts and WebAssembly. Stages:

1. **Lex.** Layout tokens (SPEC §2.2), literals, keywords, unit suffixes looked up in the vocabulary. Every byte belongs to a token or to trivia (whitespace and comments).
2. **Parse.** A lossless syntax tree that reproduces the source byte for byte, with a typed view over it. Code outside the parser reaches the tree only through views that borrow the parse result, so the parse result is the tree's last owner and dismantles it without recursion. The parser recovers from errors (§7.1), so a broken file still yields a tree. A placement whose kind is a wire kind of the vocabulary parses as a wire placement.
3. **Collect.** The top-level namespace across all files, the prelude and the vocabulary, with duplicate detection and prelude shadowing; the call graph of functions, which marks recursive ones, and the use graph of patterns. States join the namespace, and the call graph marks bounded functions and the cycles of calls each recursive function lies on.
4. **Expand.** Pattern uses are expanded and `for` loops unrolled. A value that structure needs (a range bound, an iterable, a recursive call, and any argument or `let` they read) is resolved, typed as SPEC §4.6 says and evaluated when expansion reaches it; such a value that reads a cell is an error. Every expanded item keeps the spans of its uses and of its definition. A structural value never depends on a default unit, since SPEC §3.6 reads a plain value only in the unit of an attribute its family value reaches and structure takes plain numbers; the values of an iterable's unit elements are computed once Infer knows their units.
5. **Resolve.** Names, paths and `to` in the expanded program; calls to non-recursive functions are inlined with their parameters bound (SPEC §6.3); each argument is bound once and shared by every use of its parameter, so nested calls grow the program linearly; two equal calls that are the operands of one operator or the arms of one `if`, with the same names or paths as arguments, share one inlined body, since they meet the same expected type and feed the same attributes; SPEC §4.6's inline limit is found before any body is inlined, by counting each value's calls over the call graph once Expand is done, so a compilation past it costs no inlining; each value is classified as compile-time or reactive by what it reads. Cycles among declarations (SPEC §6.2) are found here, before Infer. A bare name that names no declaration is left for Infer to resolve as a variant. A call of a bounded function that reads a cell is expanded level by level as calls of non-recursive functions are inlined, a compile-time condition expanding only its chosen branch, and a call nested inside its bound yields its fallback; its levels count toward SPEC §4.6's inline limit as they expand, so its expansion stops at the limit.
6. **Infer.** Types and unit families over the whole program (SPEC §3.6, §4), bare variants from their expected types, and then the remaining calls to recursive functions, evaluated in the types inference gives them. Recursive calls are memoized by argument values, argument types and expected type, which is what SPEC §4.6's limit counts. A recursive call whose result is an array is evaluated in the element type its use expects, or with the defaults where it expects none, before that use is typed, and takes the shape of the value it yields; a reactive value whose shape comes from such a call alone takes the shape the call yields with the defaults, found when Resolve builds its bindings. A recursive call's kind and family follow from its body under its arguments' kinds and families, settled over the call graph before cell types are found. Exact compile-time integers never grow past SPEC §4.6's bound: an operation whose result would reach 2¹²⁷ reports the error before computing it, so `2 ** 1000000` costs no more than `2 ** 200`. Every array value's length or capacity follows SPEC §3.7, and the types and capacities of states are the least solution of SPEC §6.6's equations, found in time that does not depend on the capacities' values. A bounded function's compile-time calls are memoized with their nesting in their function's calls, which SPEC §4.6's recursive-call limit counts. A native call takes its signature's types; one whose arguments are compile-time values is evaluated by calling the function, its result counting as the compile-time value of the call.
7. **Check.** The vocabulary checks (SPEC §10.2), cycles among wires, fit checks, and the warnings of SPEC §4.4 and §10.2.
8. **Lower.** Constant folding with the canonical NaN, the cell graph in dependency order, paths, filled defaults, the string table, the Program. Lower folds and lowers each node of a shared `Expr` once, so a Program grows with its distinct nodes, not with the paths through them, and its output depends on the sources and the vocabulary alone, never on hash or allocation order.

Every diagnostic teaches. It carries a stable code in kebab-case (`unknown-attribute`, `count-exceeded`); a summary of one to three lines in the program's own names; a one-line help that states the fix, when one exists; a primary span; and related spans, each with a label (for a pattern use: the use and the line in the pattern; for an error inside an inlined function body: the calls, innermost first). Every code is a variant of the compiler's `Code` enum, whose `section` method returns the SPEC section of its rule and whose `lesson` method returns its lesson: a short plain-language document per code in `crates/compiler/lessons/` that states the rule, says why it exists, and shows a program that draws the code beside its fix, its `sym` and `sym-error` examples tested as the SPEC's are, the second drawing the lesson's code.

```rust
pub struct Diagnostic {
    pub code: Code,
    pub summary: String,
    pub help: Option<String>,
    pub span: Span,
    pub related: Vec<Related>,
}

pub struct Related { pub span: Span, pub label: String }
```

The tests keep a table mapping each error rule of the SPEC, by section and sentence, to its code; a test checks that every code appears in the table and every code in the table exists. A lookup that misses inside the compiler is a compiler defect: it returns an error, and the run draws one `internal-error` diagnostic, the one code no SPEC rule names and the table leaves out, and fails. The compiler collects every diagnostic of a run instead of stopping at the first. Diagnostics that differ only in the calls or pattern uses between their fault and the outermost call or use merge into one, which names the first such chain found, so a fault draws one diagnostic per outermost call or use however its bodies are shared. It returns them in source order: by file in top-level order, then by span start, span end and code. A name, type or unit family that an error leaves unknown takes an error placeholder that every later stage accepts without a diagnostic, so one fault draws one diagnostic; a placeholder arises only where a diagnostic is reported, so a program that draws no error holds none. A trial evaluation that settles a recursive call's kind (stage 6) is no part of the program: what it meets in code that nothing expands draws nothing (SPEC §6.5), and the evaluation of code that is expanded reports it. A top-level name declared twice is such a placeholder at every reference beside its one duplicate diagnostic, and so is a name that a region skipped by a fault (§7.1) holds, wherever it is used. A compilation that a limit stops reports that limit's one diagnostic and the diagnostics found before the stop, and no stage after the one it stops in runs: a stop during Expand reports no diagnostic of Infer or Check, and a diagnostic that depends on work the limit cut off, such as a flow's type whose uses lie past the cut, is not reported.

Each change recompiles the whole project.

No source overflows the native stack, however deeply it nests. Every stage, and every operation on the syntax tree and on the Program (building, walking, comparing, cloning, hashing, printing, validating and dropping), works without recursion that grows with the source's nesting, as the runtime evaluator does (§4).

### 7.1 Error recovery

The lexer and the parser record a diagnostic for a fault, skip the fault's region and continue. Recovery holds three properties:

- A valid source never reaches recovery, so its tree does not depend on it.
- Every fault outside the region of an earlier fault is reported exactly once, at its span.
- Recovery costs time and memory linear in the source.

The lexer recovers in place: a line with a dedent mismatch is laid out at the enclosing column, a line that misses a clause's column only by the display width of wide characters continues the clause, and an unterminated string ends at the end of its line, leaving the brackets opened before it open.

A fault's region is the rest of its line and the block indented under that line, except that the block of a construct that takes one parses as usual, also when the fault skips the `:` that ends the construct's line: a placement's body, a `for` or `when` block, a pattern's or a function's body, and a `match` block of a `let`, a function body or a wire's destination. Inside an unclosed bracket, where newlines are whitespace (SPEC §2.2), the region runs to the bracket's next resynchronization line, or without one to the bracket's closer and the end of that line. A bracket's lines are the non-blank lines of its region, and its column is the indentation in force where its outermost bracket opened.

An unclosed bracket ends before the first of its lines, at or left of its column, that cannot continue its innermost open bracket, and that line starts an item. The missing closer is a fault reported at the line's first token, unless the bracket already holds a fault, to which it then belongs:

- in any bracket, a line whose first word is `input`, `let`, `state`, `fn` or `pattern`;
- in an expression bracket (an array, a parenthesized expression, arguments, an index, a conversion or a history read), a line that begins with a name followed by a name or `as`;
- in a bundle, a line holding a `:` at the bundle's depth that closes no pending `for` header of the bundle.

A bracket holds a fault when a parser complaint falls inside it, its opener included, when an unterminated string does, or when it opens in the skipped rest of a faulty line; a lexer diagnostic on a token the parser accepts is no such fault. The parser then resynchronizes at the first of the bracket's lines that begins at or after the fault, sits at or left of the bracket's column, and starts an item:

- a line that begins with `input`, `let`, `state`, `fn`, `pattern`, `(`, `[`, `for` or `when`;
- at column 0, a line that begins with a name followed by a name, `as`, an adjacent flag or nothing;
- in a body, a line that begins with a name followed by a name, `as`, an adjacent flag, `|`, `when` or `:`; with a node or wire kind followed by `/`; or with another name directly followed by `/` and a value SPEC §7.5 allows there that ends the line or meets `|`, `when` or `:`, on a line that holds no closer at the bracket's outermost depth.

One fault draws one diagnostic:

- A logical line draws at most one parser diagnostic, a complaint that belongs to a lexer diagnostic included. A bracket that spans lines is one logical line until a bracket end or a resynchronization starts a new one.
- A parser complaint at an error token, starting inside a lexer diagnostic, or directly after an error token, an unterminated string or the first character of a line with a dedent mismatch belongs to that lexer diagnostic and is not reported.
- A missing closer reported at a resynchronization line stays reported beside a layout fault of that line, since the two are separate faults.
- A missing block counts against the line of the construct that lacks it, not the line where it is reported, so that line reports its own faults; it stays reported beside a fault of its construct's line, since the two are separate faults.
- An unexpected indented block is reported unless the line it sits under drew a parser complaint, and is skipped either way.

A diagnostic at a zero-width INDENT or NEWLINE takes the span of the next token that holds text; one at a DEDENT or at the end of input stays empty at its position. A missing block's diagnostic and an empty one mark no item as faulty, so the item after them is built as usual.

## 8. Delivery

- **Library.** A host calls `compile` and `Runtime::new` in its own process. Edits reload live. `eval` compiles and loads a single source in one call.
- **Generated Rust.** `symphonyscript::codegen::generate(sources, &vocab) -> Result<String, Failed>` returns the text of a Rust module containing `pub fn program() -> Program`. A framework's `build.rs` reads the sources, calls it and writes the module into `OUT_DIR`; the firmware or application includes it and never contains the compiler. The vocabulary lives in its own crate, used by the framework both as a dependency and as a build-dependency, because a build script cannot call the crate it builds. Generated builds do not reload.

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

fn play(project: &Project, args: &[String]) -> Result<(), ToolError> {
    symphonyscript_tool::watch(project, &music::vocabulary(), &mut music::Engine::new())
}
```

| Subcommand | Provided by | Does |
|---|---|---|
| `new <name>` | tool | creates a project: `symphony.toml` and `src/` with a starter file |
| `check` | tool | compiles and prints diagnostics |
| `dump` | tool | prints the Program |
| `lsp` | tool | runs the language server |
| any other | framework | playing, rendering, exporting |

- **Commands.** A framework command is `fn(&Project, &[String]) -> Result<(), ToolError>`, receiving the project and the command's arguments.
- **Projects.** A `Project` holds its root, the settings of its `symphony.toml`, and its sources: every `.sym` file under `source`, found recursively, each with its path relative to `source`. Those paths fix the top-level order (SPEC §2.1).
- **Single files.** `check`, `dump` and the framework's subcommands accept a lone `.sym` file in place of a project; it is a project of one source whose path is the file's name.
- **Watch.** `watch(project, vocab, consumer)` recompiles on save, prepares a reload, lets the consumer accept it, installs it, and drives the consumer (§5, §6). Frameworks call it from their own commands.
- **`symphony.toml`** holds `tool`, the name of the framework's binary, `source`, the source root (default `src`), and an optional `[limits]` table, each key a field of `Limits` (§7). Every tool command, the language server's included, takes `--limit <name>=<value>`; a flag overrides the file, and the file the default.
- **Editor.** A VS Code extension and a JetBrains plugin. Each holds a static syntax grammar, or for JetBrains the server's semantic tokens where its platform reads no grammar, and a language client that reads `symphony.toml` and starts `<tool> lsp`. For a lone `.sym` file each starts the tool its `symphonyscript.tool` setting names; with no setting it stays inactive and says why. Other editors use their own language-client configuration.
- **Language server.** Diagnostics; completion of attributes after `|`, accepted child kinds in a body, valid destinations after a wire's `:`, enum variants where an enum is expected; hover with documentation and inferred types with their origin, where a compile-time value shows its value and the type each use chose, and a cell shows that it is reactive, its one type and the inputs it reads; go to definition; find references; rename of instances, patterns, functions, inputs and `let`s; an outline of top-level items; semantic tokens, which mark inputs and reactive `let`s apart from compile-time values.
- **Diagnostics.** `check` and the framework's commands print each diagnostic as its summary and help, then its lesson under `── why ──`, then its spans under `── where ──`, primary first, each with its label; `--brief` leaves out the lesson. The language server sends the summary and the help, and the lesson on request.

```text
error[reactive-recursion]: `lowest` calls itself, so it runs while the program compiles, but its argument `step` is an input that changes while the music plays.
help: give `lowest` a bound to run it while the music plays: `fn lowest(i) bounded 10 else 127:`

── why ──
A recursive function without a bound runs to its end once, while the program
compiles, and its answer becomes a constant. An input is not known then. A
bounded function runs at most N levels each time what it reads changes, and
gives its fallback if it gets that deep.

── where ──
src/main.sym:7:12  lowest(step)  the call; `step` is an input
src/main.sym:3:1   fn lowest(i)  `lowest` calls itself here
```

## 10. The synaptic-kernel adapter

`symphonyscript-kernel` implements `Consumer` for synaptic-kernel. It is generic over vocabularies: it maps structure and values, and the engine on the kernel's consumer thread interprets kinds.

### 10.1 Structure

| Program | Kernel |
|---|---|
| Instance | a node; `kind` is the vocabulary's node-kind index |
| An instance's sequence | a sub-chain that starts with a head node and holds the children in sequence order; a `contains` synapse goes from the instance to the head |
| Bundle | a bundle node in the parent's chain; its members form a sub-chain with its own head node behind another `contains` synapse |
| Wire | a wire-point node at the wire's position in the chain, holding the wire's attributes, activation and serial; a synapse whose `kind` is the wire-kind index goes from it to the destination |
| Destination chosen at runtime | one synapse per distinct candidate, each holding a flag that marks the chosen one |
| Top-level instances | a sub-chain after a root sentinel node, in top-level order |
| Entry | the sentinel's slot, written to `mem_metadata` once at load and never changed |

`kind` is 8 bits. The adapter reserves four node kinds (sentinel, head, bundle and wire point) and one synapse kind (`contains`), so a vocabulary used with the adapter has at most 252 node kinds and 255 wire kinds. `load` refuses a larger vocabulary.

The engine enters at the sentinel on every cycle, walks the top-level chain, and follows `contains` synapses to the heads of sequences. A head never moves, so reordering a sequence never touches its `contains` synapse.

### 10.2 Values

Each node and synapse holds the adapter's own cells, followed by attribute cells:

- **Adapter cells**, always in meta: a node's active flag and a serial number unique to the instance's or wire's lifetime, and a candidate synapse's chosen flag. The engine keys its per-instance state (playheads, sounding notes, phases) by serial, because a freed slot is reused by another instance.
- **Attribute cells**, placed by type:

```rust
fn plane(ty: &Ty) -> Plane {
    if !ty.is_scalar() {
        return Plane::Array;
    }
    match ty.scalar {
        Scalar::I64 | Scalar::F64 | Scalar::Str => Plane::Meta,
        _ => Plane::Attr,
    }
}
```

A `bool`, an enum value, a `u8`, an `i32`, an `f32` and a string's `StrId` each take one cell; an `i64` or `f64` takes two, low half first; an array takes its elements' cells in row order. A value that fits one cell is untorn in the attribute plane and needs no publish. synaptic-kernel's cells are 32-bit, so a 64-bit value spans two cells and goes to meta, where `publish()` makes it visible all at once.

Array attributes never sit in a node's stride. A node with array attributes keeps them in a separate, triple-buffered array region, reached through an index in its meta cells, so only nodes with arrays pay for them, and they become visible at the same publish as meta. The adapter computes a layout table from the vocabulary once, giving every attribute of every kind a plane and an offset. The engine reads through the same table, so the two never disagree. A variable-array attribute keeps its capacity's cells and its live length in the array region.

`load` and `reload` write the Program's string table into a kernel LUT, as UTF-8 bytes packed four to a cell after a table of offsets, and size the LUT from the table, so the engine reads strings without the Program. `StrId` cells live in meta, so a reload's new ids and its new table appear at the same publish.

`apply` writes each change to its location and calls `publish()` only when a change landed in meta or in the array region. The guarantee to engines, which is this consumer's statement under SPEC §12.2: every single value is consistent; meta-plane and array values are consistent with each other at every publish; independent attribute-plane values changed by the commits between two engine reads may be seen mixed in that read, and an attribute-plane value may be seen before a meta-plane value of the same commit, such as an activation, is published.

Strides are uniform per store: the meta stride and the attribute stride are the largest any kind's scalar attributes need.

### 10.3 Capacity and reload

`load` sizes the kernel from the Program's counts plus headroom equal to those counts. A removed slot counts until the engine has acknowledged the publishes that follow its removal, so `reload` publishes first, which lets the slots the previous reload freed be reclaimed, and then inserts added nodes, removes removed ones together with their children's chains, writes changes and publishes. A reload that replaces every node and synapse fits without `grow` once the engine has acknowledged the previous reload's publishes; a reload that outgrows capacity calls `grow`, and capacity only grows. Kept and reordered nodes move to their new chain positions with `move_node_after` and `move_node_before`, which keep a node's slot, synapses and cells and preserve the kernel's invariants. The adapter requires these from synaptic-kernel, and PLAN M5 adds them.
