# RFC: Ductus Mini — a curated-preset mechanism for casual domains

## Status

Draft. Captured from a design conversation; not yet locked into DECISION_LOG.md.
Scope is positioning + architectural framing, not concrete syntax or any
particular stdlib inventory. The framing is **domain-agnostic** — any single
domain with a defined casual audience can ship its own Mini preset using the
same mechanism. Music is used as the worked example throughout this RFC
because it is the immediate motivating case, not because Mini is music-specific.

## Problem

Ductus is a heavy language by design: per-cell reactivity, ownership and borrow
rules, citizenship, view/bundle/acceptance machinery, Handle/WeakHandle/Portal,
operator-trait dispatch, the full §007–§033 vocabulary. Advanced users get a
language that does exactly what they want. **Casual users in a single domain**
get a wall.

The "single-domain casual user" shape recurs across many target domains. They
typically want to:

- never declare a new `node` type (only place instances of known ones for their
  domain),
- never write a numeric type annotation (just `120`, `0.5`),
- never write an import gesture (the domain's stdlib is "just there"),
- never see `Handle[T]` / `WeakHandle[T]` / `Cell[Iterator[Handle[T]]]` in an
  error message, ever.

The question: does Ductus need a separate language ("Ductus Mini") to serve
these audiences, or can the same language serve them with packaging?

## Candidate domains (illustrative, not exhaustive)

| Domain | Casual surface (instances they place) |
|---|---|
| Music / live coding | `Clip`, `Instrument`, `Bus`, `Tempo`, `Trigger` |
| Data viz / dashboards | `Chart`, `Series`, `Filter`, `Bin`, `Axis` |
| Lighting / live shows | `Fixture`, `Cue`, `Scene`, `Chase` |
| Interactive art / installations | `Sensor`, `Actuator`, `Mapper`, `Trigger` |
| IoT / embedded control | `Sensor`, `Relay`, `Threshold`, `Schedule` |
| Game scripting | `Entity`, `Trigger`, `Spawner`, `Zone` |
| Education / kids | reactivity learning toys: `Counter`, `Light`, `Button` |
| Automation / scripting | `Job`, `Watch`, `Pipe`, `Event` |

Each line is one potential Mini *preset*. The mechanism is the same; the
curation differs.

## Position

**Ductus Mini is a preset of the executor, not a subset of the language.**

Same grammar, same parser, same semantics, same interpreter. A Mini preset
adds three configuration layers on top of the full language:

1. A curated stdlib pre-mounted in the module graph.
2. `@default` conformances set so numeric literals resolve to chosen defaults
   without annotation.
3. A Mini-mode error formatter that hides or rewrites internal-vocabulary
   terms in domain language.

Everything a Mini user writes is valid full Ductus. An advanced user opens the
same file and adds `node MyOsc:` (or `node MyChart:`, or `node MySensor:`) and
it just works. The upward path is free — that is the prize, and it falls out
at zero cost so long as the grammar isn't forked.

## What a preset ships (preset-shaped, illustrative)

| Layer | Mini preset default | Full Ductus default |
|---|---|---|
| Stdlib in scope | the preset's curated domain module | nothing domain-specific |
| Numeric default | `@default(i64)` / `@default(f64)` (or whatever fits the domain) | per-trait `@default` declared by user |
| Module imports | implicit from project tree (modules-as-folders, already in spec) | same, no change |
| Script template | starts inside `main` instance-placement context | user writes the script shell |
| Error vocabulary | domain terms; types hidden when inference succeeded | full type vocabulary |

The first four layers are already supported by Ductus's existing machinery:
modules-as-folders, the `@default` trait family, the interpreter mode, and
type inference. None of them are new language features — they're
configuration of an existing executor.

The fifth (error formatting) is the only genuinely new piece of engineering,
and it lives in the interpreter, not in the language spec.

### Worked example: a music preset

Concretely, a music Mini preset (`ductus --profile=music` or `ductus-music`,
flag detail aside) ships:

- a `music/` module pre-mounted in the module graph, exposing `Clip`,
  `Instrument`, `Bus`, `Tempo`, `Trigger`, etc.,
- `@default(i64)` for ints (so `120` resolves), `@default(f64)` for floats
  (so `0.5` resolves),
- a script template that already opens `main` so the user starts inside an
  instance-placement context,
- an error formatter that says "this expects one Clip, but you gave it a list
  of Clips" instead of "expected `Handle[Clip]`, got `Bundle[Clip]`".

The same shape applies to a data-viz preset (`Chart`, `Series`, `Filter`…),
an IoT preset (`Sensor`, `Relay`…), and so on. The preset's content differs;
the preset *mechanism* is identical.

## What a preset does NOT change

The reactive/cell/gating machinery stays exactly as specified. It is *not* what
makes Ductus heavy for casuals — it is what makes it map onto these domains
naturally:

> "this clip fires when this trigger commits" (music)
> "this chart updates when this filter changes" (data viz)
> "this relay closes when this sensor crosses threshold" (IoT)

All three are the same gating semantics, expressed in domain-natural terms.

What makes Ductus heavy for casuals is the **type-level vocabulary** (Handle,
Bundle, Cell, slice forms, traits), **module declaration syntax**,
**ownership/borrow language**, and **explicit node-type declarations**.
Mini-as-preset removes all four from the user's typing surface without
touching the semantics they actually rely on. That is the right slice, in
every candidate domain.

## The two real tradeoffs (apply to every preset)

### 1. Inference dead-ends

Even with `@default(i64)`/`@default(f64)`, inference can still fail. In full
Ductus, the user annotates. In a Mini preset, the user shouldn't have to.
This means **the preset's curated stdlib must be designed so the ambiguous
cases that require annotation in full Ductus simply cannot arise** at the
casual user's surface — e.g. every domain function is monomorphic in its key
type parameters at its API boundary; no casual call site is polymorphic
enough to need explicit type witnesses.

This is a stdlib design discipline imposed on whoever curates a preset, not
a language change. It is also a real constraint on how each domain library is
shaped.

### 2. Error-message leakage

If a Mini preset doesn't restrict the grammar, internal-vocabulary terms leak
through errors:

```
error: cannot convert WeakHandle[Cell[Iterator[Handle[Note]]]] to Bundle[Note]
```

For a casual audience this is hostile (and the specific types involved are
just as opaque if you swap `Note` for `Chart` or `Sensor`). A preset needs an
error formatter that:

- **hides types entirely** when inference resolved them cleanly (the user did
  not annotate, the system should not surface the result),
- **rewrites internal-vocabulary terms** in surfaces that must show types into
  the preset's domain language (e.g. "this expects one *Clip*", or
  "*Chart*", or "*Sensor*"),
