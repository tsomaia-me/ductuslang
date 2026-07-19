# Wire-Rename + Channel-Vocabulary Amendment — Execution Plan

*2026-07-19. Final batch of Part 2. Authority: audit-plan-2 rulings ledger (CONNECTION-RENAME RULED IN PRINCIPLE; NAME RULED "wire"; PIN (2) SUPERSEDED — CHANNEL VOCABULARY RULED; PRELIMINARY CHANNEL-SIDE SITE LIST; FINISH-LINE AUTHORIZATION covers execution without a separate approval gate). Survey below is fresh (2026-07-19, /usr/bin/grep on the working tree).*

## 1. The two rulings (condensed from the ledger)

**WIRE (owner, 2026-07-18: "wire").** The connection construct renames connection → wire under three locked pins: (1) FULL UNIFORMITY — declaration keyword `wire`, intrinsic marker trait `Wire`, IR primitive `wire`, and connection-view vocabulary → wire-view all rename together; no surface/IR divergence. (2) originally: wiring-type vocabulary kept with one disambiguating sentence — SUPERSEDED by the channel ruling below. (3) sequencing: own amendment after main-removal, before the Part 3 survey. Blast radius: §019 wholesale, 002-3 keyword list, 005-183 marker triple, sealed-kind lists, SPEC §13.6 chapter + corpus-wide prose, GRAMMAR productions/keyword box, IR_GRAMMAR six-primitives + connection entries. `Link` (022-75 example) stays a free user identifier; example type names (`WiresTo`, `Drives`, …) are user identifiers and stay. "wire-candidate envelope" stays (becomes literal). The `incoming:`/`outgoing:` clause KEYWORDS are unchanged.

**CHANNEL (owner, 2026-07-19: "go with channels. settled.").** The "wiring type" class term RETIRES; the applied kind annotations (`cell T`, `signal T`, `derived T`, `recurrent[N] T`, `stream[P] T`, `yielded T`, `dynamic view T`) are CHANNELS — a prose/LOG term, NOT a keyword (nothing reserved; `Channel` stays a legal user identifier, unlike `wire`). Class definition keeps type-system membership: "a channel — a type-system member, unstorable by nature: a mechanism that delivers values reactively, never a value." Two flavors: VALUE CHANNELS (the cell umbrella — deliver the latest committed value, snapshot-readable) and EVENT CHANNELS (streams — deliver each event once per cursor); yielded = a membership channel, dynamic view = a reference channel. 016-286's host-written "two channels" is PROMOTED to an instance of the general class (one value-channel form, one event-channel form), not reframed as contradiction. Crossings razor respells: the arrows mint between channel shapes (`sig->changes` mints an event channel from a value channel; `->latest(fallback)` mints a value channel from an event channel). Informal "wiring" prose audited site-by-site: class-sense → channel; verb/activity-of-connecting sense → may stay (now harmonious with wire).

## 2. Survey — corpus state before edits

File sizes: DECISION_LOG.md 4456 lines; SPEC.md 25842; GRAMMAR.md 6630; IR_GRAMMAR.md 618.

Pattern counts (grep -c, lines):

| pattern | LOG | SPEC | GRAMMAR | IR_GRAMMAR |
|---|---|---|---|---|
| `\bconnection\b` | 250 | 411 | 77 | 3 |
| `\bConnection\b` | 16 | 32 | 9 | 0 |
| `\bconnections\b` | 52 | 93 | 7 | 0 |
| `\bConnections\b` | 6 | 18 | 0 | 0 |
| `connection-view` | 43 | 48 | 5 | 0 |
| `wiring type` | 10 | 13 | 13 | 0 |
| `wiring-type` | 2 | 1 | 0 | 0 |
| `wiring` (any) | 30 | 50 | 15 | 0 |
| `\bwired\b` | 1 | 2 | 2 | 0 |

