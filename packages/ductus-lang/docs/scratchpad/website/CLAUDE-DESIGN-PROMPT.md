# Claude Design Prompt — Ductus Landing Page
*Attach the general-purpose `DESIGN-SYSTEM.md` (tokens, type, spacing, motion tokens, components). Everything language-specific — the semantic layer below — belongs to THIS page and is specified here, not in the design system.*

---

## Goal

Build the landing page for **Ductus** — a statically-typed language where compositional reactive systems are first-class and the compiled program literally *is* a graph of nodes, wires, and cells. The page's thesis: **for every other language, a diagram is a metaphor; for Ductus, it is the artifact.** The entire landing is one running program shown in two synchronized views — editable code on the left, the live circuit on the right — that the visitor touches, edits, and watches respond. Not a page *about* a program: a program that *is* the page.

Deliverable for this phase: a high-fidelity interactive React prototype (desktop-first, dark-only) with scripted-but-faithful runtime behavior (see "Behavior to implement"). The real semantics engine will be wired in later via Claude Code handoff — design every animation and state so that swap is invisible.

## Audience

Experienced engineers (systems, audio/DSP, frontend, PL enthusiasts) who are allergic to marketing pages and will close the tab at the first fake particle animation. They convert on *demonstrated* novelty. Secondary: curious developers arriving from a conference talk or HN.

---

## THE LANDING'S SEMANTIC LAYER (this page only — governs all visuals below)

### Three laws of this page
1. **Truth of representation.** The diagram is a view of the program, never an illustration next to it. If the language wouldn't do it, the pixels don't do it.
2. **Semantic motion only.** Every animation maps 1:1 to a runtime event (stage, commit, emit, evict, reject, freeze, snap, mount, dismount, behavior-swap). The page is calm until semantics happen.
3. **The runtime owns time.** Commits happen on the runtime's cadence — the visitor never presses "commit." The heartbeat is visible, but it belongs to the host.

### Semantic assignment of the accent tokens (kind = hue; this page's syntax AND diagram both use it)
- `accent-cyan` → **signal** cells (external input)
- `accent-violet` → **derived** cells (computed)
- `accent-magenta` → **recurrent** cells (memory; may show a decaying echo trail)
- `accent-amber` → **stream/event** cells and packets
- `accent-ember` → **effect** cells (the reality boundary)
- `accent-copper` → **wire** conduit bodies (structural, muted — never competes with cell glow)
- `highlight` → the commit wavefront and heartbeat
- the sanctioned 35% overlay → **frost** (gated regions: muted, *not* destroyed)
Code pane syntax tints the kind keywords (`signal`, `derived`, `recurrent`, `stream`, `wire`, `effect`) with the same hues — the two panes rhyme.

### The three entities (the diagram's whole vocabulary — instantly distinct at any zoom)
- **CELL — the atom of data. The only thing that glows.** Small uniform capsule; kind = hue; current value = luminance (value cells) or occupancy (stream cells). Cells live in two habitats: inside nodes and embedded along wires. A stream cell carries an attached buffer (ring or gate; see components).
- **NODE — the vessel.** Large rounded matte container (`surface`, 1px border, radius 14); holds its cells as a cluster of lights; named ports on the hull; child nodes nest inside. A node never glows as a whole.
- **WIRE — the conduit. The biggest structural element.** Thick directional copper cable (8px heavy stroke) from node A's port to node B's; direction always readable (taper/arrowhead at the `to` end). **A wire encapsulates its own small cells** — rendered as a strip of cell-lights embedded along the cable body, like inline components on a physical cable. A dynamic (re-pointing) destination shows an articulated tip; an unresolved or gated wire frosts.
- Hairline 1px **traces** connect cells to cells (the dependency graph) — much lighter than wires (the topology graph). Never confuse the two weights.
- Instant read: **vessel = node, cable = wire, light = cell. Data lives only in lights.**

