# The code standard

This document is the bar for every line of code in `packages/symphonyscript`, in every crate, as the SPEC is the bar for the language. The SPEC decides what the code does; this document decides how it is written. A builder reads it whole before it writes, and learns the form from it alone: no existing code, kept or old, is a model. A reviewer cites its lines by number. What a reviewer may report is `FINDINGS.md`.

A unit's page bounds what its builder touches. Inside that bound every line here holds.

The standard caps no stand-in for quality: no limit on a function's or a file's length, on a count of parameters or of fields. What it bounds is the product's own promise, and it bounds it with tests: work that grows in line with the input, the counts of 2.2, a time on the reference machine.

It has four parts: architecture (1), cost (2), the method from a SPEC sentence to code, with one worked example (3), and a builder's own tests (4).

## Words

| Word | Meaning |
|---|---|
| unit | One piece of work, built by one builder to one page. |
| page | The unit's design page, approved by the owner. It names what the unit builds and the SPEC sections it serves; what its stage takes and hands on; the shared tables, each with its writer; its imports and approved libraries; its invariants; its growth rows, clock rows and gate jobs; its budget in rounds; for kept code, each line it does not meet and the unit that owes it; the catalogue entries it proposes. |
| stage | A part of the compiler that takes listed inputs and hands on a result. |
| pass | One walk over a stage's input: one function, with the helpers only it calls. |
| table | A collection a pass fills and others read: a column, a side table, a map, an interner. |
| writer | The one function, with the helpers only it calls, that makes a table's entries (1.1). |
| lasting type | A type that owns a table, or that is a stage's result, or that is kept between two calls. The type of a table's entry and the type of its id belong to the table's layout and are listed with it. |
| scratch | What a function makes and nothing reads after it returns. A function may hand its scratch to its own helpers. The one buffer that outlives a call is 1.1's. |
| construct | What a mark names (3.3): a production, one of its alternatives, or a rule a SPEC sentence states. |
| kind | An enum that names the forms of the language or of a stage's result: a token kind, a node kind. |
| structure proposal | The builder's first hand-back (1.7). The accepted proposal is kept with the page, and its invariants are the page's. |
| stated test | A test that names the SPEC sentence it proves. The test author, who is never the builder, writes it before the build, and it is frozen. One stated test is one case file (4.8). |
| rule list | The test author's list of the unit's rules, one entry a SPEC sentence or a grammar alternative, quoted. Each entry has its stated tests. |
| growth row, clock row | The two frozen tests of cost (2.6). |
| gate | The checks that run on the reference machine: on every pushed commit, and the named jobs when their page says (4.7). |
| reference machine | The one machine a time is taken on: `wb2`. |
| corpus | The robustness corpus: valid programs and their mutations, run as a gate job for no panic, a whole result and the source reproduced. |
| round | One hand-back of the builder and its review. |
| READY | The builder's hand-back of a unit it holds finished. |
| catalogue | The one list of the compiler's messages (3.8). |
| kept code | Code older than the unit that the unit's page keeps (3.7). |
| core crate, runtime | `symphonyscript-core`, which builds without `std` and holds the Program and the runtime; the runtime is the code that evaluates a Program on the host's device. |
| peer | The session that writes pages with the owner. |

## 1. Architecture

**1.1 Written once, by one owner.** Every table has one writer: one function, with the helpers only it calls, that makes its entries. Two passes that fill one table are two tables, or one pass. A table may grow, and an entry once written never changes. A finished result takes no `&mut` and holds no interior mutability, and no pass takes a finished result by value and returns it changed: what a later pass learns goes into a table of its own. Scratch dies in its function, with one exception: a buffer a function needs on every call comes in as an argument, empty, and goes back empty. Its capacity is reused, and nothing reads its contents between two calls.

**1.2 Dependencies point one way.** A stage imports the results of earlier stages, the base layer and the libraries 1.10 allows, and nothing else. Its page lists them. The base layer holds ids, spans, hashing and message order, and imports no stage.

**1.3 Nothing ambient.** What holds for the length of a call is an immutable argument, never a field written for a callee to read. No stage reads a global, the environment or a file. No result depends on a hash map's order, an address, a thread or a clock. A value computed at compile time uses the float functions the runtime uses, `libm`'s, never the platform's.

**1.4 One rule, one implementation.** A sentence's list of forms is data, and the rule is one function over it. Where the SPEC says "as X is", the code calls X's code. A message has one constructor. No fact is derived twice.

**1.5 A fault is a value.** A fault leaves a placeholder, or a region, in the data. A placeholder has no type and no value: no type is kept behind a fault, and no value is tried. Silence comes only from that data: a check that needs a fact a placeholder would give finds none and draws nothing, and a fault that reads no placeholder draws. Nothing reads the message list to decide what to check, draw or build. Nothing de-duplicates it or keeps a flag beside it. Ordering the list for output, and returning failure when it is not empty, are allowed at the compiler's edge and nowhere else.

**1.6 Every construct is handled or refused by name.** In a stage a construct has one handler: one function, with the helpers only it calls. A construct whose production holds itself, directly or through others, cannot wait in a call for its inside (2.4): it leaves an entry on a stack and is entered again each time a part of its inside is complete. Its handler is then one type, that entry, with the one `impl` block that holds every way into it, and its mark stands above the block. What it remembers between two entries is held in the type's own fields. The stack's enum has one variant a construct, never one a place inside a construct to resume at. Outside its block an entry is only handed back by a chooser or asked a question beside that enum. Constructs that one sentence lists as the forms of one rule are rows of data and share that rule's handler (1.4). Code that picks among handlers is a chooser, and it does none of the handlers' work: each arm of the `match` that picks is one call or one value. Applying what a handler hands back, and moving to the next piece of input, are the chooser's own. There is no fallback path: a construct a stage does not handle is refused by its name, with the catalogue's message for it, and never falls to another construct's handler or to a default. No `match` on one of our own enums has a catch-all arm. An `if let`, a `let … else` or a `matches!` that names a variant of one of our own enums is a catch-all arm, and so is an `==` or a `!=` against such a variant where it chooses among constructs. A variant in a pattern is written with its enum's path, never through a glob import, because a misspelt bare name is a binding, and a binding is a catch-all arm the compiler accepts. One form of `==` is allowed, the expectation: at a grammar position that takes one exact token, the code compares the kind with that one variant, and the other branch is that position's fault and picks no handler. A position that takes one of a few exact tokens is a run of expectations, and the run ends in a fault drawn by name, never in a default that accepts. Each question about a kind is answered once, by a function beside the enum that names every variant, so that a new variant stops the build at every question it must answer.

