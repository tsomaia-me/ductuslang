# Implementation findings ledger

*Numbered-atomic, same discipline as the audit ledger (STRATEGY §4-G). A finding is a
corpus gap/contradiction discovered by implementation. Fix direction: owner ruling →
DECISION_LOG → SPEC/IR_GRAMMAR → code (STRATEGY §2.3); never a code workaround. Line
numbers cite IR_GRAMMAR.md at commit d5aa03e.*

Status legend: OPEN (awaiting owner ruling) · RULED (ruling recorded here + M0.md §8) ·
EXECUTED (corpus edited).

## FND-1 — comments: grammar silent. RULED (a) — no comments (owner, 2026-08-02)
IR_GRAMMAR never admits nor rejects comments in `.duir` text (its `//` lines are EBNF
metalanguage). Hand-written witnesses would benefit from comments; round-trip byte
stability then needs the canonicalization answer (emit strips comments ⇒ corpus files
must be comment-free or the gate compares canonical forms). Options: (a) no comments —
machine format stays comment-free, witnesses explain themselves in the paired test file;
(b) admit `//` to end-of-line, emitter strips, corpus files must be pre-canonical.
Recommendation: (a) — simplest, honest to "serialization, not human document" (R16);
witness prose lives in the Rust test.

## FND-2 — INT carries no sign; negative init/else underivable. RULED (a) — sign in the lexeme (owner, 2026-08-02)
`INT ::= decimal integer literal` (77) says nothing of sign and no unary-minus
production exists, yet `init -5` / `else -1` are obviously intended (214/215/347).
Options: (a) sign is part of the INT/FLOAT lexeme (lexer-level `-?`), no unary
operator; (b) parser-level sign production. Recommendation: (a) — one place, no
expression machinery in a data format. Needs LOG/IR_GRAMMAR amendment spelling the rule.

## FND-3 — FLOAT lexically undefined; no NaN/inf lexeme. RULED — FLOAT = -? digits '.' digits, no exponent form; nan/inf/-inf spellings (owner, 2026-08-02)
`FLOAT ::= floating-point literal` (80) has no character rule, no exponent form, no
examples; NaN/infinity have no spelling anywhere, so an f64 cell cannot init to them,
and R34's canonical trace rendering (`NaN`/`inf`/`-inf`, no exponent) has no `.duir`
counterpart. Recommendation: define FLOAT as `-? digits '.' digits` (no exponent form —
matches the R34 no-exponent canon; shortest-roundtrip decimal always fits it) and add
`nan` / `inf` / `-inf` value spellings for float-typed positions. Also fix 80's
"behavior IR literals only" contradiction with `value` (347).

