# Implementation findings ledger

*Numbered-atomic, same discipline as the audit ledger (STRATEGY §4-G). A finding is a
corpus gap/contradiction discovered by implementation. Fix direction: owner ruling →
DECISION_LOG → SPEC/IR_GRAMMAR → code (STRATEGY §2.3); never a code workaround. Line
numbers cite IR_GRAMMAR.md at commit d5aa03e.*

Status legend: OPEN (awaiting owner ruling) · RULED (ruling recorded here + M0.md §8, or
M1.md §9 from FND-28 on) · EXECUTED (corpus edited) · PARKED (ruled out of the current
milestone by a scope ruling; the entry stays the record until its milestone opens) ·
CONSEQUENCE (no ruling is needed — the engineering is forced by an earlier one; the entry
exists so the correction is recorded rather than hidden).

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

## FND-28 — a recurrent's data model is unserializable in `.duir`: injected history params, per-input lookback, per-`k` fallbacks. RULED (owner, 2026-08-23) — fork 1 = (b) + (c): `cell` gains BOTH `lookback` and `fallbacks`; M1.md R71
IR_GRAMMAR §5's own worked example lowers `recurrent total: i32 = total.past(1, 0) +
count` to `recurrent App.total : i32 uses B@d2 inputs [App.count] depth 1 init 0` with
`behavior B@d2 (count: i32, past1: i32)` — one `inputs` entry, TWO behavior parameters.
§15.4.4's ABI table says a recurrent takes input cells, self-history slots (`pastK`) and
input-history slots (`<input>_pastK`), and §15.4.1's data model carries an
`input_lookback_map` for recurrent dependency edges. The `.duir` `cell` line encodes none
of it: it cannot say how many `pastK` params exist (only `depth`, which is the
ALLOCATION, not the usage), it has no `lookback` clause at all (the production exists
only on `stream`), and one `('init' value)?` cannot carry the corpus's own two-fallback
example `recurrent[2] fib = fib.past(2, 0) + fib.past(1, 1)`. Nor can a `value` be a
PATH, so the canonical source idiom `count.previous(start_value)` has no spelling. Five
gaps, all blocking an implementer: C-1 (ABI arity/order), C-2 (lookback), C-3 (multiple
fallbacks), C-4 (non-literal fallbacks), C-5 (`init`'s missing normative definition).
Options: (a) positional convention driven by `depth` alone, input-history declared out of
scope; (b) add a `lookback` clause to `cell`, mirroring `stream`; (c) (b) plus a fallback
MAP replacing the scalar `init`. Recommendation was (b) with (c) filed.
RULED (owner: *"Fork 1 - (b) + (c) now (regardless of whatever further forks (c)
opens)"*): BOTH clauses land. `cell` gains `('lookback' lookback_map)?` after `depth` and
`('fallbacks' fallback_map)?` after `init`, with `fallback_map ::= '[' (INT ':' value (','
INT ':' value)*)? ']'`; key `k` is the fallback a `.past(k)` read returns before `k`
advances exist. `init v` is the uniform shorthand (every `k` falls back to `v`),
`fallbacks` the general form, both present on a recurrent is a LOAD error, and canonical
emission writes `init` when all depths share one value and `fallbacks` otherwise. The
behavior ABI order is pinned: inputs in list order, then `past1 … pastDepth` ascending,
then per `lookback` entry in list order `<input>_past1 … <input>_pastK` ascending — so
arity is exactly `inputs.len() + depth + sum(lookback)` and the loader checks it.
**Sub-fork FILED-OPEN inside this ruling (C-4):** fallback values are LITERALS only; a
PATH fallback is a dependency edge, i.e. a language-level question, not a serialization
one. Deferred by the ruling's own words, never a blocker. Detail in M1.md §4.9.
**Note added to this open sub-fork (wave 2, 2026-08-23) — the input-history reset
fallback.** The sub-fork has a runtime residue that the build had to answer and did:
`init`/`fallbacks` seed SELF-history only, so a `lookback` input ring has **no fallback
spelling in the IR at all** — yet §13.2.4's `@reset_on_reopen` says the first post-gap
read is a fallback read. The landed behavior stands in for the unspellable value: the
engine seeds each input ring's fallback area with **the input cell's construction-era
value** (the only value of that cell's type the runtime holds at that moment), and
`clear_history` empties the rings so a post-reset read lands back on it. KEPT as landed
and recorded here rather than left in a code comment; it is a stand-in, not a ruled
semantics, and the general answer (non-literal and per-input fallbacks) is exactly what
this sub-fork still owes. Mirrored in M1.md §4.7.
EXECUTED (M1 corpus batch, 2026-08-23): DECISION_LOG 033-266 (the `lookback` clause,
reusing `lookback_map`), 033-267 (the `fallbacks` clause; a fallback is a literal, the
PATH form named as an open extension), 033-268 (`init` as the uniform shorthand, both
clauses on one recurrent a load error, canonical emission by content), 033-269 (the
pinned clause order), with 033-169 and 033-200 amended in place (the recurrent line's new
clauses; the pinned ABI parameter order); SPEC §15.4.6's `cell` production plus its three
new paragraphs (clause order, the two history clauses, `init` versus `fallbacks`),
§15.4.4's recurrent-parameter-order paragraph, §15.4.1's history/fallback sentences, and
§13.2.4.1's **Fallbacks** paragraph; IR_GRAMMAR §3's `cell` production, the new
`fallback_map` production, the clause-order and two-history-clause notes, and the §6
`cell` row. The C-4 sub-fork stays FILED-OPEN — no corpus edit is due for it.
**Correction to the RULED paragraph above (recorded 2026-08-23, remediation of the M1
build).** Its closing clause — "and the loader checks it" — is this ledger's elaboration
of the arity law, not the owner's words: the ruling as spoken is "Fork 1 - (b) + (c) now
(regardless of whatever further forks (c) opens)" (GATE.md, 2026-08-23). The ABI ORDER and
the arity FORMULA are ruled and are executed; the load-time CHECK is deferred, because it
cannot be written through the D5 provider seam — see **FND-37**. A wrong-arity stub still
fails, as a D8 trap naming its cell.

## FND-29 — the history cadence is unstated: does a ring shift per COMMIT or per ADVANCE, and does a bit-identical evaluation advance at all? RULED (owner, 2026-08-23) — forks 2 + 9: per-ADVANCE, and evaluate ⇒ advance; M1.md R72/R77
§13.2.4.3 says `.past(k, fb)` "reads the recurrent's value **k commits ago**".
§13.2.4.1/016-73 says "Recurrents whose expressions did not re-evaluate in this pass **do
not advance**; they retain their existing values." The two diverge the moment a recurrent
skips a commit: for a `recurrent[2] r` dirty at commits 1 and 3 but not 2, the per-commit
reading duplicates the held value into `past1` at commit 2 and yields `past2 = v1` at
commit 3, while the per-advance reading yields `past2 = v0`. Different programs, same
text (C-9); the same question is open for INPUT history (C-10). Independently,
§13.2.4.4's value-change rule ("A recurrent whose expression re-evaluates and produces
the same value … does not propagate a change downstream") answers only the PROPAGATION
half of a second question: does a recurrent that DID evaluate to a bit-identical value
still shift its ring? Options for the first: (i) per-commit shift; (ii) per-advance shift;
(iii) split the reading (self-history per-advance, input-history per-commit). For the
second: (a) evaluate ⇒ advance, gating governs propagation only; (b) bit-identical ⇒ no
advance either.
RULED (owner: *"Fork 2 - (ii) per-advance / Fork 9 - (a)"*): rings shift PER-ADVANCE — a
commit in which the recurrent does not evaluate consumes no slot — and an evaluation
advances even when the result is bit-identical, with value-change gating governing
downstream propagation only. Option (iii) was not taken, so input-history rings follow
the same per-advance law (C-10 resolved with C-9). Together: **the ring shifts exactly
once per evaluation and never otherwise.** Consequence recorded in M1.md §8.1: a live
recurrent whose input sits in a frozen subtree (C-12) needs no separate rule — a
never-dirty input triggers no advance, so it contributes no shift.
EXECUTED (M1 corpus batch, 2026-08-23): DECISION_LOG 016-52, 016-77, 016-79 and 016-82
amended in place ("k commits ago" → k *advances* ago; the fallback-availability rule
restated as "the read's history ring has advanced fewer than `k` times"; input history
indexed by the reading recurrent's advances; the stream-recurrent parallel restated as
"what one step is"), plus new entries 023-71 (the ring shifts per advance) and 023-72
(evaluation implies advancement); SPEC §13.2.4.1's two new paragraphs (**History shifts
per advance, not per commit**, **Evaluation implies advancement**), §13.2.4's fallback
paragraph, §13.2.4.3's self- and input-history bullets and its "Common rules" `fallback`
bullet, §13.2.4.4's value-change paragraph, and §13.10.2 step 4's advance sentence.
(Round-3 amendment: the fallback-availability sites — DECISION_LOG 016-52, SPEC §13.2.4's
fallback paragraph, and SPEC §13.2.4.3's `fallback` bullet — were swept after the batch and
are folded into the list above; they carried the stale per-commit reading, which diverges
from the ruled per-advance law for any recurrent that skips commits.)
(Round-4 amendment: three more sites of the same phrase-class were swept and are folded
into the list above. GRAMMAR.md §5.24's `fallback` note carried the pre-amendment sentence
verbatim while citing §13.2.4.3, so it contradicted the section it pointed at; it now reads
"Returned while the read's history ring has advanced fewer than `k` times — the reading
recurrent's own advances, not elapsed commits." SPEC §13.2.4's `counter` example comment
("on first commit" → "on its first evaluation") and the two `recurrent[1] avg` example
comments in §13.2.4 and §13.2.4.3 ("over last 3 commits of `input`" → "over the last 3
advances of `avg`") were editorial only — advances and commits coincide in those examples —
but carried the register the batch replaced. Scope flag for the owner: M1.md line 4 names
the corpus baseline as DECISION_LOG.md + SPEC.md + IR_GRAMMAR.md only, so GRAMMAR.md sat
outside the declared M1 batch; it was swept here on the M0 precedent (M0.md R62, which
swept GRAMMAR.md as part of a corpus batch). Ruling needed on scope, not on content.)
(Round-5 amendment: two more sites of the same phrase-class were swept and are folded into
the list above. SPEC §13.11.4's opening sentence ("a one-commit delay element" → "a
one-*advance* delay element", with the per-advance rule and its §13.2.4.1 cross-reference
appended); the two `recurrent[1] avg` example comments were re-worded once more to keep the
averaged quantity visible — "Moving average of `input` over `avg`'s last 3 advances" — since
round 4's phrasing named the window but dropped the subject. DECISION_LOG 024-13, the entry
§13.11.4 mirrors, was amended in the same pass so the LOG leads the SPEC: "one-commit delay
element" → "one-advance delay element", with the no-slot-consumed clause appended.)

## FND-30 — a gate transition must be known at step 1, but the predicate is a derived that evaluates at step 3: the scheduling is circular as written. RULED (owner, 2026-08-23) — fork 3 = (a) pre-pass, literal prior-commit reads; the rider was later ruled (a) and WITHDRAWN (owner, 2026-08-24 — FND-36); M1.md R73
§13.9.1 makes a `when` predicate "for reactive-evaluation purposes, a **derived**
expression of type `bool`"; §13.9.8 and §13.10.2 step 2 make a gate transition "a **dirty
root**"; dirty roots are fixed in step 1 and deriveds evaluate in step 3. Step 1 must
therefore know something that does not exist until two steps later (C-34). Compounding
it, §13.9.7/§13.9.8/023-57 say **unqualified** that "the gate predicate itself keeps
reading prior-commit values" while §13.10.2 step 3 gives ordinary deriveds this-commit
values, and §13.9.6 introduces the prior-commit rule specifically for CYCLIC
self-reference (C-35); gate predicates are never listed among step-2's DAG node kinds.
(Cite correction, post-batch: both SPEC sites were rewritten by this batch to route the
rule through the step-1a pre-pass, so the unqualified sentence now survives only in
DECISION_LOG 023-57.) **C-36 was mis-stated in this entry and is corrected here.** The
original text read "whether a predicate may read cells INSIDE the subtree it guards is
never stated (C-36)". That is false. SPEC §13.9.6 *Self-conditional gates* opens with "A
gate predicate may reference cells of the gated instance itself", works the
`when: weight > 0.0` example against the instance's own `attr weight`, extends the
permission to node types, and derives the latching consequence; DECISION_LOG 022-36,
022-37, 022-38, 022-39 and 022-55 carry the same rules. C-36 is answered by the corpus,
affirmatively. The consequences for R73's rider are ledgered as **FND-36**.
Options: (a) a step-1 predicate pre-pass against prior-commit values, with the
transition set joining the roots; (b) predicates as ordinary step-3 DAG nodes with
transitions applied next commit (contradicts 022-59's explicit same-commit snap); (c)
predicates as ordinary step-3 nodes scheduled FIRST, reinterpreting the prior-commit
sentence as loose prose.
RULED (owner: *"Fork 3 - (a)"*): predicates evaluate in a STEP-1 PRE-PASS (step 1a)
against PRIOR-COMMIT values — the live buffer at commit start — and the transition set
joins the dirty roots in step 1b. §13.10.2's "No new dirty bits are added during the rest
of the commit cycle" therefore stays true without reinterpretation. The unqualified
prior-commit sentence is taken LITERALLY: a staged write publishes at commit N and the
predicate sees it at N+1, so the flip and its same-commit snap happen at N+1 — the
observable one-commit delay is witnessed (M1.md §6.3's witness 4). **RIDER, as originally
put:** a predicate reading any cell inside its own guarded subtree is REJECTED AT LOAD
with a named class rather than defined. The rider was put and ruled on this entry's
mis-statement that C-36 was a corpus gap; it is not, so the rider collided with landed
corpus law and was **referred to the owner as FND-36 — who ruled (a) on 2026-08-24 and
WITHDREW it**. C-37 does NOT go with it: 022-55 is a positive corpus rule, not a carve-out
that self-subtree rejection dissolved, so it stays OPEN as the ruling's residue.
EXECUTED for the C-34/C-35 half (M1 corpus batch, 2026-08-23): DECISION_LOG 023-69 (step
1's two sub-steps, the pre-pass, and the N/N+1 reading) with 022-37 amended in place (the
prior-commit rule generalized off self-reference); SPEC §13.10.2 step 1's rewrite into
steps 1a and 1b, §13.9.6's opening paragraph, §13.9.7's snap sentence, and §13.9.8's
transition paragraph. The R79 gate cadence defaults landed with it: 022-123 (C-39/C-40 —
an inner gate's predicate still runs under a closed ancestor, only EFFECTIVE activation is
observable, an ancestor reopen replays nothing), 023-74 (C-13 — `@reset_on_reopen` clears
in step 1a, on the effective reopen edge, before step 3's reads), 033-270 (C-45 — gate
entries take entry rank), 033-271 (C-50 — a gate's boolean occupies no graph cell and every
predicate runs each commit in rank order); mirrored in SPEC §13.9.7's **An inner gate under
a closed ancestor still runs** paragraph, §13.2.4's **When the clearing happens** paragraph,
§13.10.2 step 1a's every-predicate/rank-order/reset paragraph, §15.4.1's **A gate's boolean
occupies no graph cell** paragraph and §15.4.6's **Entry order is semantic** paragraph, plus
IR_GRAMMAR §3's gate note, entry-order note and the §6 `gate` row. The trace vocabulary
(C-38, and C-39/C-40's trace half) is FND-32's plan law and takes no corpus edit.
The rider half takes NO corpus edit, and it is now RULED (a) BY THE OWNER (2026-08-24):
WITHDRAWN. The wave-1 code had enforced it as `gate_predicate_reads_guarded`; that check
and its class are deleted — see FND-36, CLOSED.

## FND-31 — a deferred's dispatch has no pinned commit position and its readiness has no named verb. RULED (owner, 2026-08-23) — fork 4a = after step 4 / before step 5; fork 4b = `has_staged_work`; M1.md R74
§13.10.2 says a deferred dispatches "At the commit's end — after evaluation has settled,
so the values being published are known" and snapshots "this commit's **newly committed
values**". The first clause points BEFORE step 5's flip, the second AFTER it: the values
are identical either way (a flip changes visibility, not values) but the trace position
and `teardown` semantics differ (C-19). Separately, 027-128 promises "the runtime
surfaces staged-work presence to the host through the same surface as queued writes", yet
§13.14.2 defines no readiness query and §13.14.11's mandatory-core list has none — while
M0's `pub fn commit(&mut self)` returns nothing, so a host cannot learn that a deferred
is pending (C-31, rework register RW-6). Options for 4b: (a) `commit()` returns a status
struct; (b) a separate `has_staged_work(&self) -> bool` query; (c) neither in M1.
RULED (owner: *"Fork 4a - dispatch after step 4, before step 5 / Fork 4b - b), with (c)'s
restraint about scope"*): dispatch happens AFTER step 4 and BEFORE step 5's flip, so
`publish` stays the last value-bearing line of a commit block. Under M0.md R1's
sequential rendering, dispatch = evaluate the behavior synchronously over the pending
values and HOLD `(value, due = commit_index + depth)`; nothing waits and the publication
barrier is vacuous. The readiness surface is exactly `pub fn has_staged_work(&self) ->
bool` — true iff a held run has not yet landed — with `commit()`'s signature untouched
and nothing more in M1. Cadence defaults executed with it (M1.md R79, veto standing):
provenance is dirty iff any input cell's dirty bit is set this commit (C-28); a landing
this commit counts as provenance-dirty for a downstream deferred dispatching at this
commit's end (C-29); landings fold in by the rank walk and dispatches run in rank order
(C-20/C-21); construction evaluates a deferred synchronously like a derived (C-23,
016-298); `depth 0` on a deferred is a load error (C-24/C-7/C-8).
EXECUTED (M1 corpus batch, 2026-08-23): DECISION_LOG 023-73 (the pinned dispatch
position) with 027-49 amended in place; SPEC §13.10.2's step-4/step-5 dispatch sentences
and its **Deferred cells in the commit** paragraph, §13.2.12's **Dispatch** paragraph,
§13.14.4's and §14.6's `commit()` descriptions. The R79 cadence defaults carried by this
finding landed with it: 023-75 (C-28 + C-29, the dirty-bit test and the chained-landing
rule), 023-76 (C-20/C-21, rank at both ends), 023-77 (C-23, synchronous construction),
016-301 (C-7/C-8/C-24, positive depth and `invalid_history_depth`), mirrored in SPEC
§13.2.12 (**What "provenance is dirty" tests**, **Multiplicity is ordered by rank**,
**Initial value**, and the `deferred[0]` sentence), §13.10.2's deferred paragraph,
§13.2.8's `N = 0` paragraph, §15.4.6's **Depth is positive** paragraph, and IR_GRAMMAR
§3's depth note plus the §6 `cell` row. `has_staged_work` (fork 4b) is a runtime-surface
ruling with no corpus footprint due — it lands as code, not as corpus law.
(Round-5 amendment: the depth-zero sweep was incomplete at four sites, now swept and folded
into the list above. DECISION_LOG 016-296 still assigned `deferred[0]` a meaning ("degenerating
to `derived T`") and so contradicted 016-282 and 016-301 inside the LOG itself; it now reads
"`deferred[0]` never written — the in-commit rung of the kind ladder is spelled `derived`, and
a written `[0]` is rejected at load". SPEC §13.2.8's consolidated kind table carried the same
superseded reading in its `recurrent[N] T` and `deferred[N] T` rows, 120 lines below the
qualifying **`N = 0` is a rung of the ladder, not a spelling** paragraph and unqualified by it;
both rows now state that `[0]` is never written and is rejected under `invalid_history_depth`.
GRAMMAR.md's **`deferred[N]` latency depth** note carried it verbatim and now conforms. The
GRAMMAR.md site falls under the same scope flag recorded on FND-29's round-4 amendment.
Also in round 5, three SPEC sites that read "history depth (default 0)" for a stream
recurrent were made explicit about WHICH case defaults to 0 — §13.18.3's **Self-history
depth** bullet, its `H`-omitted sentence, and §13.2.8's `recurrent[H] stream` table row now
say `H = 0` is the *prefix-absent* case, since a written bare `recurrent stream` is
`recurrent[1] stream` by 030-99 / 030-207 / §13.18.8.1. No law changed and 016-301 was not
re-scoped: its unqualified "an omitted depth on a recurrent is 1" is the LOG's own rule for
both cell and stream recurrents; only the SPEC's word "omitted" was ambiguous between
"no bracket" and "no prefix".)

## FND-32 — no trace record exists for ANY M1 construct: advances, landings and gate transitions have no vocabulary. RULED (owner, 2026-08-23) — fork 5 = (c): reuse `dirty`/`eval`, add exactly one event kind; M1.md R75
The corpus defines "the output trace" ONLY as the deterministic evaluation schedule
(§13.10.3). There is no trace-record content for a recurrent advance (C-14), for a
deferred landing or dispatch (C-22), or for a gate open/close (C-38) — and R24's
minimal-semantic ruling, the only authority, was written before any of them existed
(C-51). Answering the three questions one at a time would produce an incoherent
vocabulary, so they were put as one ruling. Options: (a) reuse the existing five events
(an advance is an `eval`, a landing is a `dirty`), leaving a gate transition
unrepresentable; (b) add construct events (`land`, `advance`, `gate`); (c) (a) for
everything with a value shape, plus one new event for the thing that has none.
RULED (owner: *"Fork 5 - (c)"*): a deferred landing renders as `dirty <path> <old> ->
<new>` — it IS a dirty root by the corpus's own words; a recurrent advance renders as
`eval <path> -> <value>` — it IS the production of the cell's value; and a gate
transition earns EXACTLY ONE new event kind, `gate <path> open` / `gate <path> close`,
byte-exact and one line, canonical exactly like the five existing events. A separate
`land`/`advance` event would record WHICH MECHANISM produced a value — implementation
shape, which R24 bans. Cost accepted with the ruling: a reader cannot distinguish a
landing from a host write in the trace, which is correct because a downstream consumer
cannot either. Cadence rider (M1.md R79, C-39/C-40): the `gate` line records EFFECTIVE
activation, so a nested gate flipping under a closed ancestor updates its own state
silently and traces nothing.

## FND-33 — step 1's own text is written for writable cells only; the general form with non-host dirty roots is never stated. RULED (owner, 2026-08-23) — fork 6 = (a): one rank-ordered walk, all sources interleaved; M1.md R76
§13.10.2 step 1 reads "For each writable cell (signal, attr), compare its staged value to
its previously-committed value", yet the same section names three more root sources — gate
transitions, deferred landings, and membership changes (C-52). M0's `dirty_set` is the
literal transcription of the quoted sentence: a rank-order walk over cells, dirtying
`Input`s whose staged ≠ committed (rework register RW-3). Generalizing it decides the
ORDER of the trace's `dirty` lines, which is currently rank order and is the property that
makes traces permutation-invariant (`reversing_each_commits_writes_changes_nothing`).
Options: (a) one rank-ordered walk with all root kinds interleaved by rank; (b)
source-major — landings, then transitions, then host writes, each rank-ordered within its
group; (c) rank-ordered with a distinct trace event per source kind (depends on FND-32).
RULED (owner: *"Fork 6 - (a)"*): ONE rank-ordered walk with ALL root sources interleaved
by rank, so the trace's root lines are a single rank sequence whatever produced them. (b)
would make the trace reveal the mechanism, which R24 forbids; (c) is foreclosed by
FND-32's ruling. The landed permutation-invariance property survives verbatim, and the
step-1a predicate pre-pass (FND-30) sits in front of this walk, not inside it.
EXECUTED (M1 corpus batch, 2026-08-23): DECISION_LOG 023-70 (step 1's general form as one
rank-ordered walk over all three root sources); SPEC §13.10.2's **Step 1b — the root
walk** paragraph, with the writable-cell comparison demoted to "one case of the walk, not
its definition". The rank-at-both-ends corollary for deferred multiplicity is 023-76
(ledgered under FND-31), and the gate half — that a `gate` entry takes entry rank like
every other entry — is 033-270, mirrored in SPEC §15.4.6's **Entry order is semantic**
paragraph and IR_GRAMMAR §3's entry-order note.

## FND-34 — the corpus does not define what a wire propagates or when engagement runs: `wire` is unbuildable at runtime. PARKED by the scope ruling (owner, 2026-08-23: "Temporal + gates") — M1.md R70 / §8.4
Measured against the corpus: §13.6 (the entire wire chapter) is 373 lines ≈ 1.4 % of
SPEC.md; **§13.10.2 — the six commit steps — contains the string "wire" ZERO times**;
§13.14 (the whole Runtime Interface) mentions wires once, in the shutdown drop order;
chapter 14 never describes a wire's runtime object; IR_GRAMMAR gives one production and no
worked example. Eighteen gaps were catalogued (M1.md §8.4's C-53…C-70), two of them
MILESTONE-BLOCKING rather than merely blocking: **C-53** — the corpus never states what a
wire PROPAGATES (§13.9.7 says a wire "delivers"/"propagates" six times; §13.6.2 and
§13.3.7.6 say the meaning is EXTRINSIC — "the wire type itself fixes only acceptance,
never what engaging it does" — and no section names a payload); and **C-57** — engagement
has no runtime realization (§13.3.7.6 fixes an ordering, but no commit step performs
traversal, and "engagement vs liveness" is STRATEGY §3's named wire deliverable). The rest
compound them: "input cells" is a term of art defined nowhere (C-55); a wire's own cells
have no IR home (C-56); `attrs` has three incompatible encodings across two unmapped
vocabularies (C-60); re-point's one-commit skew has no mechanism (C-58) and collides with
the Model-B same-commit snap (C-59); §13.10.3's tiebreak does not reach a wire's cells
(C-65); and no trace event exists for any wire phenomenon (C-68), so the deliverable has
no observable to assert against. PARKED with FND-35 to the successor milestone (M1.md
§2.3) whose FIRST HALF IS CORPUS WORK — these are authoring questions, not implementation
questions, and no amount of runtime code answers them. The M1 loader keeps rejecting
`wire` with a named diagnostic whose message states the corpus boundary, not a slice
boundary.

## FND-35 — the stream corpus is detailed and internally contradictory: four flat contradictions plus an observation surface that does not type-check. PARKED by the scope ruling (owner, 2026-08-23: "Temporal + gates") — M1.md R70 / §8.5
§13.18 is large and detailed (sixteen subsections, 274 decision entries) — the opposite
problem from FND-34's silence. Twenty-one day-one gaps were catalogued (M1.md §8.5's
C-71…C-91). The four flat contradictions: recurrent-stream default capacity (§13.18.2 and
030-24 say the self-history `N` "is *not* added to the buffer"; §13.18.8.1's own four
worked examples compute exactly `capacity + N`); signal-only stream sources (030-28 lists
one as VALID; 030-29, 030-88, 030-91, §13.18.2 and §13.18.7.4 all make it a compile
error); capacity-default scope (030-22 sums across all reactive inputs, adjacent 030-23
and §13.18.2 sum stream inputs only); and minimum source arity (030-237 requires at least
one reactive input, §13.18.15/030-89/030-91 require at least one STREAM input). Beside
them the observation-cell surface is broken in three independent ways: the six cells have
no IR lowering (C-78), their ids collide with R66's one-namespace law (C-79), and their
types do not type-check under §4.5.5's explicit-cast boundary (C-80) — while
`last_overflow_at` needs a host clock the language core cannot construct (C-77), in direct
tension with "same program, same inputs, same output trace". Add: `pending_count` is
undefined for zero consumers and for `ring` policy (C-75/C-76); push visibility is stated
three incompatible ways (C-81); streams are named in no commit step (C-83); ordering is
undefined in four places (C-84); and no trace event exists for a push, emission, overflow,
rejection, or cursor advance (C-89). PARKED with FND-34 to the successor milestone (M1.md
§2.3): this is ledger work — a corpus repair round, ruled unhurried — not code work, and
discovering it mid-build is exactly what the parking prevents.

## FND-36 — R73's rider makes SPEC §13.9.6 unloadable: a gate predicate reading its own guarded subtree is corpus-legal, with a worked example — and the landed code enforced the rider anyway. RULED (a) BY THE OWNER ("(a) continue.", 2026-08-24, quoted in GATE.md) — the rider is WITHDRAWN; EXECUTED; CLOSED (raised by verification of the M1 corpus batch, 2026-08-23)
**Background.** R73's unstruck rider (FND-30) says a gate predicate that reads any cell
inside its own guarded subtree is REJECTED AT LOAD, under a named class — the plans call
it `predicate_reads_guarded_subtree`, the landed code registers it as
`gate_predicate_reads_guarded`. The rider was put and ruled on the premise, stated in
FND-30 and repeated in M1.md §4.7 and §8.3's C-36 row, that the corpus never opened the
question. **The premise is false**, and FND-30 now carries the correction.

**The problem.** SPEC §13.9.6 is a whole subsection — *Self-conditional gates* — that
answers it affirmatively, with an example:

```
wire WeightedLink:
  from: Node
  to: Node
  attr weight: f32 = 1.0
  when: weight > 0.0                                  // self-conditional
