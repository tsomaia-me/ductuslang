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

## FND-16 — graph_section arity: `scope+` vs the empty module. RULED (b) — relax to `scope*`; EXECUTED (Chunk A, 2026-08-22; owner veto window per LEARNINGS #35, authorized by "proceed and take it from here")
IR_GRAMMAR §3's `graph_section ::= 'graph' '{' scope+ '}'` requires at least one scope,
so the minimal module `module App { types {} graph {} behaviors {} }` — increment 4's
round-trip witness — is outside the grammar as written. Options: (a) parser enforces
`scope+` (the minimal witness gains a dummy scope; empty graphs unrepresentable);
(b) relax the production to `scope*` (empty graph legal; the loader may still reject
empty modules as it sees fit); (c) keep `scope+` in the grammar, move arity to the
loader under F4 (parser over-admits by design). Due at increment 7 (scope row) planning.
EXECUTED as (b): DECISION_LOG 033-256, IR_GRAMMAR §3's `graph_section` production and
note, SPEC §15.4.6's copy and its **Empty graph** paragraph. The two existing witnesses
(`module_header.duir`, `types_table.duir`) are inside the grammar again with no edit.

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

## FND-19 — typed-value literals need a canonical digit spelling. RULED-BY-CADENCE (clear winner; veto standing) — textual, value-safe normalization (Chunk A, 2026-08-22)
Canonical uniqueness (already law for `STRING` content and for key quoting) fails for
`value` if `init 7` and `init 007` are both stable: two spellings of one value both
survive a round trip, so the corpus byte gate accepts two canonical forms. Structural
numbers do not have the problem — the parser converts them to `u64` and the emitter
writes the number back (R57) — but typed values cannot be converted at parse: the
loader owns their type. So canonicalization for them is TEXTUAL and must be value-safe
under every type the position may later take: an INT strips leading zeros to at least
one digit and `-0` canonicalizes to `0`; a FLOAT preserves its sign (`-0.0` is
meaningful and is not `0.0`), strips integer-part leading zeros to at least one digit
and fraction trailing zeros to at least one digit; `nan`, `inf` and `-inf` are their own
spellings. Each rewrite names the same real number, so no reading under any width can
change. EXECUTED: DECISION_LOG 033-257, the canon note beside IR_GRAMMAR §3's `value`
production, SPEC §15.4.6's **Canonical digit spelling** paragraph, and
`ValueLiteral::int` / `ValueLiteral::float` in `ductus-ir`.

## FND-20 — value/wire-to keywords shadow bare one-segment paths. RULED-BY-CADENCE (clear winner; veto standing) — EXECUTED (Chunk A wave 2, 2026-08-22)
In a wire's `to` position and in `value` positions, the soft keywords `null` / `true` /
`false` / `nan` / `inf` win over a bare ONE-SEGMENT path of the same name, decided by
one-token lookahead: `null.x` and `true:0` still parse as paths; only the bare single
segment is shadowed. This is the grammar's own ambiguity; the lookahead reading is the
only one that keeps every multi-segment path spellable while the keywords stay parseable.
A cell named bare `null` referenced from a wire-to is unspellable — pathological by
construction, and the loader's slice rejects nothing here (parse-level only). Pinned in
IR_GRAMMAR prose at the wire/value productions in the wave-2 rider.
EXECUTED: DECISION_LOG 033-258, the **Keyword-over-path shadowing** notes beside
IR_GRAMMAR §3's `wire` and `value` productions (plus their §6 rows), and SPEC §15.4.6's
**Keyword-over-path shadowing** paragraph. The law is scoped to the two positions that
admit BOTH a path and a literal — a wire's destination and a `binding`'s right-hand
side; a lone `value` position (`init`, `else`) admits no path, so nothing is shadowed
there and the finding's wider "in value positions" phrasing is narrowed accordingly.
SUPERSEDED BY FND-21 (owner ruling, 2026-08-02; executed Chunk A wave 3): the
lookahead this finding designed is retired. With `null`/`true`/`false`/`nan`/`inf`
reserved language-wide, no `NAME` segment may bear one, so no `PATH` competes with
the literal at either position and no disambiguation is needed. 033-258 is amended
in place to record the collapse; 033-259 states the `.duir` half. Nothing here is
"shadowed" any more — `null.x` is now a `reserved_word` rejection, not a path —
and the finding's unreachable-cell consequence is gone with it: a cell so named is
unspellable everywhere, not merely from two positions.

