# Reviewing

This file lists the defects a code reviewer in this repository looks for, each shown by one bad and one good example. A reviewer checks the ordinary paths first (the acceptance criteria, the design, `CODING.md`, the tests run), then reads the diff for these patterns beyond them. `CODING.md` shows how good code is written; this file shows the defects a review catches.

## Known anti-patterns

**Stringly typed kinds.** A misspelt string compiles and falls into the default arm.\
**Bad**
```rust
match node.kind.as_str() { "osc" => osc(node), "gain" => gain(node), _ => {} }
```
**Good**
```rust
match node.kind { Kind::Osc => osc(node), Kind::Gain => gain(node) }
```
**Wildcard arm on an enum we own.** A new variant compiles and takes the default path unseen.\
**Bad**
```rust
match op { Op::Add => add(a, b), _ => Value::ZERO }
```
**Good**
```rust
match op { Op::Add => add(a, b), Op::Sub => sub(a, b), Op::Mul => mul(a, b) }
```
**Bare numbers that carry units.** Seconds, samples and beats in one `f64` add without complaint.\
**Bad**
```rust
fn schedule(at: f64, length: f64) -> Event;
```
**Good**
```rust
fn schedule(at: Beats, length: Frames) -> Event;
```

## Subtle, deep patterns

### General

**Growth without a bound.** A table that only grows survives every test and fails after hours of reloads.\
**Bad**
```rust
fn reload(&mut self, src: &Source) { for name in src.names() { self.symbols.intern(name); } }
```
**Good**
```rust
fn reload(&mut self, src: &Source) { self.symbols = Interner::from_names(src.names()); }
```
**Swallowed error.** A `Result` dropped with `let _`, `.ok()` or a default turns a failure into silent wrong output.\
**Bad**
```rust
let rate = parse_rate(text).unwrap_or_default();
```
**Good**
```rust
let rate = parse_rate(text)?;
```
**Lost cause.** Mapping an error to a fresh one drops the path, the span and the source the user needs.\
**Bad**
```rust
let bytes = read(path).map_err(|_| Error::Load)?;
```
**Good**
```rust
let bytes = read(path).map_err(|source| Error::Load { path: path.to_owned(), source })?;
```
**Arithmetic at the edges.** Empty, zero, `MIN` and `MAX` break the formula, and release builds wrap instead of panicking.\
**Bad**
```rust
let last = &events[events.len() - 1];
```
**Good**
```rust
let last = events.last().ok_or(Error::Empty)?;
```
**Order from scheduling.** Results gathered in completion order differ from run to run.\
**Bad**
```rust
let units: Vec<Unit> = results.iter().take(jobs).collect();
```
**Good**
```rust
let mut units: Vec<(JobId, Unit)> = results.iter().take(jobs).collect();
units.sort_unstable_by_key(|(id, _)| *id);
```
**Quadratic on large input.** A scan per item passes on a test file and takes minutes on a real one.\
**Bad**
```rust
let line = src[..span.start].matches('\n').count() + 1;
```
**Good**
```rust
let line = line_starts.partition_point(|&start| start <= span.start);
```

### Tests made to look good