```

Its opening sentence is "A gate predicate may reference cells of the gated instance
itself"; it extends the permission to node types; and it derives a consequence — a
self-gate reading a same-instance `recurrent` is latching (the recurrent freezes, so the
predicate's input never changes back), while one reading an `attr` or `signal` can reopen
(input cells stay live under a closed gate). DECISION_LOG carries the same as 022-36 (the
permission plus this example), 022-37 (what a self-referential predicate reads), 022-38
and 022-39 (the latching consequence and its exception), and 022-55 ("Deriveds read by
the `when` predicate stay current while gated"). Under the rider, every construct in
§13.9.6 fails to load — including the compiler-synthesized **wake gate** (§13.3.7.6),
which is self-referential by construction and which §13.9.6 explicitly makes an heir of
this rule. Aggravating: this batch amended §13.9.6's prior-commit sentence while leaving
its permission sentence and its example untouched, so the landed corpus and the landed
plan now flatly disagree — the exact position M1.md §4.1 says no ruling is in.

**A second, positive rule goes with it.** M1.md §8.3's C-37 row calls §13.9.7's
"minimum subgraph needed for predicate evaluation live" a non-issue "with self-subtree
reads rejected". But 022-55 is a rule, not a carve-out contingent on the rider: a derived
read by a `when` predicate stays *current* while gated. If the rider falls, C-37 is a live
implementation obligation, not a CONSEQUENCE row.

**Options.**

(a) **Withdraw the rider; the corpus stands.** The landed check and its
`gate_predicate_reads_guarded` class come OUT (a code change, wave 3 or later).
A self-subtree read is legal, and R73's own main ruling is what defines it: the
step-1a pre-pass evaluates every predicate against prior-commit values, so a predicate
reading a cell it guards reads that cell's previously-committed value — no circularity,
no special rule.

```
gate  App.g0  pred B@p1  inputs [App.count]  guards [App.count]   // legal under (a)
```

C-37 then needs its own answer: either implement 022-55's liveness carve-out (a derived
in the guarded subtree that a predicate reads is exempt from freezing) or ledger it as a
separate finding.

(b) **Re-scope the rider to the residue the corpus really leaves open.** §13.9.6 covers
`attr`/`signal`/`recurrent`/`derived` reads on the gated instance. It does not reach a
predicate reading a `deferred` inside its own guarded subtree, where gate-close cancels
in-flight runs (023-65) and the predicate's own input is what the cancellation depends
on. Reject exactly that shape, keep the class, and leave §13.9.6 whole.

(c) **Execute the rider as ruled and withdraw the corpus law LOG-first.** 022-36, 022-38,
022-39 and 022-55 are struck or narrowed, §13.9.6 is deleted or reduced to the wake-gate
case, and §13.3.7.6's wake gate needs an explicit exemption — the compiler synthesizes a
self-referential predicate, so a blanket rejection would reject the compiler's own
output.

**Recommendation: (a).** The rider's justification was "the corpus never opened this
question"; the corpus opened it, answered it, worked an example, and derived a
consequence. M1.md's own law settles the collision — where a ruling and the corpus
disagree the corpus wins and the ruling becomes a finding to be executed LOG-first — and
here there is nothing to execute, because R73's main ruling already supplies the
definition the rider said was missing. (c) is the only option that deletes a documented
feature, and it costs a wake-gate exemption on top. (b) is defensible if the owner wants
a class kept, but its residue is narrow enough to be a lint rather than a load error.

**WAVE-2 UPDATE (2026-08-23) — what is actually in the tree, and what this entry now
says.** Two facts were established after the paragraphs above were written, and they
change the entry's status wording, not its options:

1. **The code ENFORCES the rider.** The wave-1 build landed the load-time rejection:
   `crates/ductus-runtime/src/loader/load.rs`'s check 8 rejects a predicate reading any
   cell its own gate guards, **transitively through the `gate_parent` chain**, with the
   registered class `gate_predicate_reads_guarded`. It is exercised by two loader unit
   tests; the property generator declines to draw the shape
   (`crates/ductus-conformance/tests/properties.rs`); **no corpus witness asserts it**,
   because a corpus witness would be corpus law and the rider contradicts §13.9.6. The
   plan and this ledger meanwhile recorded the rider as "BLOCKED / not implemented".
   **That internal inconsistency is itself part of the finding** — the documents and the
   tree disagreed about what the runtime does.
2. **The corpus evidence is stronger than "a collision", and it is now confirmed on
   disk.** SPEC §13.9.6 *Self-conditional gates* PERMITS the pattern outright ("A gate
   predicate may reference cells of the gated instance itself"), with the
   `when: weight > 0.0` example, the node-type extension and the latching consequence;
   the 022 gate family carries the same rules (022-36 permission + example, 022-37 what a
   self-referential predicate reads, 022-38/022-39 latching and its exception, 022-55
   deriveds read by the predicate stay current while gated); and §13.3.7.6's
   compiler-synthesized **wake gate** is self-referential by construction and explicitly
   inherits §13.9.6 — so a blanket rejection rejects the compiler's own output.

**Disposition for wave 2: NOTHING WAS DECIDED.** The enforcement code and its tests stayed
**exactly as landed** — wave 2 neither withdrew nor blessed them. The entry stayed OPEN,
and the plan documents were corrected to the truth at every rider site — M1.md §4 (the
preamble), §4.7, §5.3's row, §6.3's witness 12, §8.3's C-36 row, R73's ledger entry and
exit criteria 8 and 10 — in the shorthand *enforced in code; ruling pending*. The fork
then went to the owner.

**RULED (a) BY THE OWNER, 2026-08-24 — *"(a) continue."*, quoted in GATE.md.** The rider is
**WITHDRAWN**. A gate predicate reading cells inside its own guarded subtree is LEGAL;
SPEC §13.9.6 and 022-36/37/38/39/022-55 stand untouched, and nothing replaces the check,
because R73's MAIN ruling already defines the read — the step-1a pre-pass evaluates every
predicate against prior-commit values, so a predicate reading a cell it guards reads that
cell's previously-committed value. No circularity, no cycle to break, no special handling.

**EXECUTED.** Nothing was landed corpus-side: a withdrawal executes as a code deletion,
since the corpus already said what the rider had denied. In the tree now:

- `crates/ductus-runtime/src/loader/load.rs` — the wave-1 check (its check 8) and its
  rejection unit tests are **gone**; a unit test named
  `a_predicate_reading_a_cell_its_own_gate_guards_loads` asserts the shape loads,
  carrying the ruling in its comment;
- `crates/ductus-ir/src/diag/diag_class.rs` — the `GatePredicateReadsGuarded` member and
  its `gate_predicate_reads_guarded` id are **deleted**, so no path can emit it;
- `crates/ductus-runtime/src/engine/engine.rs` — a unit test pins the observable
  consequence: a self-conditional gate's transition trails its own guarded change by
  exactly one commit, which is the prior-commit read made visible.

**Two things this ruling leaves behind, both named rather than closed with it.**

1. **C-37 stays OPEN.** 022-55 — *"Deriveds read by the `when` predicate stay current
   while gated"* — is a positive corpus rule, not a carve-out the rider dissolved. With
   the rider withdrawn it is a live implementation obligation: whether M1 implements that
   liveness exemption still needs its own answer (M1.md §8.3's C-37 row). It is **NOT
   implemented in M1** — a derived inside a closed gate's subtree freezes even when the
   gate's own predicate reads it — and the implement-vs-park question is raised at the M1
   finish report.
2. ~~**M1.md §6.3's witness 12 has no corpus pair yet.**~~ **DISCHARGED (wave 2).** Under
   (a) the witness inverts into an assertion that a §13.9.6-shaped self-conditional gate
   LOADS and runs, and the pair now exists:
   `corpus/duir/m1/self_conditional_gate.{duir,trace}`, hand-derived and byte-compared by
   `crates/ductus-conformance/tests/gates.rs`. Its golden pins the ruling's consequence as
   well as its letter — the transition trails the guarded change by one commit, and the
   gate then LATCHES, because the value its predicate reads is the value it froze. The
   stale `properties.rs` comment that cited the deleted check is gone with the generator's
   other wave-2 restrictions (FND-38).

## FND-37 — two M1.md §5 load-time checks are unwritable through the D5 provider seam: behavior arity and predicate result type. CONSEQUENCE, with one sub-question FILED-OPEN (raised by remediation of the M1 build, 2026-08-23)
**Background.** D5 (M0.md) makes a behavior an opaque
`Rc<dyn Fn(&[Value]) -> Value>` supplied by a `BehaviorProvider`, and the `.duir` module's
own `behaviors` table is empty throughout M0 and M1 — the provider registry is the whole
resolvability answer. Nothing about a behavior's SIGNATURE crosses that seam: not how many
parameters it expects, not what it returns.

**The problem.** Two rows of M1.md §5 were planned as load-time checks and cannot be:

- **§5.1, `behavior_arity_mismatch`** — R71 pins the ABI order and therefore makes the
  expected arity exactly computable, `inputs.len() + depth + sum(lookback)`. The
  computation is not the obstacle; the COMPARISON is. There is no counterpart to compare
  it against, so the loader has nothing to check. Recorded in the code at
  `crates/ductus-runtime/src/loader/load.rs`'s check-7 comment.
- **§5.3, `gate_predicate_not_bool`** — §13.9.4 says "The predicate must have type `bool`.
  A non-bool predicate is a compile error." That sentence binds the `.duc` FRONTEND, which
  sees a typed expression; the `.duir` loader sees a closure. Recorded at
  `crates/ductus-runtime/src/engine/engine.rs`'s `predicate` doc comment.

**What happens instead.** Both surface as D8 TRAPS naming their construct, which is the
landed precedent for every wrong-typed stub: a cell whose behavior returns the wrong type
already traps through `checked()` rather than failing to load, and a non-bool predicate
traps naming its gate ("predicate for gate `App.g` produced a `i64`"). Nothing is silently
wrong; the failure moves from load time to first evaluation.

**Status: CONSEQUENCE, not a ruling request.** Neither row needs an owner decision — the
engineering is forced by D5, the diagnostics exist as traps, and the compile-time check for
the predicate's result type belongs to the frontend milestone, where a type is visible.
What is recorded here is the CORRECTION: M1.md §5.1 and §5.3 promised two **NEW** registered
classes, neither is registered, and the plan rows now say so. Adding a `DiagClass` member no
loader path can ever emit would be worse than the trap.

**One sub-question FILED-OPEN.** Extending the seam — e.g. a
`BehaviorProvider::arity(handle) -> Option<usize>` hook — would make the arity check
writable at load. That is a change to D5's shape, which is an M0 ruling, and M1's ruling
record does not authorize it. Filed for the owner, never a blocker: the trap already
catches the mistake, one evaluation later.

**Ledger effect.** FND-28's EXECUTED clause carries a correction sentence for its "and the
loader checks it" elaboration; M1.md §4.9's arity sentence, §5.1's and §5.3's rows, and
exit criterion 8's "nothing else waits on it" are conformed to this entry.

## FND-38 — whether a reader of a `recurrent` ever sees the advance depends on DECLARATION ORDER: the landed schedule is nondeterministic and the corpus never opened the question. RULED-BY-CADENCE (wave-2 orchestrator disposition, veto standing; raised at the M1 finish report) — the advance-root law, M1.md §4.12 / R80
**Background.** A recurrent's result is HELD through step 3 and written in step 4 (R72 +
R77, M1.md §4.4). So the value a consumer of a recurrent should read, and the commit it
should read it in, is decided by one thing the corpus never states: **what an advance does
to the dirty set.**

**The defect wave-1 review found.** In the landed build a recurrent's dirty bit is set in
the MIDDLE of step 3 — it is an evaluation OUTCOME, not a step-1 root — and a read of a
recurrent is not an edge of the per-commit DAG (§13.11.3, which treats history reads as
inputs to break cycles). Put those together and the schedule reads off the ENTRY ORDER:

```ductus-ir
// order A — the reader is declared second
recurrent App.r : i64 uses B@r inputs [App.x] depth 1 init 0
derived   App.d : i64 uses B@d inputs [App.r]

