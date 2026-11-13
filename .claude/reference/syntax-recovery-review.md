# Cold review: the lexer and the parser on broken source, at `6491d802`

Reviewer: one agent on Opus at high effort, given the code under `crates/compiler/src/syntax/`, SPEC §1, §2 and the grammar, the two rules for broken programs and the goal, with no hints. The text below is its report, word for word, copied by the peer session on 2026-11-13 from `/home/cdagent/cargo-targets/peer-syntax-review/recovery/REPORT.md`; its probe, generators and run outputs stay in that folder.

---


# Cold review: lexer and parser on broken source (commit 6491d802)

Work directory: `/home/cdagent/cargo-targets/peer-syntax-review/recovery`. Code reviewed: `packages/symphonyscript/crates/compiler/src/syntax/`. All line numbers below are in that tree.

## Verdict

A rebuild can rely on these two stages for robustness. Across about 700,000 broken inputs I saw no panic, no hang, no stack growth with nesting (every construct at depth 100,000 on a 1 MiB stack), and no result that was not whole. Valid source never entered recovery. Two things it cannot rely on yet. First, time is not linear in the source on two hostile shapes. One is quadratic on broken input with a large constant: 4,000 short lines take 12 s. The other is quadratic on valid input. Second, the two message rules do not hold in several recurring patterns:
- a second parser fault on the same line is never reported, even though the parser builds the rest of that line normally;
- an indentation fault draws a second message on the following line;
- a faulty or missing keyword makes the lines under it draw one message each.

I am confident about the crash, wholeness and timing results: each is measured, and the timing behaves as a clean power law. The message-count findings come from my own judgement of about 1,200 cases with too few or too many messages, out of 78,000. I read the small and frequent groups but not every case.

## Findings, most severe first

### F1. Time quadratic in the number of lines that each hold an unclosed nested group (broken input). Verified by run.

Smallest shape: N lines of `Clip c{i} | gain=f((1` at top level. The same happens with `    Cue/(f((1` lines inside a body, and with `Clip c{i} | gain=Foo[(1` (a type-argument group).

- N = 500, 1000, 2000 and 4000 lines (11 KB to 91 KB) take 183, 722, 2906 and 11,776 ms. Each doubling costs x3.95, x4.02 and x4.05.
- Total allocation is quadratic too: 6, 23, 89 and 351 MB. Peak memory is linear: 0.4 to 3.5 MB.
- The control shape is the same lines without the inner `(` (`gain=f(1`). It is linear: 16.8 ms at 2,000 lines and 68.5 ms at 8,000.

Property 7 requires linear time.

Cause, read, not run:
- `Parser::group` (`parser/mod.rs:1094-1126`) looks for the group's closer. It stops only at NEWLINE, INDENT or DEDENT (`:1117`), and inside an unclosed bracket none of those is emitted, so an unclosed group is scanned to the end of the bracket's region. Its `kind_at` calls also force the lexer to lex that far.
- The memo `groups` is cleared at every resynchronization (`mod.rs:451`, `mod.rs:822`). Each such line is a resynchronization line, and the lexer truncates and relexes from it (`lexer/mod.rs:346-415`). So every line rescans and relexes the rest of the file.
- The callers are `at_lambda` (`lambdas.rs:11`), `type_start` (`types.rs:40`), `group_has_comma` (`types.rs:264`) and the `{` check (`expressions.rs:216`).

### F2. Time quadratic in the number of `{` left open on separate lines, valid or broken. Verified by run.

- Broken: `let y = ` followed by N lines of `{`. N = 5k, 10k, 20k and 40k take 34.9, 106, 353 and 1,267 ms; doubling costs x3.04, x3.33 and x3.59.
- Valid: a nested map literal with one `{1:` per line, closed at the end (`nested-map-lines-valid`). It takes 27.6, 93.2, 324 and 1,221 ms; doubling costs x3.38, x3.48 and x3.77.
- The same nesting on one line is linear.
- The depth probe `([{` per line at N = 100,000 (400 KB) took 7.9 s.

Cause, read, not run:
- `Lexer::unterminated_start` (`lexer/mod.rs:590-597`) walks the whole mode stack at every line end (`:545`).
- Each `{` pushes a `Mode::Brace` (`:655-660`), so with D braces open each line end costs O(D).
- JSON `{` pushes no mode after the first, so multi-line JSON stays linear.

This is the only superlinear cost I found on valid input.