Regression baselines (must be unchanged after edits): `.changes` member-form = 0/0/0/0; `->changes` = 18/43/4/0; `to_signal` = 0 everywhere; `vec::new` = 2/12/0/0; `public(` = 0; `three-level` = 0; backticked `shared` = 0; `\bmain\b` whitelist = 4 (LOG) / 11 (SPEC) / 3 (GRAMMAR) / 0; `@root` = 6/9/13/0. `Channel` as user identifier: LOG 2 (017-37, 017-52), SPEC 5 (13930, 13997, 16886, 16968, 17101 — all `node Channel` example family), GRAMMAR 1 (4559) — ALL STAY.

### 2.1 DECISION_LOG.md — wire side (300 lines carry [Cc]onnection)

Per-section [Cc]onnection line tallies: 001:2, 002:3, 003:3, 004:1, 005:16, 008:1, 009:1, 013:3, 015:7, 016:17, 017:89, 018:7, 019:53, 020:5, 021:39, 022:16, 023:4, 024:7, 027:1, 028:1, 029:3, 030:2, 031:3, 032:4, 033:12.

Named must-hit sites:
- 002-3 (line 74): declaration keyword list — `connection` → `wire`.
- 002-5 (line 76): connection-view direction semantics sentence.
- 005-183 (544): intrinsic markers — `Connection` → `Wire`; 005-195 (556) not-@derive-eligible triple.
- 005-241 (602) + 005-243 (604): sealed lists — `sealed connection` → `sealed wire`; diagnostic class names unchanged unless they contain "connection" (they do not).
- §019 wholesale (lines 2660–2741): header `## 019. Connections — 79 Rules` → `## 019. Wires — 79 Rules`; all 79 entries dense 019-1..019-79 (verified dense pre-edit); construct word respells in every grammatical form (noun/plural/adjectival); `from:`/`to:`/`pairs:`/`pair` endpoint machinery, Circularity, cardinality content UNCHANGED.
- 017 family (89 lines incl. 017-17, 017-106, 017-109, 017-125, 017-127, 017-132, 017-167 DOUBLE-HIT, 017-170, 017-176, 017-219 exposition sum tag `Connection` → `Wire`, 017-269, 017-271, 017-272).
- 033 IR entries: 033-56 (4233) six primitives → "cell, wire, gate, stream, effect, scope"; 033-63/64 (4240/4241); 033-65 (4242); 033-86..89 (4263–4266) incl. `connection_type` → `wire_type` (full-uniformity pin; FLAGGED default); 033-124 mentions none.
- All remaining construct-sense mentions in 001, 003, 004, 008, 009, 013, 015, 016, 018, 020, 021, 022, 023, 024, 027, 028, 029, 030, 031, 032 conform. Expected legitimate English-sense survivors ≈ none; classify any found.

### 2.2 DECISION_LOG.md — channel side (32 wiring/wired lines)

Class-definition family: 016-62 (1948), 016-163 (2049), 016-166 (2052), 016-178 (2064, THE taxonomy home — ruled definition sentence + both flavor sentences land HERE), 016-180 (2066, outermost-only razor). 017-189 (2373, never-nests law). 030-36 (3593, "a stream is wiring, not a value" → "a stream is a channel, not a value"), 030-45 (3602, wiring-type family), 030-47 (3604), 030-51 (3608), 030-261 (3818). Crossings razor: 030-269 (3826), 030-270 (3827), 030-271 (3828, "mints new wiring" → channel), 030-272 (3829, "arrow access mints new wiring" → channel; dot-projects razor). 016-286 (2172) PROMOTION. DOUBLE-HIT 017-167 (2351).

