# RFC: Ductus Mini — a curated executor for casual domains

## Status

Draft. Captured from a design conversation; not yet locked into DECISION_LOG.md.
Scope is positioning + architectural framing, not concrete syntax or stdlib
inventory. Music is the worked example throughout, but the framing generalizes
to any single-domain casual audience.

## Problem

Ductus is a heavy language by design: per-cell reactivity, ownership and borrow
rules, citizenship, view/bundle/acceptance machinery, Handle/WeakHandle/Portal,
operator-trait dispatch, the full §007–§033 vocabulary. Advanced users get a
language that does exactly what they want; casual users in a single domain
(music being the immediate case) get a wall.

The casual audience for a music dialect wants to:

- never declare a `node` type (only place instances of known ones: `Clip`,
  `Instrument`, etc.),
- never write a numeric type annotation (just `120`, `0.5`),
- never write an import gesture (the music stdlib is "just there"),
- never see `Handle[T]` / `WeakHandle[T]` / `Cell[Iterator[Handle[T]]]` in an
  error message, ever.

The question: does Ductus need a separate language ("Ductus Mini") to serve this
audience, or can the same language serve them with packaging?

## Position

**Ductus Mini is a preset of the executor, not a subset of the language.**

Same grammar, same parser, same semantics, same interpreter. The Mini binary
ships with three configuration layers on top of the full language:

1. A curated stdlib pre-mounted in the module graph.
2. `@default` conformances set so numeric literals resolve to chosen defaults
   without annotation.
3. A Mini-mode error formatter that hides or rewrites internal-vocabulary
   terms.

Everything a Mini user writes is valid full Ductus. An advanced user opens the
same file and adds `node MyOsc:` and it just works. The upward path is free —
that is the prize, and it falls out at zero cost so long as the grammar isn't
forked.

## What ships preconfigured (illustrative for music)

| Layer | Mini default | Full Ductus default |
|---|---|---|
| Stdlib in scope | `music/` module mounted (Clip, Instrument, Bus, Tempo, …) | nothing music-specific |
| Numeric default | `@default(i64)` for ints, `@default(f64)` for floats | per-trait `@default` declared by user |
| Module imports | implicit from project tree (modules-as-folders, already in spec) | same, no change |
| Script template | starts inside `main` instance-placement context | user writes the script shell |
| Error vocabulary | musical terms; types hidden when inference succeeded | full type vocabulary |

The first four are already supported by Ductus's existing machinery:
modules-as-folders, the `@default` trait family, the interpreter mode, and
type inference. None of them are new language features — they're configuration
of an existing executor.

The fifth (error formatting) is the only genuinely new piece of engineering,
and it lives in the interpreter, not in the language spec.

## What Mini does NOT change

The reactive/cell/gating machinery stays exactly as specified. It is *not* what
makes Ductus heavy for casuals — it is what makes it map onto music naturally.

> "this clip fires when this trigger commits" is intuitive, not jargon.

What makes Ductus heavy for casuals is the **type-level vocabulary** (Handle,
Bundle, Cell, slice forms, traits), **module declaration syntax**,
**ownership/borrow language**, and **explicit node-type declarations**.
Mini-as-preset removes all four from the user's typing surface without
touching the semantics they actually rely on. That is the right slice.

## The two real tradeoffs

### 1. Inference dead-ends

Even with `@default(i64)`/`@default(f64)`, inference can still fail. In full
Ductus, the user annotates. In Mini, the user shouldn't have to. This means
the **music stdlib must be designed so the ambiguous cases that require
annotation in full Ductus simply cannot arise** at the Mini-user surface — e.g.
every audio function is monomorphic in its sample type at its API boundary; no
casual call site is polymorphic enough to need explicit type witnesses.

This is a stdlib design discipline, not a language change. It is also a real
constraint on how the music library is shaped.

### 2. Error-message leakage

If Mini doesn't restrict the grammar, internal-vocabulary terms leak through
errors:

```
error: cannot convert WeakHandle[Cell[Iterator[Handle[Note]]]] to Bundle[Note]
```

For a casual music audience this is hostile. Mini needs an error formatter
that:

- **hides types entirely** when inference resolved them cleanly (the user did
  not annotate, the system should not surface the result),
- **rewrites internal-vocabulary terms** in surfaces that must show types into
  musical/domain language (e.g. "this expects one Clip, but you gave it a list
  of Clips"),
- **falls back to full vocabulary** only when the user explicitly opted into
  advanced features.

This formatter is a real engineering item; it is not free.

## Secondary thing worth saying out loud

A Mini user with zero imports cannot tell what symbols are in scope just by
reading the file. Advanced users dropping into a Mini script will be even more
confused. Mini needs some way to view the implicit prelude:

```
ductus mini --show-prelude
```

This is small but easy to forget.

## Recommendation

Build it like this:

- **A binary** (`ductus-mini` or `ductus` with `--profile=music` — flag detail
  unimportant) that selects the executor preset.
- **A curated `music/` module** that meets the stdlib design constraint above.
- **A small `@default` conformance set** for numeric defaults.
- **A Mini-mode error formatter** living in the interpreter.

The language stays one thing. DECISION_LOG.md does not gain Mini-specific
decisions. SPEC.md does not branch. There is no "Mini grammar" to maintain.

Advanced users keep everything they have. Casual users get a language they can
actually use. Files written in Mini are real Ductus.

## What this RFC does not answer

- Concrete music stdlib API surface — needs separate design.
- Exact error-formatter mapping table — needs domain pass.
- Whether the `--profile` mechanism should be per-script (`#!ductus mini`),
  per-project (`ductus.toml`), per-binary (`ductus-mini`), or all three.
- Whether other casual domains (data viz, scripting, embedded control) want
  similar presets and whether the preset mechanism should be generalized.

## Status by section

| Section | Status |
|---|---|
| Position (preset, not subset) | Draft — needs review |
| What ships preconfigured | Illustrative — exact list TBD |
| Tradeoffs (inference, error msgs) | Identified — not yet costed |
| Recommendation | Draft — pending review |
| Stdlib API surface | Out of scope for this RFC |