### F3. Only the first parser fault on a line draws a message, though the rest of the line is not skipped. Verified by run.

Smallest inputs:
- `let = 1 + + 2` draws 1 message (missing name). The doubled `+` draws nothing.
- `let x = f(1 + , 2) + g(3 +, 4)` draws 1 message. The tree holds both calls fully built, with no ErrorNode around the second fault.
- `Scene s:\n    Cue/(1 + ) Cue/(2 + )` draws 1 message for two faulty placements.

Lexer faults on the same line are each reported, for example `let x = 1 == 2 == 3` draws 2 messages. Cause: `error_with` returns at once while `line_error` is set (`parser/mod.rs:772-786`), and the flag clears only at NEWLINE (`:653-656`). After a missing name, the parser keeps parsing, and every later complaint on the line is dropped.

Property 5 says every fault outside an earlier fault's skipped region draws its own message. Here nothing is skipped in the tree. (ARCHITECTURE §7.1 instead defines the region as "the rest of its line" and allows one parser diagnostic per logical line.) In the property 5 run this explains most of the 269 `one-line` cases that drew fewer messages. A logical line includes every line an unclosed bracket spans, so a fault on a later physical line inside the bracket is silenced the same way.

### F4. One mis-indented line draws two messages: lexer DedentMismatch, then parser UnexpectedIndent. Verified by run.

Smallest input: `Rig r:\n    Lamp as a/1\n  Lamp as b/2\n    Lamp as c/3\n` draws 2 messages.
- The DedentMismatch reports the mismatched line's indentation and says "the enclosing block is at 0".
- The parser then reports UnexpectedIndent at the next line. That line is correctly indented for the block it was in.

The lexer pops every column deeper than the faulty line (`lexer/mod.rs:521-538`), so the block's own column is gone. The next line at that column then emits INDENT, which `unexpected_indent` reports (`parser/mod.rs:1051-1061`).

The second message is suppressed only when the mis-indented line happens to draw a parser complaint of its own. So `   Osc` (no name) draws 1 message, and `   Osc as b | ...` draws 2.

| Single-fault class | cases | two messages |
|---|---:|---:|
| indent-minus-one | 1,594 | 100 |
| indent-plus-one | 3,138 | 34 |