**1.7 The page names the edges, and the proposal names the inside.** The page names what crosses a stage's edge, and the tables that stages share, each with its one writer. Inside a stage, the builder's first hand-back is a structure proposal: the lasting types, each with its one writer and its layout (2.3), the passes in their order, the counts of 2.2, and the invariants. It is read cold against parts 1 to 3 before any body is written. The builder builds to the accepted proposal. A lasting type or a pass the accepted proposal does not hold comes in by an amendment: the same hand-back, read the same way, before its body is written.

**1.8 A value means one thing, and its type says which.** An id is typed by its table and made only by that table's owner. A file-local id is used bare only by a function that holds one file's tables. Anywhere else it travels as a pair with its file, made only by that file's table, and a table of more than one file takes only the pair. No number means something by its range. No node is read by arithmetic on a position: a view finds a child by its role. A child's role is what the node's maker recorded for it, a field or a tag; a count among siblings is a position. Packing stays behind the owner's accessors.

**1.9 Decided once.** Each decision is made once, at the first stage whose listed inputs settle it. Until then it travels in a neutral form the page names. No stage guesses, and none reaches ahead.

**1.10 Written or borrowed.** The core crate takes nothing but `alloc` and `libm`. Generic jobs at the edges come from libraries: Unicode tables, hashing, big integers, shortest-float printing, the editor protocol, the command line, checksums, fetching. The compiler's own machinery is written, never borrowed: the lexer, the layout, the lossless tree, recovery, the evaluator, inference. A builder never adds a dependency: it proposes one, with the job it does. The page lists the approved ones. Each was checked, before the page was approved, to exist in that form and version, and one on a path that reaches a value was checked to give the same result on every target and in every run. The Unicode tables are on such a path: they decide which identifiers are valid, and the NFC form of paths. Their version changes only as the language does.

**1.11 Impossible states.** An impossible state is typed away where Rust can type it. Each one that is left goes through the one call that names the invariant and draws `internal-error`, and its site is listed in its module's invariants. `unwrap`, `expect`, `unreachable!`, `panic!` and an assertion are never that call.

**1.12 No panic, in two rules.**

- The compiler: no input draws a panic, and the corpus tests it. A lookup by a typed id is direct indexing through the table owner's one accessor, under three invariants the module lists: only the owner makes an id, and only for a row that exists; a table never shrinks; a bare id never meets another file's table. The compiler's public entry is the one place that catches a panic, and no other code does.
- The runtime: no path can panic. It runs on devices, the compiler's licence to index does not reach it, and every lookup there has an answer for every input.

## 2. Cost

**2.1 Three sizes.** A compilation has three sizes: the source, the expansion under SPEC §4.6's limits, and the values built. A stage's work grows in line with the size its page names, broken input included. In line with a size means at most a fixed multiple of it. A sort stands only where the page names it.

**2.2 The counts.** For each node and each expanded item the builder counts, and its proposal states:

| Count | Allowed |
|---|---|
| passes | a fixed number |
| allocations | none a node and none an expanded item; a built value allocates in line with its own new elements, the third size; a table grows by doubling |
| hash operations | one, when text becomes an id; an expanded item resolves nothing again |
| copies of source text | none |

A node here is what a stage makes one of for a piece of the source: a token, a tree node, a declaration. A function that needs a buffer on every call takes it by 1.1's one exception, so that the call allocates nothing.

**2.3 The default layout.** Unless the page names another: a dense column indexed by id, in the order the source fixes, for what every id has; a side table appended in id order and searched by halving, for what few have; an interner. A hash map's key is content, never an id, a node or an address. The hash and its key are the page's.

**2.4 No recursion on depth.** No function calls itself, directly or through others, on a path whose depth the source or an option sizes. Only such a path takes a stack: a production holds itself when it lies on a cycle of the grammar's productions, and a production that cannot hold itself reads its parts by calls, in the order its production writes them. A call into a machine, one loop with its own stack, that never calls back is no self-call. The page lists each production of its stage with its form, an entry on a stack or a function. The functions Rust writes count: a type that holds itself through a `Box` is dropped, cloned and compared by self-calls on that depth, so no lasting type holds itself. Each stage whose input nests carries a check of 100,000 levels on a 1 MiB stack, a gate job (4.7).

**2.5 A loop states its total.** A loop whose turn is not constant says in a comment what its total is and why the input bounds it, and holds the counter its page's growth row names. The comment is a claim, and the reviewer checks it as one.

**2.6 Growth rows and clock rows.** A growth row counts work at four doubling sizes, on any machine. A clock row takes a time, on the reference machine only. Both are frozen tests. Each states what the design promises. It is never set from what the code under test happens to cost, and no builder edits one.

**2.7 Budgets.** A budget is the page's, never a builder's guess. Kept code holds the time it was measured at before the unit. A new stage carries growth rows and the counts of 2.2, and a clock ceiling only when its page states one. Peak bytes a node is a growth row.

**2.8 Who runs what.** The builder runs the growth rows itself, on any machine. The gate runs the clock rows on every pushed commit. Review starts only on a tip the gate marked green; a red tip goes back to the builder and spends no round. A builder has no shell on the reference machine, unless its unit's job is speed and its page gives it one, under the gate's lock.

## 3. From a SPEC sentence to code

**3.1 The breakdown.** Code comes from the sentence, never from an example, a test or old code. For each rule of its page the builder writes a breakdown before it opens its tests and before any body. It is a list and not prose: each form and each condition, with its section. It takes in every sentence that speaks of the construct, the prose beside a production included; each alternative; each item of a list; each position the construct stands in. The builder hands it back with the structure proposal (1.7), whose reviewer compares it with the test author's rule list. A form in one list and not the other is settled before any body. The breakdown is not kept as a record: what survives it is the marks in the code (3.3).

