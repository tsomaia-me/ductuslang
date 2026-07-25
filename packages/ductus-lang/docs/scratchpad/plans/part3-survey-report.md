# Part 3 — Re-anchor Survey Report

*2026-07-20 · read-only survey against baseline d719517 (Part 2 closed). Produced per part3-survey-plan.md: 13 topic batches (Opus verifiers, content-relocation, /usr/bin/grep evidence) + 3-verifier adversarial pass (full DISSOLVED bucket attacked; mechanical/changed/settled and ruling samples re-verified). All file:line cites below are CURRENT (fetched during this survey); all audit-doc line/id cites are known-stale and were re-anchored by content.*

**Survey-wide data-quality finding:** the audit doc's LOG entry ids in the 012 area are systematically off by one (012-46→012-45, 012-100→012-99, 012-106→012-105, 012-121→012-120, 012-82→012-81, 012-92→012-91, 012-29→012-28, 012-115→012-114, 012-111→012-110, 012-120→012-119, 016-209→016-210, 016-217→016-218, 016-285→016-286…). Every id below was re-anchored by content; any future execution must do the same, never trust the audit's ids verbatim.

## 1. Summary counts

| Bucket | Count |
|---|---|
| DISSOLVED (incl. dismiss-upheld DP items) | 17 |
| SETTLED (pre-annotated, verified on disk) | 2 |
| CHANGED (mutated under Part 1/2; residuals classified below) | 6 |
| STANDS-MECHANICAL (consent-list) | 28 |
| STANDS-RULING (ruling queue) | 50 |
| CONTESTED | 0 |
| **Total** | **103** |

Ruling queue = 50 STANDS-RULING + 2 CHANGED items whose residual is a ruling (I-06, I-74) = **52 entries**.
Consent list = 28 STANDS-MECHANICAL + 4 CHANGED items whose residual is mechanical (I-20, I-35, I-45, I-70) = **32 one-liners**.

Adversarial pass outcomes: 42 spot-verifications, 5 corrections applied — I-08 MECHANICAL→DISSOLVED; I-35 DISSOLVED→CHANGED (normativity-homing defect survives); DP-16 dismissal upheld for the effect seam only (stream residue lives as I-59); DP-17 annotation corrected (rec survives as a live option); I-61 evidence corrected (a narrow carve-out exists). DP-03/DP-09/DP-11 normalized from a batch's "STANDS-MECHANICAL (dismissal holds)" labeling to DISSOLVED (dismiss-upheld) — verified independently.

## 2. Per-item table