**Expected value from the code.** An expectation computed by the formula under test proves only that the code equals itself.\
**Bad**
```rust
assert_eq!(beats_to_frames(b, bpm, rate), (b * 60.0 / bpm * rate) as u64);
```
**Good**
```rust
assert_eq!(beats_to_frames(1.0, 120.0, 48_000.0), 24_000);
```
**Golden blessed unread.** An accepted golden pins today's bug as the specification; the reviewer reads every changed golden line against the spec.\
**Bad**
```rust
if env::var_os("BLESS").is_some() { fs::write(&golden, &out)?; }
assert_eq!(out, fs::read_to_string(&golden)?);
```
**Good**
```rust
assert_eq!(emit(&graph), "osc 440 -> gain 0.5 -> out\n");
```
**Case that cannot fail.** An assertion behind a match or a guard passes when the code breaks.\
**Bad**
```rust
if let Ok(value) = eval("1 + 2") { assert_eq!(value, 3); }
```
**Good**
```rust
assert_eq!(eval("1 + 2"), Ok(3));
```
**Name that promises more.** The name claims the behaviour; the body proves a call returned.\
**Bad**
```rust
#[test] fn reload_keeps_state() { assert!(start(SRC).reload(SRC_V2).is_ok()); }
```
**Good**
```rust
#[test] fn reload_keeps_state() { let mut p = start(SRC); p.set(GAIN, 0.5); assert_eq!(p.reload(SRC_V2).unwrap().get(GAIN), 0.5); }
```
**One behaviour from many angles.** Ten tests of the trivial path stand in for the one rule that is real logic.\
**Bad**
```rust
#[test] fn lexes_plus() { assert_eq!(lex("+"), [Tok::Plus]); }
#[test] fn lexes_minus() { assert_eq!(lex("-"), [Tok::Minus]); }
```
**Good**
```rust
#[test] fn product_binds_tighter_than_sum() { assert_eq!(sexp("1 + 2 * 3"), "(+ 1 (* 2 3))"); }
```
**A clause no test isolates.** Each condition of a rule needs a row that fails when that condition alone is deleted; a row another condition already decides proves nothing about it.\
**Bad**
```rust
assert!(!resyncs("a / 2)"));
```
**Good**
```rust
assert!(!resyncs("a/b when c)"));
assert!(resyncs("a/b when c"));
```

### Fixes

A warm review checks each fix as a change of its own: the reported case, then the class of input the fix touches, then what the fix newly allows.

**A fix that moves the cost.** A fix tested on the reported shape leaves the same growth at the next shape: one copy per region becomes one per bracket group.\
**Bad**
```rust
assert_eq!(lexed(&deep_body_with_long_bracket()).stacks.len(), regions);
```
**Good**
```rust
for source in [deep_body_with_long_bracket(), deep_body_with_many_groups_on_one_line()] {
    assert!(lexed(&source).stack_entries() <= source.len());
}
```
**A fix that rejects valid input.** A recovery or validation rule tested only on broken input can split or refuse a valid program no reported case holds; every such rule runs over generated valid input too.\
**Bad**
```rust
assert_eq!(parse(BROKEN_BUNDLE).diagnostics.len(), 1);
```
**Good**
```rust
assert_eq!(parse(BROKEN_BUNDLE).diagnostics.len(), 1);
for variant in line_break_variants(&valid_corpus()) { assert!(parse(&variant).diagnostics.is_empty()); }
```
**A proposed fix stated as a mechanism.** A reviewer's fix names the property the code must hold and the valid input it must keep, checked against the specification first, because a mechanism copied as written fixes the reported case and breaks the next one.\
**Bad**
```text
Fix: end the region before a line whose first word is followed by a name.
```
**Good**
```text
Fix: inside an expression bracket, a line no expression continues ends the region; multi-line bundles and parenthesized placements (SPEC §7.6 to §7.8) keep parsing, shown by the line-break variants of the valid corpus.
```

### Rust

