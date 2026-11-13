# The size of repairing the lexer and the parser, at `6491d802`

Sized by one agent on Opus on 2026-11-13, from reading the code and the two reviews (`syntax-conformance-review.md`, `syntax-recovery-review.md`); it built and ran nothing. It was told to measure and not to recommend. Its constraints: the parser sees one file; where only what a name names tells two readings apart, the parser builds one tree form for both; the lexer decides nothing that depends on what the parser reads. Recovery is sized twice: (i) the present design patched so that the findings hold; (ii) the design replaced by "a fault's region is the rest of its line and the block indented under it, nothing in it is built or reported, an unclosed bracket is a fault of the line that opens it, and no list of keywords ends a bracket". Design (ii) was the peer's proposal and not yet the owner's ruling when this was sized. The text below is the agent's report, word for word.

---

## Sizing the syntax repair at `6491d802`

I read all of the non-test code under `syntax/` and both reviews in full. I read SPEC §2.2, and took the other productions from the reviews' quotes. I built nothing and ran nothing; every number below comes from reading. Lines are physical lines, blanks and `use` lines included. Function spans are written `name@first-line/length`.

**How the four parts were split.** A file's header lines go to the file's main part.
- **(c) Recovery** takes every whole function, field, constant and branch whose only job is broken source. Mixed functions are split by line range.
- (c) has two halves: **r** = detecting and reporting at the point of detection (lexical errors and help strings included), and **m** = the machinery that skips, resyncs, suppresses and keeps region records.
- An `expect(...)` call counts as grammar, because it consumes a valid token; its error path counts as (c).

Abbreviations: lx = lexer/mod.rs, pm = parser/mod.rs, it/bl/pl/ex/ty/de/lo/js = parser/{items, blocks, placements, expressions, types, decls, loops, json}.rs.

### 1. The parts (non-test total 10,913 lines)

| Part | Lines | Files |
|---|---:|---|
| (a) lexer and layout | 823 | lx 714, token_columns 53, nfc 11, token 9, lex 9, lexed 8, scan 8, mode 7, clause 4 |
| (b) parser grammar (engine included) | 3,593 | pm 637, expressions 580, placements 410, items 356, types 321, decls 277, task 245, blocks 245, loops 148, json 124, lambdas 124, level 59, tail 44, cost 8, checkpoint 6, parse.rs 9 |
| (c) recovery | 2,182 | lexer 796 = r 386 + m 410 (lx 684, line_start 39, scan 36, raw_diagnostic 26, suspension 7, region 4). Parser 1,375 = r 778 + m 597 (pm 499, items 159, starts_line 143, placements 143, blocks 107, expressions 78, loops 75, decls 53, lambdas 35, types 35, json 21, loop_header 19, bracket 8). parsed.rs `detached`/`at_layout` 11 |
| (d) tree, builder, views, ast/ | 4,315 | ast/ 3,009, syntax_kind 459, node_view 158, parsed 156, tree_builder 107, token_view 85, preorder 70, tree_columns 56, element_view 55, children 50, child_pool 38, mod.rs 35, descendants 30, walk 7 |
| Tests (left out of the parts) | 4,218 + 11 | parser/tests 3,280 (fault_catalogue 2,025, generated_file 516, line_shape 190, single_faults 187, valid_items 164, kind_reads 114, repeated 45, mod 39), front_end_goldens 406, cost_tests 383, ast/tests 106, lexer/tests 43. The 11 are `run_counting`, pm 97–107, which is `cfg(test)` |

So recovery code is 20% of the non-test module, and 38% of the parser's (b)+(c).

### 2. One row per finding

"Lines in fns" is the size of the touched functions, split by part. All change sizes are estimates.

