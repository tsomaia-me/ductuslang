# The cold review of the second loop drafts

One cold reviewer's report on the second draft set (`LOOPS.md`, `CODING.md`, `TESTING.md`, `running.md` and six agent files), word for word below the line, saved by the peer session on 2026-11-13. The owner refused that set before the report came back, and its `CODING.md`, `builder.md` and `reviewer.md` are deleted, so the report's line numbers point at files that are gone. Nothing here is a rule and the drafts it reads are no base.

How to read it:

- Moot, since the owner refused every cap on a stand-in for quality and settled what a comment is for: A5, A7, A11's length conflict, A19, A20, and every point that only fits a lint to a refused numeric rule.
- For the code standard's conversation (`code-standard-brief.md`): A1 and the last part of B (the one shared object returns inside a single page module, and a field cap is met by nesting); A16 (a stage that takes the last stage's object by value and returns it, and interior mutability, are the shared object in another form); A12 (the work is linear in the program and the values it builds, not in the source, and a linear tree walk holds two loops); A2 and A3 (which bar binds old code a repair touches); A17 (a reviewer's question must answer no on correct code); A18 (the route when a rule of the standard cannot be met); E8 (an example must keep the SPEC).
- For the held loop, test and agent files when they are revised: C1 to C7, D1 to D6, E1 to E7, E9, F1 to F5, G1 to G6, H1 to H4, J. The report read the second set and not the first, so each point is checked against the text in hand before it is used.
- Checked by the reviewer with clippy 0.1.94 and usable wherever a lint is named: A8, A9, A10, and the lint facts under Coverage.

---

# Cold review of the peer-loops drafts (`/home/cdagent/cargo-targets/peer-loops-draft/`)

## Verdict

The drafts close most of what went wrong in the last build: they bind the test author, keep tests frozen, separate the roles and reject the old CODING text. They do not yet meet the owner's later requirement (5), and as written they would not stop finding 6.

- **Finding 6 can recur.** The gate checks structure only on the items the design page lists. Inside a module folder the builder chooses every type and every way state flows, and no rule bounds that. The 253-field object with 1,100 `&mut self` methods can come back inside one page module, split into structs of 12 fields or fewer.
- **Several named checkers don't do what the rule says.** I checked these against clippy 0.1.94: `excessive_nesting`, `min_ident_chars`, `iter_over_hash_type`, `indexing_slicing` and `arithmetic_side_effects`.
- **Some rules can't all be met at once.** Examples follow.
- **A few file examples break their own rules or the SPEC.**
- **The baseline lets an old breaching function grow without limit.** That matters most for the trial, which repairs existing code.

Severity runs roughly from high to low inside each section.

---

## A. Owner requirement 5: judgment left to the builder, and checkers that cannot check

**A1. The builder still designs all internal structure.**
- Where: CODING.md:18-19 says "The design page decides the structure. The builder decides none of it." and "Inside a module's folder the items are the builder's."
- builder.md:33 says "You write the bodies and the private items inside a module's folder."
- builder.md:77 asks for "Decisions: each call you made inside a module's folder that a reader would not expect".
- Why it's wrong:
  - Finding 6 lived inside one module: an internal `Core` that every sub-stage mutates.
  - No CODING rule covers how private items share state. "A stage returns what it builds" binds only page-listed entry functions and output types (CODING.md:23-24).
  - The builder must therefore judge decomposition, which is exactly what the owner said it must not do.
  - "a reader would not expect" is a judgment too.
  - No rule says how fine-grained a page's modules must be. A page with one module, `semantics`, makes every structure rule vacuous.
- Fix must hold: a checkable rule for private code. Examples of the property:
  - no type passed as `&mut` to more than N functions;
  - no `&mut self` method on a type with more than K fields reachable through nesting;
  - a size cap per module folder.
  - Also a rule on page grain, so that every stage the code has is a page module.

**A2. Editing old breaching code is two-way.**
- Where: CODING.md:14 says "Code written before these rules is on the gate's baseline: … no count rises."
- Why it's wrong:
  - In a repair, which is what the trial is, the builder edits functions that already breach the rules: `bool` parameters, `as`, wildcard arms, lengths over 60 lines.
  - In the r3 syntax code outside tests I count about 51 `as` casts, 19 `bool`-parameter functions, 56 `_ =>` arms, about 101 `±1` lines and about 12 functions over 60 lines.
  - Nothing says whether the builder must fix a breach in a function it touches or may leave it. That is the "code outside your change stays as it is" judgment the owner objected to.
- Fix must hold: one stated reading for code the change touches, for example "a function the diff changes meets every rule", with no choice left.

**A3. The baseline does not ratchet, and doesn't stop growth.**
- Where: CODING.md:14 and LOOPS.md:41 ("whose counts never rise").
- The count is per file and per breach kind:
  - A 200-line function counts 1 and can grow to 900 lines.
  - A file over 400 lines counts 1 and can grow.
  - Removing five `as` casts leaves room for five new ones, because the baseline file stays as it was and only the page may list a change to it (LOOPS.md:34).
  - Moving old code into a new file is refused ("a new file starts at none"). Any file split in a repair therefore forces a rewrite of every moved line, or a page entry.
- Fix must hold:
  - Counts ratchet down automatically at each green commit.
  - Growth inside an item that already breaches is refused: a length rule measured per function, not as a count.
  - The rule for moved code is stated.

**A4. The names rule asks the reviewer to judge, and it refuses ordinary code.**
- Where: CODING.md:214-216, "A name is a whole word of the SPEC or the page", with the reviewer question "is any name a word that neither the SPEC nor the page uses for the thing?"
- Why it's wrong:
  - Implementation names such as `stack`, `cursor`, `checkpoint`, `seen`, `cause` and `duplicates` are not SPEC words "for the thing". Several are the file's own Good forms (lines 140, 158, 111).
  - The answer is yes almost everywhere, so the reviewer either floods findings or quietly applies judgment.
  - "for the thing" has no yes-or-no answer.
- Fix must hold: a name rule a reader can apply without judging meaning, or a page vocabulary that covers implementation terms.

**A5. `min_ident_chars` at 3 refuses the file's own Good forms and SPEC words.**
- Where: CODING.md:215.
- Verified on clippy 0.1.94: the lint fires on struct names, field names, enum variant names and fields, bindings and generic parameters.
- At 3 it refuses:
  - `at` and `by` in the Good `Bracket` (CODING.md:186);
  - `id` in CODING.md:110 and 141;
  - every generic `T`;
  - SPEC terms `to` (§8.3) and `s`/`ms` (§1.3, §3.6).
- Fix must hold: a threshold or allow-list consistent with the Good forms and the SPEC's own short words.

**A6. `disallowed_names` bans `flag`, which is a SPEC word.**
- Where: CODING.md:214, "no `tmp`, `data`, `info`, `flag`, `ctx`".
- §7.4 is "Flags", and the production is `Flags?`.
- The two halves of one rule contradict each other.
- Fix must hold: the banned list holds no SPEC term.

**A7. `excessive_nesting` at 4 doesn't count what the rule says.**
- Where: CODING.md:76, "Blocks nest at most 4 deep."
- By clippy's documentation of the lint, an inline `mod`, an `impl` and the fn body each count as a level.
- Inside an `impl` method, `while let … { match … { Arm => { if … {` already reaches 5, and the lint fires.
- An explicit-stack parser loop, which CODING.md:134 requires, hits this constantly.
- The builder can't predict the checker from the rule's words.
- I couldn't set a threshold without writing a clippy.toml, so this is from the lint's documented counting, not a run.
- Fix must hold: the rule's text states the lint's counting, or the threshold allows for impl and fn levels.

**A8. `arithmetic_side_effects` is broader than the rule, and the rule is two-way.**
- Where: CODING.md:91, "`+`, `-`, `*` on the compiler's own counts, offsets and ids outside a newtype's methods", against CODING.md:168, "A bare integer is a count".
- Verified: the lint fires on every integer `+`, including `i + 1` on a bare count. It doesn't fire on floats.
- So `count += 1` is refused on the very thing the file calls a bare count.
- "written once in the newtype" leaves open whether an inline `saturating_add` on a count is allowed.
- The lint also fires on program-value arithmetic (CODING.md:127) and can't tell the two kinds apart.
- Fix must hold: the rule's text equals what the lint refuses, and it says how a count is incremented.

**A9. `iter_over_hash_type` misses most hash-order leaks.**
- Where: CODING.md:96.
- Verified: it fires on `for (k, _) in map`, but not on `map.keys().copied().collect()`. By extension the same holds for `.values()`, `.iter().map()`, `extend(map)`, `for_each` and `.drain()`.
- Output order can still follow the hasher.
- Fix must hold: a checker that refuses any iteration of a hash container, for example `disallowed_methods` on `HashMap::iter`, `keys`, `values` and `drain`, or `disallowed_types` on `HashMap`/`HashSet` with lookups through `BTreeMap`.

**A10. `indexing_slicing` misses map indexing.**
- Where: CODING.md:89, "an index `a[i]`".
- Verified: `map[&1]` on a `HashMap` draws nothing, and it panics on a miss.
- Fix must hold: map `Index` refused too, for example through `disallowed_methods` on the `Index` impls or a script.

**A11. `wildcard_enum_match_arm` plus `too_many_lines` at 60 plus "a match names every variant" can't be met for large enums.**
- Where: CODING.md:92, 74 and 211.
- r3 `SyntaxKind` has about 182 variants, and rustfmt puts one alternative per line (see `syntax/parser/starts_line.rs:11-22`).
- A full match is therefore 180 lines or more, and any function holding one is refused.
- The only escapes defeat the rule: `matches!`, where the lint doesn't fire (verified), `if let`, or `let … else`.
- The lint also fires on foreign enums such as `std::cmp::Ordering` (verified), which is broader than "an enum of ours". That is harmless.
- Fix must hold: a stated form for classifying over a large enum that passes every rule, with `matches!` and `if let` on our enums either allowed or refused explicitly.

**A12. "Linear in the source" is false for a correct compiler of this SPEC.**
- Where: CODING.md:152-153, "does any loop over something the source sizes hold another such loop".
- Compile-time `for`, pattern uses and recursion expand up to 1,000,000 iterations and 4,194,304 elements (SPEC §4.6). The work is linear in the expansion, not the source.
- Nested source-sized loops are also normal in linear walks. The file's own Good walk (CODING.md:140-145) runs `stack.extend(children)` inside a loop over nodes: two loops over source-sized things, so the answer is yes.
- The question flags correct code, and the verifier's gate 1 ("the words are there and say what the finding claims") will admit it.
- Fix must hold: state the bound as REVIEWING.md did ("linear in the program and the values it builds"), and phrase the question so a linear tree walk answers no.

**A13. The scope of the lints and scripts is undefined: do they apply to tests and the harness?**
- Where: CODING.md:98, 103-105, 77, 79, against TESTING.md:58, "Gate: a cost case reads no clock, by the lint that refuses the clock".
- If the lints and scripts apply to tests:
  - The harness can't read case files, the corpus or the 700,000 inputs, because the file system is banned.
  - A case's program and expected tree are "a string of words outside the diagnostics module".
  - Cases repeat 12 lines.
  - Rules files can pass 400 lines.
  - The speed rows need the clock.
- If they don't apply, TESTING.md:58 has no checker.
- Fix must hold: each rule names the targets it binds (src, tests, harness, benches, build scripts), and the cost-case no-clock check works in that scope.

**A14. Script rules with no definable match.**
- Where:
  - CODING.md:104, "a string of words outside the diagnostics module". Is that one word, two words, or any space?
  - CODING.md:105, "a kind, attribute, unit or enum name of any vocabulary as a string — script over the fixture vocabulary's names".
- The fixture names include `s`, `at`, `open`, `file`, `rate`, `mode` and `label` (SPEC §1.3).
- An exact match refuses ordinary strings. A substring match refuses diagnostic text such as "left open", and this row has no diagnostics-module exemption.
- The keyword table (`"channel"`, `"out"`, §2.5) collides with any vocabulary that names an attribute `channel` (§2.5 says one may).
- Fix must hold: each script's match is stated exactly, with its exemptions.

**A15. The import rule's checker misses ways to import.**
- Where: CODING.md:40-41, "every `use crate::` path of a file is in the page's import list".
- `super::`, `self::`, fully qualified `crate::a::f()` in expressions and `pub use` re-exports all bypass it.
- Fix must hold: the script resolves every path to a module, not only `use crate::` lines.

**A16. The gate rule for "a stage returns what it builds" checks less than the rule says.**
- Where: CODING.md:23-24.
- The gate refuses `&mut` in the signature only. By-value threading still passes: `fn infer(r: Resolved) -> Resolved` mutates and returns the previous stage's object, which is the shared-object pattern in owned form. `needless_pass_by_value` doesn't fire when the value is mutated.
- `Cell`/`OnceCell` in an output type also pass.
- The script has to read the page's item list, and no machine-readable page format is specified.
- Fix must hold:
  - entry functions take only `&` references, as the rule's text says;
  - interior mutability is refused;
  - a page format the script can read.

**A17. Reviewer questions with no yes-or-no answer, or that fire on correct code everywhere.**
- CODING.md:182: "of which one combination never occurs" needs semantic judgment.
- CODING.md:194: "is one condition on one value tested in two functions?" answers yes for every `kind == Newline` check in a parser.
- CODING.md:38: "a second implementation of its own job" is judgment.
- CODING.md:169: "Is any value read at a position computed by arithmetic?" answers yes for every parser lookahead `get(position + 1)`.
- CODING.md:200: a `zip` with a range or with `repeat` answers yes.
- Fix must hold: each question answers no on the file's Good forms and on ordinary correct idioms, decided by syntax a reader can see.

**A18. CODING has no path when a rule cannot be met.**
- Where: CODING.md:12, "A rule that cannot be met in a real case is a QUESTION to the runner". LOOPS.md:99 sends QUESTION to "the peer, who answers from the page or amends it".
- The peer can't amend CODING, which is the owner's text. A11 and A5 will produce these QUESTIONs.
- The unit stalls, and the lost session isn't "counted apart" (LOOPS.md:111 covers only a wrong case or a page gap).
- Fix must hold: a defined route to the owner for a CODING rule, and the round counted apart.

**A19. Numeric limits given to the builder.**
- Where: CODING.md:74-79 (60, 5, 4, 400, 12).
- The owner's own principle (brief 5): "a builder told a number writes to the number." Expect functions of 58-60 lines and files of 395-400 lines.
- Not a contradiction of an owner order, since that ruling concerned the 320-line stop. Put it to the owner.

**A20. Comments are still banned, though the review names that as a cause.**
- Where: CODING.md:103.
- compiler-review §2 cause 6: "a 938-line function, no comments by rule. Each fresh session must relearn it." Its recommendation 3 asks for "one map of what each field means".
- The drafts keep the ban and give private fields no meaning anywhere. The page covers only public items.
- Put it to the owner.

---

## B. Would it have stopped the last engine (finding 6), rule by rule

At the commit that made `semantics/pipeline/core.rs` (848 lines, `Core` at lines 34-295 with 253 `pub` fields) and `values.rs` (3,268 lines):

| What the engine did | Checker under the drafts | Refused? |
|---|---|---|
| `Core` passing 12 fields | the 12-field struct script (CODING.md:78) | Yes, at the commit adding field 13 |
| `core.rs` over 400 lines, `values.rs` 3,268 lines | the file script (CODING.md:77) | Yes |
| `evaluate_frames`, 938 lines (values.rs:697) | `too_many_lines` at 60; `excessive_nesting` | Yes |
| values.rs:794, `Some(at) if at as usize >= self.ir.lets.len()`, then `at as usize - …` | `as_conversions`, `arithmetic_side_effects`; reviewer's one-meaning question | Yes, by the gate |
| coverage.rs:238-240, `shape / 2`, `start + 2*at + 1` | `arithmetic_side_effects`; reviewer's newtype question | Yes |
| Save and restore of mode fields (capacities.rs:144 `self.inst = outer`, detached.rs:270 `self.unchecked = outer`) | reviewer question only (CODING.md:45) | Only if the reviewer sees it. `inst` is a `u32` sentinel, so `fn_params_excessive_bools` and `struct_excessive_bools` don't apply |
| `pub` fields | gate only on page-listed output types; reviewer "page's surface" only if visible outside the folder | Not if `Core` is private to the stage folder |
| 49 files adding about 1,100 methods to one `impl Core` | nothing | No |
| "a stage owns its output and later stages only read it" | gate on page items only | No, inside one page module |

**What still gets through.** The builder would answer the 12-field refusal by nesting: `Core { names: Names, types: Types, … }`, each holding 12 fields or fewer. That gives 144 fields at depth 2 and more at depth 3. The rest of the shape is untouched:

- 1,100 `&mut self` methods of 60 lines or fewer, in 49 files of 400 lines or fewer;
- mode fields as enums that are set and never restored;
- all inside one page module.

Nothing refuses it.

**Other bad code no checker catches:**
- Near-duplicate logic with renamed identifiers. The duplicate script only removes whitespace.
- Hash-order leaks through iterator adaptors (A9).
- Map-index panics (A10).
- Unchecked growth inside old functions (A3).
- Quadratic work in a shape the page has no cost case for.
- Code that recognises a case's program (E1).
- Interior mutability or by-value threading between stages (A16).

**Finding 3 (text-matched exceptions).** Mostly closed:
- Builders write no tests.
- Values never come from another implementation.
- The vocabulary-name script exists, though its match is undefined (A14).
- `#[ignore]` must be listed on a page.

Two openings remain:
- The corpus verdict comparison (LOOPS.md:45) is "same as base", and closing needs "every verdict the gate lists as differing is one a stated case requires" (LOOPS.md:89). Nobody is assigned to judge "requires" (H1). That can grow into a prose exception list kept by the runner.
- A builder can add `#[cfg(test)]` modules inside src files the page lists. The gate refuses "a test … changed that the unit's page does not list", but no script is said to detect a test inside a listed src file (LOOPS.md:34).

**Findings 1, 2, 4, 5, 7, 8, 9 and 10.**
- 1, 2 and 4: covered by stated values and the prober.
- 5: covered only where the page lists a cost case for the shape, or the reviewer sees it.
- 7: CODING's `Vec::insert` ban catches column.rs:191.
- 8: speed rows are part of green.
- 9: the "stage handles every program" reviewer question.
- 10: budgets in sessions and rounds. There is no line or day budget per unit, although the review recommended one ("Name a line count and a day count up front, and stop at twice either"). Put it to the owner.

---

## C. Owner requirements (brief items 3-5) missed, weakened or exceeded

**C1. "Shown to fail once" is only half kept.**
- Owner: "For whole-tree comparisons one deliberately wrong tree proves the harness; breaking the code per test is for tests with weaker checks."
- TESTING.md:60-61 keeps only the first half.
- Diagnostic cases compare "code, span and order … where the SPEC or the page states them" (TESTING.md:31), so many of them are partial, weaker checks.
- Cost cases, the corpus byte-for-byte check, the 700,000-input no-panic check and the 1 MiB stack check are all weaker checks with no fail-once and no named agent.
- Fix must hold: each check that compares less than a whole value has a named agent who breaks the code once for it, and the grain is stated.

**C2. The design page for what the parser hands on is missing as a precondition of the trial.**
- Owner (item 5): "The design page for what the parser hands on comes first … It must settle the text after the colon before the trial: `Soft: Lamp/0.5` … and `Soft: [a, b][i]`."
- LOOPS.md:126-134 lists `trial/base`, the harness and the §7-§9 trees, but not this page or its two settled forms.
- Fix must hold: the trial can't start until that page is approved with both forms settled.

**C3. Dispute: "the round is not spent on it".**
- LOOPS.md:111 spares the budget only for "a round or a session lost to a wrong case".
- A dispute the test author answers as right, or one that ends at the peer, still spends the round. The owner's words don't make that distinction.
- LOOPS.md:105 adds that the peer's answer ends a held dispute. The peer then reads the SPEC with authority, short of a GAP. Put it to the owner.

**C4. The verifier goes beyond the owner's gates.**
- Owner: the gates are "does the finding quote a SPEC sentence or a design-page invariant, and does it reproduce?" The drafts add:
  1. a CODING rule as a valid source (verifier.md:33; LOOPS.md:84). This is needed, or no code-quality finding can ever be admitted;
  2. a further rejection ground: "a sentence there that decides the case the other way rejects the finding" (verifier.md:33);
  3. a third gate, "its fix lies inside the page", with ROUTE as an outcome and four destinations (verifier.md:35, 45);
  4. re-gating an OBJECTION, then sending it to the peer (LOOPS.md:98);
  5. `Same cause as` grouping, and `Seen` lines.
- Gate 3 also asks the verifier to judge a fix while it is told "propose no mechanism" (verifier.md:51).

**C5. Other changes beyond the owner's words, for the owner.**
- A builder-session budget before the first READY for repair units (LOOPS.md:19, 110). The owner set only "three fix rounds a unit".
- The corpus verdict comparison and the closing condition on verdicts (LOOPS.md:45, 89).
- The harness built as a unit by a builder, on `harness/` branches reviewed only by the code reviewer (LOOPS.md:63).
- The 18:00Z-02:00Z blackout on this host (running.md:9).
- The trial base cut from `symphonyscript/r3` (LOOPS.md:129).
- Comments banned and numeric limits (A19, A20).
- MISSING and INVARIANT lines (LOOPS.md:101-102).
- Wholesale dismissal of CLAUDE.MD and the package CLAUDE.md (D5).

**C6. Counting rules passing.**
- Owner: "Counting rules passing is a script in the gate."
- LOOPS.md:38-47 doesn't list it among the gate's steps. Only running.md:37 says a result names "the rules passing, pending and untested". Small, but the canonical gate list should hold it.

**C7. "The reviewer is not told the tests passed": a weak form.**
- The reviewer is started "On READY", and READY means no case is listed (builder.md:36).
- Its tip export holds `tests/stated/cases` (LOOPS.md:82 removes only the pending list).
- LOOPS.md:122 says it "never … learns what a test shows", yet it can read every expected value.
- Fix must hold: decide whether the reviewer's export holds the cases, and make LOOPS and reviewer.md say the same.

---

## D. Contradictions between drafts

**D1. The builder may not write the harness that LOOPS assigns to it.**
- LOOPS.md:63: "a builder writes it on a `harness/` branch".
- builder.md:37: "Change no file under `crates/compiler/tests/stated/` and commit no test." The harness lives at `tests/stated/harness/` (TESTING.md:11), and its own test (TESTING.md:20) is a test.
- The harness unit also has no closing condition, since it has no stated cases.
- Nobody reviews the harness's own test as a test.

**D2. Hand back at once, or build on through a dispute?**
- builder.md:44 says to hand back at once for a DISPUTE, QUESTION, GAP or OBJECTION.
- LOOPS.md:104 and builder.md:46 say "The builder builds to the sentence."
- LOOPS.md:93 says the runner resumes it "with the answer", so the builder is idle through the test author, the test review and the gate. That is waiting, against "never waits" (builder.md:39).
- builder.md:39 also says "fix in your next commit every case … it names", which collides with a disputed case the builder may not change.

**D3. The lessons the drafts tell every agent to read first contradict running.md.**
- `.claude/lessons/GENERAL.md` is read first by every role (builder.md:23 and the others).
- G-4 (line 10): "Run a long check in the background with no outer timeout". G-5: "Wait on a run by its completion notice". running.md:17 says the opposite: "Run a command in the foreground … never start it detached."
- G-7: "Decide a call yourself when the owner's rulings, the documents and the code leave one answer". That tells the builder, reviewer and verifier to decide from code, against builder.md:33 and requirement 5. Only test-author.md:13 neutralises it, and only for expected values.
- Fix must hold: the lessons agree with the drafts before any agent starts, or each agent file says which wins.

**D4. Paths for the rule files.**
- LOOPS.md:25, 41 and 99 say `docs/TESTING.md` and `docs/CODING.md`. The agent files say `.claude/docs/…` from the main checkout.
- `packages/symphonyscript/docs/` also exists, so an agent can read the wrong file.
- Also, the main checkout's branch isn't fixed (it is on `docs` today), so "the main checkout" can hold an older CODING than the one the gate enforces on `trial/base`.

**D5. "Nothing in them binds you" also drops live hard rules.**
- builder.md:13 and every other agent file.
- The package CLAUDE.md has hard rules the drafts don't restate, for example "`symphonyscript-core` builds without `std`".
- The SPEC-examples test still runs in the suite, but the rule goes unstated.
- Fix must hold: name which loaded sentences stand, or replace both CLAUDE files before the agents start.

**D6. `#[ignore]`: "pages list" or "the gate's list"?**
- LOOPS.md:42 says "a test marked `#[ignore]` that no page lists".
- CODING.md:95 says "the gate's list". Make them one list.

---

## E. Holes: an agent follows its file and the purpose still fails

**E1. Code that recognises a case's program: each agent assumes another catches it.**
- builder.md:35: "Code that tells a case's program from its neighbours is a defect the prober finds."
- The prober has no cases (prober.md:17), so it can't know which programs are cases, and its file doesn't ask it to look.
- The reviewer has the cases but no question for this.
- Fix must hold: one agent with the cases has a stated check for it.

**E2. The test reviewer can't derive "blind" with the tools it has.**
- test-reviewer.md:35 and LOOPS.md:70 say to derive before reading the stated value.
- TESTING.md:10 puts "a program and the expected value" in one case. Read shows both at once.
- Fix must hold: the case layout keeps each expected value in a file apart from its program.

**E3. "Commit before the first run" can't be checked.**
- Where: test-author.md:36 and TESTING.md:33-34.
- The author has Bash and the worktree holds `src/`. A local run before the commit leaves no trace, so the runner's git list proves nothing.
- The list also tracks only values. A program changed after a run ("a slip in its program you repair", test-author.md:37) isn't listed, so the program can drift to one the code already answers.
- Fix must hold:
  - the first run is observable, for example the gate's run on the first push, with the author running nothing locally;
  - program changes after a run are listed too.

**E4. A routed true finding lets a unit close with a known wrong answer.**
- verifier.md:35 says a true finding whose fix the page lacks is ROUTEd.
- LOOPS.md:106 and 113 say it becomes "one PLAN line" and the unit closes.
- A valid program rejected, the worst class, can close that way.
- Fix must hold: a routed finding that shows a wrong answer blocks closing until the page is amended, or the owner accepts it.

**E5. Frozen cases can change meaning through the harness.**
- `harness/` branches change how every case is compared (LOOPS.md:36, 63), reviewed only by the code reviewer.
- Normalising spans or whitespace would loosen every frozen case with no test reviewer and no refusal.
- Likewise the printer for the "surface its cases observe" may be unit code in src that the builder can change.
- Fix must hold: any change to comparison or printing passes the test reviewer, or is fixed by the page.

**E6. Nobody is told to merge the base into a `tests/` or `harness/` branch.**
- The gate refuses any commit that lacks the base tip (LOOPS.md:33).
- test-author.md doesn't say to merge the base, and running.md bans rebase. A tests branch that waits out a long review can't go green.
- Two green tests branches on one base can't both fast-forward.
- Fix must hold: the agent owning each branch kind merges the base, as the builder does.

**E7. A test author's value can still come from the code.**
- test-author.md:37 lets the author see the code's output in a failing run and "read the sentence again".
- For the trial (kept behaviour, existing parser), most first-run failures will show today's tree. Old tests are also readable as program sources and hold old expected trees.
- The test reviewer's blind derivation is the only guard (but see E2).
- Fix must hold: the author sees pass or fail without the actual value, or works without `src/`.

**E8. The CODING Good example for "Linear" breaks SPEC §6.5.**
- CODING.md:158: `if !seen.insert(name) { duplicates.push(name); }`.
- §6.5: "A duplicate is one error naming every declaration of the name."
- The example pushes once per extra declaration and keeps no earlier declaration, so a builder copying it for duplicate names, which is review finding 5's exact case, draws two errors for three declarations and can't name the first.

**E9. Three verdicts are ambiguous or unowned.**
- builder.md:39: "A differing verdict your rules do not require is a defect of your change." That is a builder judgment.
- LOOPS.md:89 needs the same judgment at closing, with no owner (H1).

---

## F. The gate as LOOPS describes it

**F1. `deny` plus the baseline can't both hold.**
- CODING.md:9 says "A lint set to `deny`". LOOPS.md:38 and 41 say each step is "judged by its own exit code", with counts "against the baseline".
- With deny-level lints and baseline breaches in the compiler lib, clippy fails that crate. Dependent targets (integration tests, the harness) are then not checked, and the step's exit code is red at every commit.
- No commit can go green.
- Rustc lints at deny in `[lints]` (`unsafe_code`, `unused_must_use`) would also fail `cargo build` and the suite on any baseline breach.
- Fix must hold: lints run at warn with a counting wrapper whose exit code is the baseline comparison, and that is stated.

**F2. Thresholds need `clippy.toml`.**
- Thresholds and `disallowed_*` lists can only be set in `clippy.toml`, not in `[workspace.lints]`.
- The refusal "a setting … changed" (LOOPS.md:34) and CODING.md:106 must name that file.

**F3. What the refusals miss.**
- Unit branches:
  - LOOPS.md:34 refuses tests, dependencies, settings and baseline the page doesn't list, but not src files outside the page. Only the reviewer catches those (reviewer.md:33).
  - `#[cfg(test)]` modules added to listed src files (B, finding 3).
  - Changes to a unit-owned printer or fixture vocabulary that cases depend on (E5).
- The gate's own tests (LOOPS.md:53) leave out:
  - a unit commit that edits the harness;
  - one that changes `Cargo.toml`, `clippy.toml` or the baseline;
  - a `harness/` commit outside `harness/`;
  - an unlisted `#[ignore]`;
  - an `#[allow]`.

**F4. States in which no commit goes green.**
- F1, as written.
- An open dispute. That is by design, but it isn't counted apart unless the case was wrong (C3).
- A CODING rule that can't be met (A11, A18).
- Speed rows inside 5% of 8.3 ms on every commit, with wb2 also running the full suite, 700,000 inputs and two corpus passes. Noise past 5% makes green random (LOOPS.md:44, 131).
- Fix must hold: the speed rows run in isolation, with a stated repeat count and a statistic.

**F5. After-the-fact report of base pushes.**
- LOOPS.md:51 says the gate "reports" a direct push, but nothing says what happens next.
- With one shared identity, that is acceptable only if the report stops the loop.

---

## G. Instructions agents can't carry out with their tools or brief

**G1.** Test reviewer, blind derivation: see E2.

**G2. The reviewer can't isolate a fix round's changes.**
- reviewer.md:59 asks it to "review all the round changed".
- It is resumed with new merge-base and tip exports (LOOPS.md:88), which show the whole unit's diff, and the previous tip's export isn't given.
- Fix must hold: the previous tip is given in every fix round.

**G3. The builder's pre-commit local runs.**
- builder.md:38 says to run "the proven checks the page names" before each commit.
- For the trial these include 700,000 inputs, the corpus byte-for-byte check and a wb2 timing.
- On this host they may pass running.md's 600,000 ms limit, which turns into a QUESTION every commit. The timing row is meaningless here (running.md:8).

**G4. Size probes have no way to count work.**
- prober.md:40 says "with the work counted". Counting needs instrumentation on the public surface.
- Unless the page exposes counters, the prober has to time runs instead.

**G5. Resumed agents rebuild from scratch.**
- running.md:25 says "Delete your target directory … when you hand back."
- The builder hands back at every line and is resumed, so each resume pays a full rebuild (5.5 minutes in the review's run).

**G6. The SPEC is 2,028 lines and 181 KB.**
- "Read … whole" with Read stops at 2,000 lines by default. Every agent has to page through it, and the files don't say so.

---

## H. Two agents each assume the other does it

**H1. Verdict differences at closing.**
- Closing needs "every verdict the gate lists as differing is one a stated case requires" (LOOPS.md:89).
- No role file assigns the check. The runner has no file at all.

**H2.** Case-recognising code: see E1.

**H3. The harness's rules.**
- The reviewer doesn't read TESTING.md, so it can't check that the harness enforces TESTING.md:17-20 unless the harness page restates it.

**H4. The baseline file.**
- No role lowers the baseline after breaches are removed (A3).

---

## I. Examples whose Good form breaks a rule of the same file or the SPEC

- **CODING.md:110 and 173.** `NodeId(u32)` with `self.nodes.get(id.index())`. There is no `From<u32> for usize`, `as` and `unwrap` are refused, and `unwrap_or` is "a Result turned into a default" (CODING.md:197). So `index()` can't return a plain `usize` under the rules.
- **CODING.md:186.** Field names `at` and `by` break `min_ident_chars` at 3 (A5).
- **CODING.md:112-116 and 141.** The names `job`, `tasks`, `clock`, `rate` and `Event` come from the kernel domain and aren't SPEC or page words (A4). `id` breaks `min_ident_chars`.
- **CODING.md:140-145.** The Good explicit-stack walk answers yes to the Linear question (A12).
- **CODING.md:158.** Breaks SPEC §6.5 (E8).
- **TESTING.md:101-102.** The Good name case asserts "the body's printed tree holds `Cue`, `Triggers`, `Cue` in that order". That is a partial comparison, against TESTING.md:30, "The expected value is the whole of what the page's surface prints", and TESTING.md:60.

---

## J. Text an agent will read as optional or two ways

- **CODING.md:91.** Is inline `saturating_add` on a bare count allowed? (A8)
- **CODING.md:14.** Fix touched breaches, or leave them? (A2)
- **LOOPS.md:31-34.** "changed" relative to what: base tip or merge-base? Does adding a file count as "changed"?
- **LOOPS.md:80 against LOOPS.md:33.** The build loop says the gate refuses "when the stated tree moves". The refusal rule is "does not contain the base tip", which is any base move.
- **LOOPS.md:110-112.** Is a builder resumed after a hand-back the same "session"? That decides whether a DISPUTE or QUESTION spends the trial's one session.
- **test-reviewer.md:64 against LOOPS.md:71.** "Derived equals the number on your list" when part goes to "another review". Who adds the two counts?

---

## Nits and preferences (disclosed, not for action)

- TESTING.md:76: §8.1's `(':' Destination)?` is an optional part, not an "alternative".
- TESTING.md:54 ("A help is applied"): the SPEC states almost no message text, so the rule rarely applies.
- prober.md:52: the class list lacks "page not kept", though `Source` allows a page invariant.
- The 12-field script can be dodged with a struct-like enum variant of 253 fields.
- `fn_params_excessive_bools` skips trait impls (verified) and closures.
- REVIEWING.md's "byte offset shown as a column is wrong past ASCII" was dropped. SPEC §2.2 counts columns in scalar values; a stated case covers it.
- `disallowed_methods` on `f64::sin` and the other primitive inherent methods: confirm the pinned clippy resolves those paths before relying on it. I did not test it.
- Three agents (reviewer, prober, verifier) each make a full build of the tip on the slow shared host at once.
- `too_many_arguments`: confirm whether `self` counts toward 5.
- running.md:9: the blackout hours have no stated source.

---

## Coverage

**Read whole:**
- all ten drafts: LOOPS.md, CODING.md, TESTING.md, running.md and the six agent files;
- owner-brief-2026-11-13.md, peer-loops-proposal-2026-11-13.md, owner-messages-2026-11-13.md (both messages) and compiler-review.md;
- the old `.claude/docs/CODING.md` and `REVIEWING.md`;
- `/home/cdagent/ductuslang/CLAUDE.MD` and `packages/symphonyscript/CLAUDE.md`;
- `.claude/lessons/GENERAL.md`, `BUILDER.md` and `REVIEWER.md`.

**Read in part:**
- SPEC §1.3, §2.2-§2.5, §3.6, §4.6, §5.1-§5.2, §6.5, §8.1-§8.3 and §16, plus greps for §6.4 stand-ins and underscores;
- r3 `semantics/pipeline/core.rs:80-90`, `values.rs:788-800` and greps for mode-field assignments;
- `syntax/parser/starts_line.rs:1-30`;
- grep counts over r3 `syntax/` for `as`, `bool` parameters, wildcard arms, arithmetic, indexing and long functions. These are regex counts, so read them as approximate.

**Checked with clippy 0.1.94:**
- every named lint exists;
- behaviour on stdin source, with nothing written to disk: `min_ident_chars` on fields, variants and generics; `arithmetic_side_effects` on ints but not floats; `wildcard_enum_match_arm` on std enums but not inside `matches!`; `iter_over_hash_type` on `for` but not on `.keys().collect()`; `indexing_slicing` not on `HashMap` index; `fn_params_excessive_bools` not in trait impls.

**Not covered:**
- thresholds I couldn't set without writing a `clippy.toml`. `excessive_nesting` counting is from the lint's documentation; `min_ident_chars` at 3 is inferred from its behaviour at 1;
- the rest of the SPEC;
- ARCHITECTURE.md and PLAN.md;
- the r3 pipeline beyond the lines cited;
- the old engine;
- whether the 700,000-input and corpus runs fit a per-commit gate on wb2;
- the `first/` folder, `.memory/` and `.claude/worktrees/docs/.claude/`, as instructed.