- **falls back to full vocabulary** only when the user explicitly opted into
  advanced features.

This formatter is a real engineering item; it is not free. A well-designed
formatter framework can be **per-preset configurable** (a mapping table from
internal types to domain phrasings) so each domain reuses the same machinery.

## Secondary thing worth saying out loud

A Mini user with zero imports cannot tell what symbols are in scope just by
reading the file. Advanced users dropping into a Mini script will be even more
confused. Each preset needs some way to view its implicit prelude:

```
ductus --profile=<preset> --show-prelude
```

Small but easy to forget.

## Recommendation

Build it as a **preset mechanism**, not a fixed binary:

- **A preset selection mechanism** in the executor: `--profile=<preset>`
  flag, or per-script shebang (`#!ductus --profile=music`), or per-project
  config (`ductus.toml`'s `profile = "music"`). All three should work; pick
  one as canonical.
- **A preset interface** the executor recognizes — each preset declares its
  curated module path, its `@default` conformances, and its error-formatter
  mapping table. Anyone can write a preset.
- **A first batch of curated presets** — music being the obvious first
  candidate, but the framework supports as many as get built.
- **A reusable error-formatter framework** in the interpreter that consumes
  the preset's mapping table.

The language stays one thing. DECISION_LOG.md does not gain preset-specific
decisions. SPEC.md does not branch. There is no "Mini grammar" to maintain.

Advanced users keep everything they have. Casual users in each curated domain
get a language they can actually use. Files written in any Mini preset are
real Ductus.

## What this RFC does not answer

- Concrete API surface for any specific preset's curated module — needs
  per-preset design.
- Exact error-formatter mapping table format — needs a small spec.
- Whether `--profile` should be per-script, per-project, per-binary, or all
  three.
- Governance: who can publish a preset, where presets are discovered, how
  preset names are reserved.
- Whether presets can compose (a project pulling in `music` + `lighting`
  preset prelude simultaneously) and how preset prelude collisions are
  resolved.
- Whether the preset mechanism should land before or after the first compiler.

## Status by section

| Section | Status |
|---|---|
| Position (preset mechanism, not subset) | Draft — needs review |
| Candidate domains | Illustrative — not a commitment |
| What ships preconfigured | Illustrative — exact list per-preset |
| Worked example (music preset) | Illustrative concrete |
| Tradeoffs (inference, error msgs) | Identified — apply per preset |
| Recommendation (preset framework) | Draft — pending review |
| Per-preset stdlib API surfaces | Out of scope for this RFC |
| Preset governance | Out of scope for this RFC |