## FND-21 — quoted-key spellings of shadowed words silently change meaning; Key has a CR totality hole. RULED (owner, 2026-08-02): (1) LANGUAGE-WIDE reservation — null/nan/inf join true/false as reserved spellings at the Ductus surface and are never NAME segments in .duir (dissolves the shadowing; 033-258's lookahead collapses into the reservation); a KEY whose content is one of the five renders QUOTED (data stays spellable, meaning stable); (2) half two RULED (A) — QuotedKey escapes extend to the STRING family with the same canonical rule
Two related defects in the quoted-key/canonical-quoting family, found by both reviewers:
(1) `wire … to 'null'` parses as a PATH (a cell literally named null), but the emitter's
canonical rendering unquotes NAME-shaped keys (bare-iff-NAME-shaped), so round 2 reads
bare `null` = the KEYWORD — a fixpoint-stable, meaning-changing canonicalization; same at
binding-RHS with 'true'/'false'/'nan'/'inf'. (2) `Segment::Key` content admits any string
but QuotedKey's closed escape set {\', \\} cannot spell a carriage return — Key("a\r")
emits unlexable text and Key("a\r\n") silently corrupts to "a\n". Options are in the
in-session round; both amend 033-243/033-258-family law.
EXECUTED (Chunk A wave 3, 2026-08-22). Half one, the language-wide reservation:
DECISION_LOG 002-39 (the surface law, §1.4) with the family header bumped 38→39,
GRAMMAR.md's new `ReservedLiteralSpelling` production in §2.4 plus the §2.3 and
§2.8 notes, and SPEC §1.4's **Reserved literal spellings** paragraph. Its `.duir`
rider: DECISION_LOG 033-259 (033 header 258→259), 033-258 amended in place to
record the lookahead's collapse, IR_GRAMMAR §2's **Reserved spellings** preamble
law + the `NAME`/`BareKey`/`PathSegment` productions + both §3 notes + the §6 rows,
and SPEC §15.4.6's **Reserved spellings** paragraph replacing the shadowing one.
Half two, the escape family: 033-243 amended (bare iff identifier-shaped AND not
reserved; the quoted form's escape set and canonical emission are the string
form's), IR_GRAMMAR's `QuotedKey`/`KeyRaw`/`KeyEscape` productions and key
canonical-emission note, SPEC §15.4.1.1's **Quoted-key payload and its canonical
emission** paragraph. Code: `DiagClass::ReservedWord` (`reserved_word`), the
parser's reservation check at every `NAME` position with the lookahead removed,
the lexer's key escapes extended to the six, and one canonical quoting routine
shared by the `STRING` and `QuotedKey` emitters.

## FND-22 — declaration-order carrier into the IR is unstated. RULED-BY-CADENCE (near-single-path; veto standing) — graph entry order is the normative carrier; EXECUTED (Chunk B, 2026-08-22)
§13.10.3 pins the composed same-level total order (instance placement order, then
intra-instance declaration order) and makes the deterministic output trace depend on
it, but a full-corpus sweep (SPEC, DECISION_LOG, IR_GRAMMAR) found NO statement of what
carries that order into `.duir` — entry order was neither blessed nor denied, while the
format's only other candidates (path-lexicographic rank, an explicit rank field)
respectively fail the law as written and add machinery the spec never asks for. The
`.duir` sequence is the only order the serialization even has; the §15.4.5 worked
example already emits in declaration order; round-trip byte-stability already preserves
it. EXECUTED: DECISION_LOG (033 family), IR_GRAMMAR §3 entry-order note, SPEC §15.4.6
paragraph + §13.10.3 carrier sentence. The loader reads entry rank (scope-major, then
body order) as the same-level tiebreak.

## FND-23 — the dirty-set/value-change comparison is unstated for floats. RULED-BY-CADENCE (near-single-path; veto standing) — bitwise comparison; EXECUTED (Chunk B, 2026-08-22)
§13.10.2 step 1 says 'differ'/'identical'; §13.2.4.4 says 'changes from'/'the same
value'. Under IEEE equality a NaN-holding cell is perpetually dirty (NaN ≠ NaN) and a
0.0→-0.0 write is clean; under bitwise both invert. The corpus already knows both
hazards — §4.9's hash table bans float keys because 'NaN ≠ NaN violates Eq → Hash', and
the canonical digit law keeps '-0.0 being a value of its own' — and §14.3.1 stores
cells bit-exact, so bitwise is the only completion consistent with the corpus's own
commitments. EXECUTED: DECISION_LOG (023 family), SPEC §13.10.2 step-1 clause +
§13.2.4.4 paragraph. The runtime's comparison is Value::bits_eq.

## FND-24 — the trap-stamping `catch_unwind` vs the corpus's literal "no `catch_unwind` boundary". RULED (a) BY CADENCE (raised at finish, unstruck; veto standing) — isolation-only reading; EXECUTED (M0 closure, 2026-08-23)
The engine's sole behavior-invocation path wraps every call in `catch_unwind` and
re-panics immediately as `behavior trap at <path>: <payload>` — D8's ruled trap shape, a
harness-visible panic carrying the trapping cell's path. But SPEC §13.13.1, restated at
§14.6.2 and pinned as DECISION_LOG entries in the trap family, bans a "`catch_unwind`
boundary" BY NAME and lists it separately from "no continuation past a trap" — while the
isolation semantics that ban protects are preserved here: nothing runs between the catch
and the re-panic beyond message formatting, no value is invented, no continuation
happens. The chosen mechanism leaves observable residue: a trap prints TWO panic reports
(the stub's own site, then the engine's re-panic site), and the surviving report
attributes the panic to the engine's source line rather than the behavior's; under a
`panic = "abort"` profile (nothing sets one today) the catch would never fire and the
path-bearing shape would silently vanish. Options: (a) rule the prohibition
ISOLATION-ONLY — keep catch-and-re-panic and amend the three corpus sites LOG-first to
carve out an observe-and-re-panic stamp; (b) rule it LITERAL — replace the mechanism with
a non-catching one (a `Drop` guard on a path-carrying struct checking
`std::thread::panicking()`, or a panic hook), which also yields a single report at the
behavior's own location; (c) accept the mechanism and park the polish (D8 already defers
trap-diagnostics polish to M6). Recommendation: (a) for the wording, plus adopting (b)'s
Drop-guard mechanism at a later milestone if the double-report residue bites — the
semantics are identical under both readings; only the wording and the report shape
differ. EXECUTED: the prohibition now reads ISOLATION-ONLY at the three corpus sites —
SPEC §13.13.1 (its original three prohibitions stand, with one carve-out sentence
added), SPEC §14.6.2, and the trap-family DECISION_LOG entries 026-5 and 032-113,
amended in place. The mechanism is untouched; (b)'s Drop-guard rewrite stays available
if the double-report residue bites.