| id | short title | classification | current-state evidence pointer |
|---|---|---|---|
| I-01 | \|> effect-kind method operand at bootstrap | STANDS-RULING | SPEC:20720 Case-3 rule (RHS-method) vs LOG:2457 017-273 + SPEC:1299 + LOG:3495 029-65 |
| I-02 | provenance exceptions: one vs two | STANDS-MECHANICAL | SPEC:19238 "The one exception" vs SPEC:19027 + 023-46/025-6 "two sources" |
| I-03 | wake gate absent from SPEC | STANDS-MECHANICAL | 5 LOG sites (017-272 etc.); "wake gate" = 0 in SPEC |
| I-04 | two same-level tiebreakers overlap | STANDS-RULING | 023-33 (LOG:3088) vs 023-34 (LOG:3089); SPEC:18933-18941 |
| I-05 | wire derived reading `to` frozen from construction | STANDS-RULING | SPEC:18493 invariant `from`-only; SPEC:16739 carve-out; 019-53/019-60 |
| I-06 | scope drop vs six-step commit | CHANGED (residual: thin RULING) | SPEC:18855-18860 re-key = step-1 dirty root; SPEC:19921-19926 teardown subsumes suspend; scope_drop still unanchored |
| I-07 | gated-off static feeder of dynamic view | STANDS-RULING | SPEC:18500-18523 four read-paths unchanged; 017-80/105/198 |
| I-08 | self-gate recurrent read at startup | DISSOLVED | SPEC:12768-12771 startup owns initial gate state; SPEC:12784; values converge |
| I-09 | `here::` "innermost" vs body scope | STANDS-MECHANICAL | SPEC:16912 pins scope 2; drifted labels at 020-34/SPEC:16848/17068 |
| I-10 | string[10] cell layout | STANDS-RULING | 025-58 (LOG:3202) known-offset vs 025-29 dynamic-size; §13.12.4 silent on dynamic elements |
| I-11 | desired-stream default capacity 1024 vs source | STANDS-RULING | 031-32 (LOG:3865)+SPEC:23049 vs 030-21/22/25 |
| I-12 | membership-driver tag vocabularies unbridged | STANDS-RULING | 034-3 (LOG:4425) vs 034-11/12, 035-10, 033-81; bridge only SPEC:23912 |
| I-13 | fold cost cites nonexistent loop O() rules | STANDS-MECHANICAL | 035-5 (LOG:4449)+SPEC:24175; 014-149 qualitative only |
| I-14 | duplicate-scope-key warning unclassed | STANDS-MECHANICAL | 018-81 (LOG:2594) no class; siblings 018-142/143 classed |
| I-15 | undefined `[N..M]` bracket form | STANDS-MECHANICAL | 017-40 (LOG:2224), 017-109 (LOG:2293), SPEC:13960 |
| I-16 | endpoint carrier: LOG WeakHandle-only vs SPEC +Portal | STANDS-RULING | 019-59 (LOG:2720) closed set vs SPEC:16743 "WeakHandle/Portal" |
| I-17 | borrow-storage slot list omits cell | STANDS-MECHANICAL | 001-25 (LOG:55) 4 slots no cell; SPEC:14762 lists cell but no indexed slot |
| I-18 | IR type vocabulary closed-but-extended; Handle unenumerated | STANDS-RULING | 033-43 "is" (LOG:4220); 033-47/48 "join"; yielded<T> new instance; no Handle entry |
| I-19 | Portal/array cells lack a cell type tag | STANDS-MECHANICAL | 033-79 (LOG:4256) four families; Portal tag absent both grammars |
| I-20 | bundle[g] slice length vs Index Output | CHANGED (residual: MECHANICAL) | SPEC:14304 reframed to row-length; §4.9.5 (SPEC:4437) + 017-101 not conformed |
| I-21 | bundle_in_repeat_rejected unelaborated in SPEC | STANDS-MECHANICAL | 018-143 (LOG:2656); class = 0 in SPEC; §13.3.3.5 one direction only |
| I-22 | standalone view cardinality vs caller | STANDS-RULING | 017-19/017-9 "never widens" vs 021-41 (LOG:2825) unscoped + SPEC:13936 |
| I-23 | trait with effect method + required cell unsatisfiable | STANDS-RULING | 005-51 (LOG:412) + 005-59/61; no declaration-site rule; waiver-note moot |
| I-24 | Subject-typed first param: mandatory vs conventional | STANDS-RULING | LOG 005-68/029-65 mandatory vs SPEC:1306 "conventionally" + SPEC:1276 |
| I-25 | slice reversed bounds undefined | STANDS-RULING | SPEC:6794-6796 two failure modes only; range empty-convention SPEC:10491 |
| I-26 | Map Clone insertion order unstated | STANDS-MECHANICAL | 012-114 (LOG:1343)+SPEC:7801-7806: Eq/Hash pinned, Clone not |
| I-27 | .count return type unpinned | STANDS-RULING | no normative type; examples usize (SPEC:7375,14192), u32 (20605), i32 (11970) |
| I-28 | assoc-type projection `.` vs `::` | STANDS-RULING | 014-42 (LOG:1714) `Iter::Item` vs 014-157 (LOG:1829) `T.Iter.Item` |
| I-29 | consuming enumeration omits indexed slot | STANDS-RULING | 013-42 (LOG:1461) vs 013-3/52/72; SPEC:8748 "includes" |
| I-30 | duration suffix scope: same vs any | STANDS-RULING | 005-236 (LOG:597) "same scope" vs its own SPEC:3050 "any scope" + 012-142 |
| I-31 | "interpretation context" undefined | DISSOLVED | term = 0 hits; 009-90 (LOG:1056) + SPEC:5576-5588 self-defining |
| I-32 | reload reconciler-registration window never opens | STANDS-RULING | SPEC:20084-20121 10 steps, no host step; SPEC:20289 asserts requirement |
| I-33 | postfix tier drops ?. ?[] ?() | STANDS-MECHANICAL | 007-74 (LOG:721) has them; SPEC:3588 tier 15 drops them |
| I-34 | letter glued onto flag run | STANDS-RULING | 021-113 (LOG:2897); no trailing-boundary rule; grammar default = start-next-element |
| I-35 | interpolation-hole lexing | CHANGED (residual: MECHANICAL) | balanced scan exists ONLY in non-normative GRAMMAR:568-601; SPEC/LOG silent |
| I-36 | loop else: vs when/given arm | DISSOLVED | 002-6 (LOG:77) else = two senses; arm fallbacks otherwise:/default: |
| I-37 | `yield if` parse rule unstated | STANDS-MECHANICAL | 034-7 (LOG:4429); GRAMMAR YieldStmt forecloses; prose rule absent |
| I-38 | ref-transparency "inputs" excludes read cells | STANDS-RULING | SPEC:88-89 floor vs 025-15 (LOG:3159) provenance folding |
| I-39 | drop rule names break, omits fn return | STANDS-MECHANICAL | 032-137 (LOG:4132) vs SPEC:11489-11491 both paths |
| I-40 | division `\` typo | DISSOLVED | false premise: 007-54 (LOG:701) `\` IS integer division; no stray `\` |
| I-41 | `in` missing from precedence table | STANDS-RULING | SPEC:3572-3589 no `in` row; 007-231 (LOG:878) assumes comparison tier |
| I-42 | flag-run opener over-reach | STANDS-MECHANICAL | 021-119 (LOG:2903)+SPEC:17907+GRAMMAR:5772 "any non-letter" |
| I-43 | dangling inline else | DISSOLVED | 009-126 (LOG:1092) mandatory else forces association |
| I-44 | import collisions: only glob-vs-glob ruled | STANDS-RULING | 003-47 (LOG:158)+SPEC:8127; selective-vs-glob and glob-vs-local unruled |
| I-45 | 004-4 broken pointer + ring[1024] | CHANGED (residual: MECHANICAL) | 004-4 (LOG:206) respelled ->changes, still cites §2.1.2; home now SPEC:21847-21860 |
| I-46 | Map key bounds omit duration/instant | DISSOLVED | 012-100 (LOG:1329) + 018-64 (LOG:2577) + SPEC:7644 one canonical set (fd3cf60) |
| I-47 | composite-map slot path: brackets vs dot | STANDS-RULING | 012-110/119 brackets vs SPEC:7788 dot |
| I-48 | [for N: v] Clone/eval-count/N=0 | STANDS-RULING | 012-81 (LOG:1310)+SPEC:7095-7104 all three unpinned |
| I-49 | value-recurrent history across reload | STANDS-RULING | 028-6 (LOG:3358) silent on history; step 8 deriveds-only |
| I-50 | late reconciler writebacks post-reload | STANDS-RULING | SPEC:19942-19951 no validity check; 028-24/028-38 |
| I-51 | trap during reload-time first evaluation | STANDS-RULING | 016-153 (LOG:2039) startup-bound; no reload rule |
| I-52 | Map+Map: left order vs right order | STANDS-MECHANICAL | 012-105 (LOG:1334) "left order" vs 012-120 + SPEC:7708 "right order" |
| I-53 | structural carve-out counts disagree | STANDS-RULING | 001-30 (=3), 012-45 (=2), 013-187 (=2), 012-99 (Map), NEW 005-243 (LOG:604) (=2) |
| I-54 | string sharing optional vs mandatory | STANDS-RULING | 012-36 (LOG:1265) "may" vs 013-101/103 refcounted+O(1) |
| I-55 | variant imports vs absolute-path rule | STANDS-RULING | 003-24 (LOG:135) unqualified "all" vs 009-70..72 (LOG:1036-1038) bare-type-rooted |
| I-56 | host write surface: which two channels | STANDS-RULING | 016-157 (LOG:2043) unfixed vs 016-286 (LOG:2172) promoted; SPEC:12819 "for signal cells" |
| I-57 | structural self-emission nontermination | STANDS-RULING | SPEC:13907-13910 termination claim caller-only; no acyclicity rule |
| I-58 | observed: computed outputs parse contradiction | STANDS-MECHANICAL | 031-14/15 restrictive vs 031-49/146/152 + §13.19.5 (SPEC:23087-23167); §13.19.2 self-contradicts (22769/22815 vs 22834-22837); dueling hints 23181 vs 23781 |
| I-59 | scope_drop cursor hold on outer gate stream | STANDS-MECHANICAL | cursor lifecycle: advance/freeze/completion/reload only; 028-49 reload-scoped (§13.15.5) |
| I-60 | `.<key>.` path segment undefined in wire format | STANDS-RULING | SPEC:25313-25332 §15.4.1.1 lexical+:N only; §13.5.3 (SPEC:15850) self-supplies key path |
| I-61 | gate-open snap vs frozen DAG | STANDS-RULING | 022-59 (LOG:2990) same-commit vs 023-12 (LOG:3067) freeze; narrow carve-out SPEC:18548-18551 (fold/descriptor cells ONLY); hooks 027-122/031-159 presuppose same-commit |
| I-62 | shift count ≥ bit width undefined | STANDS-RULING | SPEC:3400-3445, 4126-4129 silent; no overflow variants for shifts |
| I-63 | const-overflow rejects wrapping/checked in-range | STANDS-RULING | 007-137 (LOG:784) + SPEC:3810-3822 "regardless of variant" vs 007-120/132 |
| I-64 | Copy-storability vs closure-in-cell ban | STANDS-RULING | 013-153 (LOG:1572) blanket; SPEC:9735-9738 SAME blanket (audit's align-to-SPEC basis false) |
| I-65 | error sections: §8.9 derived-only vs §13.13 | STANDS-RULING | 011-79/80 (LOG:1224-1225)+SPEC:6627 vs 026-2 (LOG:3214)+SPEC:19531 |
| I-66 | `?` early-return target in reactive body | STANDS-MECHANICAL | bridge stated nowhere (SPEC:6465-6474, 19544-19570, 033-145 LOG:4322 unconnected) |
| I-67 | byte_len vs x_count rule | STANDS-RULING | 012-91 (LOG:1320) cites byte_len as x_count example; 012-28 `byte_len` intact |
| I-68 | own param as placement-attr RHS | STANDS-RULING | 013-248 vs 013-249 (LOG:1667-1668); adjacent in SPEC:10356-10363 |
| I-69 | bare Index[Range] | STANDS-MECHANICAL | 007-230 (LOG:877) bare vs 007-229 + SPEC:4428/4435 parameterized |
| I-70 | pairs-form `default:` arm | CHANGED (residual: MECHANICAL) | `default:` now EXISTS (given/observe, 002-6/022-120) but match catch-all = `_:` (SPEC:5561); 019-46 (LOG:2707) says "as any match" yet spells default: |
| I-71 | misplaced move rule in Copy run | SETTLED | 013-79 (LOG:1498) now a §11.4.1 Copy rule; move cluster 013-134..142 contiguous (f89c179) |
| I-72 | "evaluable" vs "known" | STANDS-MECHANICAL | 4 LOG entries (001-28, 004-100, 016-116, 016-150) + 7 SPEC lines use "evaluable" — wider than audit said |
| I-73 | glob-collision rule delegates to variant section | STANDS-MECHANICAL | SPEC:8119/8127 "Per §6.2.3" survives barrel rework |
| I-74 | private effect at host boundary | CHANGED (residual: LOW RULING) | 031-77 barrel-amended; no visibility-does-not-cross-host note anywhere; gap now spans all 3 reach levels |
| I-75 | reconciliation failure landing site | STANDS-RULING | 031-129/130 + SPEC:23636 "typically" + SPEC:19986 "if such a cell is declared" |
| I-76 | attrs written only at placement time | STANDS-MECHANICAL | 016-15 (LOG:1901) absolute vs 021-16 (LOG:2800) bridge; 016-285 already qualifies |
| I-77 | trait import recaptures bare calls | STANDS-RULING | 005-124 (LOG:485) + SPEC:2181-2187 step 5; hazard undocumented; contrast SPEC:16978-16988 |
| I-78 | nearest-wins vs body/module ambiguity error | STANDS-RULING | 020-3/020-10 unconditional vs 020-17; trap phrase SPEC:16883-16884 |
| I-79 | operator-internal cell type change on reload | STANDS-RULING | 032-168 vs 032-169 (LOG:4163-4164); SPEC:20143/20853 pin per-instance, LOG doesn't |
| I-80 | reload-unsafe "two classes" vs reject | STANDS-RULING | 032-166 (LOG:4161) vs 032-170/171; SPEC:24876-24895 |
| DP-01 | fold else vs loop else collision | DISSOLVED (dismiss upheld) | no inline loop-with-else exists; comprehension else-less (SPEC:7083) |
| DP-02 | move field-access operand | SETTLED | 013-142 (LOG:1561) bare-identifier only; ALL old permissive SPEC sites now restrictive (SPEC:9607-9639, 24766); zero residue |
| DP-03 | impl-visibility same-module row | DISSOLVED (dismiss upheld) | SPEC:8344-8365 reachability framing survives barrel rework; row = outcome, not gate |
| DP-04 | portal stamp on reload | DISSOLVED (dismiss upheld) | 033-113 (LOG:4290) "at the new path" scoping; §13.3.6.3 now owns portal reload (SPEC:15054-15062) |
| DP-05 | .exposition two vs five kinds | STANDS-MECHANICAL | 017-219 (LOG:2403) five (Node/Wire/Bundle/DynamicView/Gated) vs 017-247 (LOG:2431) "node and wire entries" |
| DP-06 | behavior body cell I/O at ABI | STANDS-MECHANICAL | 032-108 (LOG:4103)+SPEC:24644-24649 vs 033-151/152 (LOG:4328-4329) pure/no-I/O |
| DP-07 | Circularity: opt-in only vs fixpoint gate | STANDS-RULING | 019-78 (LOG:2739) absolute vs 024-26 (LOG:3140) "only when guarded" |
| DP-08 | calling a closure stored in a field | STANDS-RULING | 009-21 (LOG:987); dispatch step 9 unknown-method (SPEC:2210-2218); 013-178 binding-only; tier 15 |
| DP-09 | Type[C] in record fields | DISSOLVED (dismiss upheld) | SPEC:4901-4908 value-vs-bound; SPEC:4942-4944 permits fields |
| DP-10 | exactly-three materialization boundaries | DISSOLVED (dismiss upheld) | 016-210/016-218 (LOG:2096/2104); plain-let born concrete |
| DP-11 | @derive six vs Copy/Circularity | DISSOLVED (dismiss upheld) | SPEC:5223-5227 scoped to implementation-generating; SPEC:2861-2863 no body |
| DP-12 | static arg: constant cell vs literal | DISSOLVED (dismiss upheld) | SPEC:20460-20478 + 25266-25269 cell-param/value-param split; CONSTANT-WRAP consistent |
| DP-13 | runtime match yielding Type[C] | STANDS-RULING | 008-64/66/69 (LOG:953-958); SPEC:4935-4938 routes the same case both ways; SPEC:13390-13399 |
| DP-14 | repeat scopes in the five kinds | DISSOLVED (dismiss upheld) | 017-234/018-56; SPEC:15338-15341 flattened wire entries |
| DP-15 | bundle as-name in repeat | DISSOLVED (dismiss upheld) | 018-143 (LOG:2656) parse-time error both directions |
| DP-16 | scope drop over gated-off subtree | DISSOLVED (effect seam; stream residue = I-59) | SPEC:19921-19926 teardown subsumes suspend; stream-seam citation (028-49) is reload-scoped — that gap lives as I-59 |
| DP-17 | fold members slot vs colon expressions | STANDS-RULING | SPEC:24119-24126 "names the members", no delimiter; GRAMMAR:4726-4731 self-flagged GAP admits anonymous collect (non-normative) |
| R-1 | duplicate keyed-by keys on value collections | DISSOLVED | 018-81 (LOG:2594) source-agnostic first-wins+warning; SPEC:16077-16091 |
| R-2 | strict-kind vocabulary sweep | STANDS-MECHANICAL | 016-167 (LOG:2053), 029-124 (LOG:3554), 030-48/49 (LOG:3605-3606) call annotations "kind"; 016-178 says channel |
| R-3 | Portal[WeakHandle[T]] LOG mirror | STANDS-MECHANICAL | SPEC:15072-15074 two-step check; 0 LOG hits; nearest 017-191/017-202 don't cover it |
| R-4 | 033-207/208/209 vs IR_GRAMMAR InstrSep | STANDS-MECHANICAL | LOG:4384-4386 `instr*`/`op operand*` vs IR_GRAMMAR:345-357 `(instr InstrSep)*`/`instr_rhs`; SPEC §15.4.4 (25514-25516) diverges too |
| R-5 | GRAMMAR 005-30 stale cites | STANDS-MECHANICAL | 15 GRAMMAR cites (1173, 3771-3880) of a narrow method-origin rule (LOG:391); AssocTypeDecl:3806 disproves anchor-convention defense |
| R-6 | mixed own-line + inline directive stacks | STANDS-RULING | 002-37 (LOG:108)+SPEC:185-192 homogeneous examples only; GRAMMAR:6163 permits mixing (non-normative) |

## 3. Ruling queue (52 entries — facts only; owner presentations authored by the main session)

### Theme A — piping, interpretation, dispatch

**I-01 — Which operand of `|>` carries the effect-kind method at interpretation bootstrap.** [adversarially re-verified, all quotes fresh]
Finding against current text: SPEC:1299 declares `effect render(value: Subject):` — render IS the effect-kind method. 017-273 (LOG:2457): "Interpretation bootstraps as an effect call in an `effects:` clause — `render(song).audio |> audio_out` — with projection written explicitly" — method on the LHS, `audio_out` a plain downstream effect. But 029-65 (LOG:3495) codifies the opposite as the third dispatch case: "an effect-kind trait method whose first parameter is Subject-typed and whose LHS is a node reference" — method on the RHS. SPEC §13.17.7 Case 3 (SPEC:20720-20735) states the RHS-method rule, then illustrates with the LHS-method example, and its comment (20733-20735) is internally garbled (calls `render(song)` "a node reference" while calling `render(song).audio` "the walked result"). `audio_out` is declared nowhere (sole occurrence SPEC:20732).
Provenance: 029-65's Option-B clause landed 1b0c6b8 (2026-07-06, one day pre-audit); d719517 only respelled "connection kind dispatches"→"coupling kind dispatches" — contradiction untouched by Part 2.
Rec survives: YES in substance (Option A), but the audit's site list is INCOMPLETE — the divergence now lives in the record itself (029-65), not just SPEC prose. Any Option-A execution must amend 029-65's third case AND the Case-3 rule statement, not only "the prose."
Options: (A) method on the left, called conventionally — `render(song)` is the effect call, `audio_out` a plain effect; Case 3 collapses into Case 2 and the special dispatch case retires; fix 029-65 + §13.17.7. (B) method on the right — keep `node |> method`; then 017-273's example and render's declaration must be rewritten. Behavior-observable; genuine fork.

**I-24 — Subject-typed first parameter: mandatory or conventional.**
Finding: LOG categorical (005-68 LOG:429 "whose first parameter is `Subject`-typed"; 029-65 same) vs SPEC softened (SPEC:1306 "conventionally `Subject`-typed"; SPEC:1276-1278 "may have any parameter list — including no `Subject` parameter at all") while SPEC's own dispatch relies on it as mandatory (SPEC:20725-20727, 20949).
Rec survives: YES unchanged (make it mandatory for effect-kind methods; leave "any parameter list" to value methods). D-13 did not touch this. Couples to I-01 (same Case-3 neighborhood).
Options: (a) mandatory — amend SPEC:1306 + carve effect-kind methods out of SPEC:1276; (b) conventional — soften 005-68/029-65 and give Case-3 dispatch a fallback.

**I-57 — Structural self-emission never terminates and nothing rejects it.**
Finding: SPEC:13907-13910 "Self-recursive placements terminate because each placement is an explicit user act" covers caller-supplied recursion only; 017-54/56 (LOG:2238/2240) let a type's `expose:` emit instances with no constraint on the emitted type; the finite-closure carve-out (SPEC:557-563, 004-45 LOG:247) presupposes the finiteness it should guarantee. No acyclicity/self-emission guard exists (swept; nothing landed in Part 2).
Rec survives: YES unchanged (reject: containment-acyclicity rule + named diagnostic class, catching transitive cycles).
Options: reject at compile time vs scope the termination claim + implementation-defined depth (needs an indirection primitive the corpus lacks). Verifier caution: grammar admissibility of unconditional `expose:` self-placement was not independently re-derived (~85%).

**DP-13 — Runtime `match` yielding `Type[C]` vs monomorphization (audit verdict: pin).**
Finding: 008-64 (LOG:953) no-erasure + 008-66 (LOG:955) per-site monomorphization vs 008-69 (LOG:958) runtime match may yield Type[C]. SPEC:4935-4938 routes the same case both ways ("Selecting among type values of the *same* slot type at runtime is allowed" vs "Selecting among *different* node structures at runtime is `given`" — two Type[Drivable]-satisfying types are both). SPEC:13390-13399 states observable behavior, no mechanism. D-20 touched match-in-exposition, not this.
Rec survives: YES (pin; surface as a choice).
Options: (a) all arms must share one concrete type (runtime match inert); (b) genuine runtime variance (needs the forbidden runtime tag); (c) reject multi-concrete-type match→Type[C].

### Theme B — wires and endpoints

**I-05 — Wire derived reading `to`, frozen from construction.**
Finding: cell-value invariant justified for `from` only (SPEC:18493-18494 "All wire-level deriveds compute against `from` which always has defined cells"); frozen-read fallback = "initial value if never active" (SPEC:18496-18498), undefined for `derived target: WeakHandle[Clip] = handle to` (SPEC:16739) when 019-53 (LOG:2714) freezes the wire from construction (body never runs).
Rec survives: YES unchanged (rename-only drift).
Options: (A) extend the invariant — pin frozen-from-construction value of `to`/`pair`-reading wire deriveds to an unresolved/None weak handle; (B) forbid external observation before first activation.

**I-16 — Endpoint carrier: LOG closes to WeakHandle, SPEC adds Portal.**
Finding: 019-59 (LOG:2720) "No other surface for endpoint data exists" (WeakHandle) vs SPEC:16743 "the wire publishes a `WeakHandle`/`Portal` of its own."
Rec survives: YES (owner picks add-Portal vs drop-Portal). Sharpener found in passing: `Portal[T]` is defined for NON-GRAPH slots (§13.3.6.3) while an endpoint is a graph entity — Portal-as-endpoint may be type-incoherent, which weights toward dropping it from SPEC.
Options: add Portal to 019-59, or drop from SPEC §13.6.2.

**DP-07 — Circularity: opt-in only vs fixpoint precondition (audit verdict: pin soft).**
Finding: 019-78 (LOG:2739) "no effect beyond opting the wire type into cycle participation" + SPEC:16824 vs 024-26 (LOG:3140) fixpoint resolves "only when guarded by a `Circularity` wire". Wake-gate/fixpoint claims live only in the LOG (022-37 LOG:2968); "wake gate"/"fixpoint" = 0 in SPEC.
Rec survives: YES. Options: (A) qualify 019-78 (+ "and the resulting temporal-break/fixpoint semantics"); (B) soften 024-26's "only when." Overlaps audit F170; settle together; couples to I-03's SPEC elaboration.

### Theme C — commit engine and gates

**I-04 — Two same-level tiebreakers both match a cross-instance pair.**
Finding: 023-33 (LOG:3088) unconditional source-order vs 023-34 (LOG:3089) cross-instance placement-order; SPEC:18933-18941 mirrors both, no precedence. Untouched by Part 2.
Rec survives: YES. Options: scope 023-33 intra-instance / 023-34 owns cross-instance (audit rec), or explicit precedence. Behavior-observable.

**I-61 — Gate-open snap has no legal ordering inside the frozen commit DAG.** [evidence corrected by adversarial pass]
Finding: 022-59 (LOG:2990) same-commit snap mandate + SPEC:18429-18432 vs 023-12 (LOG:3067) "no new dirty bits after dirty-set computation" + gated subtree excluded from the DAG (SPEC:18540-18542). CORRECTION vs first pass and audit: a narrow gate-transition carve-out DOES exist — SPEC:18548-18551: "a gate transition — open or close — is a dirty root for every fold cell (§13.21.7) and membership-descriptor cell (§13.20.4.3) with a member guarded by that gate" — but it reaches fold/descriptor cells ONLY; general frozen-subtree output-affecting deriveds still have no dirty-root or injection rule.
Rec survives: YES and GAINS force: the landed hook entries 027-122 (LOG:3347) / 031-159 (LOG:3992) — "on reopen the gate-open desired re-snap followed by `resume`-then-`update`" — presuppose the same-commit snap. And the narrow carve-out is already a partial instance of the audit's Option 1; the decision is whether to generalize it to all output-affecting deriveds of the opening subtree (plus the ordering spec vs the flipping predicate and recurrent advance).
Options: (1) same-commit injection + generalized carve-out (audit rec; consistent with landed hooks); (2) relax DAG-freeze wholesale; (3) next-commit snap — requires restating 027-122/031-159 (resume at flip boundary, update next) and rewriting 022-59; ship together if chosen.

**I-07 — Gated-off static feeder of a dynamic view: frozen-present or absent.**
Finding: four read-paths unchanged (SPEC:18500-18523); path 3 covers static views, path 4 folds; the dynamic-view static-feeder case in neither. 017-105 (LOG:2289) "gates freeze propagation rather than remove existence — distinct from `dynamic`/`repeat`, which change a view's membership" cuts both ways.
Rec survives: YES but weakened — D-10 (live membership) and D-15 (views structural-only) lean membership toward live/activation-based, adding weight to Option B.
Options: (A) frozen-present member (counts in .count, mounts a repeat scope); (B) activation-based absent. Behavior-observable.

**I-06 residual (CHANGED) — scope_drop's numbered-step placement.** [thin]
What closed: the re-key diff is now a step-1 dirty root (SPEC:18855-18860) and the suspend/teardown seam is law (027-122; SPEC:19921-19926 "teardown subsumes the release suspend").
What remains: no text anchors `scope_drop`/`scope_evaluate` (SPEC:16069, §13.5.4.2) to a numbered §13.10.2 step, and whether a dropped scope's final state lands in THIS commit's snapshot is still not directly stated. Verifier note: this retains a small observable-ordering kernel — thin ruling, not pure mechanics.
Options: anchor the repeat iterate/diff/scope_* sequence to numbered steps (designating §018 or §023 as owner), or accept the step-1-dirty-root text as sufficient and add only a snapshot-visibility sentence.

### Theme D — views, placement, lexing

**I-22 — Standalone view cardinality: enforced against the caller or selection-only.**
Finding: 017-19 (LOG:2203) / 017-9 (LOG:2193) "never widens acceptance" vs 021-41 (LOG:2825) "Cardinality declared in the parent's views is enforced at placement" (unscoped) + SPEC:13936 "Each view's cardinality constrains the count of supplied children." D-15 does not touch call-site enforcement.
Rec survives: YES (A: selection-only; scope 021-41/SPEC:13936 to acceptance-entry views). Options: A vs B (enforced; carve "never widens").

**I-34 — Letter glued onto a flag run.** [adversarially confirmed a genuine ruling]
Finding: 021-113 (LOG:2897) fixes the nine flag chars; GRAMMAR:5766-5769 FlagsRun; only the LEADING adjacency is ruled (GRAMMAR:5559-5561); nothing governs the trailing edge (`Pin'!x`).
Rec survives: YES (lex error) but note: the grammar as written would actually accept `Pin'!x` as type+flags+name (start-next-element), so the audit's rec goes AGAINST the grammar's default — a real choice, not a formality.
Options: lex error ("insert a space") vs start-next-element (whitespace-free instance naming).

