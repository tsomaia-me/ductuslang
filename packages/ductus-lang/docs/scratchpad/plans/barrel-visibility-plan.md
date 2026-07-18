# Barrel-Visibility Amendment Plan — the public API becomes `public.duc`

*2026-07-16 · Executes the owner rulings recorded in the audit-plan-2 ledger (BARREL-VISIBILITY
entries). Status: ALL rulings LOCKED incl. §3 = option (a), `type(private) Email:` (owner, 2026-07-16); keyword confirmed `export` (never `expose` — taken by the node structural-output clause).
Survey pin: HEAD fd3cf60 + streams batch in flight; ALL line numbers drift — locate by
content/entry-id; re-grep SPEC.md with raw /usr/bin/grep (the rtk hook corrupts line numbers
in the file's back half).*

## 1. The rulings (owner, 2026-07-16 — locked)

- **In-source `public` and `shared` RETIRE.** One in-source visibility keyword survives:
  `private` (module-only). The unnamed default stays package-visible.
- **A package's public API = ONE barrel file, `public.duc`, at the package root.** Entries
  are headed by the reserved keyword `export`:
  ```
  export audio::Mixer
  export audio::Mixer as Desk
  export audio::(Mixer as Desk, SomeOtherThing, SomeOtherOtherThing as Tldr)
  ```
  The entry grammar mirrors the `use` selection-list grammar (parenthesized groups, `as`
  renames, multi-segment paths, nesting per the §10.4.1 model) **minus glob**: `export x::*`
  is REJECTED — the barrel is a curated contract; globs silently grow the API.
- **No barrel = nothing public.** Applications need no file. A library is a package with
  exports and no roots; an app is roots and no exports (mirrors the @root philosophy).
- **Leak rule = compile error.** The exported surface may reference only exported (or
  built-in) types, applied member-wise to public members of exported types.
- **Member model (i).** Members (fields, attrs, cells, constructors) keep `public`/`private`:
  `private` = module-only; unnamed default = package-visible; member-`public` = *part of the
  exported surface when the enclosing type is barrel-exported* — inert otherwise, so no
  action-at-a-distance. Exported data-carrier records mark consumer-facing fields `public`;
  that is the explicitness price, accepted.
- **One barrel per package.** `public` concerns exactly one boundary — the package boundary;
  non-root module barrels would be semantically vacuous. Deep entries use paths.
- **`export` is a reserved word everywhere** (no contextual keywords), legal only in the
  barrel file's grammar.
- **The parenthesized constructor-visibility form (`public(private) type Email`) retires** —
  replacement is the ONE OPEN POINT (§3).

## 2. Survey results (2026-07-16; full detail in the survey transcript, key structure here)

- **LOG 003 (§10.x, 77 rules)**: PURE retire = 003-2 (shared level) + 003-63..66 (constructor
  form). REWORD ≈ 20 entries (003-1, 003-3, 003-5, 003-19/20/21, 003-26..34, 003-42,
  003-49/50, 003-67, 003-68/69, 003-72, 003-77). SURVIVES = the rest (module model, use
  machinery, orphan, dispatch).
- **Other LOG sections**: 009-36/37/38 (fields) REWORD; 009-40/41, 009-121/122, 013-234 PURE
  (constructor form mirrors); 013-235 survives; 004-87 (const default) REWORD; 029-78/79/80,
  030-32, 031-76/77 (per-kind three-level mirrors) REWORD; 005-113/119/154 survive.
  DO-NOT-TOUCH false positives (shared-state senses etc.): 007-11, 012-100, 013-101, 016-11/
  42/93/98, 017-138/152/198, 018-64, 020-28, 027-60, 031-156, 032-23, 032-92/94/96/125,
  033-5/37.
- **SPEC**: §10 chapter (≈7786–8365) is the core rework — §10.1 (levels), §10.3 (specifier
  master site), §10.4.2 (re-export → superseded by barrel), §10.5 + §10.5.1 (retire), §10.6,
  §10.7, §10.8. NEW section: the barrel file (home it as §10.5's replacement or a new §10.x):
  file name/location, entry grammar, no-barrel default, leak rule, one-per-package.
  Outside §10: §2.4.1.4 (const), §6.1.6/§6.1.7 (fields/constructor), §6.2.6, §6.3.4,
  §11.12.1 (smart-constructor prose), §13.9.4, §13.17.2, §13.17.9 (operators mirror —
  DEFINITIONAL duplicate, must move in the same batch), §13.18.2, §13.19.2, §13.19.10
  (effects mirror — same), §14.2.3 (manifest — public.duc lands beside it; do NOT touch its
  `main` mention, main-removal owns it). 26 worked-example prefix lines respell (drop
  `public`/`shared` prefixes or re-frame around the barrel).
- **GRAMMAR**: keyword box visibility line; §7.1 Visibility production collapses to
  `Visibility ::= 'private'`? — actually the production becomes optional-`private` at 26
  decl-head sites; §7.2 ConstructorVis retires (pending §3); §7.3 FieldVisibility collapses
  to `'public' | 'private'` with the re-grounded public meaning; NEW: barrel-file grammar
  (ExportItem mirroring UseItem minus glob) + `export` in the keyword inventory; re-anchor
  the loose §7.1/7.2/7.3 citations (currently all cite 003-1).
- **IR_GRAMMAR: zero visibility content — untouched** (visibility is source-only, erased
  before IR; consistent with 032-23).

## 3. THE ONE OPEN POINT — where does a private constructor live now?

The smart-constructor pattern (validated construction: type visible, raw construction
restricted — 003-66/009-41/009-122/013-234, SPEC §10.5.1/§11.12.1) currently rides the
parenthesized form. The constructor has no declaration line of its own to prefix, so
retiring `public(private)` leaves the capability homeless. Options:

```
(a) host the inner spec on the `type` keyword:   type(private) Email:  wraps string
(b) a body metadata item:                        type Email:  constructor: private  wraps string
(c) kill the feature: private ctor via factory   (validated construction = private field trick?)
```

RULED: **(a)** (owner, 2026-07-16). Original recommendation: **(a)** — the surviving inner spec moves onto `type`, reading "type with
private construction"; minimal grammar delta (ConstructorVis relocates, inner values now
`private` only — the outer keyword is gone and `shared` is dead, so the middle case
"package-only constructor on an exported type" is deliberately unsupported: use a private
constructor + package-visible factory). Unmarked constructor follows the type's full reach
(exported type ⇒ callable cross-package), preserving 003-64's spirit.

## 4. Execution (phased, LOG-first — after §3 is ruled)

A. LOG 003 rework (PURE + REWORD per §2; new entries: the barrel file, entry forms, no-glob,
   no-barrel default, leak rule, one-per-package, member-public re-grounding, constructor
   rule per §3; `export` reserved; `shared`+in-source-`public` freed as identifiers SILENTLY
   — same policy as satisfies/main, zero reservation notes).
B. LOG satellites (004-87, 009 cluster, 013-234/235 check, 029/030/031 mirrors).
C. SPEC §10 rework + the NEW barrel section + all outside-§10 sites incl. BOTH definitional
   mirrors (§13.17.9, §13.19.10) and the 26 example prefixes; §14.2.3 gains the public.duc
   sentence (manifest untouched otherwise).
D. GRAMMAR: productions per §2; barrel grammar; keyword inventory (visibility box shrinks;
   `export` added; classes per D-05 item-2 PARKED note now resolve: visibility class =
   {private} in source + export as barrel keyword).
E. Gates: counts/invariant-2/xref/regressions (all standing ones); gate-barrel: `'shared'`
   visibility-sense = 0 (audit each surviving 'shared' against the DO-NOT-TOUCH list);
   in-source `public` survivors = member-position only; `public(private)` = 0;
   `export` grammar present in GRAMMAR + SPEC mirror; leak rule present in LOG + SPEC;
   §13.17.9/§13.19.10 desync check. THREE scoped blind reviewers (LOG/SPEC/GRAMMAR — this
   is satisfies-retirement-sized). Diff capture. No commits.

## 5. Sequencing

After the streams batch lands (in flight) and the policies batch (six approved D-05 items)
executes — this amendment is bigger than both and touches the keyword taxonomy the policies
batch writes, so POLICIES FIRST, then this. The D-05 visibility-class item stays parked and
resolves here.