## FND-25 — §13.10.3's "same level" is undefined and the composed order is underdetermined; the loader silently pins greedy rank-least-ready Kahn. RULED (a) BY CADENCE (raised at finish, unstruck; veto standing) — the composed order is the rank-lexicographic minimum topological order; EXECUTED (M0 closure, 2026-08-23)
§13.10.3 defines the same-level tiebreak (declaration order within an instance, placement
order across) and says the deterministic output trace "is defined by this composed order"
— but "level" is defined nowhere. Under the only gloss the corpus offers ("neither
depends on the other") the pairwise constraints are UNSATISFIABLE for legal modules whose
entry ranks interleave with derived→derived dependency edges: rank pairs demand x < z and
z < y while an edge demands y < x. Under the alternative depth reading the law is instead
SILENT on cross-level pairs. Forward references are legal (declarations are
order-independent), so both shapes are reachable, and none of the landed witnesses
discriminates the candidate readings — only the implementation pins the observable trace.
The loader computes the lexicographically-smallest topological order by entry rank (Kahn,
rank-least-ready-first), which provably REPRODUCES declaration order whenever declaration
order is itself topological, and which a level-synchronous schedule would not. The
carrier ruling (FND-22) pinned only what carries the order — entry rank — not how the
tiebreak composes with topology. Options: (a) bless the landed rule — the composed order
IS the rank-lexicographic minimum topological order — amending §13.10.3 and its LOG
mirrors LOG-first and adding a rank-interleaved witness; (b) rule it level-synchronous
(rank within topological depth waves) — change the loader, add the witness.
Recommendation: (a) — it is the unique completion that degenerates to pure declaration
order exactly when the text is written dependency-first, and it is what every landed
golden already witnesses. Rider either way: the two code comments claiming the current
rule as already-ruled law are softened to cite this finding until it is ruled.
EXECUTED: DECISION_LOG 023-33 and 023-34 amended in place to claim a RANK rather than a
schedule, plus a new 023-family entry defining the composed order as the
rank-lexicographic minimum topological order of the per-commit DAG; SPEC §13.10.3 gains
the realized-schedule paragraph. The rank-interleaved witness — a module whose
dependency is declared after its reader — joins the corpus.

## FND-26 — scope-path legality is an unruled surface: duplicates, containment, module-root anchoring, scope/cell namespace. RULED BY OWNER ("Approved, continue", 2026-08-23) — reject all four; EXECUTED (M0 closure)
The loader is the sole legality authority (R21) and it checks a scope path's SHAPE only.
Four sub-questions, none pinned by the corpus. (1) Duplicate scope paths: `scope App {}
scope App {}` loads silently; §15.4.1 makes every primitive "identified by a stable
path", but the one corpus precedent for a scope-identity collision (the surface scope-key
rule) rules FIRST-WINS with a non-fatal warning, and M0's loader has no warning channel —
so reject-vs-tolerate is a genuine fork. (2) Cell-under-scope containment: nothing
requires a cell's path to extend its enclosing scope's path (`scope App { input
App.panel.x … }` loads), and the obvious "exactly one segment deeper" rule is REFUTED by
the corpus's own worked example, which places `App.print:0.text` directly inside `scope
App`. (3) Module-root anchoring: §15.4.1.1 pins a cell ID as the path "from module root"
and every corpus witness obeys it, but the loader never consults the module name, so a
module named `Foo` whose cells root at `App` loads. (4) Scope-path/cell-path collisions:
`scope App.x` beside `input App.x` loads unchecked. Blast radius in M0 is nil — scopes
are discarded at load — but the surface becomes load-bearing at M2 placement and at
hot-reload identity. Options, per sub-question: reject with a new `DiagClass` id /
tolerate and document / defer to the milestone that consumes scopes. Recommendation: rule
the family at once before Chunk B commits, or explicitly park it to M2 with this entry as
the record; if rejection is ruled the classes are ordinary F3 additions (e.g.
`duplicate_scope_path`) and the checks are mechanical.
EXECUTED: DECISION_LOG 033-261 (scope paths are unique, blame the second declaration),
033-262 (a cell's path extends its scope's as a proper prefix, at ANY depth), 033-263
(module-root anchoring, through containment reaching every cell path), and 033-264 (one
scope/cell identity namespace); IR_GRAMMAR §3's scope-path well-formedness note beside
the entry-order note plus the §6 `scope` row; SPEC §15.4.6's **Scope-path
well-formedness** paragraph. Four loader classes: `duplicate_scope_path`,
`cell_outside_scope`, `unanchored_scope_path`, `scope_cell_collision`.

## FND-27 — M0.md §4's "Writes during a commit are forbidden (assert)" has no assert and no emission site: the law is structurally unrepresentable. RULED (a) BY CADENCE (raised at finish, unstruck; veto standing) — record the structural enforcement, keep the class registered; EXECUTED (M0 closure, 2026-08-23)
`commit()` takes `&mut self` and never calls back into host code (behaviors receive only
`&[Value]`), and a transaction closure exclusively borrows the engine — so a write during
a commit, and a nested transaction, are both UNREPRESENTABLE in M0's sequential API. The
plan's "(assert)" is therefore substituted by a compile-time guarantee that is
structurally stronger; `write_during_commit` stays a registered class with no M0 emission
site (its id is constructed and asserted in registry tests only). The vacuity is
disclosed in the transaction type's doc comment but was never filed. Options: (a) amend
the plan line to record structural enforcement, keeping the class registered for the
milestone that makes it reachable (a threaded host API); (b) add a runtime in-commit flag
plus the assert anyway, to honor the letter; (c) park. Recommendation: (a) — an assert
that cannot fire is dead code pretending to be law.
EXECUTED: M0.md §4's "(assert)" line now records the STRUCTURAL enforcement — `commit`
takes exclusive access and never calls back into host code, so the violation is
unrepresentable — and the `write_during_commit` class's standing registration for the
milestone that makes it reachable.