### Theme E — reload

**I-32 — Reload never opens the promised reconciler-registration window.**
Finding: 027-67 (LOG:3292) + SPEC:19800-19804 mandate registration "during a hot reload's pre-live phase"; the 10-step sequence (SPEC:20084-20121) has no host step (step 2 locks and queues, step 9 goes live, step 10 applies queued writes after); §13.15.6 (SPEC:20289-20291) asserts the requirement with no mechanism; startup has abort-on-missing (SPEC:19858-19861), reload does not. D-08's reload-tail ruling fires existing reconcilers only.
Rec survives: YES (add post-diff/pre-commit registration window + reject-on-missing). Options: add a step vs restate where registration already happens; exact placement = owner call.

**I-49 — Value-recurrent history across a body edit on reload.**
Finding: 028-6 (LOG:3358) preserves same-path-same-type cell VALUE, silent on the `.past` buffer; step 8 (SPEC:20110-20114)/028-21 recompute deriveds only; recurrent-STREAM reload pinned (030-225..228, SPEC:22533-22558), plain value recurrent not.
Rec survives: YES. Options: (A) preserve history by identity (audit rec); (B) reset on body change.

**I-50 — Late reconciler writebacks aimed at torn-down instances.**
Finding: worker-thread writeback path exists (SPEC:19942-19944); reload queues host writes and applies to new state (028-24 LOG:3376) with no target-validity check; per-instance restart tears down old instances (028-38 LOG:3390). No stale-target rule anywhere.
Rec survives: YES. Options: (A) quiesce; (B) queue+apply blindly; (C) queue, validate, reject stale-instance targets (audit rec).