### Motion grammar (exhaustive — nothing else moves; durations from the system's motion tokens)
| Runtime event | Animation |
|---|---|
| write staged | dim halo grows on the signal cell (`fast`) — charge accumulating; wiggling back to the committed value collapses the halo |
| commit | heartbeat fires + one `highlight` wavefront sweeps the graph in topological order (`sweep`); dirty value-cells *step* to new luminance (`fast` settle); clean commit = ring fires, nothing propagates |
| stream emit | amber packet travels source→buffer→consumer and *arrives* (never loops) |
| ring overwrite | oldest slot flash-dies; `dropped_total` ticks |
| gate reject | packet bounces off the full rack; `negative` flash; `rejected_total` ticks |
| gate close / open | frost overlay in (`base`) / out riding the same commit's wavefront |
| repeat scope mount / dismount | keyed vessel assembles with its key label (`voice_7`) / collapses inward (`slow`) |
| behavior swap (code edit) | only the cell's hash chip flips (`B@a3f1 → B@7c02`); values never blink |
| structural edit | new cell/wire/vessel assembles in place; neighbors ease (`sweep`) |

`prefers-reduced-motion`: same semantic moments as opacity/step changes; packets become slot-count changes; the wavefront becomes a sequential brighten.

### Landing-specific components (built from the system's primitives)
- **CodePane** — real editor surface; softly marked guided-editable ranges; spec-styled inline errors elsewhere.
- **CircuitPane** — the diagram canvas (Canvas/WebGL for wires+cells; DOM for labels); hand-tuned layout for the demo program.
- **HeartbeatRing** — fixed bottom-right of CircuitPane: fills across the inter-commit interval, fires at commit, label `runtime.commit() — host cadence`; host-styled cadence slider (0.5–8 Hz). **No commit button exists.**
- **BufferRing / BufferGate** — a stream cell's buffer: circular slot track (ring) vs linear slot rack (gate), each with a thin pressure gauge — which is a real derived in the program, not chrome.
- **HashChip** — tiny Mono content-hash chip on behavior-bearing cells.
- **CellDossier** — click any cell: kind, type, committed value, staged value, behavior hash, full path (`studio.mixer.voice_7.sample`).
- **ViewToggle** — CircuitPane corner: `circuit ⟷ netlist` (the same program as IR text, kind hues preserved).
- **AphorismBlock** — the system's Display block carrying this page's law lines (below).
- **DecisionCard** — the system's stat card: live counter "4,400+ numbered decisions", plus one real quoted rule with its real id, e.g. `019-3 — A wire carries its own reactive content — attrs, recurrents, deriveds.` Ids are always `NNN-M` format.

### Bidirectional highlight (ship before any animation)
Hover a name in code → its cell/vessel/wire glows and its traces light; hover a diagram element → its declaration highlights in code. This linkage makes the two panes one truth.

---

## Layout

Full-viewport split-screen persistent stage: **CodePane left (~45%), CircuitPane right (~55%)**, fixed while a narrative rail scrolls through six movements; each movement swaps the guided-edit focus and highlights code region + circuit region. Between movements: full-bleed AphorismBlocks. Slim top nav (wordmark placeholder "DUCTUS" — trivially replaceable; links: Docs / Decision Log / GitHub). Footer carries the honesty chip: `this page runs the Ductus semantics, hand-lowered · the compiler replaces it by construction`.

## Content — the running program (law-checked Ductus; verbatim, do not alter)

```
// studio.duc — this program is running on this page
signal clock: i64 = 0                 // the host's metronome
signal cursor_x: f32 = 0.0            // your mouse — already wired in
signal cursor_y: f32 = 0.0

derived gain: f32 = clamp(cursor_x, lower: 0.0, upper: 1.0)
derived cutoff: f32 = 200.0 + cursor_y * 4200.0

node Voice:
  attr freq: f32
  derived sample: f32 = sine(freq, clock) * gain

node Reverb:
  attr size: f32 = 0.4
  derived wet: f32 = diffuse(size, clock)

node Mixer:
  children:
    dynamic voices: Voice*
  yielded levels: f32 = collect:
    repeat v in voices:
      yield v.sample
  derived mix: f32 = fold levels:
    by: fn(a, b): a + b
    else: 0.0

wire Sends:
  from: Mixer
  to: Reverb
  attr amount: f32 = 0.3              // the wire itself carries cells

node Studio:
  children:
    mixer: Mixer
    verb: Reverb
  effects:
    p = pointer_in()
  stream ring[8] taps: Tap = p.taps

@root Studio studio:
  Mixer as mixer:
    repeat cfg in setup keyed by cfg.id:
      Voice | freq=cfg.freq
    Sends | amount=0.3: verb
  Reverb as verb
```