| # | Functions a fix changes | Lines in fns | Kind | Est. change | Why |
|---|---|---|---|---|---|
| A1 | pl wire_after_attrs@68/23 | b14 c9 | local | +4–8 | Make the `:` optional; where a wire may omit its destination is left to a later stage (E5). |
| A2 | bl body_item@92/20, block@8/31 (inline path); pl line@92/10, line_after_head@103/9, placement_tail_body@144/16, wire_after_attrs@68/23; a new parser for what follows `:`; syntax_kind +1 kind; views Placement.body@53, Body.block@20, Block.members@35, Destination.value@21 and match_block@25, BodyItem.cast@19, plus one new view | b71 c38 d26 | tree | +90–160 | `Kind head : rest` becomes one node whose rest reads as a Destination or as InlineChildren. |
| A3 | pl top_placement@24/16, wire_placement@41/10, placement_head@228/19; views Placement.kind@25 and name@37, WirePlacement.kind@25 and name@37 | b31 c14 d12 | tree | +25–45 | The kind becomes a QualifiedName node, so the views' `name_at(0/1)` would return the instance name as the kind. |
| A4 | ty type_args_optional@248/20, type_arg@275/48; TypeRef.base@20, lengths@42, capacity@47, plus an arguments accessor | b61 c7 d36 | tree | +30–60 | The first bracket list after a name is always built as one neutral TypeArgs node; the dimension reading moves to the views or a later stage. |
| A5 | pm at_attr_entry@891/7, ex at_operator@652/6 | b13 | local | +3–8 | Accept any keyword as a later entry's name. A bare keyword operator after a value stays ambiguous (E6). |
| A6 | lx line_break@565/19, token@613/65, json_value@679/40, json_token@720/29, ident@953/31, resynchronize@346/70, push@1258/53; ex at_alt_type@190/30, primary@344/131; ty size@228/19, type_arg@275/48; js json_lit@7/5, json_value@13/65 | a214 b240 c151 | structural | +80–150 | The lexer cannot tell the type `json`, a member `.json` and a literal `json` apart, and JSON strings tokenize differently from code strings (`"{"`), so the parser must ask for JSON lexing (a protocol change). Only the `.json` member case is a local fix, about 3 lines in `ident`. |
| A7 | lx layout@461/80 (lines 462–470), ends_region_here@1312/9, starts_with_item_keyword@1322/14, resync_line@220/50 | a29 c124 | structural | −32, +0–20 | Deleting the lexer's keyword rule changes the layout algorithm and the recovery design: unclosed regions then end only at a parser resync. |
| A8 | pl atom@328/64 | b53 c11 | local | +6–10 | Read `.Name` after the `::` branch. |
| A9 | de at_record_literal@104/25 | b25 | local | +6–12 | The `::` branch reuses the bracket-group test. |
| A10 | ex cmp_loop@159/30, at_alt_type@190/30, alt_type@221/3; BinaryExpr.rhs@25 and alt_type@36; AltType.ty@7 | b57 c6 d9 | tree | +25–50 | The right operand of `is` becomes one neutral node, built on A4's bracket list. `{…}[` is a map type only when `[..` follows. |
| A11 | it top_item@22/50 (lines 38–49) and the INPUT_FORM constant | b19 c31 | local | −13 | Delete a help branch that fires on valid code. |
| A12 | ex at_alt_type@190/30; ty type_args_optional@248/20, type_arg@275/48 | b91 c7 | local | +6–12 | `json` + name + `=` is a type; `{` starts a type only before `[..`. |
| B1 | ty base_tail@210/17, base@107/69; ex alt_type@221/3; bl match_label@308/44; task.rs Base/BaseTail@199/2; pm step@183/193 | b296 c32 | local | +10–20 | Pass a "no `?`" flag through two task variants. `step` alone is 193 of these lines. |
| B2 | pl line_after_head@103/9, line_members@113/14, members_loop@173/7, bundle_loop@449/21, bundle_for_members@483/10, attrs_loop@412/30 | b73 c18 | local | +20–35 | One adjacency helper, checked before each later member or entry. |
| C1 | TypeRef base@20/17, enum_name@38/3, lengths@42/4, capacity@47/15 | d39 | local; view API changes | +30–50 | Dimensions in order, each a length or a capacity, Size nodes included. Callers outside syntax change. |
| C2 | Lambda.params@8/3 | d3 | local | +8–12 | Add accessors for the bare parameter and the result type. |
| C3 | ArgList.positional@22/3, lambdas@26/6; NamedArg.value@25/3 | d12 | local; view API changes | +20–35 | Arguments in order as one enum; a named argument's lambda value. |
| C4 | AttrEntry.is_negated@21/3, value@29/3 | d6 | local | +4–10 | An accessor that tells `=>` from `=`, and one for the bound path. |
| C5 | PlacementEntry.count@11/3 | d3 | local | +15–25 | Read the count from a `+`/`*` token or a Count node; add a name accessor. |
| C6 | bl match_label@308/44; ty size@228/19, qualified_type_name@177/7; ex qualified_name@476/7, variant_ref@484/7; MatchArm.label@21 and alt_type@25; TypeRef lengths and capacity; VariantRef.enum_name@20 and variant@24; QualifiedName.package@7 and name@11; Expr.cast@45/23 | b72 c12 d60 | tree | +30–60 | Make the shapes uniform: Size always a node, one node kind for a qualified name, and a bare match label in a form both readings accept. |
| G | RecordLit, TypeRef, MapType, Use, UseName, Require, PublishName, Count | d30 | local | +90–130 | Accessors are only added. Four of these views are bare macro lines. |