My judgement: one fault each.
- `indent-minus-one` is a clear finding.
- For `indent-plus-one` on the first line of a block (the first line sets the block's column), it is one edit, though the SPEC lets the second line also be read as wrong.

The same mechanism applies when a tab replaces four spaces, because the tab counts as one column:
- `Rig r:\n\tLamp as a/1\n    Lamp as b/2` draws TabIndentation plus UnexpectedIndent.
- `Scene s:\n    Cue/1\n\tCue/2` draws TabIndentation plus DedentMismatch.

One keystroke gives two messages.

This is the only pattern I found where the lexer and the parser both report what is one fault. They report it at different spans: the mismatched line, then the next line. I found no case where the two stages report the same span.

### F5. A missing or faulty keyword makes every line of the block under it draw a message. Verified by run.

- Deleting `let` from `let level = match wave:` followed by 3 arms gives 4 messages. The parser reads `level` as a placement kind, keeps the indented arms as the placement's body, and reports each arm (`expected a placement, found 1.0`, and so on).
- Deleting `fn` from `fn scale(degree, root = 60):` followed by 3 `let` lines and a value gives 5 messages. Each line draws `a let line stands only at the start of a pattern body`, and the value line draws one more.
- `area = match shape:` (the `let` deleted) with two arms gives 3 messages.

This is one fault each. 106 of the 26,956 single token deletions behaved this way. Cause: `colon_or_skip` (`parser/mod.rs:926-937`) and `final_colon` keep a block that ends in `:` as a body even when its header is wrong.

### F6. Other single faults that draw two messages. Verified by run.

The second message is a consequence of the first:
- `Clip c:\nNote/1` (the indentation of the only body line missing) draws "expected an indented block" plus "expected the top-level instance's name, found `/`". The unindented line is parsed as a top-level item.
- `Synth s:\n    let Osc` draws "a let line stands only at the start of a pattern body" plus "a block holds at least one body item". `block_items` does not count the misplaced `let` as an item (`parser/blocks.rs:48-77`).
- `fn f(x):\n    let a = (x + 1\n    a * 2` draws the missing `)` (reported at `a`) plus "expected an expression, found the end of the block". The value line was swallowed into the bracket.
- `let v = match ( w:` followed by its arms draws "a lambda stands only as an argument..." plus "expected the match arms, indented".
- A body line made faulty by a stray `let`, `}`, or a deleted `let`, followed by "a function body ends with one value".
- `let x = 1__0` draws one MisplacedUnderscore per underscore, so 2 here (`lexer/mod.rs:1115-1128`). A run of underscores is one fault.

### F7. Faults that draw no message although they lie outside every skipped region. Verified by run.

An indented line under any line that drew a parser complaint draws nothing. For example, `state ] wave(0.0)\n let level = 1` reports only the `]`, and the unexpected indentation of the next line is silent (`parser/mod.rs:1051-1059`, `previous_line_error`). The indented block is then skipped as an ErrorNode, so its own faults and declarations vanish silently.

This is deliberate in ARCHITECTURE §7.1, because a faulty header may have been meant to open a block. But it also applies to lines that cannot open a block, like the `state` line above. In the property 5 run it accounts for 115 of the 170 misses in `separate-items` and `separate-lines-one-item`. I judged every other miss in those two categories to lie inside an unclosed bracket's skipped region.

### F8. A stray `}` is a lexer error; a stray `)` or `]` is a parser error. Verified by run.

`let y = 1)]}` draws 2 messages: the parser's "expected the end of the line, found `)`" and the lexer's "unexpected `}`". The `]` between them draws nothing.

The source holds three faults of the same kind, and each closer type gets different treatment. A `}` at depth 0 outside a brace mode falls through to `unexpected()` (`lexer/mod.rs:661-671`, `:1228-1230`). An N-closer line `)]}` x N draws N+1 messages. No message rule is broken by a fault drawing two messages, but the treatment is inconsistent.

### F9. Memory per byte is high on bracket-heavy broken input. Verified by run. Not a linearity fault.

- Peak memory is 150 to 230 bytes per source byte for lines of open brackets (`open-lines-brace`, `open-lines-mixed`).
- Total allocation is about 440 bytes per byte for an unclosed `(` x N on one line.
- For ordinary faulty lines it is 60 to 80 bytes per byte.

Peak memory stayed linear everywhere I measured. Total allocation is quadratic only in F1.

## Counts

### Property 1 and 2: crashes and hangs

| Input set | inputs | panics | hangs (> 60 s) | not whole | ranges not nested | bad diagnostic span | clean but with recovery marks |
|---|---:|---:|---:|---:|---:|---:|---:|
| `probes.jsonl.gz` | 375 | 0 | 0 | 0 | 0 | 0 | 0 |
| `breakers.jsonl.gz` | 20,293 | 0 | 0 | 0 | 0 | 0 | 0 |
| mutants of 400 valid programs, <= 800 B, sorted order | 323,830 | 0 | 0 | 0 | 0 | 0 | 0 |
| mutants of 300 valid programs, <= 3 KB, shuffled | 354,357 | 0 | 0 | 0 | 0 | 0 | 0 |
| single-fault cases (property 4) | 62,120 | 0 | 0 | 0 | 0 | 0 | 0 |
| multi-fault cases (property 5) | 15,573 | 0 | 0 | 0 | 0 | 0 | 0 |
| hand-made faults (property 4, 8) | 118 | 0 | 0 | 0 | 0 | 0 | 0 |

- The 5,182 valid programs are corpus sources with no parse diagnostic, plus the SPEC `sym` examples that parse clean.
- The mutants are:
  - truncation at every character boundary;
  - every significant token deleted, doubled, and swapped with the next one;
  - random insertion of `)` `]` `}` `(` `[` `{` `"`, tab, CR, NUL, U+0001, U+007F, BOM, ZWSP, combining acute, `é`, `名前`, an emoji, `#`, `_`, `^`, `;`, `==`, `\`, `{"`, `json {`, `json [`, `"{`, `//`, `|`, `:`, `::`, `..`, `'`, and indentation runs;
  - every LF replaced by CR alone, and every LF replaced by CRLF;
  - each run of four spaces replaced by a tab or by two spaces;
  - ASCII letters in identifiers replaced by non-ASCII ones;
  - a 2,000-times repeated line added in front.
- Property 1 is checked on every source that drew no diagnostic. Such a source must hold no Error token, no ErrorNode, no `detached` range and no `at_layout` range: 0 violations. The converse also held: no input held an Error token or ErrorNode without a diagnostic.
- Property 6 is checked on every input:
  - the token starts run from 0 to the end of the text with no gap;
  - the joined token texts equal the source;
  - every node's token range lies inside its parent's, and siblings are ordered and do not overlap;
  - the root is `File` and covers all tokens;
  - every diagnostic span lies on character boundaries inside the source.
- The slowest single corpus input was the 2 MB probe at 244 ms.

### Property 3: nesting at depth 100,000 on a 1 MiB thread

All runs passed, both valid and broken, with the result whole and its ranges nested. The constructs:
- parentheses, arrays, maps, record literals, call chains `f(f(...`, index chains, conversions, string interpolation `"{"{...`;
- JSON arrays and objects;
- unary minus with and without spaces, `not`, `**`, `??`, `+`, nested `if`, curried lambdas, `for` expressions, `.` chains;
- parenthesized types, type arguments, function types, sum types;
- parenthesized placements, bundles, `/(` default values.

Broken extras:
- unclosed `a[0` x N, `f(1, ` x N, `([` x N with mismatched closers, and `([{` on N lines;
- closers only, on one line and on N lines; openers on N lines;
- open interpolations, JSON mixed with code, `if (a then` x N, `[Cue` on N lines, `(Cue` x N, `/(f(` x N, `Hit { note:` x N;
- open generic and pattern-type brackets, attribute values, `when` gates, `match` scrutinees.

Indentation nesting (`when`, child placements and `for` blocks one column deeper each, valid and with every other header broken) ran at depth 10,000 (50 MB of source) and passed. Depth 100,000 would need 5 GB of indentation.

### Property 4: one fault each, 200 shuffled valid programs

| Class | cases | exactly 1 | 0 | >= 2 |
|---|---:|---:|---:|---:|
| delete one token | 26,956 | 25,040 | 1,798 | 118 |
| stray token inserted | 26,956 | 26,017 | 905 | 34 |
| missing closer | 1,877 | 1,876 | 0 | 1 |
| string left open | 5 | 5 | 0 | 0 |
| line indented one space more | 3,138 | 2,865 | 239 | 34 |
| line indented one space less | 1,594 | 1,255 | 239 | 100 |
| one leading space made a tab | 1,594 | 1,594 | 0 | 0 |

- **Zero-message cases.** I read every group of deleted or inserted text: about 150 groups, at least one example each, and all rare groups in full. Every one I read is a valid program, so none is a miss. Examples: `brightness=thick shade` (a flag entry), `amount (pace - 1)` (a call), `[1, ]`, `held[..]`, `db : (x)` (a lambda value), `velocity let =x` (a keyword as an entry name). The 239 + 239 indentation cases are one-line blocks, where shifting the line is valid.
- **Two-or-more cases** are covered in F4, F5 and F6. The `missing-closer` case is a type `i32[3 = match ...` whose arms are then missing.
- **Hand-made single faults**, covering strings, interpolations, JSON, `==`, `;`, numbers, reserved words, control characters, CR alone, BOM, NBSP, ZWSP, tabs, clause columns, brackets, blocks, headers, arms and fields: 92 programs. 3 are valid and drew 0 messages. 78 drew exactly 1, 10 drew 2, and 1 drew 3.
  - Of the 2s, `bundle-unclosed-twice` holds two faults and is correct.
  - The other 2s and the 3 are one fault each: `deleted-fn`, `deleted-let-in-fn`, `deleted-let-top` (the 3), `double-underscore`, `missing-indent` x 2, `stray-let-before-only-item`, `stray-paren-match`, `tab-for-block` (see F4) and `unclosed-paren-next-expr-line`.

### Property 5: two and three faults, each drawing exactly one message alone, 400 programs

"Inside-skip" means a later fault lies inside an ErrorNode of the earlier fault's lone parse.

| Category | cases | exact | fewer | more |
|---|---:|---:|---:|---:|
| separate top-level items | 9,483 | 9,388 | 95 | 0 |
| separate items, one inside-skip | 39 | 30 | 9 | 0 |
| separate lines, one item | 1,791 | 1,716 | 75 | 0 |
| separate lines, inside-skip | 450 | 31 | 419 | 0 |
| one line | 361 | 92 | 269 | 0 |
| one line, inside-skip | 854 | 19 | 834 | 1 |
| mixed | 1,431 | 1,089 | 342 | 0 |
| mixed, inside-skip | 1,164 | 44 | 1,120 | 0 |

The 95 + 75 misses on separate items or lines:
- 115 hold an indentation fault right after a faulty line (F7).
- I judged the rest to lie inside an unclosed bracket's region, which runs to the next resynchronization line. Examples: `state s = y.past(1, 0\nClip :\n...`, where the header lacks its name, and `fn dbl(k: if ...\n\nbig = dbl(50)`.

The one-line misses are F3, or a later fault inside the first fault's ErrorNode. Some multi cases are my generator's artefacts: deleting a `(` and a `)` together, or shifting two lines alike, gives a valid program.

## Property 7 tables

Release build, pinned to core 15 with nothing else of mine running. Minimum of 5 runs.
- The ratio is the time against the previous size; 2 is expected.
- "alloc" is total bytes allocated during the parse; "peak" is the peak live bytes during the parse. Both are counted by a global allocator in the probe.

| Shape | N | bytes | ms | ratio | alloc MB | peak MB |
|---|---:|---:|---:|---:|---:|---:|
| N unclosed `(` on one line | 10k / 20k / 40k / 80k | 10 K-80 K | 18.6 / 36.0 / 72.8 / 145.8 | 1.93 / 2.02 / 2.00 | 4.4-35.2 | 2.1-17.1 |
| N unclosed `{1: ` on one line | same | 40 K-320 K | 25.6 / 52.3 / 100.6 / 206.7 | 2.04 / 1.92 / 2.05 | 5.1-41.1 | 3.1-25.0 |
| N lines of `(` | same | 20 K-160 K | 22.0 / 45.3 / 92.5 / 187.4 | 2.05 / 2.04 / 2.03 | 5.0-40.3 | 2.8-22.2 |
| N lines of `[` | same | 20 K-160 K | 20.9 / 42.6 / 84.9 / 175.0 | 2.04 / 1.99 / 2.06 | 4.0-31.6 | 2.4-18.9 |
| **N lines of `{`** | 5k / 10k / 20k / 40k | 10 K-80 K | 34.9 / 106.1 / 353.2 / 1266.5 | **3.04 / 3.33 / 3.59** | 3.5-28.2 | 1.5-12.1 |
| **N lines of `([{`** | same | 20 K-160 K | 55.0 / 148.5 / 441.4 / 1424.5 | **2.70 / 2.97 / 3.23** | 9.2-73.8 | 4.5-36.4 |
| **valid nested map, one `{1:` per line** | same | 25 K-200 K | 27.6 / 93.2 / 324.0 / 1220.8 | **3.38 / 3.48 / 3.77** | 2.0-15.8 | 2.0-15.8 |
| N lines `let xi = (1 +` | 10k-80k | 169 K-1.43 M | 67.3 / 136.2 / 274.8 / 534.0 | 2.03 / 2.02 / 1.94 | 12-98 | 7.1-58 |
| N stray `)` on one line | same | 10 K-80 K | 1.3 / 2.7 / 5.5 / 11.0 | 2.10 / 2.02 / 1.99 | 0.2-1.5 | 0.2-1.5 |
| N lines `let xi = 1)` | same | 149 K-1.27 M | 36.2 / 74.8 / 149.5 / 306.2 | 2.07 / 2.00 / 2.05 | 9.8-79.5 | 6.7-54.7 |
| N faulty lines `let xi = = 1` | same | 159 K-1.35 M | 44.6 / 84.8 / 167.7 / 340.7 | 1.90 / 1.98 / 2.03 | 10-83 | 6.3-51 |
| N faulty lines in one open `(` | same | 120 K-960 K | 15.1 / 31.3 / 65.2 / 132.1 | 2.08 / 2.08 / 2.03 | 2.5-20 | 2.5-20 |
| N faulty lines in one open call `f(` | same | 90 K-720 K | 11.1 / 23.0 / 47.3 / 97.8 | 2.08 / 2.06 / 2.07 | 2.0-16 | 2.0-16 |
| one fault, then N valid lets | same | 168 K-1.5 M | 15.0 / 30.8 / 64.0 / 126.7 | 2.06 / 2.08 / 1.98 | 1.9-17 | 1.9-17 |
| one open `(`, then N valid lets | same | 168 K-1.5 M | 15.2 / 30.7 / 61.6 / 123.3 | 2.02 / 2.01 / 2.00 | 1.9-17 | 1.9-17 |
| one open `(`, then N placements | same | 238 K-2 M | 41.2 / 85.3 / 161.6 / 322.2 | 2.07 / 1.89 / 1.99 | 5.8-31 | 5.8-31 |
| one open `f((`, then N placements | same | 238 K-2 M | 42.2 / 82.5 / 163.4 / 339.8 | 1.96 / 1.98 / 2.08 | 5.8-31 | 5.8-31 |
| N open strings, one per line | same | 169 K-1.43 M | 30.7 / 60.0 / 118.7 / 234.7 | 1.95 / 1.98 / 1.98 | 7.9-65 | 6.1-50 |
| N open `"{` on one line | same | 20 K-160 K | 16.7 / 33.9 / 67.6 / 136.3 | 2.03 / 1.99 / 2.02 | 5.0-40 | 3.1-25 |
| N open interpolations, one per line | same | 169 K-1.43 M | 49.3 / 98.4 / 194.3 / 394.3 | 2.00 / 1.98 / 2.03 | 10-85 | 6.7-55 |
| N mis-indented lines in a block | same | 109 K-872 K | 25.7 / 53.4 / 104.4 / 215.3 | 2.07 / 1.95 / 2.06 | 6.4-52 | 4.2-34 |
| N unexpected indents | same | 298 K-2.5 M | 36.6 / 78.3 / 149.6 / 296.6 | 2.14 / 1.91 / 1.98 | 11-92 | 8.2-67 |
| N tab-indented lines | same | 80 K-640 K | 24.8 / 47.8 / 93.8 / 186.3 | 1.93 / 1.96 / 1.99 | 6.6-53 | 5.0-40 |
| one line of N `+ 1` | same | 40 K-320 K | 6.3 / 12.3 / 25.0 / 50.1 | 1.96 / 2.04 / 2.01 | 0.8-6.4 | 0.8-6.4 |
| one line of N `+ + 1` | same | 60 K-480 K | 18.1 / 36.1 / 73.4 / 147.0 | 1.99 / 2.03 / 2.00 | 2.5-20 | 1.2-9.6 |
| one line in `(` of N `1 1` | same | 40 K-320 K | 5.4 / 10.7 / 21.8 / 43.2 | 2.00 / 2.03 / 1.98 | 0.7-5.9 | 0.7-5.9 |
| one body line of N `Cue/1` | same | 60 K-480 K | 9.7 / 18.2 / 37.1 / 74.1 | 1.88 / 2.04 / 2.00 | 1.2-9.6 | 1.2-9.6 |
| attribute clause over N lines | same | 190 K-1.5 M | 10.8 / 21.4 / 42.8 / 86.8 | 1.98 / 2.00 / 2.03 | 2.2-17.5 | 2.2-17.5 |
| same, every line faulty | same | 210 K-1.7 M | 10.1 / 21.1 / 41.5 / 84.5 | 2.09 / 1.97 / 2.03 | 2.4-19 | 2.4-19 |
| N nested `when = x:` blocks, every header broken (depth cycles 1-50) | same | 630 K-5 M | 45.2 / 92.6 / 178.6 / 361.0 | 2.05 / 1.93 / 2.02 | 15-118 | 10-83 |
| N function bodies, each broken | same | 349 K-2.9 M | 74.4 / 151.3 / 288.0 / 571.2 | 2.03 / 1.90 / 1.98 | 14-114 | 9.5-78 |
| N body lines `[Cue/1 Cue/2` | same | 170 K-1.36 M | 71.2 / 144.2 / 289.1 / 571.7 | 2.03 / 2.01 / 1.98 | 11-89 | 7-56 |
| N body lines `Cue/(f(1` | same | 130 K-1.04 M | 83.1 / 170.9 / 342.8 / 685.8 | 2.06 / 2.01 / 2.00 | 12-97 | 7.3-59 |
| **N lines `Clip ci \| gain=f((1`** | 500 / 1k / 2k / 4k | 11 K-91 K | 182.7 / 722.2 / 2906.2 / 11776.1 | **3.95 / 4.02 / 4.05** | 6 / 23 / 89 / 351 | 0.4-3.5 |
| **N body lines `Cue/(f((1`** | same | 7 K-56 K | 126.9 / 497.2 / 1993.3 / 8093.4 | **3.92 / 4.01 / 4.06** | 10 / 37 / 146 / 577 | 0.4-3.4 |
| **N lines `Clip ci \| gain=Foo[(1`** | same | 12 K-99 K | 191.3 / 755.2 / 3016.9 / 12569.0 | **3.95 / 3.99 / 4.17** | 6 / 23 / 89 / 351 | 0.4-3.6 |
| N dedent levels, one mismatch (bytes grow as N^2) | 1k / 2k / 4k / 8k | 0.5 M-32 M | 3.8 / 11.5 / 38.7 / 140.4 | 3.0 / 3.4 / 3.6 (x4 bytes) | 6-370 | 6-370 |

Raw output is in `probe/timing.txt`, and the full table in `probe/timing-table.md`.

## Property 8: where recovery cuts and resumes (description, not judgement)

The rules, as the code behaves:

1. **One complaint per logical line.** After the first parser complaint on a logical line, the parser reports nothing more until the next NEWLINE. It may still parse the rest normally. A logical line includes every physical line an unclosed bracket spans until a resynchronization. Lexer diagnostics are not limited.
2. **A token the construct cannot take.**
   - **What is skipped:** the token and the rest of its physical line go into one ErrorNode. The ErrorNode is the last child of the construct being built: the LetDecl, CellDecl, Placement, MatchArm or FieldDecl.
   - **Where parsing resumes:** at the next line.
   - **What the tree keeps:** the construct keeps everything before the fault, including its name. So `let x = = 1` still declares `x`, and `Lamp as a/1 = 2` still declares `a`.
   - In a match arm, the faulting token itself can stay outside the ErrorNode: `1 2: 3` gives two Literals and then ErrorNode `: 3`.
3. **A missing token** (a missing name, a missing operand, a missing `=`). Nothing is skipped. The node is built without the missing child, so `let = 1` gives a LetDecl with no Ident. Parsing continues on the same line under rule 1.
4. **An error token from the lexer** (`;`, `==`, `!=`, an unexpected character, CR alone, NUL).
   - The parser does not report the token again.
   - The error token and the rest of its line go into an ErrorNode.
   - A declaration written later on that physical line is not a declaration: `let x = 1 ; let z = 2` and `let x = 1\rlet z = 2` declare only `x`.
5. **An open string.** The string token ends at the end of its line. The StringLit node has no StringEnd. Brackets opened before the string stay open. Nothing else is skipped.
6. **An unclosed bracket.**
   - **Where it ends:** layout stays suspended across the following lines until one of these:
     - the lexer itself meets a line at or left of the bracket's line column whose first word is `let`, `state`, `stream`, `channel`, `fn` or `pattern` (`lexer/mod.rs:1312-1335`);
     - in an expression bracket, the parser meets a line starting with a name followed by a name or `as`;
     - in a bundle, the parser meets a line with a stray `:`.
   - **Where the message goes:** to the first token of the line that ends the bracket. If an earlier line inside the bracket cannot continue it, the message goes to that line's first misfit token instead, for example `1` in `f(1\n1 2 3`.
   - **Where parsing resumes:** at the first following line, at or left of the bracket's column, that `starts_line` (`parser/starts_line.rs`) accepts: an item keyword, `(`, `[`, `for`, `when`, or a name followed by a name, `as` or a flag. In a body, a kind followed by `/` is also accepted.
   - **What the tree holds:** the lines in between are parsed as continuation of the bracket for as long as they fit the grammar. In `fn f(x:\n    x`, the body line `x` becomes the parameter's type. From the first misfit on, they go into an ErrorNode attached to the faulty construct.
   - A header that is not a resynchronization line, such as `Clip :` (no name), is skipped together with its block. Declarations inside the skipped part do not appear.
   - A resynchronization line is parsed normally, and its own fault is reported.
7. **A dedent mismatch.**
   - The lexer pops every column deeper than the line and reports the line. The line becomes an item of the enclosing level, so `  Lamp as b/2` under `Rig r:` becomes a top-level placement named `b`.
   - Following lines at the old block column form an "unexpected indent" block, which is skipped as one ErrorNode. Its declarations disappear.
   - That block is reported unless the mis-indented line itself drew a complaint, even a silenced one.
8. **An unexpected indent.** The whole indented block (to its DEDENT) is one ErrorNode at the current level. Names declared in it disappear (`let z` in `let x = 1\n    let z = 2`). It is reported unless the line above drew a parser complaint.
9. **A tab in leading whitespace.** It is reported, and then counted as one column. Layout continues from that column, which can produce rule 7 or rule 8 on the next lines.
10. **A faulty block header.**
    - If the line's last significant token is `:` and an INDENT follows, everything from the fault to the `:` goes into an ErrorNode inside the header node. The body is parsed as usual, and its declarations appear.
    - Otherwise the rest of the line goes into an ErrorNode, and the block under it is skipped silently as rule 8 with no message. Its declarations disappear.
    - A missing `:` (`Rig r`) leaves a complete header-only node. The block becomes rule 8 with a message.
11. **A missing block** (`Rig r:` followed by an unindented line). The Body or Block node holds only the `:`. The message sits at the next line and is recorded in `detached`. The next line is parsed as an item of the outer level, and its own fault is reported.
12. **A misplaced `let` in a body.** It is reported, then parsed as a LocalLet inside the Block, so the name is declared. It does not count as a body item.
13. **Faults inside a function body, `match` arms, or record fields.** These follow rule 2, inside the FnBody, MatchArm or FieldDecl. The enclosing construct resumes at the next line.

## Differences from ARCHITECTURE §7.1 (read after writing the above)

- §7.1 states "Recovery costs time and memory linear in the source". F1 is quadratic in time and total allocation on broken input. F2 is quadratic in time, also on valid input, though that case is not recovery.
- §7.1 states that every fault outside an earlier region "is reported exactly once". In the code, a dedent mismatch is reported and then the correctly indented next line draws UnexpectedIndent (F4). A tab used for indentation draws two messages, and a run of underscores draws one per underscore (F6). Missing or faulty keywords draw one message per line of the following block (F5). Only the F5 behaviour follows from §7.1's own rule that the body of a construct "parses as usual".
- §7.1 defines a fault's region as "the rest of its line". The code does not skip the rest of the line after a missing token: it builds it normally and silences it (F3). The documented region and the tree's skipped region differ.
- §7.1: "An unexpected indented block is reported unless the line it sits under drew a parser complaint." The code also suppresses it when that complaint was itself not reported, because it belonged to a lexer diagnostic, such as a line with a dedent mismatch (`previous_line_error` follows `line_error`, not the reported diagnostics).
- §7.1 does not mention a tab's column. In the code, the tab counts as one column and drives layout, which leads to a second message (F4).
- §7.1 does not describe the different classes of stray closer: `}` is a lexer error and `)` and `]` are parser errors (F8).
- §7.1 says the missing closer of an unclosed bracket is reported "at the line's first token" of the line that ends the bracket. When an earlier line inside the bracket misfits, the code reports at that line's first misfit token instead (`f(1\n1 2 3` reports at `1`). §7.1 may intend that case to be covered by "unless the bracket already holds a fault".
- Matches, as far as I tested:
  - valid source never reaches recovery;
  - a header fault that skips its `:` still parses its body;
  - an unterminated string ends at the line end and leaves brackets open;
  - the resynchronization line sets;
  - spans for faults at NEWLINE, DEDENT and end of input;
  - a missing block's message is kept beside a fault of the next line.

## Not covered

- `syntax::lex` on its own: I checked the wholeness of the tokens inside `Parsed` only, not of `Lexed.tokens`.
- Indentation nesting beyond depth 10,000, because the source grows as N^2.
- The bundle region-end rule (a stray `:` at bundle depth): only met incidentally.
- Package-file constructs (`require`, `use`, `publish`): not targeted.
- The causes in F1 and F2 are from reading. The quadratic times are measured, and the linear control runs isolate the triggering construct, but I did not instrument the scans.
- The judgement of "valid" for zero-message mutants is mine, against SPEC §1-§2 and the grammar productions. I did not check it against any oracle, and I read samples per group, not every case.
- Two-fault combinations from my generator that cancel each other (a `(` and a `)` both deleted, two lines shifted alike) remain in the property 5 counts.

## Files kept

- `packages/symphonyscript/crates/compiler/examples/probe.rs`: the probe, with subcommands `corpus`, `valid`, `mutate`, `single`, `multi`, `depth`, `time`, `gen`, `depthgen`, `show` and `showstr`.
- `packages/symphonyscript/crates/compiler/src/syntax/parsed.rs`: read-only `probe_*` accessors added. No behaviour change.
- `probe/corpus_to_bin.py`, `probe/hand_cases.py`, `probe/p8_cases.py`, `probe/triage.py`, `probe/zeros.py`, `probe/timing.sh`.
- Run outputs:
  - `probe/out-*.txt`;
  - `probe/single-cases.txt` and `probe/multi-cases.txt`: every case that did not draw exactly one message per fault;
  - `probe/timing.txt` and `probe/timing-table.md`;
  - `probe/hand/*.out` and `probe/p8/all.txt`;
  - `probe/depth-*.txt` and `probe/depthfiles/*.out`.
