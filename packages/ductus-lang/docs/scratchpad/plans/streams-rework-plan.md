# Streams-Rework Amendment Plan — signal-only observe; the wiring-arrow crossings

*2026-07-16 · Executes the WORLD-B ruling chain (audit-plan-2 ledger, OBSERVE-GRANULARITY
SUPERSEDED entry). Status: rulings LOCKED; plan awaiting owner approval. Survey pin: HEAD
69caff7 (note: that commit's message says barrel-visibility but its LOG/SPEC hunks are the
streams batch — disclosed). Line numbers drift; locate by content/id; raw grep for SPEC back
half.*

## 1. The rulings (owner, 2026-07-16 — locked)

- **`observe` is KIND-HOMOGENEOUS** (supersedes flat signal-only; owner 2026-07-16): the
  observe's kind = its triggers' kind; a trigger set mixing signal and stream triggers is a
  compile error. Quadrants: signal->value LEGAL (at most one firing per commit — value-land
  chain-free, lockstep total); stream->stream LEGAL (fires per event in commit order, one
  output event per input — the standing 030-66 law with structured arms; signals READ in arm
  bodies are sampled per 030-67's no-implicit-to_stream, never drive); stream->value BANNED
  (crossing = `->latest` only); signal->stream BANNED as implicit drive (spell it
  `on sig->changes as e:`). Kind is TRIGGER-determined (016-247 rework); use context must
  agree per 030-82. FLAGS 1/2/3 still dissolve (all were value-context-chaining artifacts).
- **The two stream↔cell crossings are dual LANGUAGE forms** spelled with the wiring-arrow as
  the minting-access operator (postfix, expression-position):
  ```
  mouse_x->changes                 // signal → stream: the sequence of commits
  deposits->latest(0)              // stream → derived: latest event, fallback before first
  url->changes |> skip_first
  (deposits |> scan(0, fn(a,e): a + e))->latest(0)     // the userland fold
  ```
  Razor, now syntactic: **dot PROJECTS existing state** (observation cells, `.previous`,
  `.past`, `.count` stay members); **arrow MINTS wiring**. `->` gains a third positionally-
  disjoint role (return types / pairs entries / expression postfix) — zero parse ambiguity;
  bang-access was withdrawn (owner refuted: `!` = handle!'s non-Option marker + the
  attribute-clause boolean-false form; no third sense).
- **`to_signal` RETIRES** (one crossing, one spelling). The loss-law entry (now 034-16) and
  the cannot-read-stream diagnostic hint repoint to `->latest(fallback)`.
- **Member-form `.changes` RETIRES** — every `.changes` site respells `->changes`; ALL
  existing `.changes` semantics carry over form-unchanged (per-use-site identity, placeholder
  policy resolution, initial-value event 0, one-event-per-commit, same-value-no-emit).
- **Capability intact, zero privileged operators**: scan-family stays plain stdlib code;
  `accumulate(src, init, f)` = `(src |> scan(init, f))->latest(init)` — stdlib code over the
  language crossings; `event_count`/`any`/`all` likewise.
- **D-19's half of the landed streams batch SURVIVES untouched** (030-134, 030-262..267, the
  §13.18.9 completion block, §13.18.12 drain paragraph). 030-266's consumer list keeps its
  members (they remain stdlib operators, now compositions).

## 2. Execution defaults (disclosed; veto any)

- **`changes`/`latest` are NOT reserved words** — they form a closed CrossingName list in the
  grammar, exactly the DirectiveName precedent (@root/@derive names are closed-list, not
  reserved identifiers). No D-05 violation (the no-contextual-keywords law governs keywords;
  these are grammar-position names), and existing `derived latest = …` example bindings stay
  legal. Veto → reserve both + rename ~6 example bindings.
- **`->latest` keeps its name** despite soft adjacency to stdlib `combine_latest` and example
  bindings named `latest` — cognate semantics, disjoint positions. Veto → propose alternates.
- The `->` postfix production allows ordinary operator spacing (parser disambiguates by
  position, not whitespace).

## 3. Execution (phased, LOG-first)