**3.2 A branch that changes a program's fate has a source.** A program's fate is what it is accepted as, what it means and what it draws. A branch that changes any of the three has a source: a SPEC sentence or an invariant of the page. Its source is the mark of the home it stands in (3.3), or the mark of the row of data it reads. One condition, one branch. A fate-changing branch with no source is not written: no recovery guess, no leniency, no default. A condition with no branch is a miss. A branch that changes none of the three, a fast path or a table that grows, needs no source.

**3.3 One home.** A rule has one home, found before writing. The home is the one place the rule's mark stands: on the line above a function, an `impl` block, an arm or a row of data. The code under the mark, and the helpers only it calls, are the rule's. The home, and no other code but the test of 4.2, carries the mark:

```text
// SPEC §<section> "<the sentence's or clause's first words, as the SPEC writes them>"
// SPEC §<section> <Production>
// SPEC §<section> <Production> "<one alternative's first symbols>"
// PAGE "<the invariant's first words>"
```

The quoted words are enough to be the only match in their section, in their production, or in the page with its accepted proposal. The third form marks the home of one alternative, where a production's alternatives have different homes; the production's own mark then stands where it refuses what is none of them. A mark is compared whole, to the end of its line. One search of the source for a mark finds the home, and shows when a rule has two. A case file carries the mark of the rule it proves (4.8), and the search does not count it. A new form goes through the home as data.

**3.4 The SPEC is never completed in code.** Where the SPEC is silent or contradicts itself, the builder writes a GAP that quotes the sentences. The answer lands as a SPEC sentence or a stated test. The round is counted apart.

**3.5 Where the standard is silent,** in this order:

1. Behaviour is a SPEC sentence or a GAP, never the builder's choice.
2. What lasts is the page's, then the accepted proposal's; anything else is an amendment to the proposal (1.7).
3. A body meets the lines of part 1, then the counts of 2.2.
4. Then the form a reader can check against its sentence with the least else in view wins. A reviewer may raise this step, with a concrete consequence.
5. Only what is left after that is equal: the builder takes either, and no reviewer prefers one.

Two lines of the standard that cannot both be met go to the owner through the peer.

**3.6 A page held wrong** draws an OBJECTION: the page's line, what it breaks, a concrete consequence, the better design. The builder builds neither what it holds wrong nor its own design on that part, and goes on with the rest. An accepted proposal found wrong goes the same way.

**3.7 Kept code.** The page lists each line the kept code does not meet, and the unit that owes it. The builder fixes what is listed for its unit and nothing in passing. New functions and types meet every line. A count of 2.2 that kept code fails binds it after the unit its page names, and that unit's page carries the row.

**3.8 Messages.** There is one catalogue. A page proposes entries, the owner approves them with the page, and a builder writes none. A stated test compares a message's code and its place, never its words.

**3.9 Comments.** A comment states what the code cannot: an invariant relied on, the reason for a choice that looks odd, the SPEC section implemented, a meaning neither a name nor a type can carry. It never restates code and never tells history. A comment restates code when a reader of the code alone loses nothing by its removal. A module opens with its job, what it takes and hands on, and its invariants. A comment that restates code is a finding, and so is a relied-on invariant nobody wrote.

**3.10 The three lines a builder writes.**

| Line | Against | Holds |
|---|---|---|
| GAP | the SPEC | the sentences, quoted, where it is silent or contradicts itself (3.4) |
| DISPUTE | a frozen test | the SPEC sentence the test contradicts, quoted; the builder never edits the test |
| OBJECTION | the page, the accepted proposal, or two lines of the standard that cannot both be met | the line, what it breaks, a concrete consequence, the better design (3.6) |

### 3.11 The worked example

One piece of a unit, written twice: as this standard asks, and as it is written unasked.

The piece is cut from the unit that reads expressions. It reads the operators of SPEC §5.1, levels 2 to 10, over names, numbers and parentheses. The real unit also holds level 1, level 11, `is` and `is not` and every other `Primary`, in the same table and on the same stack; the cut leaves them out to stay short. The example's types are its own and are no design: the real tokens and the real tree are the parser's page's, and where a syntax fault stands and what it leaves of its line are SPEC §16's.

The unit takes one file's token table, whose last row is its end and holds no token, with brackets paired by the lexer. It hands on the expression's nodes, appended to the file's tree, the root, and the first token it did not take.

**Step 1: the breakdown (3.1).** Every sentence of the SPEC that speaks of how these operators are written, as a list. The sentences on what they mean (§4, §5.2, §5.3) are the pages' of the units that type and evaluate.

| Form | Operators | The table's row | Operands, from the production |
|---|---|---|---|
| §5.1 OrExpr | `or` | level 2, left | AndExpr |
| §5.1 AndExpr | `and` | level 3, left | NotExpr |
| §5.1 NotExpr | prefix `not` | level 4, prefix | NotExpr |
| §5.1 CmpExpr, alternative `CmpOp Fallback` | `<` `<=` `>` `>=`; `is` and `is not` are left out of the cut | level 5, none | Fallback |
| §5.1 Fallback | `??` | level 6, right | left AddExpr, right Fallback |
| §5.1 AddExpr | `+` `-` `without` | level 7, left | MulExpr |
| §5.1 MulExpr | `*` `/` `div` `%` | level 8, left | UnaryExpr |
| §5.1 UnaryExpr | prefix `-` | level 9, prefix | UnaryExpr |
| §5.1 PowExpr | `**` | level 10, right | left Postfix, right UnaryExpr |
| §5.1 Primary, alternatives `NumberLit`, `Name` and `'(' Expr ')'` | | | the others are left out of the cut |

| Condition | What it asks of the code |
|---|---|
| §5.1 "Comparisons do not chain" | A second operator of level 5 after a complete comparison is an error. |
| §5.1 PowExpr | The right operand of `**` is a UnaryExpr, one level looser than `**`. A prefix `-` stands there, and §4.1 writes `2 ** -1`. The table alone does not say so. |
| §5.1 NotExpr | `not` stands only where a NotExpr does: at the start, and after `(`, `or`, `and` and `not`. `a + not b` is no expression. The table alone does not say so. |
| §5.1 "`-2 ** 2` is −4" | The operand of a prefix `-` takes a `**`. |
| §5.1 "binds looser than arithmetic and tighter than a comparison" | `port ?? 80 > 1024`, `x ?? base + 1` and `x ?? -1` are the table's levels 5 to 7 and 9; they ask nothing more. |
| §5.1 Primary | The inside of `( )` starts again at the loosest level. |
| §5.1 "An `if` that is an operand of another operator is parenthesized", and §5.10's sentence for `for` | Level 1. Left out of the cut: neither word is among its tokens. |
| §5.1 "`-x ?? 1` is an error" | An error of types. The tree is `(-x) ?? 1`. Another unit's. |
| §2.6 "A minus sign is always an operator", and §4.1 "A minus applied directly to a literal forms one value" | `n-1` arrives as three tokens, the lexer's. The tree keeps the minus and the literal apart, and the fit check that reads them as one value is another unit's. |
| §2.5 "Everywhere else it is a keyword" | `and`, `or`, `not`, `div` and `without` arrive as their own token kinds. The lexer's. |