## FND-4 — STRING lexically underspecified. RULED — totality design approved (owner, 2026-08-02)
`STRING ::= double-quoted string literal` (83) with no escapes stated. First
recommendation (minimal closed escape set, raw newlines illegal) REJECTED by the owner
as least-resistance drift (LEARNINGS #33): raw newlines MUST be legal, and the design
must be derived from the format's real requirements. Requirements: TOTALITY — every str
value has a spelling; CANONICAL UNIQUENESS — the emitter has exactly one spelling per
value (round-trip byte stability); raw-newline multi-line readability (owner mandate);
CRLF-normalization safety (a raw CR would be corrupted by file-level newline
normalization). Proposed design (pending owner ruling):
- Grammar: `STRING ::= '"' (raw | escape)* '"'` where raw = any Unicode scalar except
  `"` and `\` — raw newlines and tabs included.
- Input escape set: `\"  \\  \n  \r  \t  \u{H...H}` (1–6 hex, any scalar).
- Canonical emission: raw everywhere legal, EXCEPT `"`→`\"`, `\`→`\\`, CR→`\r`, and
  control scalars other than LF/TAB → `\u{...}`; LF and TAB emit raw. Non-canonical
  spellings parse but fail the corpus byte-stability gate, exactly like non-canonical
  key quoting.

## FND-5 — `%0` in the normative example is not derivable. RULED — SSA temporaries are `'%' (NAME | INT)` (owner, 2026-08-02)
`instr ::= '%' NAME '='` (383) with NAME not digit-initial (74), but the worked example
uses `%0`, `%1`, `%2` (560–574). Recommendation: SSA temporaries are `'%' (NAME | INT)`
— amend 383/431. (Behavior-IR concern; lexer just needs Percent + Ident/Int.)

## FND-6 — `keyed-template` / `gate-guarded` hyphens vs NAME charset. RULED (a) — respell snake_case (owner, 2026-08-02)
Member drivers (253) contain `-`, which NAME (74) forbids and no token supplies.
Options: (a) respell in the corpus as `keyed_template` / `gate_guarded` (snake_case like
every other multiword keyword: `reset_on_reopen`, `gate_parent`, `source_deps`);
(b) lex them as special multi-char keywords. Recommendation: (a) — consistency; LOG-first
edit (033-81's driver vocabulary + IR_GRAMMAR 253 + SPEC mirror).

## FND-7 — BareKey charset is unboundedly wide. RULED — BareKey = identifier charset exactly (owner, 2026-08-02: "key is same as identifier. only those chars are allowed that identifiers allow. nothing more, nothing less")
BareKey (103–104) excludes only `.` `:` `'` `\` whitespace — so `]` `,` `{` `}` `%` `#`
`"` are all legal bare-key characters, and a keyed path inside `members [...]` can
contain the very delimiters that bound it. Recommendation: narrow BareKey to exclude the
structural delimiters (`[ ] { } ( ) , ;` and `"`), everything else stays; anything
needing those quotes as a QuotedKey. Canonical-quoting law (114–117) is unaffected.
OWNER RULING is STRICTER than the recommendation: BareKey admits the NAME charset and
nothing else. Disclosed consequence (owner may veto): a purely numeric key is not
NAME-shaped, so IR_GRAMMAR's own example `App.rows.42.render_row` (123) becomes
`App.rows.'42'.render_row` — the corpus example is amended in the execution batch.

## FND-8 — dotted mnemonics vs PATH: who owns the dot. RULED — lexer never composes; parser composes both (owner, 2026-08-02)
`add.i32` (op '.' PRIM, 458) and `App.count` (PATH, 93) are lexically identical
IDENT-dot-IDENT runs; `const.`/`cast.` glue the dot into the literal (461–462) while
tuple.make/enum.tag/call.dyn are whole dotted literals. Recommendation: lexer emits
IDENT/Dot/INT tokens only — never a composed PATH or mnemonic token; the parser
composes both (position always disambiguates: mnemonics sit in instr-RHS head position,
paths in operand/graph positions). Spell this resolution in IR_GRAMMAR prose so the
one-token-or-three question is closed.

## FND-9 — lookback_map contradicts the reopen_set colon rule. RULED — colon-ordinal rule scoped to reopen_set; lookback_map's LAST ':INT' is the value (owner, 2026-08-02)
The stated disambiguator "colon then INT ⇒ ordinal" (330–333) makes every
`lookback_map` entry `PATH ':' INT` (335) parse as an ordinal-suffixed path with no
value. Recommendation: scope the 330–333 rule to reopen_set only and give lookback_map
its own rule (a lookback pair's colon binds to the map, ordinals inside lookback keys
require... simplest: ordinals in lookback keys must be followed by a second `:` INT
pair — the parser reads the LAST `:INT` as the lookback value). Needs an owner pick;
flag as the parser-increment blocker, not lexer.

## FND-10 — NEWLINE's module-level status unstated. RULED — NEWLINE is a normative entry terminator in types/graph/behaviors sections (owner, 2026-08-02)
Prose (67) says line breaks separate top-level statements, but no §3 production carries
NEWLINE, and without it `derived … uses B@d1` ⤶ `gate App.g0 pred …` is ambiguous with
the cell's optional trailing `gate` clause (5.4). Recommendation: make NEWLINE a
normative entry terminator in the graph/types/behaviors sections (blank lines collapse;
`;` NOT admitted at module level), stated in IR_GRAMMAR §3.

## FND-11 — SPEC/IR_GRAMMAR divergence: `@ provenance` on bindings. RULED — IR_GRAMMAR gains the clause (owner, 2026-08-02)
SPEC §15.4.6 (26609–26610) has `binding ::= NAME ':' (PATH|value) ('@' provenance)?`
with `bound|constant_wrap|bridge`; IR_GRAMMAR (344–345) lacks the provenance clause,
the `@` token, and the three keywords, while claiming normativity for the text shape.
One of the two is wrong. Recommendation: IR_GRAMMAR gains the clause (SPEC's richer form
looks intentional — effect-param provenance matters to the loader); LOG-first edit.

## Deferred-by-nature notes (no ruling needed now)
- Encoding: UTF-8, LF-normalized on lex (CRLF accepted, BOM stripped) — proposed as
  loader/lexer behavior at increment 3's plan, not a corpus change.
- `HEX` lexeme (86) dangling (HexU32 is the used one) — cosmetic corpus cleanup, ride
  any FND batch execution.
- `value`/`literal` doc note at 492–493 self-contradiction — cosmetic, same ride.

## FND-12 — NAME's alphabet is unruled (Unicode vs ASCII). OPEN (queued; recommendation: defer to frontend milestone, IR stays ASCII as explicit ruling)
IR_GRAMMAR §2 pins NAME as "letters, digits, '_', not starting with a digit" without
naming the alphabet. The lexer implements ASCII letters (documented in scan.rs as
pending this ruling); a non-ASCII letter is currently invalid_character. Frontend-era
Ductus identifiers will force the question anyway. Options: (a) ASCII forever (IR is
machine-written; frontend mangles); (b) Unicode XID (Rust-style) in both. Recommendation:
defer to the frontend milestone's identifier ruling and keep IR ASCII until then — but as
an explicit ruling, not an accident.

## FND-13 — BID case-sensitivity unstated. RULED — case-sensitive 'B@' exactly, as implemented (owner, 2026-08-02; single-path item, veto-list class per LEARNINGS #26 refinement)
Grammar says 'B@'; lowercase b@d1 currently lexes as Ident("b") At Ident("d1") — to the
letter. Emitters only ever write 'B@', so this is likely correct; flagging so it is law
by choice. Recommendation: case-sensitive 'B@' exactly (accept as implemented).

## FND-14 — B@ maximal munch swallows provenance after a segment named `B`. RULED (a) — boundary rule: a Bid's hex run must end at a non-identifier byte (owner, 2026-08-02)
Confirmed breaking input: `params [m: App.B@bound]` lexes as `… Dot Bid(0xb) Ident("ound")`
— the provenance clause of 033-252 is unspellable when a path tail is the legal NAME `B`,
because the Bid scanner commits on `B@` and eats the hex-letter prefix of
bound/bridge/constant_wrap. Grammar-level ambiguity, not a scanner bug. Options:
(a) boundary rule — a Bid's hex run must end at a non-identifier byte; otherwise the
scanner yields `Ident("B") At …` (so `B@bound` provenance works, `B@d1` stays a Bid, and
`B@d1x` is no Bid — parse error in BID position); (b) forbid NAME `B`… (rejects legal
programs — out). Recommendation: (a), pinned in IR_GRAMMAR §2's Bid prose.

## FND-15 — file-intake laws live only in Rust doc comments. RULED — implemented rules become law: amend the STRING raw clause (lone CR excluded; CRLF normalizes incl. payload interiors) + intake paragraph (BOM at offset 0 only, whitespace set) in IR_GRAMMAR/SPEC, LOG-first (owner, 2026-08-02)
Three normative lexical behaviors exist in code docs but not in the corpus: (i) BOM
stripped at offset 0 only, elsewhere invalid; (ii) whitespace = space/tab/LF/CR-of-CRLF
(FF, VT, NBSP invalid); (iii) CRLF→LF normalization applies inside STRING/QuotedKey
payloads and a LONE CR is a lex error EVERYWHERE — which narrows 033-249's written "raw =
any scalar except quote/backslash" (CR included as written). A future implementer reading
the corpus builds a different lexer. Recommendation: amend the STRING law's raw-scalar
clause (lone CR excluded from raw; CRLF normalizes) and add the intake paragraph (BOM,
whitespace set) to IR_GRAMMAR §1/§2 + SPEC mirror, LOG-first.

## FND-16 — graph_section arity: `scope+` vs the empty module. OPEN (ruling due before row 7)
IR_GRAMMAR §3's `graph_section ::= 'graph' '{' scope+ '}'` requires at least one scope,
so the minimal module `module App { types {} graph {} behaviors {} }` — increment 4's
round-trip witness — is outside the grammar as written. Options: (a) parser enforces
`scope+` (the minimal witness gains a dummy scope; empty graphs unrepresentable);
(b) relax the production to `scope*` (empty graph legal; the loader may still reject
empty modules as it sees fit); (c) keep `scope+` in the grammar, move arity to the
loader under F4 (parser over-admits by design). Due at increment 7 (scope row) planning.

## FND-17 — IR_GRAMMAR §5's worked example is not R54-canonical. RULED (c) — §5 stays pretty parseable documentation; the canonical oracle is the corpus witness set; R54's §5-oracle clause struck (owner, 2026-08-02)
§5 uses column alignment (`input     App.count   : i32  init 0`), double spaces before
clauses, blank lines between entry groups, and a one-line non-empty `types { … }`; R54's
canon is two-space indent, one entry per line, single spaces. `emit(parse(§5)) == §5`
cannot hold. Options: (a) re-canonicalize §5 to machine canon (doc loses its aligned
readability); (b) emitter adopts alignment as canon (complex, fragile); (c) §5 stays a
pretty, PARSEABLE documentation example — the canonical-form oracle moves to corpus
witness files, and R54's "§5 is the oracle" clause is struck. Recommendation: (c) — docs
serve human reading, corpus witnesses serve the byte gate; degrading the doc buys nothing.

## FND-18 — type_tag nesting is unbounded; a 2 KB legal file aborts the process. RULED (a) — normative cap 64, nesting_too_deep class, LOG-first (owner, 2026-08-02)
`type_tag` is recursive and IR_GRAMMAR imposes no nesting bound, so `portal<portal<…>>`
~210 deep (a 1,780-byte file) stack-overflows the parser — SIGABRT, not a Diagnostic;
no catch_unwind contains it; the fail-fast contract (T6/R53) gains a third outcome.
Measured: ~210 levels on 2 MiB test threads, ~840 on the 8 MiB main thread; emit and
drop glue survive deeper, so the parser is the binding constraint. Options: (a) a
normative nesting cap as corpus law (new DiagClass `nesting_too_deep` at the token that
exceeds it; cap pinned in IR_GRAMMAR §2 prose) — recommend cap 64: real type nesting is
<10, 64 is safe on 2 MiB threads with margin for emit/drop/loader; (b) accept the abort
as a documented limitation (violates fail-fast; hostile to hosts embedding the loader);
(c) rewrite type_tag parsing with an explicit heap stack (no cap, no abort, permanently
more complex code in the hottest grammar path, and the loader/emitter still need THEIR
answer for hand-built trees). Recommendation: (a) — LOG-first, the cap is format law.