**I-51 — Trap during reload-time first evaluation.**
Finding: 016-153 (LOG:2039) abort rule bound to startup; reload runs identical first evaluations (028-18 LOG:3370, 028-21) under the lock; no reload-time trap rule.
Rec survives: YES. Options: widen 016-153 (abort) vs a distinct reload-rollback rule (keep old version running). Different runtime behavior.

**I-60 — Wire-format identity never defines the `.<key>.` path segment.**
Finding: §15.4.1.1 (SPEC:25313-25332) defines lexical names + `:N` for anonymous/duplicated siblings only, and still claims total authority ("the two are the same mechanism"); §13.5.3 (SPEC:15850-15861) routes repeat identity through it while self-supplying `<enclosing_path>.<key>.<template_field>`. D-30 demoted `:N` to addressing but did not touch this gap. Severity stays MED (reconcilable as a specialization; a spec gap, not a contradiction).
Rec survives: YES. Options: extend §15.4.1.1 with `.<key>.` as a first-class multiplicity device incl. wire encoding (audit rec), or stop routing repeat identity through it and let §13.5 own the keyed path.

**I-79 — Operator-internal cell type change: restart vs remove+add.**
Finding: 032-168 (LOG:4163) per-instance restart for "internal cell type changes" vs 032-169 (LOG:4164) "all other changes — cell type changes (remove + add per §13.15.2)... reload-safe" with no operator-internal exclusion; SPEC §13.15.4 (SPEC:20143) and §13.17.10 (SPEC:20853) DO pin per-instance, the LOG alone admits both; SPEC §14.8.3 (SPEC:24882-24890) lists both without precedence.
Rec survives: YES (per-instance governs; scope 032-169). Options: per-instance governs (matches SPEC) vs remove+add everywhere (deletes normative SPEC text). Behavior-observable.