Informal-wiring audit sites (classify each; activity-sense stays): 013-4 (1423, category-C "reactive wiring" — activity, stays), 013-24 (1443, parameter/return wiring — activity, stays), 013-247 (1666, stays), 013-248 (1667, "produces wiring" — activity, stays), 015-12 (1854, topology = the wiring among instances — activity, stays), 015-14 (1856, "only the wiring moves" — activity, stays), 015-16 (1858, frozen wiring — activity, stays), 017-139 (2323, parameterized wiring — activity, stays), 017-144 (2328, internal wiring — activity, stays), 021-17 (2801) + 021-21 (2805, placement wiring category C — activity, stays), 024-15 (3129, every possible wiring — activity, stays), 029-5 (3435, "wired into the reactive graph" — verb, stays), 031-33 (3866, program-side wiring — activity, stays), 033-204 (4381, behaviors hold no wiring — activity, stays), 034-9 (4431, compile-time wire set — stays, literal), 034-13 (4435, wiring positions — POSITION vocabulary, see judgment J4), 034-16 (4438, "implicit wiring-to-value reads" — loss law, see J4).

### 2.3 SPEC.md — wire side (554 [Cc]onnection lines)

Per-##-section tallies: §1:5, §2:1, §3:30, §5:5, §6:1, §10:5, §11:3, §13:461, §14:4, §15:16.

Headers renaming: 1533 (§3.1.7 "Required node/connection members"), 13300 (§13.2.10 "Node and connection type slots"), 14465 (§13.3.4 "Connection-views"), 14599 (§13.3.4.2 "Self-sourced connections …"), 15303 (§13.3.7.5 "Connections in exposition"), 16479 (§13.6 "Connections" — chapter title + body 16479–16834), 16763 (§13.6.3 "Generic connections"), 16782 (§13.6.4 "Behavior lives outside the connection body"), 16835 (§13.7 "Name Resolution in Node and Connection Scopes"), 17464 (§13.8.4 "Connections"), 17561 (§13.8.4.2 "Placing a connection type value"), 17640 (§13.8.5.1 "For connection placements"). Section NUMBERS unchanged.

Notable: 15239 `wires: Connection*` (marker as catch-all selector → `Wire*`; entry name `wires` is a user identifier, stays); 15409/15426 exposition sum tag `Connection` → `Wire`; 16486 "active channels" describing connections — COLLIDES with the new class term, respell (judgment J2); §15: six-primitives line 25083, connection entry field list ~25156–25170 (`connection_type` → `wire_type`), module-grammar mirror production 25668. §13.19 (22711–23843) wire-candidate envelopes stay.

### 2.4 SPEC.md — channel side (53 wiring/wired lines)

§11 activity-sense (8542, 8555, 8647, 10354, 10360 — category C machinery, stays). §13.1 (11820, 11843, 11848, 11853 — topology wiring, stays). §13.2.8 class-definition mirror (12846, 12848, 12910, 12920, 12974–12979, 12992 table row, 13006–13015) — ruled definition + flavors mirror HERE. §13.3 (14489, 14492, 14627, 14672, 14762 DOUBLE-HIT mirror of 017-167). 16512 "rewiring" (activity, stays). 17157/17240/17243/17284 (activity, stays). 19028/19143/19183 (activity, stays). 20345 (wired into graph, stays). 20415 (yielded as wiring — J4). §13.18: 21149, 21265, 21273, 21317, 21324, 21362, 21367 (wiring type → channel), 21880 (§13.18.9 razor — channel shapes). 23050/23289 (program-side wiring, stays). §13.20: 24009–24058 (wiring positions / loss law — J4). 25049 ("the reactive wiring — six primitives" comment — reads as activity; harmonize with wire list), 25502 (behaviors hold no wiring, stays).

### 2.5 GRAMMAR.md

Wire side (77+9 lines; per-section: §2:5, §3:1, §5:1, §6:1, §7:10, §8:26, §9:42, §10:1, §11:25, §12:5, §13:6): keyword box line 232 (`'connection'` → `'wire'`); §9 title "Connection declarations" → "Wire declarations"; production renames (shapes identical, rename only): ConnectionDecl→WireDecl, ConnectionBody→WireBody, ConnectionBodyMember→WireBodyMember, ConnectionBodyDecl→WireBodyDecl, ConnectionBodyCellDecl→WireBodyCellDecl, ConnectionDestBody→WireDestBody, ConnectionPlacement→WirePlacement, ConnAcceptanceEntry→WireAcceptanceEntry, ConnNamedAcceptance→WireNamedAcceptance, ConnUnnamedAcceptance→WireUnnamedAcceptance; bare `Connection` marker mentions → `Wire`; TopLevelDecl comments; all comment prose.