**Clone to quiet the borrow checker.** The copy costs an allocation per call and hides that the borrow is too wide.\
**Bad**
```rust
for node in self.nodes.clone() { self.visit(&node); }
```
**Good**
```rust
for node in &self.nodes { self.stats.visit(node); }
```
**Silent `as`.** `as` truncates integers, wraps signs and saturates floats, NaN to zero, without a word.\
**Bad**
```rust
let index = offset as u32;
```
**Good**
```rust
let index = u32::try_from(offset).map_err(|_| Error::OffsetTooLarge(offset))?;
```
**Derived equality over floats.** Derived `PartialEq` makes NaN unequal to itself and `-0.0` equal to `0.0`, so a change check, a memo or a dedup misfires.\
**Bad**
```rust
#[derive(PartialEq)] struct Param { value: f64 }
```
**Good**
```rust
struct Param { value: f64 }
impl PartialEq for Param { fn eq(&self, other: &Self) -> bool { self.value.to_bits() == other.value.to_bits() } }
```
**Derived recursion on deep trees.** Derived `Clone`, `PartialEq`, `Debug` and the drop glue recurse once per level, and a deep source overflows the stack.\
**Bad**
```rust
#[derive(Clone, PartialEq)] enum Expr { Int(i64), Add(Box<Expr>, Box<Expr>) }
```
**Good**
```rust
#[derive(Clone, PartialEq)] struct Exprs { nodes: Vec<Node> }
#[derive(Clone, PartialEq)] enum Node { Int(i64), Add(NodeId, NodeId) }
```
**Panic in a library path.** An index, `unwrap`, `expect`, slice or integer division on caller data aborts the host and kills the running program.\
**Bad**
```rust
let node = &self.nodes[id.index()];
```
**Good**
```rust
let node = self.nodes.get(id.index()).ok_or(Error::Dangling(id))?;
```
**Truncating adapters.** `zip` stops at the shorter side and drops the rest without a sign.\
**Bad**
```rust
for (port, value) in ports.iter().zip(values) { port.set(*value); }
```
**Good**
```rust
if ports.len() != values.len() { return Err(Error::Arity { expected: ports.len(), found: values.len() }); }
for (port, value) in ports.iter().zip(values) { port.set(*value); }
```
**Byte versus char indexing.** A byte index slices through a code point and panics, and a byte offset shown as a column is wrong past ASCII.\
**Bad**
```rust
let snippet = &line[..40];
```
**Good**
```rust
let snippet = line.char_indices().nth(40).map_or(line, |(end, _)| &line[..end]);
```
**`Rc` cycles.** A strong back edge keeps the graph alive after its owner drops, and every reload leaks a program.\
**Bad**
```rust
struct Node { parent: Option<Rc<RefCell<Node>>>, children: Vec<Rc<RefCell<Node>>> }
```
**Good**
```rust
struct Node { parent: Option<Weak<RefCell<Node>>>, children: Vec<Rc<RefCell<Node>>> }
```
**`#[allow]` over a real lint.** The attribute silences the lint for the whole item and every later line, and the defect it named stays.\
**Bad**
```rust
#[allow(clippy::cast_possible_truncation)] fn frames(len: u64) -> u32 { len as u32 }
```
**Good**
```rust
fn frames(len: u64) -> Result<u32, Error> { u32::try_from(len).map_err(|_| Error::TooLong(len)) }
```

### Compilers

**Cascading recovery.** A guessed type reports errors the user never made; skipping to the end of the file hides the ones they did.\
**Bad**
```rust
None => { self.report(Diag::UnknownName(span)); Ty::Int }
```
**Good**
```rust
None => { self.report(Diag::UnknownName(span)); Ty::Error }
(Ty::Error, _) | (_, Ty::Error) => Ok(()),
```
**Span drift through desugaring.** A desugared node with a dummy or a borrowed span points the diagnostic at the wrong code.\
**Bad**
```rust
Expr::call(Builtin::Pipe, [lhs, rhs], Span::DUMMY)
```
**Good**
```rust
Expr::call(Builtin::Pipe, [lhs, rhs], op_span.desugared(Desugaring::Pipe))
```
**Folding that disagrees with runtime.** A folder with its own arithmetic makes a constant and the same value computed at run time differ.\
**Bad**
```rust
(Op::Add, Const::Int(a), Const::Int(b)) => Const::Int(a.wrapping_add(b)),
```
**Good**
```rust
(Op::Add, Const::Int(a), Const::Int(b)) => Const::Int(runtime::add_int(a, b)?),
```
**Memo key missing context.** A key without the environment, the generic arguments or the target serves one context's answer to another.\
**Bad**
```rust
self.types.entry(expr).or_insert_with(|| infer(expr, env))
```
**Good**
```rust
self.types.entry((expr, env.id())).or_insert_with(|| infer(expr, env))
```
**Map order in emitted output.** Hash iteration order is seeded and differs per build or per run, so the output stops being reproducible.\
**Bad**
```rust
for (name, node) in &self.by_name { out.emit(name, node); }
```
**Good**
```rust
let mut names: Vec<_> = self.by_name.iter().collect();
names.sort_unstable_by_key(|(name, _)| *name);
```
**Native recursion over source depth.** One stack frame per nesting level lets a generated or hostile source overflow the stack.\
**Bad**
```rust
fn lower(&mut self, e: &Expr) -> Ir { match e { Expr::Neg(x) => Ir::neg(self.lower(x)), Expr::Int(n) => Ir::int(*n) } }
```
**Good**
```rust
let mut tasks = vec![Task::Lower(root)];
while let Some(task) = tasks.pop() { self.step(task, &mut tasks); }
```
**Exponential walk of a shared graph.** A DAG walked as a tree visits a shared node once per path, and a chain of diamonds doubles the work per step.\
**Bad**
```rust
fn latency(&self, n: NodeId) -> u32 { self.inputs(n).map(|i| self.latency(i)).max().unwrap_or(0) + self.delay(n) }
```
**Good**
```rust
for &n in &self.topo_order { latency[n] = self.inputs(n).map(|i| latency[i]).max().unwrap_or(0) + self.delay(n); }
```

