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

A framework author builds a vocabulary in Rust; `Vocabulary::builder` takes its name and its version (SPEC §15.8). `v.node(name)` and `v.wire(name)` return a kind's typed handle at once, and `v.kind(handle)` and `v.wire_kind(handle)` configure it, so a kind can accept itself or a kind declared after it. The framework's consumer keeps the handles and matches them against the Program. A handle compares, hashes and looks up by its kind's index alone, so two builds of one vocabulary give equal handles; the vocabulary stamp tells vocabularies apart, and only `build()` rejects a handle from another builder (SPEC §10.1). A part of the SPEC §1.3 vocabulary:

```rust
let mut v = Vocabulary::builder("music", "2.1.0");

let beats = v.unit_family("beats").base("beats").unit("bars", 4.0);
let wave = v.enumeration("Wave", ["Sine", "Saw", "Square"]);

let note = v.node("Note");
let clip = v.node("Clip");
let track = v.node("Track");
let osc = v.node("Osc");
let session = v.node("Session");
let pad = v.node("Pad");
let plays = v.wire("Plays");
let starts = v.wire("Starts");

v.kind(note).doc("A single pitched event inside a clip.");
let pitch: Attr<i32> = v.kind(note).attr::<i32>("pitch").default(60).default_attr().handle();
let velocity: Attr<f32> = v.kind(note).attr::<f32>("velocity").default(1.0).doc("Loudness, 0 to 1.").handle();
v.kind(note)
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
    .attr::<&'static str>("label").default("")
    .attr_str("title", 64).default("");

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

v.kind(pad)
    .top_level(Count::Any)
    .attr_record("hit", "Hit").takes(Takes::Input).default_attr()
    .attr::<f32>("gain").default(1.0);

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

- `attr::<T>` takes `u8`, `i32`, `i64`, `f32`, `f64`, `bool`, fixed-length arrays of these nested to any depth (`[bool; 8]`, `[[i32; 3]; 2]`), core's bounded list `Bounded<T, N>` for a variable array of capacity N (SPEC §3.7), whose elements may be fixed arrays and which no array holds, and `&'static str` for an attribute that takes compile-time strings only. The Rust compiler checks the default against `T`. `attr_str(name, capacity)` declares a string attribute of that capacity in characters (SPEC §3.3). `attr_enum(name, enum, default_variant)` declares an enum attribute, and `attr_enum_array(name, enum, length, default_variant)` an array of one. `attr_record(name, type_name)` declares a record attribute of a record type the prelude declares, and `attr_record_any(name)` one that takes a record of any type, which the bound value supplies; the compiler reports a type the prelude lacks as a prelude error (§7).
- Every attribute declaration yields its typed handle through `.handle()` at its end: `Attr<T>`, `T` the Rust type `rt.attr` reads it in (§4). The framework's consumer keeps the handles of the attributes it reads, as it keeps the kinds', and a handle compares with an `AttrId`. Renaming or retyping an attribute in the vocabulary stops a consumer that reads it through its handle from compiling.
- `.takes(Takes::Any | Takes::Input | Takes::Constant)` states the binding an attribute takes (SPEC §10.1); `Any` is the default. An attribute that takes an input has no default, and `build()` refuses one with a default.
- `attr_map(name, key, value, capacity)` declares a map attribute, `key` and `value` core `Ty`s (SPEC §3.10); `attr_json(name, budget)` declares a document attribute of that budget in bytes, and `attr_json_any(name)` one that takes any document, whose budget the bound value supplies (SPEC §3.11). No attribute has a sum type (SPEC §3.9), and `build()` refuses a capacity or a budget of 0.
- The stamp's serialization gains each attribute's capacity, record type, map types, budget and `takes`.
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
    pub functions: Vec<Function>,
    pub records: Vec<RecordType>,
    pub strings: Vec<String>,
    pub documents: Vec<Document>,
}