**I-80 — Reload-unsafe changes: "two classes" vs reject.**
Finding: 032-166 (LOG:4161) "fall into two classes" vs 032-170/171 (LOG:4165-4166) impl-defined reject; SPEC:24876-24895 same.
Rec survives: YES (two-axis framing: class = required restart; reject = cross-cutting disposition). Options: two-axis vs three-outcome taxonomy. Wording-only; behavior identical.

**I-74 residual (CHANGED) — visibility at the host boundary.** [LOW]
Mutation: 031-77 now reads "Module-private effects are not reachable from other modules; an effect reaches dependent packages only via the package barrel" — the barrel model makes private/unmarked/exported effects ALL equally host-visible by type name (031-119 registers by name; 033-141 IR carries no visibility field). No visibility-does-not-cross-the-host-boundary note exists anywhere (swept).
Options unchanged and still a (LOW) ruling — do not fold into the consent list: (a) affirm source-only visibility + one-line note at §13.19.10/§13.14.7/§003; (b) name-mangling/omission in the IR.

### Theme F — streams and effects surface

**I-11 — Desired-block stream default capacity: flat 1024 vs source-derived.**
Finding: 031-32 (LOG:3865) + SPEC:23049-23052 "matches a regular `stream` declaration (§13.18.2)... (defaulting to 1024)" vs 030-21/030-22 source-derived defaults and 030-25 (LOG:3582) scoping 1024 to residual cases. A known-capacity source (effect stream param, 031-27 `stream gate[4096] LogEntry`) hits both branches with different answers; the SPEC sentence self-contradicts. Survived the streams rework (spelling only changed).
Rec survives: YES. Options: inherit source-derived + drop the standalone 1024 (audit rec) vs keep 1024 + drop the §13.18.2-equivalence for the capacity axis.

**I-56 — Host write surface: which two channels.** [rec needs updating]
Finding: 016-157 (LOG:2043) "Host-driven writes occur only through `runtime.write_signal` and `runtime.transaction`" — NOT fixed by the channel amendment — vs 016-286 (LOG:2172, promoted) "exactly two channels — one value channel and one event channel: `runtime.write_signal`... and `runtime.push_stream`... There is no third channel." SPEC §13.2.7 (SPEC:12819-12822) keeps the scope 016-157 dropped: "for signal cells", listing {write_signal, transaction}; §13.14.5 shows transaction as a batching wrapper; §13.14.8 defines push_stream.
Rec survives: NO — needs updating. The audit's Option A (rewrite 016-157 to the whole surface) would now diverge 016-157 from its own SPEC anchor §13.2.7. Post-Part-2 the LOG-SPEC-coherent path is Option B: conform 016-157 to §13.2.7's "for signal cells" scope; 016-286 carries the whole surface. Residual owner choice: which entry states which scope.

**I-75 — Where reconciliation failure lands on the value track.**
Finding: 031-130 (no panic → value track) + 031-129 (via observed cells) but no rule requires an error cell; SPEC:23636-23641 "typically a `signal error: Option[E] = None`"; SPEC:19986 sentinel written "if such a cell is declared." Note: 031-158's current text no longer carries the audit's "with a failure mode" phrase.
Rec survives: YES. Options: (a) declare it author-opt-in convention, stated; (b) mandate an observed error channel for reconciler-registered effects (audit rec).

### Theme G — groups and folds