### Real-time audio and music DSLs

**Hidden allocation or lock on the commit path.** `push` past capacity, `collect`, `format!`, `Box::new` and `lock` allocate or block for an unbounded time, and one stall is an audible dropout.\
**Bad**
```rust
self.pending.push(change);
let table = self.table.lock();
```
**Good**
```rust
self.pending.try_push(change).map_err(|_| CommitError::Full)?;
let table = self.tables.read_current();
```
**Deallocation on that path.** Dropping a `Box`, a `Vec` or the last `Arc` frees on the audio thread, and a reload is exactly when the old program drops.\
**Bad**
```rust
self.program = next;
```
**Good**
```rust
self.retired.enqueue(mem::replace(&mut self.program, next));
```
**Denormals.** A decaying feedback value enters the subnormal range and costs many times a normal operation; hardware flush flags do not exist on every target.\
**Bad**
```rust
self.y = x + self.feedback * self.y;
```
**Good**
```rust
self.y = flush_subnormal(x + self.feedback * self.y);
```
**Float results that differ across targets.** `std` transcendentals call the platform libm, and `mul_add` rounds once where `a * b + c` rounds twice and falls back to a platform routine.\
**Bad**
```rust
let y = phase.sin().mul_add(depth, offset);
```
**Good**
```rust
let y = libm::sin(phase) * depth + offset;
```
**NaN poisoning state.** One NaN or infinity in a feedback state never decays, and `clamp`, `min` and `max` pass or hide it.\
**Bad**
```rust
self.state = self.state * self.coef + input.clamp(-1.0, 1.0);
```
**Good**
```rust
let next = self.state * self.coef + input;
self.state = if next.is_finite() { next } else { 0.0 };
```
**Torn 64-bit reads.** Two 32-bit halves read apart see a carry between them and the position jumps by 2^32; a target without 64-bit atomics takes an emulated or sequence-locked read.\
**Bad**
```rust
let frame = (u64::from(self.hi.load(Acquire)) << 32) | u64::from(self.lo.load(Acquire));
```
**Good**
```rust
let frame = self.frame.load(Acquire);
```
**Ordering too weak for a publish.** A `Relaxed` flag lets the reader see the flag before the data; x86 hides it and ARM shows it.\
**Bad**
```rust
self.slot.write(program); self.ready.store(true, Relaxed);
if self.ready.load(Relaxed) { run(self.slot.read()); }
```
**Good**
```rust
self.slot.write(program); self.ready.store(true, Release);
if self.ready.load(Acquire) { run(self.slot.read()); }
```
**Time accumulated in floats.** Adding a float step per sample drifts by a rounding each step, and the beat lands late after minutes.\
**Bad**
```rust
self.seconds += 1.0 / self.rate;
```
**Good**
```rust
self.frame += 1;
let due = self.frame >= self.tempo.frame_of_beat(self.next_beat);
```
**Wall clock or unseeded randomness in deterministic code.** Output that reads the clock or the OS entropy differs on every run; a generator seeded from the program and the node does not.\
**Bad**
```rust
let noise = rand::random::<f32>();
```
**Good**
```rust
let noise = self.rng.next_f32();
```