**Recovery, (i): the present design kept**

| # | Functions | Lines in fns | Kind | Est. change | Why |
|---|---|---|---|---|---|
| F1 | pm group@1094/33; lx suspended_line_at@275/20 | b33 c20 | local | +15–30 | Stop the group scan at a line resync would accept. Risk: a valid multi-line bracket with a line at column ≤ top that starts with `[` or `(`. |
| F2 (same under ii) | lx unterminated_start@590/8, close_strings@599/13, token@613/65, string_part@886/66 | a107 c45 | local | +8–15 | Keep the index of the outermost String mode instead of walking the mode stack at every line end. |
| F3 | pm error_with@772/15, step@183/193, colon_or_skip@926/12, match_after_fault@939/13, skip_to_final_colon@987/12 | b184 c61 | local | +15–30 | After a fault, skip the rest of the line into an ErrorNode (a pending flag that `step` checks); the three header helpers cancel it so the `:` body is kept. |
| F4 | lx layout@461/80; pm unexpected_indent@1051/11 | a29 c62 | local | +10–25 | On a dedent mismatch, keep the block's column. No UnexpectedIndent after a layout fault on the line above. |
| F5 | pm colon_or_skip@926/12, match_after_fault@939/13, skip_to_final_colon@987/12 | b5 c32 | local | +8–15 | A header that drew a fault skips its block silently. This changes Property 8 rule 10. |
| F6 | bl block@8/31, block_items@48/43; ex primary@344/131; it rest_of_block@479/16, fn_lets@438/16; lx digits@1104/28 | a12 b146 c107 | local | +30–55 | Six separate patches, one per pattern the review lists. |
| F7 | pm bump@646/12, unexpected_indent@1051/11, plus one field | b8 c15 | local | +10–20 | Suppress only under a line that could open a block. |
| F8 | lx punct@1159/77 | a38 c39 | local | +2–5 | Lex a depth-0 `}` as RBrace, so the parser reports it like `)` and `]`. |

**Recovery, (ii): the simpler design (all three rows structural)**