Hero headline (fades once the visitor moves the mouse): **"The program is a circuit."** Sub: *Move your mouse — you're already in it.* CTA: **touch it**.

### The six movements
1. **Hero / arrival.** `cursor_x`/`cursor_y` glow and re-derive live as the visitor moves. Bidirectional hover-highlight active. Nothing else asked yet.
2. **Write is a proposal. Commit is the truth.** Host-chrome slider writes `cursor_x`; StagePulse between beats; wavefront at the beat. Rail prompts: *"now wiggle it up and back before the next beat"* — halo grows, collapses, and at the beat **nothing fires** (net-revert, performed by the visitor against real cadence; slow the cadence slider to make it easy).
3. **Two kinds of change.** Click a pad (host chrome) → packets into the BufferRing; spam → ring overwrites oldest, `dropped_total` ticks. Guided edit `ring[8]` → `gate[8]` → buffer swaps to a rack; spam now *bounces*, `rejected_total` ticks, pressure climbs. Lossy by declaration vs lossless by declaration.
4. **Structure lives.** Host chrome add/remove-voice buttons → keyed vessels mount/dismount inside Mixer; `levels` membership follows; the fold recomputes. Gate toggle frosts the Reverb subtree (cells hold their glow) and snaps back on the beat it reopens. The `Sends` wire's embedded `amount` cell sits on the cable — drag it.
5. **Edit it while it runs.** Guided range unlocks in `sample`: change `sine` → `saw` — the HashChip flips, the graph does not rebuild, every value persists, the tone changes next beat. Identity is the path.
6. **It's all just data.** ViewToggle flips to the netlist (same program, IR text). Below: the DecisionCard section and docs links.

### Aphorism interludes (between movements; § ticks cite real law where it exists, bare § otherwise)
`§ 023-6` *Write is a proposal. Commit is the truth.* · `§ 030-272` *Dot projects, arrow mints.* · `§ 022-7` *Gates freeze; repeat mounts; nothing else moves.* · `§` *Cells carry. Wires connect. Nodes contain.* · `§` *The program is a circuit — and the compiler checks the wiring.*

## Behavior to implement in this prototype (scripted, but faithful)

A minimal commit loop over a hardcoded model of the hero graph — no parser:
- writes stage → halo; commits on the HeartbeatRing cadence → topological wavefront; staged == committed ⇒ no propagation (net-revert must actually work, not be a video);
- value-change gating: writing an unchanged value fires nothing;
- packets/buffers per the motion grammar (ring overwrite, gate reject, pressure);
- recognized guided edits only (`ring[8]`↔`gate[8]`, `sine`↔`saw`, voice add/remove); anything else typed in an editable range gets a gentle spec-styled inline error;
- audio optional: if trivial via WebAudio, one soft synth voice per mounted Voice makes movement 4 sing; else design the audio toggle and stub it;
- `prefers-reduced-motion` per the motion grammar's fallback.

## Hard bans (beyond the design system's)

No commit button. No continuous/ambient flow along value channels. No glowing node vessels (only cells glow). No simulated/fake data streams — visitor input is the data. Nothing moves unless a runtime event happened.

## Iteration notes

Start with Movements 1+2 at full fidelity (split-screen, hover-linkage, heartbeat, stage/commit/net-revert) before widening. Typography and AphorismBlocks second. Movements 3–6 as wired mockups after. Then hand off to Claude Code to replace the scripted loop with the real hand-lowered semantics engine.