Positions: an expression stands in many places, and each belongs to the code that reads it. This piece starts at the token its caller names and reports where it stopped.

**Step 2: the structure proposal (1.7).** One pass over the tokens: `expression`, with its handlers and helpers. One table: `Tree`, a dense column of `Node` indexed by `NodeId`, in the order the nodes are completed; its writer is `expression`, through `Tree::push`. One buffer of 1.1's exception: `Scratch`, whose capacity lasts and whose contents are the pass's scratch: a stack of pending operators and of open constructs. Every production of the cut lies on a cycle of the grammar, through `'(' Expr ')'`, so each construct that has an inside leaves an entry: the operators, rows of one rule, share `Op`, and the group's is `Group`. Counts: one pass; no allocation a token or a node; no hash operation; no copy of text. Invariants: the seven the module opens with. The first three are the page's, of what the unit takes and hands on.

**Step 3: the code.** Constructors for `Tree` and `Scratch` are left out.

```rust
//! The reader of operator expressions: SPEC §5.1, levels 2 to 10. The standard's worked example.
//!
//! Job: read the one expression that starts at a given token and append its nodes to the tree.
//! Takes: one file's token table, the token to start at, the file's tree, and scratch.
//! Hands on: `Expr`, the root's id and the first row the expression does not take. The root
//! of a faulty expression is a `Node::Fault`, and nothing else records the fault.
//!
//! Invariants
//! - The token table's last row is its end, which holds no token, and the table never
//!   shrinks. `Toks::end` is the only way to make a table. Only `Toks` makes a `TokId`, and
//!   only for a row that exists.
//! - An expression that is not faulty ends before the first token that can continue it
//!   nowhere, or at the table's end, and that row is the caller's. A faulty one ends
//!   before the row its fault stands at.
//! - Brackets arrive paired: every `Open` has its `Close` later in the table. The lexer's.
//!   Relied on at the `internal` call in `Group::resume`, this module's one site of an
//!   impossible state.
//! - Only `Tree::push` writes the nodes, and it only appends. A `NodeId` is made only there,
//!   as its return, so it names a row for as long as the tree lives, and a node's operands
//!   have smaller ids than the node.
//! - A token table and a tree are one file's, and an id never meets another file's table.
//! - `Scratch::stack` is empty when `expression` is entered and when it returns.
//! - A faulty expression leaves the nodes it had completed in the column, under no root. How
//!   a later pass knows them is SPEC §16's and the parser's page's.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokKind {
    Name, Number,
    Or, And, Not,
    Lt, Le, Gt, Ge,
    Fallback, // `??`
    Plus, Minus, Without,
    Star, Slash, Div, Percent,
    Pow, // `**`
    Open, Close,
}

/// A level of §5.1's table. A larger level binds tighter.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Level(u8);

/// Where an expression and the inside of a group start: the loosest level of this cut.
const LOOSEST: Level = Level(2);

/// The table's third column. A right-associative row carries the level its right operand
/// starts at, read from its production. On a row of the two other kinds that level is the
/// next tighter one.
#[derive(Clone, Copy)]
enum Assoc { Left, Right { operand: Level }, None }

/// A prefix row of the table. Its operand starts at its own level: `X ::= op X | Y`.
#[derive(Clone, Copy)]
struct Prefix { level: Level }

/// An infix row of the table.
#[derive(Clone, Copy)]
struct Infix { level: Level, assoc: Assoc }

/// What a token is where an operand is expected.
enum Starts { Leaf, Group, Prefix(Prefix), Nothing }

/// What a token is where an operator is expected.
enum Continues { Infix(Infix), Stops }

/// What arrives where an operator is expected and is none: the table's end, or a token
/// with the row after it.
enum Stop { End, Token { kind: TokKind, after: TokId } }

const fn prefix(level: u8) -> Starts {
    Starts::Prefix(Prefix { level: Level(level) })
}

const fn infix(level: u8, assoc: Assoc) -> Continues {
    Continues::Infix(Infix { level: Level(level), assoc })
}

const fn right(operand: u8) -> Assoc {
    Assoc::Right { operand: Level(operand) }
}

// The two questions the parser asks of every token's kind. Each is answered here and nowhere
// else, and each names every variant, so a new kind stops the build until both are answered.
// The rows of §5.1's table are the arms: the prefix rows in `starts`, the infix rows in
// `continues`. An arm holds a row's constants and no logic.
impl TokKind {
    const fn starts(self) -> Starts {
        match self {
            // SPEC §5.1 Primary "Name"
            Self::Name => Starts::Leaf,
            // SPEC §5.1 Primary "NumberLit"
            Self::Number => Starts::Leaf,
            Self::Open => Starts::Group,
            // SPEC §5.1 NotExpr
            Self::Not => prefix(4),
            // SPEC §5.1 UnaryExpr
            Self::Minus => prefix(9),
            Self::Or | Self::And | Self::Lt | Self::Le | Self::Gt | Self::Ge | Self::Fallback
            | Self::Plus | Self::Without | Self::Star | Self::Slash | Self::Div
            | Self::Percent | Self::Pow | Self::Close => Starts::Nothing,
        }
    }

    const fn continues(self) -> Continues {
        match self {
            // SPEC §5.1 OrExpr
            Self::Or => infix(2, Assoc::Left),
            // SPEC §5.1 AndExpr
            Self::And => infix(3, Assoc::Left),
            // SPEC §5.1 CmpExpr "CmpOp Fallback"
            Self::Lt | Self::Le | Self::Gt | Self::Ge => infix(5, Assoc::None),
            // SPEC §5.1 Fallback
            Self::Fallback => infix(6, right(6)),
            // SPEC §5.1 AddExpr
            Self::Plus | Self::Minus | Self::Without => infix(7, Assoc::Left),
            // SPEC §5.1 MulExpr
            Self::Star | Self::Slash | Self::Div | Self::Percent => infix(8, Assoc::Left),
            // The right operand of `**` is a UnaryExpr, one level looser than `**` itself,
            // so a `-` stands there: `2 ** -1`.
            // SPEC §5.1 PowExpr
            Self::Pow => infix(10, right(9)),
            Self::Name | Self::Number | Self::Not | Self::Open | Self::Close => Continues::Stops,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TokId(usize);

#[derive(Clone, Copy)]
enum Row { Token(TokKind), End }

/// What `Toks::read` finds at a row.
pub enum Read {
    /// A token, and the row after it: a token is never the last row.
    Token { kind: TokKind, after: TokId },
    /// The table's last row.
    End,
}

/// The example's stand-in for one file's token table. Its owner is the lexer.
pub struct Toks { rows: Vec<Row> }

impl Toks {
    /// The lexer's last step.
    pub fn end(kinds: Vec<TokKind>) -> Toks {
        let mut rows: Vec<Row> = kinds.into_iter().map(Row::Token).collect();
        rows.push(Row::End);
        Toks { rows }
    }

    pub fn first(&self) -> TokId {
        TokId(0)
    }

    pub fn read(&self, at: TokId) -> Read {
        match self.rows[at.0] {
            Row::Token(kind) => Read::Token { kind, after: TokId(at.0 + 1) },
            Row::End => Read::End,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeId(usize);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Node {
    /// A name or a number.
    Leaf(TokId),
    Prefix { op: TokId, operand: NodeId },
    Infix { op: TokId, left: NodeId, right: NodeId },
    /// Stands where a faulty expression would. It has no type and no value.
    Fault(Fault),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fault { pub code: FaultCode, pub at: TokId }

/// The example's stand-in for codes of the catalogue, which a page proposes (3.8).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FaultCode {
    OperandExpected,
    CloserExpected,
    ComparisonChain,
    /// An invariant of the compiler's own did not hold. The text names it.
    Internal(&'static str),
}

/// The example's stand-in for the compiler's one call for an impossible state (1.11).
fn internal(invariant: &'static str, at: TokId) -> Fault {
    Fault { code: FaultCode::Internal(invariant), at }
}

pub struct Tree { nodes: Vec<Node> }

impl Tree {
    // PAGE "Only `Tree::push` writes the nodes"
    fn push(&mut self, node: Node) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    pub fn node(&self, id: NodeId) -> Node {
        self.nodes[id.0]
    }
}

/// A pending operator: it waits for its right operand.
#[derive(Clone, Copy)]
enum Op {
    Prefix { at: TokId, row: Prefix },
    /// The left operand travels in the entry, so there is no stack of operands to run empty.
    Infix { at: TokId, row: Infix, left: NodeId },
}

/// What becomes of a pending operator when an infix operator arrives after its right operand.
enum Meets { Waits, Completes, Chains }

// Every comparison of two levels is in this block.
// SPEC §5.1 "From loosest to tightest"
impl Op {
    /// The level this operator's right operand starts at.
    fn operand(self) -> Level {
        match self {
            Op::Prefix { row, .. } => row.level,
            Op::Infix { row, .. } => match row.assoc {
                Assoc::Left | Assoc::None => Level(row.level.0 + 1),
                Assoc::Right { operand } => operand,
            },
        }
    }

    /// Whether an operator of `level` can stand inside an operand that starts at `floor`.
    fn inside(level: Level, floor: Level) -> bool {
        level >= floor
    }

    /// What `next`, arriving after this operator's right operand, makes of this operator.
    fn meets(self, next: Infix) -> Meets {
        if Op::inside(next.level, self.operand()) {
            return Meets::Waits;
        }
        match self {
            Op::Prefix { .. } => Meets::Completes,
            Op::Infix { row, .. } => match row.assoc {
                Assoc::Left | Assoc::Right { .. } => Meets::Completes,
                // The comparisons' row is the table's one row with no associativity.
                // SPEC §5.1 "Comparisons do not chain"
                Assoc::None if next.level == row.level => Meets::Chains,
                Assoc::None => Meets::Completes,
            },
        }
    }

    /// `operand` is this operator's complete right operand.
    fn over(self, operand: NodeId) -> Node {
        match self {
            Op::Prefix { at, .. } => Node::Prefix { op: at, operand },
            Op::Infix { at, left, .. } => Node::Infix { op: at, left, right: operand },
        }
    }
}

/// An entry of the stack: a pending operator, or a construct whose inside is being read.
#[derive(Clone, Copy)]
enum Pending { Op(Op), Nest(Nest) }

impl Pending {
    /// The level an operand starts at when this entry is the innermost.
    fn floor(self) -> Level {
        match self {
            Pending::Op(op) => op.operand(),
            Pending::Nest(nest) => nest.floor(),
        }
    }
}

/// The constructs whose inside nests: one variant a construct, and never one a place inside
/// a construct to resume at.
#[derive(Clone, Copy)]
enum Nest { Group(Group) }

// The questions the stack's readers ask of an open construct.
impl Nest {
    /// The level an operand starts at directly inside this construct.
    fn floor(self) -> Level {
        match self {
            Nest::Group(group) => group.floor(),
        }
    }

    /// This construct's inside is complete, and `why` arrives after it.
    fn resume(self, inside: NodeId, at: TokId, why: Stop) -> Step {
        match self {
            Nest::Group(group) => group.resume(inside, at, why),
        }
    }
}

/// What `expression` keeps between two calls: its stack's capacity, so that an expression
/// allocates nothing once the stack has grown. Nothing reads the stack between calls.
pub struct Scratch {
    stack: Vec<Pending>,
    /// The operators completed: the counter a growth row reads.
    #[cfg(test)]
    turns: u64,
}

impl Scratch {
    /// The level an operand read now starts at: the innermost entry's, or an expression's own.
    fn floor(&self) -> Level {
        match self.stack.last() {
            Some(top) => top.floor(),
            None => LOOSEST,
        }
    }

    fn turn(&mut self) {
        #[cfg(test)]
        {
            self.turns += 1;
        }
    }
}

pub struct Expr {
    pub root: NodeId,
    /// The first row the expression does not take. A faulty expression's fault stands at it.
    pub next: TokId,
}

/// What `expression` expects at the next token. An operand is in hand only where an operator
/// is expected.
#[derive(Clone, Copy)]
enum Expect { Operand, Operator(NodeId) }

/// What a handler hands back for one token.
enum Step {
    /// An entry for the stack. The token is taken, and an operand is expected at the row
    /// after it.
    Push(Pending, TokId),
    /// A complete operand. The token is taken, and an operator is expected at the row
    /// after it.
    Operand(NodeId, TokId),
    /// The expression's root. It ends before this token.
    Done(NodeId),
    Fault(Fault),
}

// PAGE "`Scratch::stack` is empty"
pub fn expression(toks: &Toks, from: TokId, tree: &mut Tree, scratch: &mut Scratch) -> Expr {
    let mut at = from;
    let mut expect = Expect::Operand;
    // Total: one turn a row at most, since a turn that does not return moves to the next row.
    // The work under a turn is one handler's, and `complete` states the total of the loops
    // the handlers hold.
    loop {
        let step = match expect {
            Expect::Operand => match toks.read(at) {
                Read::Token { kind, after } => match kind.starts() {
                    Starts::Leaf => leaf(tree, at, after),
                    Starts::Group => Group::open(after),
                    Starts::Prefix(row) => prefix_operator(scratch, at, after, row),
                    Starts::Nothing => Step::Fault(no_operand(at)),
                },
                Read::End => Step::Fault(no_operand(at)),
            },
            Expect::Operator(operand) => match toks.read(at) {
                Read::Token { kind, after } => match kind.continues() {
                    Continues::Infix(row) => infix_operator(tree, scratch, operand, at, after, row),
                    Continues::Stops => {
                        stop(tree, scratch, operand, at, Stop::Token { kind, after })
                    }
                },
                Read::End => stop(tree, scratch, operand, at, Stop::End),
            },
        };
        match step {
            Step::Push(entry, after) => {
                scratch.stack.push(entry);
                expect = Expect::Operand;
                at = after;
            }
            Step::Operand(node, after) => {
                expect = Expect::Operator(node);
                at = after;
            }
            Step::Done(root) => return Expr { root, next: at },
            Step::Fault(fault) => {
                scratch.stack.clear();
                return Expr { root: tree.push(Node::Fault(fault)), next: at };
            }
        }
    }
}

// SPEC §5.1 Primary
fn no_operand(at: TokId) -> Fault {
    Fault { code: FaultCode::OperandExpected, at }
}

fn leaf(tree: &mut Tree, at: TokId, after: TokId) -> Step {
    Step::Operand(tree.push(Node::Leaf(at)), after)
}

/// `a and not b` is an expression, and `a + not b` is none.
fn prefix_operator(scratch: &Scratch, at: TokId, after: TokId, row: Prefix) -> Step {
    if Op::inside(row.level, scratch.floor()) {
        Step::Push(Pending::Op(Op::Prefix { at, row }), after)
    } else {
        Step::Fault(no_operand(at))
    }
}

fn infix_operator(
    tree: &mut Tree,
    scratch: &mut Scratch,
    operand: NodeId,
    at: TokId,
    after: TokId,
    row: Infix,
) -> Step {
    match complete(tree, scratch, operand, row) {
        Left::Operand(left) => Step::Push(Pending::Op(Op::Infix { at, row, left }), after),
        Left::Chain => Step::Fault(Fault { code: FaultCode::ComparisonChain, at }),
    }
}

fn stop(tree: &mut Tree, scratch: &mut Scratch, operand: NodeId, at: TokId, why: Stop) -> Step {
    let (node, under) = close(tree, scratch, operand);
    match under {
        // PAGE "An expression that is not faulty ends before"
        Under::Nothing => Step::Done(node),
        Under::Nest(nest) => nest.resume(node, at, why),
    }
}

/// An open `(`.
#[derive(Clone, Copy)]
struct Group;

// SPEC §5.1 Primary "'(' Expr ')'"
impl Group {
    /// `(` arrives where an operand is expected.
    fn open(after: TokId) -> Step {
        Step::Push(Pending::Nest(Nest::Group(Group)), after)
    }

    /// Its inside is an `Expr`, which starts at the loosest level.
    fn floor(self) -> Level {
        LOOSEST
    }

    /// Its inside is complete, and `why` arrives after it.
    fn resume(self, inside: NodeId, at: TokId, why: Stop) -> Step {
        match why {
            Stop::End => Step::Fault(internal("brackets arrive paired", at)),
            // After a group's inside this position takes one token, `)`: an expectation.
            Stop::Token { kind, after } => {
                if kind == TokKind::Close {
                    self.close(inside, after)
                } else {
                    Step::Fault(Fault { code: FaultCode::CloserExpected, at })
                }
            }
        }
    }

    /// A group makes no node of its own: it is its inside.
    fn close(self, inside: NodeId, after: TokId) -> Step {
        Step::Operand(inside, after)
    }
}

/// What `complete` hands back: `next`'s left operand, or word that `next` chains a comparison.
enum Left { Operand(NodeId), Chain }

/// `next` arrives after `operand`.
fn complete(tree: &mut Tree, scratch: &mut Scratch, mut operand: NodeId, next: Infix) -> Left {
    // Total: an operator is pushed once and completed once, and one token pushes at most one,
    // so the turns of this loop and of `close`'s over a file are at most the file's tokens.
    loop {
        let top = match scratch.stack.last() {
            Some(&Pending::Op(top)) => top,
            // The operators under an open construct wait until its inside is complete.
            Some(Pending::Nest(_)) | None => return Left::Operand(operand),
        };
        match top.meets(next) {
            Meets::Waits => return Left::Operand(operand),
            Meets::Chains => return Left::Chain,
            Meets::Completes => {}
        }
        scratch.stack.pop();
        scratch.turn();
        operand = tree.push(top.over(operand));
    }
}

/// What `close` found under the operators it completed.
enum Under {
    /// An open construct, which `close` popped.
    Nest(Nest),
    Nothing,
}

fn close(tree: &mut Tree, scratch: &mut Scratch, mut operand: NodeId) -> (NodeId, Under) {
    // Total: counted with `complete`'s turns; see there.
    loop {
        let top = match scratch.stack.pop() {
            Some(Pending::Op(top)) => top,
            Some(Pending::Nest(nest)) => return (operand, Under::Nest(nest)),
            None => return (operand, Under::Nothing),
        };
        scratch.turn();
        operand = tree.push(top.over(operand));
    }
}
```

**What its cases state.** The test author's, shown here for the reader. A builder writes none of them (4.1).

| Tokens | Result |
|---|---|
| `a or b and c` | `or(a, and(b, c))` |
| `a - b - c` | `-(-(a, b), c)` |
| `a ?? b ?? c` | `??(a, ??(b, c))` |
| `-2 ** 2` | `neg(**(2, 2))` |
| `2 ** -1` | `**(2, neg(1))` |
| `not a < b` | `not(<(a, b))` |
| `(a < b) < c` | `<(<(a, b), c)` |
| `a < b < c` | a fault, `ComparisonChain`, at the second `<` |
| `a + not b` | a fault, `OperandExpected`, at `not` |
| `(a b)` | a fault, `CloserExpected`, at `b` |
| `a + b) c` | `+(a, b)`, stopped at `)` |

**What to see in it.**

- The table is data (1.4). The rows of §5.1's table are arms of `starts` and `continues`, each under the mark of its production or of its alternative, and an arm holds constants and no logic. Every comparison of two levels is in `impl Op`. Each fault code is built in one place.
- The questions about a kind sit beside the enum (1.6). Two are asked of `TokKind`, each names every variant, and no other `match` names one. No `match` has a catch-all arm, there is no `if let`, no `let … else` and no `matches!`, and every variant of our own enums is written with its path.
- `==` on a kind stands once, as an expectation (1.6): in `Group::resume`, after a group's inside, the position takes `)` and nothing else, and the other branch is that position's fault. `(a b)` draws `CloserExpected` at `b`.
- `expression` is a chooser (1.6). Each arm that picks is one call or one value: the arms on what a token starts, on how it continues, and on the table's end. The handlers it calls are `leaf`, `prefix_operator`, `infix_operator` and `stop`, the group's handler is its type, entered here through `Group::open`, and `no_operand` refuses. The `match` that applies a step names no token and no construct.
- The group is one type with one block (1.6). `Group` is the entry an open `(` leaves on the stack, and its `impl` block, under its mark, holds every way into it: where it opens, the level its inside starts at, where it is resumed and where it closes. Outside that block the group is named in seven lines: its own declaration, the two declarations that list it, `Starts` and `Nest`, the row that classifies `(`, and three arms that are one call each. The operators read `Pending::Nest` and never which construct is open.
- The stack is explicit (2.4). 100,000 opening parentheses with their closers, 100,000 prefix `-` and 100,000 `**` are read on a 1 MiB stack, and so are eight times as many: the depth is a `Vec`'s length.
- A fault is a value (1.5). `a < b < c` yields a tree whose root is `Node::Fault`, with its code and its place. No list of faults stands beside the tree, and no flag.
- Impossible states are typed away, and one that is left is named (1.11). An infix operator carries its left operand in its stack entry, and `Expect::Operator` carries the operand in hand, so there is no stack of operands to run empty. `close` returns only when it has popped an open construct or emptied the stack, and `Under` says which, so `stop` has no arm for an operator. An operator's one refusal is `Meets::Chains`, and `complete` hands it on as `Left::Chain`, so neither can carry a fault code that an operator cannot draw. Only a right-associative row carries an operand level, so no other row can carry a wrong one. `Toks::read` hands back a token together with the row after it, or the table's end with none, and a step that takes a token carries that row, so no step moves past the last row. The table's end inside an open group is not typed away: it goes through `internal`, and the module's invariants list the site. `( a` draws it.
- No input draws a panic (1.12). `Toks::read` and `Tree::node` index directly, each as its owner's one accessor, and nothing else indexes. Only `Toks` makes a `TokId` and only `Tree::push` a `NodeId`. Every sequence of up to five of the cut's twenty token kinds is read with no panic and leaves the stack empty.
- Marks (3.3). Every mark stands on the line above its home, and no mark stands twice. Each row of §5.1's table carries the mark of its production or of its alternative, the block that holds every comparison of two levels carries the table's, and the arm that finds a chain carries its sentence's. The rows that start a leaf and the group's block carry the marks of `Primary`'s alternatives, and `no_operand`, which refuses what starts no operand, carries `Primary`'s own. Three invariants carry marks.
- The loops state their total (2.5). `expression`'s loop takes one turn a row at most. The loops under it, `complete`'s and `close`'s, state their total, and `Scratch::turn` counts the operators they complete: over a run of prefix `-` and chains of `**`, of `+` and of `** -`, each at 100,000 to 800,000 levels, that count never passes the tokens.
- Allocation (2.2). Once its stack has grown, and while the tree's column has room, an expression allocates nothing: 900,000 tokens of sums, products, powers, prefix `-` and groups are read with no allocation. The stack is 1.1's buffer and is empty when `expression` returns, after a fault too. From an empty stack and an empty column the same tokens are read with fewer than 60 allocations.
- The two conditions the table does not show are constants in the rows: `2 ** -1` is read by `**`'s operand level, and `a + not b` is refused by `not`'s level.

**What it does not settle.** A faulty expression leaves the nodes it had completed in the column, as the module's last invariant says. Where a syntax fault stands, and what the tree keeps of its line, are SPEC §16's and the parser's page's. How a growth row reads the counter is the gate's. The ids are `usize` to keep the example free of a conversion: an id's width, and the one home of the conversion that makes it, are the base layer's.

**The same piece, as it is written unasked.** It passes a first look and the tests of valid programs.

```rust
// Its tokens are a slice that ends with `End`, and its nodes hold `usize` where the good
// form holds `TokId` and `NodeId`.
struct Parser<'a> {
    toks: &'a [TokKind],
    pos: usize,
    nodes: Vec<Node>,
    errors: Vec<String>,
    failed: bool,
}

impl Parser<'_> {
    fn parse_or(&mut self) -> usize {
        let mut left = self.parse_and();
        while self.toks[self.pos] == TokKind::Or {                        // [2] [8]
            self.pos += 1;
            let right = self.parse_and();
            left = self.binary(TokKind::Or, left, right);
        }
        left
    }

    // parse_and, parse_add and parse_mul are parse_or again, with other operators.    [1]
    // parse_not is parse_unary with `not`, and parse_fallback is parse_pow with `??`.

    fn parse_cmp(&mut self) -> usize {
        let left = self.parse_fallback();
        let op = self.toks[self.pos];
        if !matches!(op, TokKind::Lt | TokKind::Le | TokKind::Gt | TokKind::Ge) {
            return left;
        }
        self.pos += 1;
        let right = self.parse_fallback();
        if matches!(self.toks[self.pos], TokKind::Lt | TokKind::Gt) {     // [3]
            self.errors.push("comparison operators cannot be chained".to_string()); // [4]
            self.failed = true;                                           // [5]
        }
        self.binary(op, left, right)
    }

    fn parse_unary(&mut self) -> usize {
        if self.toks[self.pos] == TokKind::Minus {
            self.pos += 1;
            let operand = self.parse_unary();                             // [6]
            return self.unary(TokKind::Minus, operand);
        }
        self.parse_pow()
    }

    fn parse_pow(&mut self) -> usize {
        let base = self.parse_primary();
        if self.toks[self.pos] == TokKind::Pow {
            self.pos += 1;
            let exponent = self.parse_pow();                              // [6] [7]
            return self.binary(TokKind::Pow, base, exponent);
        }
        base
    }

    fn parse_primary(&mut self) -> usize {                                // [11]
        match self.toks[self.pos] {
            TokKind::Open => {
                self.pos += 1;
                let inner = self.parse_or();                              // [6]
                self.pos += 1; // the `)`                                    [10]
                inner
            }
            TokKind::Name | TokKind::Number => self.leaf(),
            _ => {                                                        // [8]
                if !self.failed {                                         // [5]
                    self.errors.push("expected an expression".to_string());
                }
                self.failed = true;
                self.leaf()                                               // [9]
            }
        }
    }

    fn leaf(&mut self) -> usize {
        self.nodes.push(Node::Leaf(self.pos));
        self.pos += 1;
        self.nodes.len() - 1
    }
}
```