| Row | Functions | Lines in fns | Est. change |
|---|---|---|---|
| lexer | Change: struct@32/30, new@64/32, run@97/8, advance@117/22, line_at@417/26, layout@461/80, push@1258/53, digits@1104/28, punct@1159/77. Delete: faulted_at@176/11, in_cut_construct@188/11, token_before@200/19, resync_line@220/50, depth_at@271/3, suspended_line_at@275/20, line_start_of@296/49, resynchronize@346/70, record_faults@444/16, ends_region_here@1312/9, starts_with_item_keyword@1322/14, line_stack_snapshot@1337/11, peak_stored_entries@172/3, and the files region.rs, suspension.rs, line_start.rs | a216 c476 | about −355; +60–100 |
| parser core | Change: struct@44/21, new@67/23, parsed@109/45 (drop lexer diagnostics inside a region), step@183/193 (unwind the task stack and builder markers to the line-level task), first_significant@415/9, raw_bump_kind@561/51, bump@646/12, missing_block@763/8, error_with@772/15, same_fault_as_lexer@788/15, expect@847/32, finish_line@1037/13, unexpected_indent@1051/11, tree_builder finish_node@70/8. Delete: end_region_before@425/30, abandon_bracket@804/6, resync@811/5, resync_at@817/9, match_after_fault@939/13, match_anchor@953/20, skip_to@974/12, skip_to_final_colon@987/12, final_colon@1000/19, holds_stray_colon@1079/13; simplify colon_or_skip@926/12; delete the files starts_line.rs, bracket.rs, loop_header.rs | b289 c480 d8 | about −360; +80–140 |
| grammar call sites (25) | it: file_step, top_item, let_value, cell_after_clock, fn_colon, pattern_after_params, fn_lets, rest_of_block. bl: block, block_items, body_item, for_after_iterable, when_block, guard_after, when_after_condition, when_else, match_after_scrutinee. pl: wire_after_attrs, placement_tail_body, default_after_atom. de: type_decl_body, field_decls. ex: string_loop, interp_end. lo: for_expr_colon | b251 c327 | ±60–120 |

- **How the lexer row works:** lex the whole file once and record the openers that never close. Then lay out again with each of those closed at the end of its own line. That is two linear passes, and it ends lazy lexing and relexing.
- **What the call-site row does:** each site drops its own skip or resync call. A skipped line counts as a block item or as the body's value (F6 b and e). A missing block's region is the next line (F6 a).
- **What (ii) covers:** F1 (the group scan stops at the NEWLINE an unclosed line now gets), F3, F4, F5, F6 a–f, F7 (a block under a faulty line is in its region by definition) and F8. F2 is unchanged.

### 3. Totals per part

Each cell is functions / lines inside touched functions / % of the part. A function counts once, in the part holding most of its lines; lines are attributed one by one.

| Part (size) | Conformance, all rows | Conformance, tree + structural | Recovery (i), F1–F8 | Recovery (ii) + F2 |
|---|---|---|---|---|
| a (823) | 6 / 243 / 29.5% | 6 / 243 / 29.5% | 2 / 186 / 22.6% | 8 / 323 / 39.2% |
| b (3,593) | 35 / 877 / 24.4% | 22 / 447 / 12.4% | 6 / 371 / 10.3% | 21 / 540 / 15.0% |
| c (2,182) | 6 / 425 / 19.5% | 5 / 340 / 15.6% | 13 / 338 / 15.5% | 52 / 1,328 / 60.9% |
| d (4,315) | 39 / 164 / 3.8% | 24 / 124 / 2.9% | 0 / 0 / 0% | 1 / 8 / 0.2% |

- **Combined, conformance + (i):** a 41.2%, b 26.5%, c 29.7%, d 3.8%.
- **Combined, conformance + (ii):** a 51.9%, b 32.5%, c 67.3%, d 4.0%.
- **Tree and structural rows only:** every (i) row is local, so (i) adds nothing to the conformance tree+structural column. Every (ii) row is structural.
- **Estimated change:**
  - conformance rows: 550–960 lines, of which 310–580 are in the tree and structural rows;
  - recovery (i): 100–195 lines;
  - recovery (ii): about 715 lines deleted and 200–360 added or changed.