Channel side (13 wiring-type lines): 1318–1349 (§3.15 KindAnnotation area — "the **wiring types**" class block → channels), 3810, 4029, 4037, 4143, 4157, 4201, 4662 (bracket-policy comments "same wiring type" → "same channel"), 4676 ("wired into node bodies" — activity, stays), 5044 ("inbound or outbound wiring" — activity, stays), 6464–6485 (Appendix B stream/cell rows — wiring type → channel).

### 2.6 IR_GRAMMAR.md (4 relevant lines)

Line 18 "on-the-wire text shape" — English, STAYS (classified survivor). 185 `entry ::= cell | gate | connection | effect | stream` → `wire`. 247 production `connection ::= 'connection' PATH …` → `wire ::= 'wire' PATH …` (name AND leading terminal). 592 §6 table row `connection` → `wire`. §5 worked example (485–553) contains NO connection entry — nothing to edit there (brief anticipated lines that do not exist; noted).

## 3. Judgment defaults (executed under FINISH-LINE AUTHORIZATION; all FLAGGED in the final report)

- **J1 — `connection_type` → `wire_type`** (LOG 033-88, SPEC 25161): the IR data-model field is keyed off the construct name; leaving it would be a surface/IR divergence pin (1) forbids. Grep `\bconnection\b` does NOT match `connection_type` (underscore is a word char), so this is explicitly gated separately.
- **J2 — SPEC 16486 "active channels"** (describing connections/wires): respelled to avoid colliding with the new formal class term; wires are not channels. Target wording: "active couplings" or equivalent; exact wording in report.
- **J3 — 017-167 final wording**: "persistent wiring between instances is a connection" → the sentence is about graph linkage; "wire" is the noun; the "wiring" phrasing is replaced, NOT respelled to channel. Target: "persistent linkage between instances is a wire". Mirror at SPEC 14762. Exact final wording in report.
- **J4 — "wiring positions" / "wiring-to-value" (034-13/16, SPEC §13.20, 20415, GROUP-SNAPSHOT vocabulary)**: these name the position class where a group/stream passes as un-materialized reactive machinery — the CLASS sense. Respell to "channel positions" / "channel-to-value" ("implicit channel-to-value reads"). This is the loss-law vocabulary; the ledger's crossing respell ("arrows mint between channel shapes") pulls the same register. FLAGGED — if reviewers find it degrades, fall back to keeping "wiring" as activity-sense.
- **J5 — exposition sum tag** `Connection` → `Wire` (017-219, SPEC 15409/15426): language-owned sum entry kind named after the construct; full uniformity renames it.
- **J6 — flavor-sentence placement**: full class definition + both flavor sentences land ONCE at 016-178 (the taxonomy's "single home") and mirror in SPEC §13.2.8/§13.2.8.1; the umbrella triplets (016-62/163/166, 030-47/51) get plain channel respells ("umbrella value channel" / "channel") without repeating the flavor definitions.

## 4. Execution phases (all subagents SYNCHRONOUS; editors opus, greps haiku; same-file editors sequential; LOG → SPEC → GRAMMAR → IR_GRAMMAR; no commits; invariant-2 on every edited entry)

- **L1** (opus): LOG sections 001–016 (lines 1–2182): 002-3/002-5, 005 family incl. marker + sealed lists, 013/015 classifications, 016 channel-definition family + 016-286 promotion.
- **L2** (opus): LOG sections 017–021 (lines 2183–2929): 017 family incl. 017-167/017-189/017-219, 018, §019 WHOLESALE, 020, 021.
- **L3** (opus): LOG sections 022–035 (lines 2930–4456): 022/023/024/027/028/029, 030 channel razor + crossings, 031/032, 033 IR entries incl. six primitives + wire_type, 034 J4 sites.
- **S1** (opus): SPEC §1–§12 (lines 1–11803): headers 1533, §3 trait-system mentions, §5/§6/§10/§11.
- **S2** (opus): SPEC §13.0–§13.7 (lines 11804–17077): §13.2.8 channel mirror + taxonomy table, §13.2.10, §13.3.4 chapter, §13.6 wholesale, §13.7 title, 14762 double-hit, 15239/15409/15426, 16486 J2.
- **S3** (opus): SPEC §13.8–§15 end (lines 17078–25842): §13.8.4/.5, §13.9–13.17, §13.18 channel razor sites, §13.19, §13.20 J4, §14, §15 six-primitives + entry fields + 25668 production.
- **G1** (opus): GRAMMAR whole file (both renames).
- **I1** (opus): IR_GRAMMAR (3 edits).
- **Gates** (haiku, per-file): see §5.
- **Reviewers**: three scoped blind opus reviewers — R1 DECISION_LOG, R2 SPEC, R3 GRAMMAR+IR_GRAMMAR; adjudicate; valid fixes sequential; unclear → flag.
- Final diff captured to scratchpad wire-channel.diff.

## 5. Gate list (exact commands, /usr/bin/grep, per file)

1. `\bconnection\b` = 0 and `\bConnection\b` = 0 and `\bconnections\b` = 0 and `\bConnections\b` = 0 in all four docs (classify any survivor; expected: none except IR_GRAMMAR line 18 is "on-the-wire", which does not match these patterns anyway).
2. `connection_type` = 0; `connection-view` = 0; `wire-view` present (LOG/SPEC/GRAMMAR counts ≈ old connection-view counts).
3. `wiring type` = 0 and `wiring-type` = 0 in all four docs.
4. LOG header exact: `## 019. Wires — 79 Rules`; §019 dense 019-1..019-79 (count = 79).
5. LOG 033-56 line reads six primitives `cell`, `wire`, `gate`, `stream`, `effect`, `scope`; SPEC six-primitives line matches.
6. GRAMMAR keyword box contains `'wire'`; `'connection'` = 0 in GRAMMAR and SPEC and IR_GRAMMAR.
7. `sealed wire` present in 005-241 and 005-243; `sealed connection` = 0.
8. LOG 005-183 contains `Wire` marker triple.
9. IR_GRAMMAR: `wire ::= 'wire' PATH` production present; entry alternative and §6 table row list `wire`.
10. Channel definition sentence present at 016-178 and SPEC §13.2.8 mirror: grep "a type-system member, unstorable by nature: a mechanism that delivers values reactively, never a value".
11. Flavor sentences present: grep "value channels" and "event channels" and "membership channel" and "reference channel" (LOG + SPEC).
12. 016-286 coherent: grep "value-channel" AND "event-channel" (or equivalent flavor words) in 016-286's line; "exactly two channels" framing preserved as instance-of-class.
13. Crossing razor respelled: 030-271/272 + SPEC §13.18.9 contain channel wording; "mints new wiring" = 0.
14. `Channel` not in any keyword list: GRAMMAR §2 keyword box and §13 reserved-identifiers contain no `Channel`/`channel`; LOG 002-3 contains no `channel`.
15. Dense headers: every touched `## NNN.` header keeps its pre-edit count (019=79; others per current file).
16. Standing regressions equal baselines of §2 above.
17. Invariant-2: changed LOG lines carry no entry-number citations beyond each entry's own id (`grep -nE '[0-9]{3}-[0-9]+' on changed lines`, excluding own ids).

## 6. Reviewer plan

Three blind reviewers (opus, synchronous, read-only), each given ONLY: the two condensed rulings (§1 above) + their file scope + the survivor-classification rules. Asked to report: missed construct-sense "connection"; missed "wiring type"; over-renames (English-sense casualties, user identifiers renamed, endpoint keywords touched); content drift beyond the word moves; header/count damage; invariant-2 violations. Adjudication: valid → sequential fix; invalid → recorded; unclear → FLAG to owner, never improvise.