pub struct RecordType {
    pub name: String,
    pub fields: Vec<(String, Ty)>,
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
    Record(Vec<Binding>),
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
- An array input occupies one slot per element, numbered row by row, and a record input one slot per leaf field, numbered in declaration order depth first (SPEC §12.1), so element and field writes address single slots; a string input and a runtime string cell are each one slot or cell. An array attribute binds `Binding::Array`, and a record attribute `Binding::Record`, in field order. `program.input_of(slot)` returns the input a slot belongs to, so a host that declared an attribute with `Takes::Input` follows its `Binding::Input` to the input it writes (SPEC §6.4).
- `Input.name` is the input's path (SPEC §11.1): `cutoff` for a top-level input and `verse.low.gain` for a pattern's.
- `cells` is in dependency order: a cell reads only inputs and cells with lower indices.
- An `AttrId` indexes its kind's attribute list.
- `gate` combines the item's own `when` with the `when` arm conditions around it in one binding (SPEC §11).
- `Select` keeps the candidates in the order of SPEC §8.2; `index` is an integer binding, clamped to them.
- `Expr` is a list of nodes: constants, input and cell reads, typed unary and binary operators, `if`, `match`, `Pick` (indexing with a runtime index, clamped), conversions, built-in and native calls, call nodes, and `For` nodes. A call node invokes a shared body (SPEC §6.3, §11) by its index in `functions`, the function table, and carries its arguments as operands; a `Function` holds its parameter and result types and its body, an `Expr` that reads its parameter slots. A `For` node is a `for` expression that reads a cell (SPEC §5.10): its iterable and start are its operands, and its body is a subtree of the list that reads the loop slots and the accumulator slot the node binds; a lambda argument of a native call (SPEC §10.1) is a subtree that reads the parameter slots the call binds. A node's operands come before it in the list, so one node may be the operand of several, and equality, hashing, printing and dropping go over the list once. Every operator node carries its operand type, and every read of a cell that its use reads in another type carries an explicit conversion (SPEC §4.4). Patterns, units and compile-time values are already resolved, and a function is inlined or invoked through a call node; nothing in `Expr` names a source declaration.
- `Ty` names a record by its index in `records` and a string with its capacity in characters.
- `Value` is `U8`, `I32`, `I64`, `F32`, `F64`, `Bool`, `Enum(EnumId, u16)` or `Str(StrId)`. A `StrId` indexes `strings`, which holds each compile-time string once, a string input's initial value among them.
- `Ty` gains `Sum(Vec<Ty>)`, `Ty::None` standing among the alternatives of an optional, an alias being resolved to its target before the Program (SPEC §3.9); a length or a capacity at every level of an array (SPEC §3.7); `Map { key, value, capacity }` (SPEC §3.10); and `Json { budget }` (SPEC §3.11). `Value` gains `None`, `Tagged(u16, Box<Value>)` for a sum, `Map(Vec<(Value, Value)>)` in entry order, and `Json(DocId)`, which indexes `documents`, holding each compile-time document once, a document input's initial value among them. A sum, a map and a document are each one slot or cell; a variable array at an inner level is one cell per row of the fixed levels above it.
- A state's triggers (SPEC §6.6) are the inputs and cells the state's `Cell.expr` reads through `Trigger` nodes: a change to one marks the state dirty and contributes no value.
- `Span` is a file, named by its `Source.path`, and a byte range, used in diagnostics and tooling. `span` is where an item is written; `expanded_from` holds the pattern uses that produced it, innermost first, and is empty outside patterns.

`Program` implements structural equality that compares floats by their bits, which the canonical NaN of SPEC §12.5 keeps stable, so two Programs compiled from the same sources compare equal.

## 4. The runtime

```rust
let mut rt = Runtime::new(program, &vocab)?;
let cutoff = rt.input("cutoff").unwrap();
let steps = rt.input("steps").unwrap();

rt.write(cutoff, Value::F32(0.7))?;
rt.write_element(steps, 3, Value::Bool(true))?;

rt.commit();
for change in rt.changes() {
    match change {
        Change::Attr { instance, attr, element } => {
            if attr == pitch { let p: i32 = rt.attr(instance, pitch); }
            let v = rt.attr_value(instance, attr, element);
        }
        Change::WireAttr { wire, attr, element } => { let v = rt.wire_attr_value(wire, attr, element); }
        Change::Active(instance) => { let on = rt.active(instance); }
        Change::WireActive(wire) => { let on = rt.wire_active(wire); }
        Change::Target(wire) => { let to = rt.target(wire); }
    }
}
```

- `Runtime::new` takes the vocabulary the Program was compiled against, refuses a Program whose stamp differs from the vocabulary's (SPEC §13.1), and takes its native functions from it, which native call nodes invoke at runtime (SPEC §10.1).
- `rt.attr(instance, handle)` reads an attribute through its `Attr<T>` handle (§2) in its Rust type, and `rt.attr_element(instance, handle, k)` one element of an array attribute; `rt.wire_attr` and `rt.wire_attr_element` read a wire's the same way. `rt.attr_value(instance, attr, element)` and `rt.wire_attr_value(wire, attr, element)` read any attribute by its `AttrId` as a `Value`, for a consumer that walks every kind.
- `Runtime::new` validates the Program (ids in range, cells in dependency order, operand types consistent) and returns an error for one that fails. A Program that passes it evaluates, commits and reloads without a panic. It then evaluates every cell, computes every activation and target, and allocates every buffer the runtime ever uses.
- `commit` never allocates. It walks the cells in index order, evaluating only cells marked dirty, and marks readers dirty when a value changes bit-for-bit. It fills the change buffer, whose capacity is the number of reportable items, in the order of SPEC §12.3. `changes` reads that buffer until the next commit.
- Each input, cell and gate has a precomputed list of the attributes, gates and targets that read it, so changes are reported without searching.
- Effective activation is computed parents first, so a gate change reaches every descendant in one pass.
- `Expr` is evaluated without recursion, each node at most once per evaluation, into buffers sized in `Runtime::new` for the largest expression, so no Program overflows the native stack and a shared node is computed once. The evaluator evaluates a `For` node by evaluating its body subtree once per element with the loop slots bound, so the body's nodes count once per iteration and the node itself once. A native call passes the native function a callback for each lambda argument, which evaluates the lambda's subtree with its parameter slots bound to the values the function passes and returns its value, each time the function calls it; the function writes its result into the buffer the runtime provides (SPEC §10.1). Callbacks nest only as deep as native calls nest in lambdas in the Program, a depth the compiler bounds by the nesting limit, so the native stack a commit uses is bounded. A call node binds its arguments to the shared body's parameter slots and evaluates the body's tree in the buffers `Runtime::new` sized for that body; call nodes nest to a depth fixed at compile time, since no function recurses at runtime (SPEC §6.3) and the nesting of call nodes is fixed by the expressions, so a call allocates nothing and never fails.
- `write`, `write_element` and `write_array` return an error for a write SPEC §12.1 rejects and for a dead handle. Nothing in the runtime panics.
- `write_text(handle, &str)` writes a string input and returns an error past its capacity in characters; the runtime stores a string as one `u32` per character and converts at the boundary. `attr_text(instance, attr)` and `wire_attr_text(wire, attr)` return a string attribute's value as `&str` from a UTF-8 buffer `Runtime::new` allocates at 4 bytes per character of the capacity, filled on the read; `commit` touches it never. A record attribute's leaf fields are read through `attr_value(instance, attr, element)` with the field's index, as an array's elements are, and a change report names the field as it names an element. `write_none(handle)` and `write_tagged(handle, alternative, value)` write a sum input; `write_row(handle, row, values)` and `write_row_element(handle, row, position, value)` address a variable array at an inner level; `write_entry(handle, key, value)` and `write_map(handle, entries)` write a map input; `write_document(handle, bytes)` writes a document input and returns an error naming the budget or the byte position of the fault; `attr_document(instance, attr)` returns a document attribute's text as `&[u8]` from a buffer `Runtime::new` reserves. Core's `document` module, the RFC 8259 parser into a node pool with kinds, views, equality and typed reads, iterative and allocating nothing after `Runtime::new`, serves the boundary and the compiler's constant documents alike.
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

1. Refuses a Program whose vocabulary stamp differs from the runtime's vocabulary (`Refused::Vocabulary`), or one that fails `Runtime::new`'s validation (`Refused::Invalid`).
2. Carries input values over by name, following SPEC §13.2. Each input element records whether the host has written it.
3. Matches instances, bundles and wires following SPEC §13.3, from the top level down: paths below matched parents for named items; then, per parent sequence, the common prefix and suffix, and Myers' difference algorithm over the keys of the remaining anonymous entries; then per-kind pairing in each gap.
4. Builds the new state and evaluates every cell.
5. Reports the structure, the staged writes it dropped, and the attribute, activation and target differences of matched items.

`install` replaces the running state with the pending one, between two commits, after staging again against the new Program, by input name, the writes staged and committed since the snapshot, as SPEC §13.2 does for writes staged before a reload, so the next commit applies them. `Runtime::with_limits(program, &vocab, limits)` sets the reload search bound of SPEC §13.3, and `Runtime::new` uses the default. Dropping a `PendingReload` instead leaves the runtime on the old Program. Preparing may allocate; installing swaps buffers and does not. Input handles are stable: the runtime keeps a registry from input names to handles across reloads, adds handles for new names, marks handles of removed inputs dead, and revives a dead handle when an input of its name returns.

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

pub struct Source { pub package: String, pub path: String, pub text: String }
pub struct Compiled { pub program: Program, pub warnings: Vec<Diagnostic> }
pub struct Failed { pub diagnostics: Vec<Diagnostic> }
```

`Compiler::new` parses and resolves the prelude once and reports its errors as vocabulary errors (SPEC §10.1). The tool and the language server keep one `Compiler`; `compile` and `eval` build one for a single call. `Source.package` is the identity of the source's package (SPEC §15.1), empty for the project, and `Source.path` is relative to that package's root (SPEC §15.2), with `/` separators; the caller supplies the sources of every library of the build at the versions SPEC §15.7 chooses. `Limits`, in `symphonyscript-core`, holds every resource limit of SPEC §4.6, §6.3, §12.6 and §13.3: recursive calls, nesting depth, inlined calls, expansion steps, expanded items, memory bytes and the reload search bound, and beside each limit but the reload search bound its warning value, a field named after the limit with `_warning` appended, whose default is half the limit's (SPEC §4.6). `Limits` also holds `shared_body_threshold`, the number of body nodes past which Lower shares a body (stage 8), 1,000 by default, set as a limit is and drawing no diagnostic. `Limits::default()` takes each default, a warning's among them, from one file of constants in core; `Compiler::new`, `compile` and `eval` use the defaults, and `Compiler::with_limits` takes others. The inline default is 100,000 (SPEC §4.6). `Compiler::with_limits` refuses no setting: a compilation whose ids fill an id space draws one `expansion-limit` diagnostic naming the bound, never `internal-error`. A limit's diagnostic prints the bound in force grouped by thousands, as the SPEC writes numbers: `1,000,000` by default, `2,000,000` when raised and `10` when lowered.

The compiler performs no file I/O; the caller supplies every source. This keeps the compiler usable in editors, build scripts and WebAssembly. Stages:

1. **Lex.** Layout tokens (SPEC §2.2), literals, keywords, unit suffixes looked up in the vocabulary. Every byte belongs to a token or to trivia (whitespace and comments). After the keyword `json` the lexer reads the next token, and when it is `{` or `[` everything to its matching closer, in JSON mode, where a string has RFC 8259's form and a number JSON's; a `(` returns to the language's mode to its matching `)` (SPEC §3.11); the same tokens serve `json[..N]` in a type.
2. **Parse.** A lossless syntax tree that reproduces the source byte for byte, with a typed view over it. Code outside the parser reaches the tree only through views that borrow the parse result, so the parse result is the tree's last owner and dismantles it without recursion. The parser recovers from errors (§7.1), so a broken file still yields a tree. Before it parses bodies, the parser reads every file's pattern declarations and classifies each as a node, a wire or a group by its body, to a fixpoint over single uses, and parses with the wire patterns among the wire names, since what follows a line's colon depends on it (SPEC §7.1). A placement whose kind is a wire kind of the vocabulary or a wire pattern parses as a wire placement. The parser reads `type` declarations in both forms, sum types with `|` and `?`, capacities at every level, `map` and `json` literals, and `without` (SPEC §3.8 to §3.11), and the `for` expression in both forms, lambdas, function types, bracket parameters and a type's bracket arguments (SPEC §5.10, §6.7, §6.8).
3. **Collect.** The top-level namespace across all files, the prelude and the vocabulary, with duplicate detection and prelude shadowing; the call graph of functions, which marks recursive ones, and the use graph of patterns. States join the namespace, and the call graph marks the cycles of calls each recursive function lies on. Record types and aliases join the namespace, an alias resolved to its target, and the cycles among aliases and record fields are reported here. Collect also records each pattern's declared type.
4. **Expand.** Pattern uses are expanded and `for` loops unrolled. A value that structure needs (a range bound, an iterable, a recursive call, and any argument or `let` they read) is resolved, typed as SPEC §4.6 says and evaluated when expansion reaches it; such a value that reads a cell is an error. Every expanded item keeps the spans of its uses and of its definition. A structural value never depends on a default unit, since SPEC §3.6 reads a plain value only in the unit of an attribute its family value reaches and structure takes plain numbers; the values of an iterable's unit elements are computed once Infer knows their units. Expand places a pattern's inputs and states under each use's path, partitions a use's body among the pattern's `children` parameters by pattern, bundle and kind, places each share at its parameter's line, and checks the parameter modifiers at the use; a pattern's bracket parameters bind here when the argument is a node reference or a placement (SPEC §6.8).
5. **Resolve.** Names, paths and `to` in the expanded program; each call to a non-recursive function is typed as the function's body under the call's argument types (SPEC §6.3), one resolved body instance per function per distinct set of argument and result types, and each call site is kept as a call of that instance; each argument is bound once and shared by every use of its parameter, so nested calls grow the program linearly; two equal calls that are the operands of one operator or the arms of one `if`, with the same names or paths as arguments, share one call, since they meet the same expected type and feed the same attributes; SPEC §4.6's inline limit is found before any body is inlined, by counting each value's calls over the call graph once Expand is done, so a compilation past it costs no inlining; each value is classified as compile-time or reactive by what it reads. Cycles among declarations (SPEC §6.2) are found here, before Infer. A bare name that names no declaration is left for Infer to resolve as a variant. A call of a parameter bound to a lambda inlines the lambda's body as a call of a non-recursive function is inlined (SPEC §6.7), and so does a call through a compile-time `let` holding a function reference, which never reaches the Program (SPEC §6.7), and a `for` expression's body is inlined once (SPEC §5.10).
6. **Infer.** Types and unit families over the whole program (SPEC §3.6, §4), bare variants from their expected types, and then the remaining calls to recursive functions, evaluated in the types inference gives them. Recursive calls are memoized by argument values, argument types and expected type, a function reference argument by its declaration, a lambda's being its source position, and the values of the names its body reads, which is what SPEC §4.6's limit counts. A recursive call whose result is an array is evaluated in the element type its use expects, or with the defaults where it expects none, before that use is typed, and takes the shape of the value it yields; a reactive value whose shape comes from such a call alone takes the shape the call yields with the defaults, found when Resolve builds its bindings. A recursive call's kind and family follow from its body under its arguments' kinds and families, settled over the call graph before cell types are found. Exact compile-time integers never grow past SPEC §4.6's bound: an operation whose result would reach 2¹²⁷ reports the error before computing it, so `2 ** 1000000` costs no more than `2 ** 200`. Every array value's length or capacity follows SPEC §3.7, and the types and capacities of states are the least solution of SPEC §6.6's equations, found in time that does not depend on the capacities' values. A native call takes its signature's types; one whose arguments are compile-time values is evaluated by calling the function, its result counting as the compile-time value of the call, and a lambda argument is passed as a callback that evaluates the lambda's body in the signature's types each time the function calls it (SPEC §10.1). A function's or a `type` declaration's bracket parameters bind here, and so do a pattern's that Expand left (SPEC §6.8). Infer also types triggers as reactive `let`s, and infers each pattern's type from its body and checks it against its declaration (SPEC §6.4, §6.6). Infer also finds strings' capacities (SPEC §3.3), records as values of bindings, record literals typed by their position, annotations as fixed types on every binder, sums (widening into a sum, `or` as the fallback, `match` narrowing per arm, `is`), capacities at every level of an array, maps (joins, `without`, slices, lookups as `V?`) and documents (views and their budgets, typed reads as `T?`, literal budgets, `typeof`, equality), each folded on compile-time values through core's `document` module.
7. **Check.** The vocabulary checks (SPEC §10.2), the binding each attribute takes, the string, record, map and document attribute rules, a `T?` bound to an attribute, cycles among wires, fit checks, and the warnings of SPEC §4.4 and §10.2. Wire cycles draw one diagnostic per strongly connected component of the wire graph that holds a wire of a kind not marked cyclic (SPEC §8.4), found in work linear in the instances and wires: it names the shortest cycle through the component's first such wire in Program order and the number of the component's wires, and a cycle of one wire reads `a → a`.
8. **Lower.** Constant folding with the canonical NaN, the cell graph in dependency order, paths, filled defaults, the string table, the Program. Lower chooses per resolved body instance (stage 5) between substituting the body at each call site and emitting the instance once as a `Function` in the function table with a call node per site (SPEC §6.3): an instance whose sharing saves more body nodes than `shared_body_threshold`, its body nodes times its call sites less one, is emitted once, and a small or single-site instance is substituted; a call with only compile-time arguments is folded. Lower folds and lowers each node of a shared `Expr` once, so a Program grows with its distinct nodes, not with the paths through them, and its output depends on the sources and the vocabulary alone, never on hash or allocation order.

Every diagnostic teaches. It carries a stable code in kebab-case (`unknown-attribute`, `count-exceeded`); a summary of one to three lines in the program's own names; a one-line help that states the fix, when one exists; a primary span; and related spans, each with a label (for a pattern use: the use and the line in the pattern; for an error inside an inlined function body: the calls, innermost first). Every code is a variant of the compiler's `Code` enum, whose `section` method returns the SPEC section of its rule and whose `lesson` method returns its lesson: a short plain-language document per code in `crates/compiler/lessons/` that states the rule, says why it exists, and shows a program that draws the code beside its fix, its `sym` and `sym-error` examples tested as the SPEC's are, the second drawing the lesson's code. A diagnostic about the program as a whole, a count of top-level kinds or a missing entry-kind instance, has the empty span at the start of the first file in top-level order.

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

The tests keep a table mapping each error rule of the SPEC, by section and sentence, to its code; a sentence that names several errors has one row for each error it names, and a test checks that every code appears in the table and every code in the table exists. A lookup that misses inside the compiler is a compiler defect: it returns an error, and the run draws one `internal-error` diagnostic, the one code no SPEC rule names and the table leaves out, and fails. The compiler collects every diagnostic of a run instead of stopping at the first. Diagnostics that differ only in the calls or pattern uses between their fault and the outermost call or use merge into one, which names the longest such chain found, the first of that length, so a fault draws one diagnostic per outermost call or use however its bodies are shared. A literal whose elements disagree draws that diagnostic at the innermost faulty literal, and every value that holds it whole takes a placeholder. A choice whose arms disagree in family, element type or dimension draws one diagnostic at the choice, mixed-families when their families disagree (SPEC §3.6) and type-mismatch otherwise, whatever chooses, whichever arm a compile-time choice takes and whatever the arms are, and every value that holds it whole takes a placeholder. A choice whose arms agree has their unified type whichever arm a compile-time choice takes (SPEC §5.4), and a position that refuses that type refuses the choice as one value, at the expression that reads it. An untyped bare-variant flow, `let`s whose variants meet and that no use gives one enum, compile-time or reactive, draws its one diagnostic at its first writer, the first `let` that writes a variant, and its uses draw nothing more; every other `let` the flow passes through, an alias such as `let b = a` included, appears once among its related spans. A related span never repeats its diagnostic's primary span. It returns them in source order: by file in top-level order, then by span start, span end and code. A name, type or unit family that an error leaves unknown takes an error placeholder that every later stage accepts without a diagnostic, so one fault draws one diagnostic; a placeholder arises only where a diagnostic is reported, so a program that draws no error holds none. A value whose type an error leaves known keeps that type and only its value is unknown: a refused conversion has its target type, an element read whose compile-time index is in error has its elements' type, and an operator whose result type does not depend on its operands (`and`, `or`, `not`, a comparison, `is`) has that type. Beside a placeholder for a name, the expression draws a diagnostic only where it draws that diagnostic, with the same code at the same span, for every value the name could hold: a value of each type a name can hold (SPEC §3, §6.7), `i32`, `f64`, `bool`, an enum, a string, an array, a record, a node reference and a function reference among them, each a compile-time value where only a compile-time value may stand; so `Note when q or 1 or 2` draws its unknown-name and one type-mismatch at `2`, which every such value draws. A placeholder for an operand that a syntax fault leaves missing holds no value at all, and the operator that holds it draws nothing, so `Plays: verse+` draws its syntax diagnostic alone. Every other fault draws its own diagnostic at its span, whatever fault stands beside it: an operand that a refused operator, index, bound, condition, conversion or annotation holds is checked in its own type, so `Note/([1, 2, 3][1.5 + [1, 2][5]])` draws an index-type and an index-out-of-range; an arm a compile-time choice does not take and an element a compile-time index does not read are checked in their own types only, with no index range or fit check. A trial evaluation that settles a recursive call's kind (stage 6) is no part of the program: what it meets in code that nothing expands draws nothing (SPEC §6.5), and the evaluation of code that is expanded reports it. A top-level name declared twice is such a placeholder at every reference beside its one duplicate diagnostic, and so is a name that a region skipped by a fault (§7.1) holds, wherever it is used. A limit's warning stops nothing. The nesting depth (SPEC §6.3) stops no compilation: it is an error of the call that passes it. A compilation that a limit stops reports that limit's one diagnostic and the diagnostics found before the stop, and no stage after the one it stops in runs: a stop during Expand reports no diagnostic of Infer or Check, and a diagnostic that depends on work the limit cut off, such as a flow's type whose uses lie past the cut, is not reported.

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

- **Library.** A host calls `compile` and `Runtime::new` with one vocabulary in its own process. Edits reload live. `eval` compiles and loads a single source in one call.
- **Generated Rust.** `symphonyscript::codegen::generate(sources, &vocab) -> Result<String, Failed>` returns the text of a Rust module containing `pub fn program() -> Program`. A framework's `build.rs` reads the sources, calls it and writes the module into `OUT_DIR`; the firmware or application includes it, calls `Runtime::new(program(), &vocabulary)` with the vocabulary crate's vocabulary, and never contains the compiler. The module also holds a typed write method for each input, whose variable-array inputs take core's bounded list `Bounded<T, N>` (§2), so an overlong write does not compile. The vocabulary lives in its own crate, used by the framework both as a dependency and as a build-dependency, because a build script cannot call the crate it builds. Generated builds do not reload.

A differential test compiles every fixture project both ways and requires equal Programs and equal commit traces.

## 9. Tooling

A framework ships one binary, named after its vocabulary and built from it:

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
| `new <name>` | tool | creates a project: a folder with `package.sym`, naming the framework's vocabulary, and a starter file |
| `check` | tool | compiles and prints diagnostics |
| `dump` | tool | prints the Program |
| `lsp` | tool | runs the language server |
| `add <source>` | `sym` | requires a library in the project's package file and fetches it |
| `remove <name>` | `sym` | removes a library from the project's package file |
| `update` | `sym` | raises the build's libraries to newer releases |
| any other | framework | playing, rendering, exporting |

- **Commands.** A framework command is `fn(&Project, &[String]) -> Result<(), ToolError>`, receiving the project and the command's arguments.
- **`sym`.** Users run one command, `sym`, under every framework. It runs `add`, `remove` and `update` itself, and runs every other command, `check`, `dump` and `lsp` among them, as the framework binary named after the vocabulary its project's package file states (SPEC §15.8), with the same arguments.
- **Packages.** `add <source>` writes `require "<source>@<version>"` into the project's package file, the newest release when the source names no version, then fetches the library and writes its lock lines. `remove <name>` removes the library's `require` and `use` items and the lock lines no other library needs. `update` raises every library, or the one it names, to its newest patch release, and with `--minor` to its newest minor release; `update --major <name>` moves that library to its next major after checking that every library of the build that requires it has a release on that major, and otherwise names each that has none.
- **Fetching.** Before a compile, the tool chooses the build's versions (SPEC §15.7), fetches each release and commit over https into a cache that every project on the machine shares, keyed by identity and by version or commit, reads local libraries in place, and passes every library's sources to the compiler (§7).
- **Lock file.** `package.lock`, beside the project's `package.sym`, holds one line per fetched release of the build: its source with its version and the SHA-256 hash of its files, as `github.com/acme/scales@2.1.4 sha256-…`. The tool writes it, and nobody edits it. A fetched release whose hash differs from its line stops the build, naming the library; a commit needs no line, since a commit's name is its hash.
- **Projects.** A `Project` holds its root, the folder holding its `package.sym`, and its sources: every `.sym` file under the root, found recursively, each with its path relative to the root, and the sources of every library of its build under its identity (SPEC §15.1). Those paths fix the top-level order (SPEC §2.1).
- **Dump.** `dump` prints every field of the Program in the Program's order, deterministically, and two floats with distinct bits print distinctly.
- **Single files.** `check`, `dump` and the framework's subcommands accept a lone `.sym` file in place of a project; it is a project of one source whose path is the file's name.
- **Watch.** `watch(project, vocab, consumer)` recompiles on save, prepares a reload, lets the consumer accept it, installs it, and drives the consumer (§5, §6). Frameworks call it from their own commands.
- **Options.** The `options` field of the project's `package` record (SPEC §15.2) holds the build's settings, each key a field of `Limits` (§7), a limit's warning value keyed `<name>_warning` beside the limit's `<name>`, and `shared_body_threshold` among them. Every tool command, the language server's included, takes `--limit <name>=<value>`; a flag overrides the record, and the record the default.
- **Editor.** A VS Code extension and a JetBrains plugin. Each holds a static syntax grammar, or for JetBrains the server's semantic tokens where its platform reads no grammar, and a language client that finds the project's `package.sym` and starts `sym lsp`. For a lone `.sym` file each starts the tool its `symphonyscript.tool` setting names; with no setting it stays inactive and says why. Other editors use their own language-client configuration.
- **Language server.** Diagnostics; completion of attributes after `|`, accepted child kinds in a body, valid destinations after a wire's `:`, enum variants where an enum is expected; hover with documentation and inferred types with their origin, where a compile-time value shows its value and the type each use chose, and a cell shows that it is reactive, its one type and the inputs it reads; go to definition; find references; rename of instances, patterns, functions, inputs and `let`s; an outline of top-level items; semantic tokens, which mark inputs and reactive `let`s apart from compile-time values; hover on a sum or an alias shows its alternatives or its target; completion of a record's fields after `.`; rename of record types, aliases and fields; hover on a pattern or a use shows the pattern's type (SPEC §6.4), the same on both.
- **Diagnostics.** `check` and the framework's commands print each diagnostic as its summary and help, then its lesson under `── why ──`, then its spans under `── where ──`, primary first, each with its label; `--brief` leaves out the lesson. The language server sends the summary and the help, and the lesson on request.

```text
error[reactive-recursion]: `lowest` calls itself, so it runs while the program compiles, but its argument `step` is an input that changes while the music plays.
help: write the loop as a `for` expression, which runs while the music plays

── why ──
A recursive function runs to its end once, while the program compiles, and
its answer becomes a constant. An input is not known then. A `for` expression
runs over its iterable each time what it reads changes, at most as many times
as the iterable can hold.

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