| | What is wrong | Line |
|---|---|---|
| 1 | One function a level. The rule that a left-associative level folds to the left is written four times, and a fifth level is a fifth copy. | 1.4 |
| 2 | `self.toks[self.pos]` is an index that nothing keeps in range: `a +` panics. | 1.12 |
| 3 | The chain check names two of the four operators: on `a < b <= c` it draws nothing. The list of forms is written twice, away from the enum, and the copies differ. | 1.4, 1.6, 3.1 |
| 4 | A message's words, written in place. | 1.4, 3.8 |
| 5 | The fault is a flag and a line in a list, and the tree knows nothing of it: on `a < b < c` the function returns `<(a, b)` as a sound comparison. The flag is a field one method writes for another to read, and `parse_primary` reads it to stay silent, so once it is set a missing operand draws nothing. | 1.3, 1.5 |
| 6 | `parse_unary` and `parse_pow` call themselves, and `parse_primary` calls `parse_or`. 100,000 `-`, `**` or `(` overflow a 1 MiB stack. | 2.4 |
| 7 | The right operand of `**` is read as a PowExpr, so `2 ** -1` draws "expected an expression". The function was written from the table and never from the production. | 3.1 |
| 8 | A catch-all arm on our own enum, and `==` used to choose whether an operator follows, which is the same arm in another spelling. A new token kind falls into both unseen. | 1.6 |
| 9 | A guess with no source: it makes an operand of a token that is none, so `2 ** -1` yields `**(2, -)`, and the program's meaning from there on is the parser's invention. | 3.2 |
| 10 | Whatever stands there is taken for the closer: `(a b)` loses `b` and draws nothing. A condition with no branch, at the one place where an expectation was owed. | 1.6, 3.2 |
| 11 | A node's id and a token's position are both `usize`, and nothing stops one being used as the other. | 1.8 |
| 12 | No mark, no module opening, no invariant written. A reader cannot find a rule's home, or the sentence a branch comes from. | 3.3, 3.9 |