// order B — the same two entries, swapped
derived   App.d : i64 uses B@d inputs [App.r]
recurrent App.r : i64 uses B@r inputs [App.x] depth 1 init 0
```

Under order A the reader is visited after the recurrent's bit is set, so it
evaluates every commit. Under order B it is visited while the bit is still clear,
skips, and **never gets another chance** — step 6 clears the bit. One dependency graph,
two declaration orders, two different settled values. Nothing in DECISION_LOG, SPEC or
IR_GRAMMAR licenses that: 033-260 makes entry order the rank carrier for TIEBREAKS among
ready nodes, not a switch that decides whether a cell evaluates at all.

**The generator's dodge — disclosed here because it was disclosed nowhere else.** The
property-test generator does not draw the losing arrangement. It structurally guarantees
the recurrent is reached first (a recurrent's inputs are `input`/`recurrent` cells only,
so it has in-degree zero; any cell reading a recurrent is EMITTED after it). The
restriction is written in `crates/ductus-conformance/tests/properties.rs`'s module doc —
**a code comment only.** So the properties were green over a generator that had been
narrowed to avoid the defect, and the narrowing never reached this ledger or the plan.
That is the second half of the finding: the dodge is not wrong as engineering, it is wrong
as an undisclosed one.

**Disposition — CADENCE COMPLETION, veto standing (M1.md R80 item 1, §4.12).** The corpus
is silent, the silence has no reading that permits declaration-order nondeterminism, and
the completion is the smallest law that removes the choice: **a recurrent's advance at
commit N — bits-changed only — makes the recurrent a DIRTY ROOT at commit N+1's
rank-ordered root walk, exactly as a deferred landing does, except that it renders NO
trace line** (the change was already observable as the advance's own `eval` line at N).
Inside commit N, in-pass readers read the pre-advance value and the recurrent's evaluation
sets NO dirty bit for step 3's staleness test — so a reader whose only dirty dependency is
an advancing recurrent does not evaluate at N under EITHER order, which is what kills the
order dependence. At N+1 the root sets the bit, consumers evaluate over the published
value, and a recurrent reading another recurrent advances **one commit behind it** — the
delay chain. Empty-commit idempotence is unchanged where no advance root is pending.

**Consequences — where each one stands.** The law landed LOG-first as **023-78** in
DECISION_LOG.md, mirrored into SPEC §13.10.2's step 4 (*"Consumers read the advance one
commit later"*), so M1.md exit criterion 10 is MET for it and the id earlier drafts left
for reconciliation is settled. Two witness goldens CHANGED — `lockstep_pair` (the reader
advances one commit behind its source) and `equal_value_advance` (the reader's `eval`
moves one block later, and the equal-valued advance is no root at all); a third,
`self_cycle`, keeps every block it had and GAINED one, because the law has a consequence
that witness alone can show: **a cycle closed through a recurrent free-runs.** Each
advance roots the next commit, which re-evaluates the reader, which dirties the
recurrent — so `self_cycle`'s graph keeps turning once per commit after the writes stop,
where before the law it went quiet. That is inside §4.12 (idempotence was only claimed
where no advance root is pending) but it was UNWITNESSED and the witness's own doc
comment asserted the opposite, so the golden gained a quiescent block and the comment was
corrected. Every other m1 golden was re-derived and is unchanged: no other m1 module
reads a recurrent at all.
**Discharged in wave 2:** the generator's two restrictions are LIFTED —
`crates/ductus-conformance/tests/properties.rs` now draws readers on BOTH sides of their
recurrent and recurrents fed by computed cells, its module doc states the law rather than
the pre-ruling mechanics, and the vacuity guard carries a floor for each of the three
shapes so a collapsed generator cannot make the claim vacuous again. The engine and the
commit-indexed oracle agree over the widened domain, which is this entry's regression
guard. The same file's stale comments naming the deleted
`gate_predicate_reads_guarded` class and the renamed `conflicting_gate_binding` are out.

**Status: not an owner ruling.** It is an orchestrator cadence action under the standing
instruction (LEARNINGS #36/#35), disclosed at the M1 finish report with the veto standing.
If the owner vetoes the completion, the alternative is not the landed behavior — that is a
defect either way — but a different law for the same question.

## FND-39 — a `deferred` reading a `recurrent` DROPPED advances and re-dispatched over stale ones: 023-73's dispatch position and 023-78's advance root collide at step 4½. CONSEQUENCE — forced by the two landed rulings; EXECUTED (wave-2 remediation, 2026-09-02)
**Background.** Two landed rulings meet at step 4½. **023-73** pins a deferred's dispatch
AFTER step 4's advance: *"the run snapshots exactly the values the commit is about to
publish, recurrent advances included."* **023-78** (the advance-root law, FND-38) says a
recurrent's evaluation sets NO dirty bit in the advancing commit; the bit is set by the
root at the NEXT commit.

**The defect.** The landed dispatch tested provenance as `dirty[input]` for every input
kind. For a recurrent that bit names the PREVIOUS commit's advance while the arguments
gathered a line later are THIS commit's — so the two disagree in both directions:

```
input     App.x  : i64 init 0
recurrent App.r  : i64 uses B@r inputs [App.x]  depth 1 init 0   // r = x + r.past(1)
deferred  App.p  : i64 uses B@p inputs [App.r]  depth 1          // p = r + 1
```

Writes `x = 1, 2, 3` then quiet. `r` advances to 1, 3, 6. The landed build dispatched
nothing at the first advance, then dispatched at each following commit over the value that
commit had JUST advanced to — `p` carried 4 and 7 and never 2 — and then dispatched a
fourth time at the quiescent commit, where the advance root fired but nothing advanced, so
the run repeated a value that had not moved. A derived reader of the same `r` saw every
advance. One cell, two readers, one of them lossy.

**Disposition — CONSEQUENCE.** No new law: 023-73 already says what the run snapshots, and
023-78 already says the dirty bit is not that. The bit that IS that is the `advanced` bit
step 4 has just written, so a deferred's provenance asks a RECURRENT input for `advanced`
and every other input for `dirty`. Each advance now reaches a deferred exactly once, in
advance order, over the value the same commit publishes.

**The visible consequence, stated rather than hidden.** A deferred sees an advance one
commit EARLIER than a derived does. That is the same law, not an exception to it: a
derived reads in step 3, which runs BEFORE the advance, so §4.12 delays it a commit to keep
the schedule out of declaration order's hands; a deferred reads at a fixed position after
step 4 that no declaration order can move, so it needs no delay and is given none.

**Where it landed.** `crates/ductus-runtime/src/engine/engine.rs` (`dispatch_deferreds` +
the new `moved` predicate), the same rule in the property oracle
(`crates/ductus-conformance/tests/properties.rs`), the re-derived engine witness
`a_dispatch_reads_the_advanced_values_and_leaves_publish_last`, and a regression pinning
the landing sequence, `a_deferred_reading_a_recurrent_carries_every_advance_exactly_once`.
No corpus golden moved: no `.duir` witness has a deferred reading a recurrent — which is
the gap that let this stand, and §6 should close it in the next witness pass.

## FND-40 — WHICH cells read a recurrent's pre-construction value is decided by declaration order, and a released reader need not touch the cycle at all. OPEN (raised by wave-2 review of the M1 build, 2026-09-02; scope corrected by round 2 of the same review, same day — the first statement confined the release to cycle MEMBERS and was wrong)
**Background.** §13.11.3 makes a read OF a recurrent return the previous-committed value
and calls that *"the rule that breaks reactive cycles"*; §13.11.4 makes a recurrent on a
cycle *"a one-advance delay element"*; §13.2.4.1 has every triggered recurrent read *"the
previous-committed values of all recurrent cells in the system (including other triggered
ones)"*. So a value cycle closed through a recurrent is legal, and the corpus says so three
times. §13.2.6's init-time-cycle rule names the OTHER category in the same breath —
*"This is distinct from runtime cycles (§13.11), which the per-commit DAG handles via
recurrents-as-delays"* — so it is not a rejection of this shape.

**What the corpus never says** is what a read of a recurrent yields at CONSTRUCTION, where
no previous commit exists. The loader's ordering pass releases the rank-least node whose
unmet predecessors are all recurrent reads, and that node reads the recurrent's slot before
the recurrent has run. Two halves of that are unspelled, and only one is now settled:

- **Settled by cadence (veto standing): WHAT it reads.** The slot is seeded with the
  recurrent's **depth-1 fallback**, not a typed zero. §4.9's fallbacks are exactly *"what
  a history read yields before the ring has advanced that far"*, and before construction no
  advance exists — so the released reader gets a value the source spells. Landed in
  `Engine::start`; `placeholder()` is now the derived/deferred seed only. Pinned by
  `a_read_of_an_unconstructed_recurrent_yields_its_fallback`.
- **OPEN: WHICH cells read it.** The release is `rank`-least, and rank is declaration
  order. The candidate test is `is_node && !queued && cyclic[rank] == 0`
  (`load.rs`, `released`), and `cyclic[rank]` counts a node's UNEMITTED derived/deferred
  predecessors. It says nothing about whether `rank` lies on a cycle. So the candidate set
  is not "the cycle's members" — it is **every waiting node whose remaining unmet
  predecessors are all recurrent reads**, which includes a plain reader sitting entirely
  outside the strongly-connected component. One cycle can also force SEVERAL releases: the
  released node emits, the ready set empties again, and `released` is called a second time.

  The probe, executed against the landed build. `App.outside` has one input and lies on no
  loop; only `App.d` is on the cycle:

```ductus-ir
input     App.x       : i64 init 1
recurrent App.r       ... inputs [App.d] depth 1 init 100   // the cycle:
derived   App.d       ... inputs [App.r, App.x]             // r -> d -> r
derived   App.outside ... inputs [App.r]                    // NOT on the cycle
```

  Declared with `App.outside` FIRST the construction snapshot is
  `outside=100, d=101, r=201` — `outside` was released and read `App.r`'s **fallback**.
  Declared with `App.outside` LAST it is `outside=201, d=101, r=201`. Same graph, same
  single in-cycle candidate, two settled snapshots, and the cell that diverges never
  touched the cycle. That is the FND-38 class of nondeterminism, moved from the commit
  schedule to construction, and nothing in the corpus licenses it.

  The in-cycle case is the same defect, one rung narrower — a cycle with two in-cycle
  candidates (`r1 -> d1 -> r2 -> d2 -> r1`, both `derived`s eligible) releases `App.d1` in
  one declaration order and `App.d2` in the mirror. Both cases are the same open question.

**Options.**
**(a) SCC-scoped construction lockstep.** Drop every recurrent read edge whose source and
reader lie in one strongly-connected component — an order-INDEPENDENT graph property — and
hold those recurrents' constructed values until the component is finished, so every
in-cycle reader reads the fallback and every downstream reader reads the computed value.
Deterministic under any declaration order. It also settles the out-of-SCC half above for
free: `App.outside`'s edge is not inside the component, so it is never dropped, so
`App.outside` constructs after `App.r` under EITHER declaration order and reads the
computed `201`. Cost: SCC analysis in the loader, contiguous per-component ordering, and a
hold pass in `Engine::construct` — plus a new corpus sentence saying construction reads a
cyclic recurrent at its fallback.
**(b) Reject the shape at load.** Read §13.2.6 as covering recurrent-closed cycles too.
Cheapest, and it contradicts §13.11.3/§13.11.4/§13.2.4.1 — it would make the mutual
recurrent pair, the canonical lockstep shape, unloadable.
**(c) Keep the rank-least release and RECORD it as law.** State in the corpus that a
cycle's construction values are read off entry order. Honest, zero code, and it makes
declaration order semantic in a place 033-260 only made it a tiebreak.

**Recommendation: (a).** It is the only option that keeps every sentence the corpus already
has and removes the choice rather than blessing it; (c) blesses exactly the thing FND-38
was ruled to kill. It is NOT taken by cadence: it needs a new corpus sentence about
construction reads, which is a language-course call, so it stands OPEN for the owner.

**The sibling that IS closed.** A recurrent naming ITSELF among `inputs` — a self VALUE
edge, not the `depth` ring — is now rejected at load as `recurrent_self_reference`
(M1.md §5.1). 016-51 puts prior self-values behind `.previous`/`.past` alone, and
§13.2.4.1 denies the current one even to those accessors, so the read has no source
spelling; the landed loader admitted it and §4.12 then made the cell its own staleness
trigger, advancing once per commit forever with no host activity. The class name is a
cadence-default naming (veto standing).