**I-12 — Membership-driver tags: two vocabularies, no LOG bridge.**
Finding: surface {permanent, key-driven, activation-driven} (034-3 LOG:4425; 022-72 LOG:3003) vs IR {permanent, keyed-template, gate-guarded} (034-11/12 LOG:4433-4434; 035-10 LOG:4454; 033-81 LOG:4258; 033-169); bridge exists only at SPEC:23912-23914; both vocabularies now coexist INSIDE section 034 unbridged.
Rec survives: YES (state the identity in the LOG; don't pick canonical unilaterally). Options: (1) equivalence sentence keeping both; (2) unify to surface names; (3) unify to IR names.

**DP-17 — Fold `<members>` slot vs colon-bearing expressions (audit verdict: pin narrowly).** [annotation corrected]
Finding: fold header grammar has no delimiter for `<members>` (SPEC:24119-24126 "names the `<members>`"); the type rule rejects value witnesses (SPEC:24162/24171 membership structures, not stored values; 035-7 LOG:4451) but the grammatical boundary is unpinned — GRAMMAR:4724-4731 self-flags the fold/collect expression grammar as a GAP and (non-normatively) admits an anonymous `collect:` block-expression as a members operand.
Rec survives: YES AS A LIVE OPTION (correction to first pass: the reference-only pin is in tension with a non-normative gap note, not defeated by landed law; no LOG/SPEC entry admits the anonymous-collect operand normatively).
Options: (A) reference-only members slot (audit's narrow pin; would foreclose the anonymous-collect direction); (B) membership-structure operands = reference OR anonymous `collect:` block, with a self-delimiting/parenthesization rule for the header (cf. observe 016-249); (C) leave the gap, rely on the type rule.

### Theme H — traits and types

**I-23 — Trait declaring both an effect-kind method and a required cell.**
Finding: 005-51 (LOG:412) effect-only ∩ 005-59/61 (LOG:420/422) node/wire-only = empty; no declaration-site rule exists (swept). The audit's old scope-note about the satisfies-waiver is MOOT (waiver retired with the satisfies clause); under fulfill-only law such a trait simply can never receive a valid fulfill — error surfaces at every doomed fulfill site instead of the declaration.
Rec survives: YES (declaration-site error). Options: reject at declaration vs legal-but-permanently-unsatisfiable.

**I-28 — Associated-type projection `.` vs `::`.**
Finding: both spellings live — 014-42 (LOG:1714) `Iter::Item`/`Iterable::Iter`; SPEC:10609-10611, 1374, 9723 (`Iterator::Item`) vs 014-157 (LOG:1829) "Associated-type constraints use `.` member-access notation: `T.Iter.Item`." Rider-1 settled the `=`/`is` binder, not this.
Rec survives: YES (one spelling, `.`; keep `::` for item paths like `Iterator::next`). Options: unify on `.` vs admit both with a disambiguation rule. Requires per-site judgment on type-projections vs item-paths.

**I-30 — Duration suffix reservation: same scope vs any scope.**
Finding: 005-236 (LOG:597) "in the same scope" now contradicts its OWN cited section — SPEC:3050 "Neither family may be re-registered for another type in any scope" — plus 012-142 (LOG:1371) and SPEC:7433 ("any scope"). A live LOG↔SPEC divergence, sharpened since the audit.
Rec survives: YES (any scope). Options: any-scope (3 sources) vs same-scope (the outlier; would permit per-module re-registration). Behavior-changing → owner-facing.

**I-38 — Referential-transparency floor never counts read cells as inputs.**
Finding: SPEC:88-89 floor ("same inputs produce the same outputs") vs 025-15 (LOG:3159) provenance folding for cell-reading fns (SPEC:19290-19294). Unchanged.
Rec survives: YES (define "inputs" = provenance set). Options: tighten the floor wording vs leave the floor abstract and rely on §13.12.2.1. Definitional/philosophy-layer edit → owner-facing.

**I-53 — Structural carve-out counts disagree (now five sites).**
Finding: 001-30 (LOG:60) three (tuples, closures, intersections); 012-45 (LOG:1274) "the only two"; 013-187 (LOG:1606) "second after tuples"; 012-99 (LOG:1328) Map structural; NEW SITE from the sealed amendment: 005-243 (LOG:604) "The structural carve-outs — tuples and closure types —" (a third "only two" assertion the audit didn't know).
Rec survives: YES (treat all four as structural; drop closed counts) with the site list widened to include 005-243. Options: four-member set vs a deliberately narrower set (would make Map or intersections nominal). Owner picks the set.

**I-77 — Trait import silently recaptures bare calls.**
Finding: 001-33 + 005-120/121/124 (LOG:63, 481-485) intact; dispatch now step 5 of a 9-step algorithm (SPEC:2181-2187, renumbered by D-13; effect methods excluded at step 2 but value-method capture unchanged); no diagnostic. Contrast sharpened: SPEC:16978-16988 brands body/module silent shadowing "a refactoring hazard" and errors on it.
Rec survives: YES. Options: (b) warn/error at the collision (audit rec); (a) document the hazard; silence off the table.

**DP-08 — No specified way to call a closure stored in a field (audit verdict: pin).**
Finding: 009-21 (LOG:987) routes `r.handler()` to method dispatch; the current 9-step algorithm ends "Nothing matches → unknown method error" (SPEC:2210-2218) with no closure-field step; 013-178 (LOG:1597) invocation form is a binding name only; closures storable in fields/Vec slots (013-195/200); precedence tier 15 (SPEC:3588) also parses `(r.handler)()`. Two parses, opposite results.
Rec survives: YES (pin which parse wins; give the invocation form for field/index-held closures).

**I-64 — Blanket Copy-storability vs the closure-in-cell ban.** [rec needs updating]
Finding: 013-153 (LOG:1572) blanket "stored in cells... like any other `Copy` value" + 013-192 closure-is-Copy vs 013-199/025-55 closure-never-a-cell-type. CORRECTION to the audit: SPEC §11.9.1 is NOT the narrower authority — SPEC:9735-9738 carries the same universal ("`Copy` values are not aliases under §11.9 — they may be stored in a cell... No exemption is at work"), merely under a carrier heading.
Rec survives: NO — needs updating: the closure carve-out must land in BOTH 013-153 and SPEC §11.9.1, not by "aligning the LOG to the (supposedly narrower) SPEC."
Options: qualify both blanket statements to except closures; or restate storability as carrier-scoped in both docs.

### Theme I — ownership

**I-29 — Consuming enumeration omits the indexed slot.**
Finding: 013-42 (LOG:1461) "Consuming comprises... record field, tuple component, or enum payload... and writing into a reactive cell" — still no indexed slot — vs 013-3 (LOG:1422) indexed assignment consumes, 013-52, 013-72; SPEC:8748-8756 uses open "includes" and also omits it. (013-42 gained the reactive-cell clause post-audit; indexed slot still missing.)
Rec survives: YES. Options: add "indexed slot" to the closed enumeration (audit rec) vs reword "comprises"→"includes". Changes whether `arr[i] = v` kills `v` per the definition → owner-facing wording call.

**I-68 — Bare `own` param as placement-attr RHS: implicit consume vs required move.**
Finding: 013-248 vs 013-249 (LOG:1667-1668) still collide; the two sentences sit adjacent in SPEC:10356-10363 with no scoping. (Category split widened B/C→B/C/D post-audit; overlap untouched.)
Rec survives: YES (placement-specific rule wins; scope 013-249 to argument/initializer positions). Options: 013-248 wins (no move) vs 013-249 wins (move required).

### Theme J — collections, strings, numerics

**I-10 — `string[10]` cell layout.**
Finding: 025-22 any-value-type + SPEC:7056 `string[10]` legal; 025-58 (LOG:3202) known-offset claim; 025-29 string is index-based dynamic; §13.12.4's `N×sizeof(T)` fit rule (SPEC:19425-19426) has no defined sizeof(string); array-of-dynamic layout stated nowhere (the "index-based variant" clause arguably implies per-element indices but never pins it, ~85%).
Rec survives: YES. Options: per-element word-size pool index (audit rec) vs disallow dynamic-size element types in fixed-extent array cells.

**I-25 — `slice`/`byte_slice` reversed bounds.**
Finding: SPEC:6794-6796 lists exactly two failure modes (mid-codepoint, out-of-range); `s.slice(5, 2)` is neither; range convention SPEC:10491 (`start >= end` empty) exists to match.
Rec survives: YES (empty string). Options: empty-string vs trap.

**I-27 — `.count` return type.** [evidence worsened]
Finding: no normative return type anywhere (012-90/91 silent); SPEC examples now disagree THREE ways — `usize` (SPEC:7375, 14192), `u32` (SPEC:20605), `i32` (SPEC:11970); LOG family is isize (007-7 length/index; 012-28/29 byte_len/char_count → isize); no implicit signed/unsigned crossing.
Rec survives: YES (isize; fix all example sites). Options: isize (family-consistent) vs usize (matches two examples, breaks the no-usize-length rule). Pervasive accessor → owner-facing.

**I-47 — Composite-map slot paths: brackets vs dot.**
Finding: 012-110/012-119 (LOG:1339/1348) `<binding>['<key>']` vs SPEC:7788-7789 `<binding>.<key>`.
Rec survives: YES (LOG-wins bracket form; program-visible so confirm direction). Options: bracket vs dot.

**I-48 — `[for N: v]`: Clone bound, evaluation count, N=0.**
Finding: 012-81 (LOG:1310) + SPEC:7095-7104 pin only "N compile-time-known non-negative"; all three sub-questions open; explicit-clone-per-site rule (013-93 LOG:1512) unchanged.
Rec survives: YES (evaluate once, T: Clone, clone N−1, N=0 evaluates zero). Options: once+Clone vs N evaluations (side-effect-visible).

**I-54 — String backing sharing: optional vs mandatory.**
Finding: 012-36 (LOG:1265) "may share... invisible to the user" + SPEC:6811-6812 vs 013-101/013-103 (LOG:1520/1522) refcounted shared backing + O(1) duplication + SPEC:9234-9243.
Rec survives: YES (make sharing normative). Options: normative+observable vs weaken O(1) to conditional. Owner decides if O(1) binds.

**I-67 — `byte_len` vs the x_count rule.**
Finding: 012-91 (LOG:1320) cites `byte_len` as an `x_count` example; 012-28 keeps `byte_len` (no rename occurred; `byte_count` = 0 hits); sibling `char_count` conforms.
Rec survives: YES (rename byte_len→byte_count). Options: rename vs amend the rule to accept `_len`. Program-visible API rename → owner call.

**I-41 — `in` has no precedence tier.**
Finding: SPEC:3572-3589 table has no `in` row; 007-231 (LOG:878) and 012-103 (LOG:1332) + SPEC:4457 justify `not k in s` by assuming a comparison-tier `in`; `x in 0..n` still two-way ambiguous (comparison tier would break it). New surface `not in` (SPEC:7682) doesn't close it.
Rec survives: YES (place `in` below the range tier; repair the justification texts). Options: below-range / comparison-tier (broken) / implementation-defined.

**I-62 — Shift count ≥ bit width.**
Finding: count type pinned (007-92/007-193), value range never (SPEC:3400-3445, 4126-4129 silent); no shift overflow variants exist.
Rec survives: YES (trap; const → compile error). Options: trap / mask-mod-width / implementation-defined; secondary: add shift overflow variants.

**I-63 — Const-overflow rule rejects defined in-range wrapping/checked results.**
Finding: 007-137 (LOG:784) + SPEC:3810-3822 ("This applies to `+%`, `+|`, `+?`... too", worked example rejects `200u8 +% 100u8`) vs 007-120 (defined 44u8) and 007-132 (Option). Jointly unsatisfiable.
Rec survives: YES. Options: scope the rule to the trapping variant vs check each operator's defined result (observationally convergent — near-editorial, but the author picks).

### Theme K — modules and scopes

**I-44 — Import collisions ruled only for glob-vs-glob.**
Finding: 003-47 (LOG:158) + SPEC:8127-8129 unchanged in substance by the barrel rework (ids survived); selective-vs-glob and glob-vs-local still unruled; the rework added export-collision rules (003-80/82) but not these.
Rec survives: YES (explicit-wins / local-shadows-glob; error only for symmetric glob-vs-glob). Options per case: specificity-wins vs hard error.

**I-55 — Variant imports vs the absolute-path rule.**
Finding: 003-24 (LOG:135) unqualified "All `use` statements use absolute paths" + 003-22 path bases (root/dependency/std) + SPEC:8057-8062 vs 009-70..72 (LOG:1036-1038) + SPEC:5445-5448 bare-type-rooted `use Result::(Ok, Err)`. No carve-out landed with the barrel rework.
Rec survives: YES (declare the exception). Options: explicit exception vs require full rooting.

**I-65 — §8.9 derived-only vs §13.13's broader construct list.**
Finding: 011-79/80 (LOG:1224-1225) + SPEC:6627-6633 derived-only vs 026-2 (LOG:3214) + SPEC:19531-19533 derived+recurrent+fold-by+collect/yield.
Rec survives: YES; note D-06 (repoint-by-default) is now standing law and largely pre-decides toward Option B (defer §8.9 → §13.13) — kept in the ruling queue as a thin confirm rather than silently executed. Options: broaden both vs defer/repoint.

**I-78 — Nearest-scope-wins vs the body/module ambiguity error.**
Finding: 020-3/020-10 (LOG:2746/2753) unconditional nearest-wins vs 020-17 (LOG:2760) ambiguity error, reconciled only by 020-19 (LOG:2762); SPEC trap phrase intact: SPEC:16883-16884 "a bare reference to a module-level name not shadowed by a member" (a member never shadows — that case errors).
Rec survives: YES (exception clause; fix the trap phrase). Options: exception clause vs restate-by-tier. Teaching-material wording → owner confirms.

### Theme L — IR

**I-18 — IR type vocabulary: closed copula, later joins, Handle unenumerated.** [reinforced post-audit]
Finding: 033-43 (LOG:4220) closed "is"; 033-47/48 (LOG:4224-4225) "join"; no Handle-enumerating entry (only 033-48's "paralleling `Handle[T]`'s representation"); type_tag grammar (IR_GRAMMAR:132-138; SPEC:25688-25691) has no slice/Portal/Handle production. NEW instance: `yielded<type_tag>` was added to the grammar and 033-79 by D-25 but is absent from 033-43's "closed" list.
Rec survives: YES (Option B: soften the copula, add the Handle entry, conform the grammar — now also covering yielded<T>). Options: one closed list (A) vs open-base-plus-extensions (B); plus whether slice/Portal/Handle earn first-class type_tag productions. Couples with I-19 (same grammar) — resolve together.

### Theme M — residue

**R-6 — Mixed own-line + inline directive stacks.**
Finding: 002-37 (LOG:108) "both layouts are legal, and directives stack in source order in either layout — own-line directives stack vertically, inline directives stack left-to-right" + SPEC:185-192 — homogeneous examples only; mixing neither blessed nor forbidden; GRAMMAR:6163 `AnnotatedDecl ::= ( AppliedDirective NEWLINE? )* Decl` permits mixing but is non-normative.
Rec: none existed (residue item). Options: (a) allow mixing, source order governs (matches the grammar; one-line pin); (b) forbid mixing (homogeneous stacks only); (c) order constraint (degenerate: mixed can only be own-line-above then inline). Borderline mechanical if the owner reads "in either layout" as already allowing it (~80% ruling).

## 4. Consent-list draft (STANDS-MECHANICAL, one-liners)

1. I-02 — Add Portal-resolution to §13.12.1's dynamic-source enumeration (currently "the one exception"); conform to 023-46/025-6's two-source law.
2. I-03 — Elaborate the wake gate in the SPEC sections its five LOG entries cite, under the same term.
3. I-09 — Replace the three drifted "innermost" labels for `here::` (020-34, SPEC:16848, SPEC:17068) with the committed body-scope wording.
4. I-13 — Drop the "loop" half of 035-5's cost-family citation (and the SPEC:24175 mirror); keep "collection cost rules."
5. I-14 — Assign 018-81's duplicate-scope-key warning a named diagnostic class + delivery channel, parallel to its siblings.
6. I-15 — Replace undefined `[N..M]` with `[N..=M]` in 017-40, 017-109, and the SPEC:13960 mirror.
7. I-17 — Reconcile the borrow-unstorable slot lists both ways: LOG 001-25 gains "reactive cell"; SPEC:14762 gains "indexed slot" (full five-way list both sides).
8. I-19 — Widen 033-79's cell-type-tag set to admit Portal (inline slot_path+generation) and arrays; add the Portal type_tag production (execute with I-18).
9. I-21 — Repoint 018-143's citation to §13.3.3.5 and extend it with both rejection directions + the `bundle_in_repeat_rejected` class name.
10. I-26 — Pin that derived Clone on Map preserves insertion order (012-114 + SPEC §9.5.13).
11. I-33 — Add `?.` `?[]` `?()` to SPEC:3588's postfix tier 15, conforming to 007-74.
12. I-37 — State that an `if`/`match` on the same line after `yield` is always the yield's operand (grammar already forecloses the other parse; prose clarification).
13. I-39 — Broaden 032-137 (and the SPEC:24751 list item) to name both exit paths (break AND enclosing function return) per its cited §12.9.3.
14. I-42 — Narrow the flag-run-opener prose from "any non-letter character" to the nine 021-113 flag chars, in LOG, SPEC:17907, and the GRAMMAR:5772 comment.
15. I-52 — Correct 012-105's "in left order" to "in the right operand's order," matching 012-120 + SPEC:7708.
16. I-58 — Widen the restrictive observed:-block sites (031-14, 031-15, SPEC:22831-22837 role list + "recurrent only in desired:" scoped to host-fed cells, SPEC:23781 hint) to admit derived/value-recurrent computed outputs; keep recurrent[N] stream forbidden. [wide surface; substantive-but-mechanical — the permissive side is landed law across §13.19.5, intro prose, IR tables]
17. I-59 — Add a §030 rule releasing a consumer's cursor + slowest-cursor hold on an OUTER gate stream at scope_drop (parallel to the reload rule 028-49); ride effect-teardown 018-110.
18. I-66 — State in 026-7/§13.13.2 that a Result/Option-typed reactive body plays the function-return role for `?`'s early return (bridge currently stated nowhere).
19. I-69 — Fix 007-230's bare `Index[Range]` to `Index[Range[isize]]`.
20. I-72 — Unify "compile-time evaluable" → "compile-time-known" (4 LOG entries: 001-28, 004-100, 016-116, 016-150; +7 SPEC lines — wider than the audit stated).
21. I-73 — Make SPEC §10.4.1 self-contained on the parentheses convention and general glob-collision rule instead of "Per §6.2.3" (enum-variant-titled) handoffs.
22. I-76 — Qualify 016-15's "written only at placement time" to acknowledge the reactive-fed bridge (016-285 already carries the qualified model).
23. DP-05 — Make the .exposition restating entries (017-247 "node and wire entries"; SPEC:15272) enumerate all five entry kinds (or state why restricted).
24. DP-06 — Restate 032-108 + SPEC §14.6.1 (24644-24649) so the behavior is a pure params→value function and the RUNTIME performs the cell I/O (matches 033-151/152 + behavior table).
25. R-2 — Optional vocabulary sweep: respell "kind" → "(value) channel" where the four flagged entries (016-167, 029-124, 030-48, 030-49) name the applied annotation, per the 016-178 two-level law.
26. R-3 — Mirror SPEC:15072-15074's `Portal[WeakHandle[T]]` two-step presence-check into the LOG (or record it as deliberate SPEC-only elaboration).
27. R-4 — Conform 033-207/208/209 AND the SPEC §15.4.4 mirror (25514-25516) to normative IR_GRAMMAR (InstrSep, instr_rhs, bare-NAME/move-NAME operands); IR_GRAMMAR's "lifted verbatim from SPEC §15.4.4" claim is currently false.
28. R-5 — Repoint the 15 GRAMMAR cites of 005-30 (method-origin rule) to each production's governing decision (per-production research, not find-replace; AssocTypeDecl:3806 shows the house convention is specific-entry cites).

CHANGED items with mechanical residuals (consent-list riders):
29. I-20 — Conform SPEC §4.9.5 (4437) to name the `Handle[T][..N]` variant and respell 017-101's index-basis to the landed row-length basis (017-99, SPEC:14304-14306).
30. I-35 — Home the interpolation-hole balanced-scan rule normatively (SPEC §9.1.9 / LOG 002-2x); it currently exists only in non-normative GRAMMAR:568-601, violating 001-3/002-29's no-normative-delegation law.
31. I-45 — Repoint 004-4's citation from §2.1.2 to the section that owns `->changes` materialization + ring[1024] (§13.18.9; note §13.18.9 itself points to §4.4 for use-site resolution — verify the target).
32. I-70 — Conform the pairs-form match fallback `default:` → `_:` in 019-46 + SPEC:16680 (entry self-anchors "as any match §6.2.5"). [flag visible: if the owner prefers pairs-form to track the given/observe `default:` family instead, strike to escalate]

## 5. DISSOLVED appendix (item → closing evidence)

- **I-08** (self-gate recurrent at startup) — SPEC:12768-12771 "`when` predicates... are evaluated alongside deriveds in the topological order. Each instance's initial gate state is established here"; SPEC:12784 "Init time has no notion of 'previous commit'"; at the first real commit the prior-commit value equals the startup value (recurrent not yet advanced), so the two readings converge — no divergent initial state exists. [Adversarial pass overturned first-pass MECHANICAL. Optional nicety, NOT a consent item: a one-line startup cross-ref at §13.9.6/022-37.]
- **I-31** ("interpretation context") — term = 0 hits in all four docs; 009-89 (LOG:1055) has no build-all-arms clause; 009-90 (LOG:1056) + SPEC:5576-5588 "match inside reactive structure — unroll or tag, never freeze" are self-defining. Closed by the D-20 batch.
- **I-36** (loop else vs arm) — 002-6 (LOG:77): "The keyword else has exactly two senses... the fallback arms of when: blocks and of given/observe blocks are spelled otherwise: and default:, so no third else sense exists"; 022-94 (LOG:3025); 014-90 (LOG:1762). An `else:` line can never head an arm.
- **I-40** (division `\`) — false premise: 007-54 (LOG:701) "`\` is the integer-division operator... `5 \ 2` is `2`" (genesis commit 4af4cbb, pre-audit); SPEC:3304 IntDiv row; tier 13 `*, /, \, %` correct; const-generic ops list contains no backslash.
- **I-43** (dangling else) — 009-126 (LOG:1092) "An if chain without a final else is a compile error" (commit a8d7b1f, pre-audit; applies to the inline form per 009-124): the one-else witness is ill-formed under either parse; the two-else witness's association is forced by well-formedness.
- **I-46** (Map keys duration/instant) — 012-100 (LOG:1329) + 018-64 (LOG:2577) + SPEC:7644-7654: ONE canonical StringifiableKey set (i8–i128, u8–u128, isize, usize, bool, char, string, duration, instant). Closed by the structure batch (fd3cf60, D-14).
- **DP-01** (fold/loop else collision) — dismiss upheld: comprehensions have no if-filter/else (SPEC:7083); loop else sits at a dedent boundary (014-90); fold else mandatory-last (035-2).
- **DP-03** (impl-visibility row) — dismiss upheld through the barrel rework: SPEC:8344-8365 "callable wherever both the trait and the type are visible... unreachable from that caller's site" — the same-module cell is a reachability outcome; fulfill may live in a third module (SPEC:1853-1854).
- **DP-04** (portal stamp) — dismiss upheld: 033-113 (LOG:4290) regeneration coordinated with "relocation or removal... at the new path"; survivors preserve (SPEC:15054-15062). The broken-pointer sub-finding also closed: §13.3.6.3 now owns a "Hot reload" paragraph (c71b816).
- **DP-09** (Type[C] in fields) — dismiss upheld: SPEC:4901-4908 scopes "only" to value-vs-bound; SPEC:4942-4944 permits record-field storage.
- **DP-10** (exactly three boundaries) — dismiss upheld: 016-210 (LOG:2096) three boundaries; 016-218 (LOG:2104) plain-let-of-with born concrete. [Reported, not resolved: GROUP-SNAPSHOT added a group→snapshot reactive-to-concrete path (034-14 LOG:4436, 017-75 LOG:2259) outside the list — strengthens the optional "scope the word exactly" nicety.]
- **DP-11** (@derive six) — dismiss upheld: SPEC:5223-5227 "generates structural trait implementations for a fixed set"; Copy/Circularity generate no method body (SPEC:2861-2863).
- **DP-12** (static-arg lowering) — dismiss upheld and strengthened post-CONSTANT-WRAP: SPEC:20460-20478 (cell-param static args constant-wrap to a degenerate `derived T` cell / value-param args snapshot to value_literal); marker is metadata, never a third binding case (SPEC:25266-25269; 033-64/033-121).
- **DP-14** (repeat scopes vs five kinds) — dismiss upheld: 017-234, 018-56, SPEC:15338-15341 (repeat-materialized wire = individual Wire entry, mounted/dismounted), DynamicView walk nodes-only (SPEC:15427-15431).
- **DP-15** (bundle as-name in repeat) — dismiss upheld: 018-143 (LOG:2656) parse-time error, both directions, class `bundle_in_repeat_rejected`.
- **DP-16** (scope drop over gated-off subtree) — dismissal upheld FOR THE EFFECT SEAM: SPEC:19921-19926 "a suspended instance whose scope then dies receives teardown directly (not a resume first) — teardown subsumes the release suspend"; no double release, no resume-before-drop. CAVEAT (adversarial): the audit's stream-seam dismissal rationale cited 028-49, which is reload-scoped (§13.15.5) — that residue is NOT dissolved and is exactly consent item I-59 (tracked there; nothing dropped).
- **R-1** (duplicate keyed-by on value collections) — 018-81 (LOG:2594) governs duplicates source-agnostically ("the first in iterator order wins... non-fatal warning; it never traps"); SPEC:16077-16091 covers Vec/ordered determinism and Map key uniqueness. Adjacent unpinned corner (out of scope, noted): duplicate-winner selection over an UNORDERED source (HashSet) rides unstable iteration order.

## 6. SETTLED verification

- **I-71** — annotation holds: 013-79 (LOG:1498) now holds a §11.4.1 Copy rule between 013-78/013-80; the §11.8.5 move cluster runs contiguously 013-134..013-142. Closing: grammar-session merge f89c179. No decision remains.
- **DP-02** — annotation holds, zero residue: 013-142 (LOG:1561) "The `move` operand is restricted to a bare identifier; a field-access l-value path... is rejected"; every old permissive SPEC site is now restrictive (SPEC:9607-9608, 9615, 9626, 9639, 24766 all "not admitted"); adversarial grep found no surviving permissive statement. Finding F046 dismissed on the ruling. No decision remains.

## 7. Residue R-1..R-6 findings (summary)

- **R-1** duplicate-keyed-by on value collections: DISSOLVED — covered source-agnostically by 018-81; no value-collection-specific gap. (HashSet duplicate-winner nondeterminism noted as an adjacent, pre-existing corner.)
- **R-2** strict-kind sweep: STANDS-MECHANICAL (optional, as the ledger marked it) — 016-167/029-124/030-48/030-49 still call applied annotations "kind"; the channel law (016-178) defined the vocabulary but did not sweep these. Caveat: 016-178/016-180 themselves say "lowercase kind annotation/spellings," so only the bare-noun usage is the target.
- **R-3** Portal[WeakHandle[T]]: STANDS-MECHANICAL — SPEC:15072-15074's two-step presence check has no LOG mirror (0 hits; 017-191/017-202 cover the Handle and Portal-Portal cases only).
- **R-4** InstrSep: STANDS-MECHANICAL and WIDER than logged — LOG 033-207/208/209 (LOG:4384-4386) AND SPEC §15.4.4 (25514-25516) both diverge from normative IR_GRAMMAR:345-357 (`(instr InstrSep)*`, `instr_rhs`, bare-NAME/move-NAME operands); IR_GRAMMAR:331's "lifted verbatim from SPEC §15.4.4" claim is now false.
- **R-5** GRAMMAR 005-30 cites: STANDS-MECHANICAL — the 4ffb4a6 re-anchor fixed only the marker-trait cite; 15 stale 005-30 cites remain (GRAMMAR:1173, 3771-3880); the section-anchor-convention defense fails (AssocTypeDecl cites its own 005-15/17).
- **R-6** mixed directive stacks: STANDS-RULING (borderline) — see ruling queue Theme M.

## 8. New defects noticed in passing (report only; none acted on)

Items folded into existing entries are cited there; genuinely new observations:
1. **SPEC §13.17.7 Case-3 comment self-garbled** (SPEC:20733-20735): calls `render(song)` "a node reference" AND `render(song).audio` "the walked result" — incompatible; standalone garble regardless of I-01's ruling.
2. **017-272's citation is stale/mistargeted** (LOG:2456 cites §13.3.8 for wire activation; SPEC:15471 §13.3.8 is "Effects") — compounds I-03.
3. **Two near-duplicate stream-reload sections** (§13.15.5 SPEC:20174-20227 vs §13.18.14 SPEC:22469+) restate buffer/cursor rules — currently agreeing, standing divergence hazard.
4. **Reload step 8 recomputes deriveds only, silent on recurrents** (SPEC:20110-20114; 028-21) — same root as I-49; pin together.
5. **I-58's contradiction propagated into user-facing diagnostics**: §13.19.5 hint (SPEC:23181-23190) vs §13.19.16 hint (SPEC:23781-23784) directly contradict.
6. **005-236 now contradicts its own cited SPEC §3.9.4** (SPEC:3050) — the I-30 divergence is a live LOG↔SPEC defect, not just multi-source drift.
7. **`.count` example types disagree three ways within SPEC** (usize 7375/14192; u32 20605; i32 11970) — all sites need conforming under whatever I-27 resolves to.
8. **Const-generic rejected-ops table mislabel suspicion** (SPEC ~1015): "Integer `/`, `%`" justified by "Truncate…" — the truncating operator is `\`, `/` is mathematical division; likely label/rationale mismatch (LOW).
9. **`->` crossing postfix has no precedence-tier row** (LOG 007-74 and SPEC tier 15 agree in omitting it — no divergence, but the crossing operator's precedence is unstated; possibly intentional as position-disambiguated).
10. **`not in` surface form** (SPEC:7682 `'cherry' not in m`): compound vs `not (x in m)` unstated — tied to I-41.
11. **GROUP-SNAPSHOT adds a second reactive→concrete path** outside the "exactly three" wording (034-14, 017-75) — strengthens DP-10's optional scoping nicety.
12. **SPEC §13.21.1 "names the `<members>`" vs GRAMMAR:4726's anonymous-collect admission** — reference-flavored prose vs expression-admitting (non-normative) grammar; DP-17-adjacent.
13. **Audit-doc id drift**: systematic off-by-one in the 012 area + scattered elsewhere (016-285→286, 012-101→100, section §9.3.7→§9.3.8, dispatch step 8→9, precedence tier 14→15) — execution briefs must re-anchor by content.