## Sources

- Ross Bencina, real-time audio programming 101: http://www.rossbencina.com/code/real-time-audio-programming-101-time-waits-for-nothing
- Timur Doumler, using locks in real-time audio: https://timur.audio/using-locks-in-real-time-audio-processing-safely
- `assert_no_alloc`, forbidding allocation on a thread: https://docs.rs/assert_no_alloc
- `basedrop`, deferred deallocation off the audio thread: https://docs.rs/basedrop
- Matt Tytel, avoiding denormals: https://tytel.org/blog/2015/02/10/unexpected-optimization-01/
- JUCE forum, compounding timing error: https://forum.juce.com/t/offsetting-the-compounding-inaccuracy-of-samples-per-duration-of-time/45864
- Rust users forum, `ln_1p` differs by platform: https://users.rust-lang.org/t/f64-ln-1p-platform-specific-behavior/77113
- Factorio forum, platform FMA and desyncs: https://forums.factorio.com/viewtopic.php?p=703180
- Rapier, cross-platform determinism: https://rapier.rs/docs/user_guides/rust/determinism/
- LLVM issue 89885, folding `sin` unlike run time: https://github.com/llvm/llvm-project/issues/89885
- Mara Bos, Rust Atomics and Locks: https://oreilly.com/library/view/rust-atomics-and/9781098119430
- Linux, tearable 64-bit reads on x86_32: https://lkml.iu.edu/hypermail/linux/kernel/2404.1/02554.html
- Reproducible Builds, stable output order: https://reproducible-builds.org/docs/stable-outputs
- Learning Rust With Entirely Too Many Linked Lists, drop: https://cglab.ca/~abeinges/blah/too-many-lists/book/first-drop.html
- Rust users forum, recursive drop overflow: https://users.rust-lang.org/t/how-to-efficiently-drop-a-linked-list-like-structure-based-on-rc-without-triggering-stack-overflow-aborts/17348
- The Rust Book, reference cycles: https://rust-lang.github.io/book/ch15-06-reference-cycles.html
- rust-lang/rust 38066, char boundary panic: https://github.com/rust-lang/rust/pull/38066
- CVE-2026-53530, char boundary panic in a parser: https://www.miggo.io/vulnerability-database/cve/CVE-2026-53530
- Rust internals, `zip` truncation: https://internals.rust-lang.org/t/non-truncating-more-usable-zip/5205
- Clippy lint list: https://clippy.rust-lang.org/master/index.html
- Rust Reference, lint attributes: https://doc.rust-lang.org/reference/attributes/diagnostics.html
- RFC 1432, quadratic repeated insert: https://rust-lang.github.io/rfcs/1432-replace-slice.html
- Walter Bright, compiler error messages: https://digitalmars.com/articles/b47.html
- Norman Ramsey, eliminating spurious error messages: https://www.cs.tufts.edu/~nr/pubs/error-abstract.html
- rust-lang/rust 46732, recovery without follow-on errors: https://github.com/rust-lang/rust/pull/46732
- rustc dev guide, Salsa: https://rustc-dev-guide.rust-lang.org/salsa.html
- RUSTSEC-2026-0308, stale memoised results: https://www.wiz.io/vulnerability-database/cve/rustsec-2026-0308
- cargo-mutants, missed mutants: https://mutants.rs/using-results.html
- insta, snapshot review: https://docs.rs/insta