## 4. A builder's own tests

**4.1 Behaviour is proved only by stated tests.** A builder keeps no test that takes a program. No test, anyone's, takes its expected value from a run, from another implementation or from a recorded output. A property copies no value, and is allowed where the SPEC promises the agreement: two runs agree; the answer is the same with the top-level items reordered.

**4.2 The one kind of test a builder keeps** is a lasting type's invariant. The invariant is one the accepted proposal lists. The test sits in that type's module, carries the invariant's `// PAGE` mark, calls only that module, works its expected values from the invariant by hand, takes no program, file or clock, and is never ignored.

**4.3 Trials are deleted before READY.** A builder's trial programs and trial tests do not stay. A trial program that shows a wrong answer no stated test catches goes to the test author with its sentence, and becomes a stated test before the fix.

**4.4 The code under test is the code that ships.** The source has no branch on `cfg(test)` but 2.5's counters, and no code that recognises a test's input.

**4.5 A builder's test is no evidence.** The gate counts the entries of the rule list whose stated tests pass, and nothing else. The reviewer is told nothing of a builder's tests and reads them as code. They are not frozen.

**4.6 The loop before a commit.** Before a commit a builder runs its unit's cases and its invariant tests. That run costs the rebuild of the code it edited, one link and the run itself, and nothing a builder adds makes it cost a second build or a second link.

**4.7 No test is ignored.** A check slow by nature, a growth row's largest size, the 100,000 levels, the corpus or a nested build, is a named gate job on a page, with its machine and when it runs.

**4.8 A stated test is data, not code.** It is a case file: the program, the expected tree or the expected codes and places, and its sentence's mark. One harness binary a stage runs the cases, so a case added or changed compiles nothing.

**4.9 Where a stage's code lives,** a crate of its own or a module, is its page's. A builder makes no crate and moves no code across that line.

**4.10 Old tests.** An old test that breaks a line is listed on the page of the unit that retires it, with the regression it caught and the stated test that takes over. Until that unit deletes it, it runs as it is, and nobody adds to it.