A. **LOG observe rework (§016) under KIND-HOMOGENEITY**: NEW homogeneity entry (trigger sets
   kind-homogeneous; kind = trigger kind; mixed = compile error; both banned quadrants with
   their explicit spellings). 016-287/288 SURVIVE reworded in place, scoped to stream-kind
   observe. 016-289/290 REWRITTEN IN PLACE to the homogeneity laws (value-kind observe =
   signal triggers only; at most one firing per commit; no chaining exists). 016-291 survives
   scoped to stream-kind observe. 016-247: kind TRIGGER-determined; context agrees (030-82).
   016-251 keeps both candidate forms split by kind. 016-253/254: simultaneous-selection for
   signal-observe; per-firing for stream-observe. 016-262/266/268 SURVIVE (stream-context
   content legal again) — verify wording. 016-270: RHS forms with trigger-kind agreement;
   binder = committed value (signal-observe) / event (stream-observe). 016-275..277 keep
   event vocabulary for stream-observe + gain signal wording. 002-9: binder names the event
   (stream-observe) or the triggering signal's committed value (signal-observe).
B. **LOG crossings (§030 + satellites)**: NEW entries — the two crossing forms, the
   minting-arrow operator (postfix, third disjoint `->` role), the projects/mints razor, the
   CrossingName closed list; RESPELL the 16 `.changes` sites (004-4, 030-25/29/88/90/91,
   030-123..129, 030-133, 030-155, 030-245); RETIRE/respell the 13 `to_signal` sites
   (016-284, 029-124, 030-48/84/86/94/130/131/175/243, 031-64, 034-16 loss law; 030-266 keeps
   its list with members as compositions); 030-268 SURVIVES reworded: the §13.18.7.1 alignment entry scopes to stream-kind observe (per-event law), consistent with homogeneity.
C. **SPEC**: §13.2.11 under kind-homogeneity (.1 binder per kind, .2 selection split by
   kind, .5 default-arm rules survive for stream-observe, .6 output kind trigger-determined +
   homogeneity/mixed-error rule, .7 use sites with kind agreement, .8 where-composition,
   .9 SURVIVES as the stream-observe firing subsection — value-context chaining and
   running-value prose DELETED, homogeneity added); §13.18.2/.7.x
   (signal-source rules respelled ->changes; error tables); §13.18.9: the bridge subsection
   retitled ("Signal-to-stream bridge — `sig->changes`"), the `to_signal` definition site
   REPLACED by the `->latest` language-form elaboration (+ the accumulate-as-composition
   note), skip_first examples; §13.18.10.1/.6; §13.18.15/16 (persist examples, diagnostic
   hint); §13.19.x worked examples (ws.inbound |> … ->latest(empty_message)); §13.20 loss-law
   elaboration; §13.2.8/§13.18.5 cell-T-exclusion prose repoints.
D. **GRAMMAR**: new Postfix alternative in §5.2 (~L1829): ArrowAccess ::= '->' CrossingName
   CallArgs? with CrossingName ::= 'changes' | 'latest' (closed list, non-reserved,
   DirectiveName precedent); comment: third disjoint role of the `->` token (return arrow
   L892/L921/L3371/L3834, PairEntry L4994 untouched); keyword inventory UNCHANGED (nothing
   reserved). IR_GRAMMAR: no-op (verify).
E. **Gates** (per file): counts/dense; invariant-2 raw diff; gate-crossings: `.changes` = 0,
   `to_signal` = 0 (both docs), `->changes` and `->latest(` present at respelled sites,
   CrossingName production present; gate-observe: homogeneity rule present LOG+SPEC (mixed trigger sets = compile error); value-context chaining vocabulary = 0 ('firings chain' / 'running (uncommitted) value' = 0 both docs); stream-observe per-event law present and scoped; both banned-quadrant spellings present; D-19 KEEP-half byte-untouched (diff shows zero hunks in 030-262..267/§13.18.12);
   standing regressions (satisfies razor, stream brackets, collect-as, policy keyword,
   zero-length row, StringifiableKey set, 'excluded at the read site' = 0). ONE blind
   reviewer (medium batch) + adjudication; capture; NO commits (verify→commit→policies per
   standing authorization AFTER owner approves this plan).

## 4. Out of scope

GRAMMAR observe-productions if any name stream triggers (verify; §8.x observe grammar —
conform trigger commentary only); recurrent-stream self-history (event-clocked .previous)
untouched — it is the stream-land fold mechanism scan builds on; the barrel/policies/
main-removal queue unchanged.
