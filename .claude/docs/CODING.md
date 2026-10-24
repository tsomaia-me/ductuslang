# Coding guidelines

What the owner expects of every line of code. Builders write to it, reviewers fail a diff against it. What the system is: `architecture/README.md`; looks and motion: `design/README.md`; commands and test runs: `running.md`.

## How you build

1. **Tests first.** A failing test for each behaviour that needs one (§ Tests), seen red, then the code, then green. A bug gets its failing test before its fix.
2. **Break it once.** After the code is green, undo each key rule in turn, see its test go red, restore it. Report each red as evidence. A test that cannot be made to fail is not a test.
3. **Reuse.** Look for an existing helper, component, hook or module before writing one.
4. **Abstract, then reuse.** When something very similar exists, extract the shared part and use it from both places instead of writing a near-identical clone. Two callers that change together share code; two that only look alike stay apart; one caller stays inline.
5. **Maintained libraries first.** Anything that is not Parlor Nights business logic (parsing, dates, validation, retries, crypto, fetching, forms, charts, animation, test and CI tooling) comes from a well-established library: released in the last year, widely used, typed, MIT, Apache or BSD, running where we need it (Workers, Expo and web, Node), its size counted against the app's bundle. Check what we already depend on first; a second library for the same job is worse than either. A library that shapes the stack (a framework, a state manager, a database layer, a UI kit) is the owner's call. Custom code for a generic job is an exception: only when nothing fits, small, used in more than one place and well tested, with what you looked at said in the PR or the hand-back, and the exception and its reasons recorded in the architecture doc (name it under Docs affected).
6. **No hard-coded product values.** A value that decides what the product does (a limit, a price, a reward, a threshold, a switch, who gets what) is an app config, never a literal. Code-specific values are named once in config or tokens. Nothing is specialised to one app: the platform hosts many.
7. **No comments** in any file we author. A name or a smaller function says it; why it is this way goes in the commit message and the hand-back.
8. **Best practice, not habit.** Follow the codebase's established patterns where they are good; where the codebase holds a worse practice, write the better one and say so.
9. **Every site, by grep.** Back every count, absence or every-site claim and every change to a shared value with a whole-tree grep by path, file-name stem and literal text (every caller of a function that gains an input, every reader and the writer of a pinned read or fallback you change, every journey, test and shots scene that matches, flows through, retries on or picks a label, link, control, config value or error text you rename or remove, package locale strings beside the app's, relative imports between packages that an import-graph scope misses), because hand counts, file-name audits and import-graph scopes each miss sites outside the files the change touches.

## Tests

The owner's bar: "I don't need trivial behaviors tested, like 'does useState work in this component, even though it works in 1000 others just like it?' ... I don't object testing real behavior, real logic. I object testing stupid stuff. I object testing stupid stuff from thousand angles even more. they are not pro-quality tests. they actually have diminishing returns, leaving less and less space for real, genuine tests. test that tickets the box 'I did add tests' for builder and nothing else - is a wrong test. test that tests genuine thing, behavior, logic whatever - is a good test. 10 tests that test the same thing - are wrong tests, 9 out of those 10, at least."

- A test proves real logic or behaviour that a player, a system would notice breaking, once. No test for what the framework or a library guarantees (state, rendering, routing working in one more component), for trivial wiring, for what another test already proves, and no test whose only job is that tests were added, because each costs every later run and every later reader and proves nothing a real test does not.
- Test behaviour through the public surface (the route, a component's props and what it renders, the engine's API), never private functions, class names, style objects, mock call counts or the wording of config and workflow files. Make every assertion and check tell apart exactly what it claims (a match on a value no generated id, timestamp or seed can contain, a removed key asserted with `toStrictEqual`, changed-or-current decided by a hash or a change time, never a count), because a fragment of a fixed id matches a random one on some seed, `toEqual` passes a key that should be gone and a same-size replace passes a count.
- State is tested through the API that owns it; a component's state through that component.
- One behaviour, one test, at the cheapest layer that proves it. The cases of one behaviour are one table holding only its boundaries and its distinct outcomes, because a row between two boundaries fails only when they do. A test that only repeats what another layer proves, or a second test of a behaviour already proven, is removed, and the PR or the hand-back names the test where the behaviour stays proven.
- A new test needs a setup no test in its file has. A case that shares an existing test's setup (the same seed, the same signed-in actor, the same rendered page or route family) is more assertions in that test, each an `expect.soft` so one failure hides none of the others, not a new test, because every test pays its setup again on every run and a second test on one setup adds a run and no proof. Reset everything a test or a proof scene changes (app definitions with `resetAppRegistry`, every Durable Object it uses, the config cache, spies per test in `isolate: false` files, every field of a fixture's shared state), and build its env and paths inside the test, never at describe time, because leaked state fails a later test or scene only in a full run and a describe-time path writes to the real /tmp.
- Each layer owns its rules and no other layer re-tests them, because a rule proven at two layers is two tests to change for one change: a pure server rule (validation, a calculation, a permission table) is unit-tested as its function, one table of its cases; an edge route test owns the wiring (one success through the route), the guard's refusal, idempotency and persistence; a component test owns one screen's behaviour against a mocked API and never re-tests a server rule; a journey owns a flow across screens and the client-server wiring; a package owns its pure logic and its consumers do not re-test it.
- A visual rule the owner set (contrast, a reserved height, a 44 px target) is tested once, where it lives.
- A value the owner ruled (a theme's colours, a size, a width, a breakpoint) is pinned once, in one table per subject in the test of the module that holds it, each row an `expect.soft` naming it, and is never removed as an echo, because the pin is what fails when the ruled value drifts, and without it only the Judge's eye catches the change.
- No sleeps and no fixed waits: wait on the event, the state, a locator or `expect(…)`.
- Every guarded path has a refusal test: the test that proves the guard refuses what it must, because a guard without one can be removed silently and every test stays green.
- An edge test imports the route it tests, not the Worker entry, so the import graph selects only its tests; the routing, middleware and auth chain are tested through the Worker entry.

## Good patterns

How good code looks in a `no_std` core with bit-identical results on every target, a commit path that never allocates or panics, and a lock-free kernel shared between threads. Each pattern gives its reason, the **Good** form and its **Bad** counterpart.

### Types

**Make invalid states unrepresentable.** A state the type cannot hold needs no check and no test.

**Good**
```rust
enum Voice { Idle, Playing { note: Note, started: Tick } }
```
**Bad**
```rust
struct Voice { playing: bool, note: Option<Note>, started: Option<Tick> }
```

**Newtypes for ids and units.** The compiler refuses a node id passed as an edge id and ticks passed as frames.

**Good**
```rust
pub struct NodeId(u32);
pub fn connect(from: NodeId, to: NodeId, delay: Ticks) -> Result<EdgeId, GraphError>
```
**Bad**
```rust
pub fn connect(from: u32, to: u32, delay: u64) -> Result<u32, GraphError>
```

**Parse, don't validate.** Check once at the boundary and return a type that carries the proof, so no later code checks again.

**Good**
```rust
pub struct Velocity(u8);
impl Velocity {
    pub fn new(raw: u8) -> Result<Self, OutOfRange> {
        (raw <= 127).then_some(Velocity(raw)).ok_or(OutOfRange { raw })
    }
}
pub fn note_on(pitch: Pitch, velocity: Velocity) -> Event
```
**Bad**
```rust
pub fn is_valid_velocity(raw: u8) -> bool { raw <= 127 }
pub fn note_on(pitch: u8, velocity: u8) -> Event
```

**Exhaustive matches on our own enums.** A new variant fails to compile at every site that must decide what it means.

**Good**
```rust
match event {
    Event::NoteOn(note) => voices.start(note),
    Event::NoteOff(note) => voices.release(note),
    Event::Tempo(tempo) => clock.set_tempo(tempo),
}
```
**Bad**
```rust
match event { Event::NoteOn(note) => voices.start(note), _ => {} }
```

### Ownership

**Borrow in the widest form.** A function that only reads takes `&[T]` or `&str`, so no caller clones to call it.

**Good**
```rust
fn total_length(notes: &[Note]) -> Ticks
let length = total_length(&score.notes);
```
**Bad**
```rust
fn total_length(notes: Vec<Note>) -> Ticks
let length = total_length(score.notes.clone());
```

**Ids into an arena, not shared pointers.** Nodes held in a `Vec` and linked by id borrow one field at a time, so the borrow checker agrees without `RefCell`, and they copy, compare and hash as integers.

**Good**
```rust
struct Graph { nodes: Vec<Node>, edges: Vec<Edge> }
struct Edge { from: NodeId, to: NodeId }
```
**Bad**
```rust
struct Node { inputs: Vec<Rc<RefCell<Node>>>, parent: Weak<RefCell<Node>> }
```

### Errors

**Errors as values with context.** A typed error names what failed and where, so the caller matches it and the user reads it.

**Good**
```rust
pub enum ParseError { UnexpectedToken { found: TokenKind, at: Span }, UnclosedBlock { opened: Span } }
```
**Bad**
```rust
return Err(String::from("parse error"));
```

**No panics in library code.** A lookup that can miss returns an error the caller handles; `unwrap`, `expect` and indexing stay out of library code.

**Good**
```rust
let voice = self.voices.get_mut(id.index()).ok_or(KernelError::UnknownVoice(id))?;
```
**Bad**
```rust
let voice = &mut self.voices[id.index()];
let tempo = self.tempo_map.at(tick).unwrap();
```

**Checked conversions and arithmetic.** `as` truncates silently and `+` overflows differently in debug and release; `try_from` and `checked_*` turn the overflow into an error.

**Good**
```rust
let count = u16::try_from(events.len()).map_err(|_| EncodeError::TooManyEvents)?;
let end = start.checked_add(length).ok_or(TimeError::Overflow)?;
```
**Bad**
```rust
let count = events.len() as u16;
let end = start + length;
```

### Functions

**Small, pure functions.** A result computed from the inputs alone is tested with one table, reused anywhere and the same on every target.

**Good**
```rust
fn ticks_to_frames(ticks: Ticks, tempo: Tempo, rate: SampleRate) -> Result<Frames, TimeError>
```
**Bad**
```rust
fn advance(&mut self) { self.frames = self.convert(); self.dirty = true; }
```

**Early returns.** The refusals come first and the main path stays unindented.

**Good**
```rust
let node = graph.node(id).ok_or(GraphError::UnknownNode(id))?;
let port = node.input(index).ok_or(GraphError::UnknownPort(id, index))?;
port.connect(source)
```
**Bad**
```rust
if let Some(node) = graph.node(id) {
    if let Some(port) = node.input(index) { port.connect(source) }
    else { Err(GraphError::UnknownPort(id, index)) }
} else { Err(GraphError::UnknownNode(id)) }
```

**Tables instead of branching ladders.** Data reads at a glance and changes in one row.

**Good**
```rust
const MAJOR_STEPS: [u8; 7] = [0, 2, 4, 5, 7, 9, 11];
let step = MAJOR_STEPS.get(degree).ok_or(ScaleError::Degree(degree))?;
```
**Bad**
```rust
let step = if degree == 0 { 0 } else if degree == 1 { 2 } else if degree == 2 { 4 }
    else if degree == 3 { 5 } else if degree == 4 { 7 } else if degree == 5 { 9 } else { 11 };
```

### Determinism

**Ordered maps wherever order reaches the output.** `BTreeMap` iterates in key order on every target and run; `HashMap` order follows its hasher. Anything else that reaches the output is sorted first.

**Good**
```rust
let ports: BTreeMap<PortName, NodeId> = collect_ports(&graph);
diagnostics.sort_by_key(|diagnostic| diagnostic.span);
```
**Bad**
```rust
let ports: HashMap<PortName, NodeId> = collect_ports(&graph);
```

**One float implementation everywhere.** Every transcendental goes through one software implementation, because the platform's `sin` and `exp` differ in the last bit between targets.

**Good**
```rust
let value = libm::sin(phase);
```
**Bad**
```rust
let value = phase.sin();
```

**Integer time.** Time counts ticks and frames as integers and becomes seconds only at the edge, because a float accumulator drifts with the block size.

**Good**
```rust
struct Clock { now: Frame, rate: SampleRate }
fn seconds(&self) -> Seconds { self.rate.seconds_at(self.now) }
```
**Bad**
```rust
struct Clock { seconds: f64, step: f64 }
fn tick(&mut self) { self.seconds += self.step; }
```

### Hot paths

**Preallocate when built, reuse when running.** Capacity is reserved once from the graph's limits; the commit path clears and refills it and writes into the slice the caller gives, so it allocates nothing.

**Good**
```rust
fn new(limits: &Limits) -> Self { Self { events: Vec::with_capacity(limits.events_per_block) } }
pub fn commit(&mut self, out: &mut [Sample]) -> Result<(), KernelError>
```
**Bad**
```rust
pub fn commit(&mut self) -> Vec<Sample>
let events: Vec<Event> = self.pending().collect();
```

**Lock-free handoff between threads.** The audio thread never waits on a lock: commands cross through a preallocated bounded queue that refuses when full instead of growing, and an atomic publishes with `Release` and reads with `Acquire`, so what the writer wrote before the store is visible after the load.

**Good**
```rust
producer.try_push(Command::SetGain(node, gain)).map_err(|_| ControlError::QueueFull)?;
self.ready.store(index, Ordering::Release);
let index = self.ready.load(Ordering::Acquire);
```
**Bad**
```rust
self.commands.lock().unwrap().push(Command::SetGain(node, gain));
self.ready.store(index, Ordering::Relaxed);
let index = self.ready.load(Ordering::Relaxed);
```

### Deep data

**Explicit stacks where depth is unbounded.** The input decides the depth of a tree, and recursion on it overflows the thread's stack; a `Vec` stack grows on the heap. Walks, clones, comparisons, hashes, printing and `Drop` all take this form, because a derived drop of an owned tree recurses once per level.

**Good**
```rust
let mut stack = vec![root];
while let Some(id) = stack.pop() {
    let node = tree.node(id);
    visit(node);
    stack.extend(node.children.iter().rev());
}
```
**Bad**
```rust
fn walk(tree: &Tree, id: NodeId) {
    tree.node(id).children.iter().for_each(|&child| walk(tree, child));
}
```

### Tests

**Expected values from the specification.** The expected value comes from the spec, the standard or a derivation by hand, never from the code's own output or its formula, because a value copied back passes whatever the code does.

**Good**
```rust
assert_eq!(pitch_to_hz(Pitch::A4), 440.0);
assert_eq!(pitch_to_hz(Pitch::A5), 880.0);
```
**Bad**
```rust
assert_eq!(pitch_to_hz(Pitch::A4), 440.0 * libm::pow(2.0, (69.0 - 69.0) / 12.0));
```

**One table per behaviour, holding its boundaries.** Each row is a boundary or a distinct outcome, so the table fails exactly where the rule changes.

**Good**
```rust
for (raw, expected) in [(0, Ok(0)), (127, Ok(127)), (128, Err(OutOfRange { raw: 128 }))] {
    assert_eq!(Velocity::new(raw).map(Velocity::get), expected, "raw {raw}");
}
```
**Bad**
```rust
#[test] fn velocity_0() { assert!(Velocity::new(0).is_ok()); }
#[test] fn velocity_64() { assert!(Velocity::new(64).is_ok()); }
#[test] fn velocity_100() { assert!(Velocity::new(100).is_ok()); }
```

**Tests through the public surface.** A test calls what a caller calls, so a refactor that keeps the behaviour keeps the test green.

**Good**
```rust
assert_eq!(compile("tempo 0"), Err(CompileError::TempoOutOfRange { at: Span::new(6, 7) }));
```
**Bad**
```rust
assert_eq!(lexer::scan_number(&mut Cursor::new("0")), Some(0));
```

### Names

**Names that say what a thing is.** Full words in the domain's terms, units in the type, a bool read as a question, and a test named for the behaviour it proves, so a failure says what broke without opening it.

**Good**
```rust
let releasing_voices = voices.iter().filter(|voice| voice.is_releasing());
fn tempo_at(&self, tick: Tick) -> Tempo
#[test] fn note_off_releases_only_the_matching_voice()
```
**Bad**
```rust
let tmp = voices.iter().filter(|v| v.flag2);
fn process(&self, t: u64) -> u32
#[test] fn test_voices_2()
```

## When this file is wrong

If a rule here forces worse code in a real case, write the better code, say why in the PR or the hand-back, and name the rule in a `Papercut candidate:` line.