- **Large functions that inflate the counts:** `step` (193 lines, a dispatch table; B1, F3 and (ii)), `primary` (131; A6, F6), lx `layout` (80), `token` (65), `push` (53).

### 4. What editing cannot reach

These follow from how the two stages are built:
- **Lazy lexer with truncate-and-relex.** The lexer is driven by the parser (pm fill 155–162, sig_token 385–394) and relexes from a resync point (lx resynchronize 346–415, called from pm 450 and 821). The lookahead memo is cleared on every relex (pm 451, 822). Causes: F1, and the tokens of `lex` and `parse` differing on broken input.
- **JSON mode chosen by the previous word.** `json_next` is set at lx 964–966, read at 621, reset at 577 and 412. Causes: A6, the `json[2]` rows of C1, E7. The lexer cannot get this right without parser context.
- **Keyword lists that end bracket regions.** In the lexer: lx 462–470 and 1312–1335. In the parser: starts_line.rs 1–143, end_region_before pm 425–454, holds_stray_colon pm 1079–1091. Causes: A7. The tests line_shape.rs and generated_file.rs restate these rules.
- **Token kinds renamed by the parser.** expect_member_name pm 899–909 records the new kind in TreeBuilder.renamed (tree_builder 219, 234–236), and the renaming is applied after parsing (pm 138–142). Causes: keyword-as-name handling (A5/E6) depends on parser state.
- **The task-stack design.** Task has about 150 variants (task.rs, 245 lines) dispatched by `step` (pm 183–375), with state carried in variant fields; precedence is handled by Tail absorption (tail.rs, ex 71–111).
  - B1 needs new variant fields.
  - A fault cannot unwind pending tasks, so error_with (pm 772–786) only sets `line_error` and the queued tasks keep parsing the line silently (F3).
- **The wire-or-node choice is made from the vocabulary.** at_wire_kind (pm 546–552, used by body_item, top_item, placement_head) and starts_line is_kind (54–60). Causes: A2 and E1.
- **Recovery state is two booleans plus side lists.** `line_error` and `previous_line_error` (pm 56–58, 653–656), and `detached`/`at_layout`. The lexer and parser report independently and are de-duplicated by offset (same_fault_as_lexer pm 788–802 with lexer faults/reach/cut_constructs). Causes: F4, F7, F8.

### 5. What I could not size, and why

- **Readers outside syntax/.** Mainly `semantics/ir/builder/*` and `semantics/ir/flat/builder/*`, plus analysis code that uses view accessors. They are affected by A2, A3, A4, A10, C6 and the view-API changes in C1 and C3. My grep found them by name; I did not read them.
- **The neutral forms of A2 and A10.** Whether one superset grammar exists for `: X` read as an Expr or as InlineChildren (`[a, b]` against a bundle, `(A | x=1)` against a paren expression) is a design choice not yet made, so those estimates are wide.
- **A6 under a strict reading** of "the lexer decides nothing": if that also forbids parser-requested modes, JSON would have to be re-scanned from raw text, and I cannot size that.
- **SPEC-dependent scope.** E5 (A1) and E6 (A5) change how much a fix must do, depending on how the SPEC is ruled.
- **Tests.** The tests that encode the current recovery rules (fault_catalogue, generated_file, line_shape, single_faults, repeated_faults; 2,963 lines) need rework under (ii) and partly under (i) and A7. I did not size that work.
- **Unverified effects.**
  - Whether the (i) patches create new double messages.
  - Whether (ii) is linear in practice.
  - Whether the parser's resync rule ever fires on valid input; I believe not, but did not check.
- **Classification error.** The split between (b) and (c) inside mixed functions is my judgement, branch by branch; read it as ±10%.
- **Out of scope.** F9 (memory per byte) is outside F1–F8.
