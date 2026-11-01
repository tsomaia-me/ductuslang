import contextlib
import gzip
import io
import json
import stat
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import corpus
import figures
import programs
import run
import verdicts
from programs import key, read_jsonl, write_jsonl_gz

TABLE = {
    "expression-syntax": {"section": "5.1", "warning": False},
    "operand-type": {"section": "5.2", "warning": False},
    "type-mismatch": {"section": "3.2", "warning": False},
    "no-entry-instance": {"section": "7.4", "warning": True},
    "internal-error": {"section": None, "warning": False},
}
WARNINGS = {"no-entry-instance"}
COMMIT = ("-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false", "commit", "-q", "--allow-empty",
          "-m")
FAKE_CHECK = """#!/usr/bin/env python3
import json, os, sys
given, taken = sys.argv[-2], sys.argv[-1]
with open(taken, "w") as out:
    for line in open(given):
        program = json.loads(line)
        text = program["sources"][0]["text"]
        if "abort" in text:
            out.flush()
            os.abort()
        slow = "slow" in text and "alone" not in sys.argv[0]
        out.write(json.dumps({"id": program["id"], "diagnostics": [{"code": c, "file": "main.sym", "span": [0, 0],
                  "message": "", "help": None, "related": []} for c in text.split() if c != "slow" and not c.isdigit()],
                  "signals": ["slow"] if slow else [], "findings": [], "elapsed_ms": None if slow else 1.0}) + "\\n")
"""
VOCABULARY_CHECK = """#!/usr/bin/env python3
import json, sys
given, taken = sys.argv[-2], sys.argv[-1]
vocabulary = sys.argv[sys.argv.index("--vocabulary") + 1]
with open(taken, "w") as out:
    for line in open(given):
        program = json.loads(line)
        out.write(json.dumps({"id": program["id"], "diagnostics": [{"code": vocabulary, "file": "main.sym",
                              "span": [0, 0], "message": "", "help": None, "related": []}],
                              "signals": [], "findings": [], "elapsed_ms": 1.0}) + "\\n")
"""
FAKE_TIME = """#!/usr/bin/env python3
import json, sys
from pathlib import Path
given, taken = sys.argv[-2], sys.argv[-1]
programs = [json.loads(line) for line in open(given) if line.strip()]
with open(Path(sys.argv[0]).parent / "calls.log", "a") as log:
    log.write(" ".join([sys.argv[sys.argv.index("--budget") + 1], *[each["id"] for each in programs]]) + "\\n")
with open(taken, "w") as out:
    for each in programs:
        words = each["sources"][0]["text"].split()
        if "panic" in words:
            out.write(json.dumps({"id": each["id"], "elapsed_ms": None, "panic": "boom"}) + "\\n")
            continue
        codes = sorted({word[5:] for word in words if word.startswith("code:")})
        out.write(json.dumps({"id": each["id"], "elapsed_ms": 2.0, "diagnostics": len(codes), "codes": codes}) + "\\n")
"""
TAGS = "example main\nnative native\nbounded bounded\nenvelope envelope\nbar bar\nsignature signature\nto to\n"
FAKE_PROBE = f"#!/bin/sh\n[ \"$1\" = --tags ] || exit 2\ncat <<'END'\n{TAGS}END\n"


def verdict(outcome, errors=(), warnings=(), signals=(), program_set="mutants"):
    return {"outcome": outcome, "errors": list(errors), "warnings": list(warnings), "signals": list(signals),
            "set": program_set}


def drawn(code, start=0, end=0, summary="", help=None, related=()):
    return {"code": code, "file": "main.sym", "span": [start, end], "message": summary, "help": help,
            "related": [{"file": "main.sym", "span": [low, high], "label": label} for low, high, label in related]}


def kept(code, start=0, end=0, summary="", help=None, related=()):
    raw = drawn(code, start, end, summary, help, related)
    return {**{name: raw[name] for name in ("code", "file", "span", "help", "related")}, "summary": raw["message"]}


def whole(outcome, errors=(), warnings=(), diagnostics=()):
    return {**verdict(outcome, errors, warnings), "diagnostics": list(diagnostics)}


def checked(outcome, *errors):
    return ({"outcome": outcome, "errors": list(errors), "warnings": [], "signals": [],
             "diagnostics": [kept(code) for code in errors]}, None)


def program(text, program_set="mutants", **fields):
    return {"sources": [{"path": "main.sym", "text": text}], "set": program_set, **fields}


class Review(unittest.TestCase):
    def test_a_key_follows_the_vocabulary_and_the_sources_and_nothing_else(self):
        base = {"sources": [{"path": "main.sym", "text": "let a = 1\n"}]}
        for other, same in [({**base, "origin": "elsewhere", "stated": "accept"}, True),
                            ({**base, "vocabulary": "main"}, True),
                            ({**base, "vocabulary": "native"}, False),
                            ({"sources": [{"path": "other.sym", "text": "let a = 1\n"}]}, False),
                            ({"sources": [{"path": "main.sym", "text": "let a = 2\n"}]}, False)]:
            self.assertEqual(key(base) == key(other), same, other)

    def test_a_corpus_appends_each_text_once_and_drops_a_program_holding_a_secret(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "probes").mkdir()
            (root / "probes" / "a.sym").write_text("let a = 1\n")
            (root / "probes" / "b.sym").write_text("let a = 1\n")
            (root / "probes" / "key.sym").write_text("let k = \"sk-ant-0\"\n")
            (root / "probes" / "notes.txt").write_text("let n = 1\n")
            (root / "programs.jsonl").write_text(json.dumps({"id": "r1", "program": "let t = 2\n", "stated": "reject"}))
            target = root / "probes.jsonl.gz"
            first = corpus.add(target, [root / "probes"], root)
            second = corpus.add(target, [root / "programs.jsonl", root / "probes" / "a.sym"], root)
            records = read_jsonl(target)
        self.assertEqual(first, {"added": 1, "held": 1, "secret": 1, "total": 1})
        self.assertEqual(second, {"added": 1, "held": 1, "secret": 0, "total": 2})
        self.assertEqual([(r["origin"], r.get("stated"), r["key"] == key(r)) for r in records],
                         [("probes/a.sym", None, True), ("programs.jsonl#r1", "reject", True)])

    def test_a_raw_verdict_compacts_to_its_outcome(self):
        rows = [({"panic": "boom", "elapsed_ms": None, "diagnostics": []}, "panic"),
                ({"elapsed_ms": None, "diagnostics": []}, "slow"),
                ({"elapsed_ms": 1.0, "diagnostics": [drawn("no-entry-instance")]}, "accept"),
                ({"elapsed_ms": 1.0, "diagnostics": [drawn("no-entry-instance"), drawn("type-mismatch")]},
                 "reject")]
        for raw, outcome in rows:
            compacted = verdicts.compact({"signals": [], "findings": [], **raw}, WARNINGS)
            self.assertEqual(compacted["outcome"], outcome, raw)
        self.assertEqual(compacted["warnings"], ["no-entry-instance"])
        self.assertEqual(compacted["errors"], ["type-mismatch"])

    def test_a_compacted_verdict_keeps_every_diagnostic_whole_in_the_compilers_order(self):
        raw = {"elapsed_ms": 1.0, "signals": ["order"], "findings": [], "parses": True,
               "diagnostics": [drawn("type-mismatch", 9, 12, "`a` is no i32", "convert it", [(0, 3, "declared here")]),
                               drawn("no-entry-instance", 0, 0, "no entry"),
                               drawn("type-mismatch", 4, 6, "`b` is no i32", None,
                                     [(1, 2, "in this call"), (7, 8, "in this call")])]}
        compacted = verdicts.compact(raw, WARNINGS)
        self.assertEqual(compacted["diagnostics"], [
            {"code": "type-mismatch", "file": "main.sym", "span": [9, 12], "summary": "`a` is no i32",
             "help": "convert it", "related": [{"file": "main.sym", "span": [0, 3], "label": "declared here"}]},
            {"code": "no-entry-instance", "file": "main.sym", "span": [0, 0], "summary": "no entry", "help": None,
             "related": []},
            {"code": "type-mismatch", "file": "main.sym", "span": [4, 6], "summary": "`b` is no i32", "help": None,
             "related": [{"file": "main.sym", "span": [1, 2], "label": "in this call"},
                         {"file": "main.sym", "span": [7, 8], "label": "in this call"}]}])
        self.assertEqual((compacted["errors"], compacted["warnings"], compacted["signals"]),
                         (["type-mismatch"], ["no-entry-instance"], ["order"]))
        self.assertEqual(verdicts.aborted(-6)[0]["diagnostics"], [])
        self.assertEqual(verdicts.unknown_tag("weird")[0]["diagnostics"], [])
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "verdicts.jsonl.gz"
            write_jsonl_gz(path, [{"key": "k", "set": "mutants", **compacted}])
            self.assertEqual(read_jsonl(path), [{"key": "k", "set": "mutants", **compacted}])
            with gzip.open(path, "rt", encoding="utf-8") as lines:
                self.assertEqual(len(lines.read().splitlines()), 1)

    def test_equal_codes_with_a_moved_place_text_or_order_are_each_a_kind_of_their_own(self):
        tm, ot = "type-mismatch", "operand-type"
        first, second = kept(tm, 0, 3, "s", "h"), kept(tm, 10, 13, "s", "h")
        operand = kept(ot, 4, 7, "o", "h")
        pairs = {
            "span": ([first], [kept(tm, 10, 13, "s", "h")]),
            "summary": ([first], [kept(tm, 0, 3, "t", "h")]),
            "help": ([first], [kept(tm, 0, 3, "s", None)]),
            "related": ([kept(tm, 0, 3, "s", "h", [(4, 7, "first")])], [kept(tm, 0, 3, "s", "h", [(10, 13, "first")])]),
            "order": ([first, operand], [operand, first]),
            "swap": ([first, second], [second, first]),
            "count": ([first], [first, second]),
            "both": ([first, kept(ot, 8, 9, "o", "h")], [kept(ot, 8, 9, "o", "h"), kept(tm, 10, 13, "s", "h")]),
            "same": ([first, operand], [first, operand]),
        }
        expected = {"span": {"spans": (tm,)}, "summary": {"summaries": (tm,)}, "help": {"helps": (tm,)},
                    "related": {"related": (tm,)}, "order": {"order": (ot, tm)}, "swap": {"order": (tm,)},
                    "count": {"counts": ("+" + tm,)}, "both": {"spans": (tm,), "order": (ot, tm)}, "same": {}}
        self.assertEqual(verdicts.whole_change(whole("reject", [tm]), whole("reject", [tm])), {})
        for name, (old, new) in pairs.items():
            self.assertEqual(verdicts.whole_change({"diagnostics": old}, {"diagnostics": new}), expected[name], name)
        text = "let a = 1\nlet a = 2\n"
        programs = {name: program(text + name + "\n") for name in pairs}
        programs.update({"codes-only": program("x\n"), "recoded": program("y\n")})
        previous = {name: whole("reject", sorted({each["code"] for each in old}), (), old)
                    for name, (old, _) in pairs.items()}
        current = {name: whole("reject", sorted({each["code"] for each in new}), (), new)
                   for name, (_, new) in pairs.items()}
        previous.update({"codes-only": verdict("reject", [tm]), "recoded": whole("reject", [tm], (), [first])})
        current.update({"codes-only": whole("reject", [tm], (), [second]),
                        "recoded": whole("reject", [ot], (), [kept(ot, 10, 13)])})
        found = verdicts.whole_moved(previous, current)
        self.assertEqual({kind: [(codes, keys) for codes, keys in classes] for kind, classes in found.items()},
                         {"spans": [((tm,), ["both", "span"])], "summaries": [((tm,), ["summary"])],
                          "helps": [((tm,), ["help"])], "related": [((tm,), ["related"])],
                          "order": [((ot, tm), ["both", "order"]), ((tm,), ["swap"])],
                          "counts": [(("+" + tm,), ["count"])]})
        self.assertEqual(verdicts.moved(previous, current), [(("+operand-type", "-type-mismatch"), ["recoded"])])
        note = ("Diagnostics compared whole, by place, summary, help, related spans and order, only where both "
                "records hold them: 9 of the 10 programs held on both sides with equal codes; compared by their codes "
                "alone, the records carrying codes only: 1, the baseline's for 1 and this run's for 0.")
        report = verdicts.summary("Verdicts at b against a", programs, current, previous, {}, TABLE, [],
                                  "diff-full.md").splitlines()
        self.assertIn(note, report)
        self.assertEqual([line for line in report if line.startswith("## Same codes")],
                         ["## Same codes, moved spans: 2 programs in 1 classes",
                          "## Same codes, changed summaries: 1 programs in 1 classes",
                          "## Same codes, changed helps: 1 programs in 1 classes",
                          "## Same codes, changed related spans: 1 programs in 1 classes",
                          "## Same codes, changed order: 3 programs in 2 classes",
                          "## Same codes, a code drawn a different number of times: 1 programs in 1 classes"])
        for line in ["| 1 | 2 | type-mismatch | §3.2 | `both`, 25 characters |",
                     "| 1 | 2 | operand-type type-mismatch | §3.2, §5.2 | `both`, 25 characters |",
                     "| 1 | 1 | +type-mismatch | §3.2 | `count`, 26 characters |",
                     "### 1. moved spans: type-mismatch: 2 · §3.2, the 2 shortest shown",
                     "#### `span` · mutants", "- Before:", "- After:",
                     "  1. `type-mismatch` main.sym 1:1-1:4 s; help: h",
                     "  1. `type-mismatch` main.sym 2:1-2:4 s; help: h",
                     "  1. `type-mismatch` main.sym 1:1-1:4 s; help: h; related main.sym 1:5-1:8 first",
                     "  2. `type-mismatch` main.sym 1:1-1:4 s; help: h"]:
            self.assertIn(line, report)
        self.assertNotIn("- Programs: `both`, `span`", report)
        full = verdicts.render("Verdicts at b against a", programs, current, previous, {}, TABLE, []).splitlines()
        self.assertIn(note, full)
        self.assertIn("- Programs: `both`, `span`", full)
        self.assertEqual(sum(line.startswith("## Same codes") for line in full), 6)
        self.assertEqual(verdicts.position("é1\nb", 2), "1:2")
        self.assertEqual(verdicts.position("é1\nb", 4), "2:1")
        self.assertEqual(verdicts.located(program("a"), "other.sym", [3, 5]), "other.sym 3-5")
        self.assertEqual(verdicts.drawn_lines("Before", program("a"), []), ["- Before: nothing"])

    def test_a_baseline_is_recorded_only_from_a_run_that_keeps_every_diagnostic_whole(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "runs" / "abc1234").mkdir(parents=True)
            source = root / "runs" / "abc1234" / "verdicts.jsonl.gz"
            arguments = mock.Mock(revision="HEAD", baselines=root / "baselines")
            write_jsonl_gz(source, [{"key": "a", **whole("accept")}, {"key": "b", **verdict("accept")}])
            with mock.patch.object(run, "RUNS", root / "runs"), \
                    mock.patch.object(run, "short_sha", return_value="abc1234"):
                with self.assertRaises(SystemExit) as refused:
                    run.record(arguments)
                self.assertFalse((root / "baselines" / "abc1234.jsonl.gz").exists())
                write_jsonl_gz(source, [{"key": "a", **whole("accept")}, {"key": "b", **whole("reject", ["x"])}])
                run.record(arguments)
            recorded = read_jsonl(root / "baselines" / "abc1234.jsonl.gz")
        self.assertIn("1 records carry codes only", str(refused.exception.code))
        self.assertEqual([each.get("diagnostics") for each in recorded], [[], []])

    def test_the_diff_groups_moves_by_code_delta_and_lists_crashes_signals_and_stated_disagreements(self):
        programs = {
            "a1": program("let a = (1 +\n"), "a2": program("let a = (1 + 2 +\n"), "a3": program("x +\n"),
            "b1": program("let b = 1.0 + 1\n"), "c1": program("crash\n"), "s1": program("sig\n"),
            "s2": program("old sig\n"), "t1": program("t1\n", "breakers", origin="t#1", stated="accept"),
            "t2": program("t2\n", "breakers", origin="t#2", stated="reject"), "n1": program("new\n", "probes")}
        previous = {"a1": verdict("reject", ["expression-syntax"]),
                    "a2": verdict("reject", ["expression-syntax"]),
                    "a3": verdict("reject", ["expression-syntax"], ["no-entry-instance"]),
                    "b1": verdict("reject", ["type-mismatch"]), "c1": verdict("accept"),
                    "s1": verdict("accept"), "s2": verdict("accept", signals=["rename"]),
                    "t1": verdict("reject", ["type-mismatch"], program_set="breakers"),
                    "t2": verdict("reject", ["type-mismatch"], program_set="breakers"),
                    "gone": verdict("accept", program_set="probes")}
        current = {**previous,
                   "a1": verdict("reject", ["expression-syntax", "operand-type"]),
                   "a2": verdict("reject", ["expression-syntax", "operand-type"]),
                   "a3": verdict("reject", ["expression-syntax", "operand-type"], ["no-entry-instance"]),
                   "b1": verdict("reject", ["operand-type"]), "c1": verdict("reject", ["internal-error"]),
                   "s1": verdict("accept", signals=["order"]),
                   "t2": verdict("accept", program_set="breakers"), "n1": verdict("accept", program_set="probes")}
        del current["gone"]
        details = {"c1": {"panic": None, "internal": ["the checker lost a type"], "findings": []}}
        self.assertEqual(verdicts.moved(previous, current), [
            (("+operand-type",), ["a1", "a2", "a3"]),
            (("+operand-type", "-type-mismatch"), ["b1"]),
            (("+signal order",), ["s1"]),
            (("accept → reject", "+internal-error"), ["c1"]),
            (("reject → accept", "-type-mismatch"), ["t2"])])
        report = verdicts.render("Verdicts at b against a", programs, current, previous, details, TABLE, [])
        for line in ["Added since the baseline: probes 1. Removed: probes 1.",
                     "## Internal errors, panics and aborts: 1", "- Internal error: the checker lost a type",
                     "## New signals: 1", "Standing signals: rename 1.",
                     "## Moved: 7 programs in 5 classes",
                     "| 1 | 3 | +operand-type | §5.2 | `a3`, 4 characters |",
                     "| 2 | 1 | +operand-type -type-mismatch | §3.2, §5.2 | `b1`, 16 characters |",
                     "| 3 | 1 | +signal order | no section | `s1`, 4 characters |", "### 1. +operand-type: 3 · §5.2",
                     "- Shortest: `a3` · mutants",
                     "## Stated verdicts: 2 programs whose compiler verdict differs, 1 new",
                     "| `t2` | t#2 | reject | accept | new |",
                     "| `t1` | t#1 | accept | reject: type-mismatch |  |"]:
            self.assertIn(line, report.splitlines())

    def test_the_summary_leads_with_the_moved_classes_and_shows_three_examples_of_each_new_signal_class(self):
        programs = {f"r{index}": program("x" * (10 - index) + "\n") for index in range(5)}
        programs.update({"h1": program("held\n"), "m1": program("let a = (1 +\n"), "m2": program("let a = 1 +\n"),
                         "c1": program("crash\n"), "t1": program("t1\n", "breakers", origin="t#1", stated="accept"),
                         "t2": program("t2\n", "breakers", origin="t#2", stated="accept")})
        previous = {"h1": verdict("accept"), "m1": verdict("reject", ["type-mismatch"]),
                    "m2": verdict("reject", ["type-mismatch"]), "c1": verdict("accept"),
                    "t1": verdict("reject", ["type-mismatch"], program_set="breakers"),
                    "t2": verdict("accept", program_set="breakers")}
        current = {**previous, **{key: verdict("reject", ["type-mismatch"], signals=["rename"])
                                  for key in programs if key.startswith("r")},
                   "h1": verdict("accept", signals=["order"]), "m1": verdict("reject", ["operand-type"]),
                   "m2": verdict("reject", ["operand-type"]), "c1": verdict("reject", ["internal-error"]),
                   "t2": verdict("reject", ["type-mismatch"], program_set="breakers")}
        report = verdicts.summary("Verdicts at b against a", programs, current, previous, {}, TABLE, [],
                                  "diff-full.md").splitlines()
        headings = [line for line in report if line.startswith("## ")]
        self.assertEqual(headings, ["## Moved: 5 programs in 4 classes",
                                    *[f"## Same codes, {label}: 0 programs in 0 classes"
                                      for _, label in verdicts.KINDS],
                                    "## New signals: 6 programs in 2 classes",
                                    "## Internal errors, panics and aborts: 1, the 1 shortest shown",
                                    "## Stated verdicts: 2 programs whose compiler verdict differs, 1 new"])
        for line in ["Every program of every section is in `diff-full.md` beside this file.",
                     "| 1 | 2 | +operand-type -type-mismatch | §3.2, §5.2 | `m2`, 12 characters |",
                     "| 1 | 5 | rename | mutants | added since the baseline |",
                     "| 2 | 1 | order | mutants | held at the baseline |",
                     "### 1. rename · mutants · added since the baseline: 5, the 3 shortest shown",
                     "| `t2` | t#2 | accept | reject: type-mismatch | new |"]:
            self.assertIn(line, report)
        shown = [line for line in report if line.startswith("### `")]
        self.assertEqual(shown, ["### `r4` · mutants", "### `r3` · mutants", "### `r2` · mutants",
                                 "### `h1` · mutants", "### `c1` · mutants"])
        self.assertNotIn("| `t1` | t#1 | accept | reject: type-mismatch |  |", report)
        full = verdicts.render("Verdicts at b against a", programs, current, previous, {}, TABLE, []).splitlines()
        self.assertEqual(sum(line.startswith("### `r") for line in full), 5)
        self.assertIn("| `t1` | t#1 | accept | reject: type-mismatch |  |", full)

    def test_the_breaker_figure_ranks_a_program_past_the_budget_above_every_timed_one(self):
        elapsed = {f"b{index}": float(index + 1) for index in range(98)}
        elapsed.update({"slow1": None, "slow2": None})
        figure = figures.breaker_figure({"profile": "dev", "budget_s": 5}, elapsed, 3.456, 4.0)
        self.assertEqual({name: figure[name] for name in ("programs", "past_budget", "median_ms", "p99_ms",
                                                          "max_timed_ms", "load")},
                         {"programs": 100, "past_budget": 2, "median_ms": 50.0, "p99_ms": None,
                          "max_timed_ms": 98.0, "load": 3.46})

    def test_every_figure_is_taken_in_one_process_once_the_load_is_below_four_beside_its_load(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fake = root / "evolve-time"
            fake.write_text(FAKE_TIME)
            fake.chmod(fake.stat().st_mode | stat.S_IEXEC)
            breakers = [program(f"breaker {index}", "breakers") for index in range(3)]
            write_jsonl_gz(root / "breakers.jsonl.gz",[{"key": key(each), **each} for each in breakers])
            rows = [{"key": key(breakers[0]), "set": "breakers", "elapsed_ms": 9.0},
                    {"key": key(breakers[1]), "set": "breakers", "elapsed_ms": None},
                    {"key": key(breakers[2]), "set": "breakers", "elapsed_ms": 9.0},
                    {"key": "p1", "set": "probes", "elapsed_ms": 1.0}]
            with gzip.open(root / "times.json.gz", "wt", encoding="utf-8") as out:
                json.dump({"profile": "dev", "jobs": 4, "budget_s": 5, "load": 9.0, "programs": rows}, out)
            measured = [(figures.INLINED, "inlined", "inlined text", 4, ()),
                        (figures.NESTING, "not", "not text", 2, ())]
            loads = iter([3.1, 3.2, 3.3])
            with mock.patch.object(figures, "measured_programs", return_value=measured):
                figures.measure(root, root / "times.json.gz", root / "breakers.jsonl.gz", root / "figures.json", 4.0,
                                lambda: next(loads))
            taken = json.loads((root / "figures.json").read_text())
            calls = (root / "calls.log").read_text().splitlines()
        self.assertEqual(calls, [f"5 {key(breakers[0])} {key(breakers[2])}", "600 inlined#0 inlined#1 inlined#2",
                                 "600 not#0 not#1 not#2"])
        self.assertEqual(taken["load_below"], 4.0)
        self.assertEqual([(each["load"], each.get("per_unit_us")) for each in taken["figures"]],
                         [(3.1, None), (3.2, 500.0), (3.3, 1000.0)])
        self.assertEqual({name: taken["figures"][0][name] for name in ("programs", "past_budget", "median_ms")},
                         {"programs": 3, "past_budget": 1, "median_ms": 2.0})
        self.assertTrue(all("one job" in each["method"] and "below 4.0 or the figures have waited 10 minutes"
                            in each["method"] for each in taken["figures"]))

    def test_the_figures_wait_for_the_load_below_the_flag_for_at_most_ten_minutes_across_the_step(self):
        def waiting(below, loads):
            readings, slept, loads = [], [], iter(loads)
            settle = run.settling(below, figures.WAIT_SECONDS, 15,
                                  lambda: readings.append(next(loads)) or readings[-1], slept.append)
            return settle, readings, slept
        settle, readings, slept = waiting(4.0, [9.0] * 50)
        self.assertEqual((settle(), sum(slept), len(readings)), (9.0, 600, 41))
        self.assertEqual((settle(), sum(slept), len(readings)), (9.0, 600, 42))
        settle, readings, slept = waiting(float("inf"), [9.0])
        self.assertEqual((settle(), slept, readings), (9.0, [], [9.0]))
        settle, readings, slept = waiting(4.0, [6.0, 5.0, 3.5, 9.0])
        self.assertEqual((settle(), slept, readings), (3.5, [15, 15], [6.0, 5.0, 3.5]))
        self.assertEqual(figures.waited(float("inf")), "at any load")

    def test_the_measured_programs_hold_a_million_calls_and_the_fixtures_depth_per_construct(self):
        self.assertEqual(sum((1 << (level + 1)) - 1 for level in figures.sizes_totalling(1_000_000)), 1_000_000)
        inlined = dict((label, text) for _, label, text, _, _ in figures.measured_programs())["inlined"]
        self.assertEqual(inlined.count("f(i, "), 1_000_000)
        self.assertEqual({line.count("f(i, ") for line in inlined.splitlines()[3:]}, {100})
        self.assertEqual(figures.inlining(200).splitlines()[:4],
                         ["state i(1)", "fn f(x, k): x + k", "Clip c:",
                          "    Note/(" + " + ".join(f"f(i, {index})" for index in range(100)) + ")"])
        self.assertEqual(len(figures.inlining(200).splitlines()), 5)
        expressions = figures.deep_expressions(dict.fromkeys(figures.CONSTRUCTS, 3))
        self.assertEqual(expressions, 'state x(true)\nfn f(v): v\nlet a = (((1)))\nlet b = [[[1]]]\n'
                                      'let c = f(f(f(1)))\nlet d = not not not x\nlet e = "{"{"{1}"}"}"\n'
                                      'let g = ---1\nClip top:\n    Note/(((60))) when d\n'
                                      '    Note/(f(a) + c + g + b[0][0][0])\n')
        self.assertIn("let a = (1)\n", figures.deep_expressions({"call": 3}))
        self.assertEqual(len(figures.measured_programs()), 2 + len(figures.CONSTRUCTS) + 1)

    def test_every_figure_program_states_the_codes_it_must_draw_at_each_size(self):
        full, quick = figures.measured_programs(figures.FULL), figures.measured_programs(figures.QUICK)
        self.assertEqual([(label, units, expected) for _, label, _, units, expected in full],
                         [("recursive", 1_000_000, ("limit-warning", "no-entry-instance")),
                          ("inlined", 1_000_000, ("limit-warning", "no-entry-instance")),
                          *[(construct, 100_000, ("no-entry-instance",)) for construct in figures.CONSTRUCTS],
                          ("every construct", 700_000, ("no-entry-instance",))])
        self.assertEqual([(label, units, expected) for _, label, _, units, expected in quick],
                         [("recursive", 100_000, ("no-entry-instance",)), ("inlined", 100_000, ("no-entry-instance",)),
                          *[(construct, 10_000, ("no-entry-instance",)) for construct in figures.CONSTRUCTS],
                          ("every construct", 70_000, ("no-entry-instance",))])
        texts = {label: text for _, label, text, _, _ in quick}
        self.assertEqual(texts["inlined"].count("f(i, "), 100_000)
        self.assertEqual(texts["recursive"], figures.recursing(100_000))
        self.assertEqual(sum((1 << (level + 1)) - 1 for level in figures.sizes_totalling(100_000)), 100_000)
        self.assertIn("let a = " + "(" * 10_000 + "1" + ")" * 10_000 + "\n", texts["parentheses"])
        self.assertIn("let d = " + "not " * 10_000 + "x\n", texts["not"])
        self.assertEqual([figure for figure, *_ in quick][:3],
                         [("A compile-time recursive call", "a program making 100,000 distinct calls"),
                          ("An inlined call", "a program inlining 100,000 calls"),
                          ("A level of nesting", "the deep-expressions fixture at depth 10,000, each construct")])
        self.assertEqual(full[0][0], ("A compile-time recursive call", "a program making 1,000,000 distinct calls"))

    def test_a_figure_whose_program_draws_another_code_or_misses_one_is_void_and_printed_so(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fake = root / "evolve-time"
            fake.write_text(FAKE_TIME)
            fake.chmod(fake.stat().st_mode | stat.S_IEXEC)
            breaker = program("breaker", "breakers")
            write_jsonl_gz(root / "breakers.jsonl.gz", [{"key": key(breaker), **breaker}])
            with gzip.open(root / "times.json.gz", "wt", encoding="utf-8") as out:
                json.dump({"profile": "dev", "jobs": 4, "budget_s": 5, "load": 1.0,
                           "programs": [{"key": key(breaker), "set": "breakers", "elapsed_ms": 1.0}]}, out)
            both, entry = ("limit-warning", "no-entry-instance"), ("no-entry-instance",)
            measured = [(figures.INLINED, "inlined", "code:limit-warning code:no-entry-instance", 4, both),
                        (figures.NESTING, "not", "code:no-entry-instance code:type-mismatch", 2, entry),
                        (figures.NESTING, "call", "code:limit-warning", 2, both),
                        (figures.NESTING, "negation", "panic", 2, entry)]
            printed = io.StringIO()
            with mock.patch.object(figures, "measured_programs", return_value=measured), \
                    contextlib.redirect_stdout(printed):
                void = figures.measure(root, root / "times.json.gz", root / "breakers.jsonl.gz", root / "figures.json",
                                       4.0, lambda: 1.0)
            taken = json.loads((root / "figures.json").read_text())
        voided = ["not", "call", "negation"]
        self.assertEqual((void, taken["void"], taken["scale"]), (voided, voided, "full"))
        self.assertNotIn("void", taken["figures"][0])
        self.assertEqual([(each["program"], each["void"], each["codes"], each["expected_codes"])
                          for each in taken["figures"][1:]],
                         [("inlined", False, list(both), list(both)),
                          ("not", True, ["no-entry-instance", "type-mismatch"], ["no-entry-instance"]),
                          ("call", True, ["limit-warning"], list(both)),
                          ("negation", True, [], ["no-entry-instance"])])
        lines = printed.getvalue().splitlines()
        for line in ["An inlined call, inlined: 500.000 us per unit, load 1.0",
                     "A level of nesting, not: 1000.000 us per unit, load 1.0; VOID: drew no-entry-instance, "
                     "type-mismatch, must draw no-entry-instance",
                     "A level of nesting, call: 1000.000 us per unit, load 1.0; VOID: drew limit-warning, must draw "
                     "limit-warning, no-entry-instance",
                     "A level of nesting, negation: past the budget per unit, load 1.0; VOID: drew nothing, panicked: "
                     "boom, must draw no-entry-instance",
                     "void figures: 3 (not, call, negation); exit 3"]:
            self.assertIn(line, lines)

    def test_the_quick_figures_take_the_slowest_breakers_three_times_each_into_their_own_file(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fake = root / "evolve-time"
            fake.write_text(FAKE_TIME)
            fake.chmod(fake.stat().st_mode | stat.S_IEXEC)
            breakers = [program(f"breaker {index}", "breakers") for index in range(4)]
            write_jsonl_gz(root / "breakers.jsonl.gz", [{"key": key(each), **each} for each in breakers])
            rows = [{"key": key(each), "set": "breakers", "elapsed_ms": ms}
                    for each, ms in zip(breakers, (9.0, None, 7.0, 3.0))]
            rows.append({"key": "p1", "set": "probes", "elapsed_ms": 50.0})
            with gzip.open(root / "times.json.gz", "wt", encoding="utf-8") as out:
                json.dump({"profile": "dev", "jobs": 4, "budget_s": 5, "load": 1.0, "programs": rows}, out)
            scale = {**figures.QUICK, "breakers": 2}
            measured = [(figures.stated(figures.NESTING, scale), "not", "code:no-entry-instance", 10_000,
                         ("no-entry-instance",))]
            with mock.patch.object(figures, "measured_programs", return_value=measured), \
                    contextlib.redirect_stdout(io.StringIO()):
                void = figures.measure(root, root / "times.json.gz", root / "breakers.jsonl.gz",
                                       root / figures.QUICK_NAME, float("inf"), lambda: 2.0, scale)
            taken = json.loads((root / figures.QUICK_NAME).read_text())
            calls = (root / "calls.log").read_text().splitlines()
        first, third = key(breakers[0]), key(breakers[2])
        self.assertEqual(void, [])
        self.assertEqual(calls, [f"5 {first}#0 {first}#1 {first}#2 {third}#0 {third}#1 {third}#2",
                                 "600 not#0 not#1 not#2"])
        self.assertEqual((taken["scale"], taken["load_below"], taken["void"]), ("quick", None, []))
        breaker_figure = taken["figures"][0]
        self.assertEqual({name: breaker_figure[name]
                          for name in ("measured_by", "programs", "past_budget", "median_ms")},
                         {"measured_by": "Check over the 2 slowest programs, median and 99th percentile of each one's "
                                         "median", "programs": 2, "past_budget": 0, "median_ms": 2.0})
        self.assertIn("3 runs each in one process, one job", breaker_figure["method"])
        self.assertEqual((taken["figures"][1]["measured_by"], taken["figures"][1]["per_unit_us"]),
                         ("the deep-expressions fixture at depth 10,000, each construct", 0.2))
        self.assertEqual(figures.slowest_breakers({"programs": rows}, 3), [rows[0], rows[2], rows[3]])

    def test_the_figures_command_writes_the_quick_figures_beside_the_full_ones_and_exits_three_on_a_void_figure(self):
        arguments = mock.Mock(binaries=Path("/b"), times=Path("/r/check-times.json.gz"), out=Path("/r/figures.json"),
                              load_below=4.0, quick=True)
        with mock.patch.object(figures, "measure", return_value=["not"]) as measure, \
                contextlib.redirect_stdout(io.StringIO()):
            with self.assertRaises(SystemExit) as stopped:
                run.measured(arguments)
            self.assertEqual(stopped.exception.code, 3)
            self.assertEqual((measure.call_args.args[3], measure.call_args.args[6]),
                             (Path("/r/figures-quick.json"), figures.QUICK))
            arguments.quick = False
            measure.return_value = []
            run.measured(arguments)
            self.assertEqual((measure.call_args.args[3], measure.call_args.args[6]),
                             (Path("/r/figures.json"), figures.FULL))
        figures_record = {"name": "figures", "exit": 0}
        rows = [([{"name": "build", "exit": 0}, figures_record], 0),
                ([{"name": "build", "exit": 0}, {"name": "figures", "exit": 3}], 2),
                ([{"name": "build", "exit": 101}, {"name": "figures", "exit": 3}], 3),
                ([{"name": "build", "exit": 0}, {"name": "figures", "exit": 1}], 1),
                ([{"name": "verdicts", "exit": 3}, figures_record], 1)]
        for records, code in rows:
            self.assertEqual(run.shared_exit(records), code, records)
        self.assertTrue(run.voided({"name": "figures", "exit": 3}))


class Running(unittest.TestCase):
    def test_a_program_that_kills_its_shard_costs_only_itself_and_a_slow_one_is_checked_again_alone(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("shards", "alone"):
                (root / name).mkdir()
                fake = root / name / "evolve-check"
                fake.write_text(FAKE_CHECK)
                fake.chmod(fake.stat().st_mode | stat.S_IEXEC)
            texts = ["type-mismatch", "abort", "slow", "operand-type", "slow type-mismatch"] * 3
            programs = {key(program(f"{text} {index}")): program(f"{text} {index}") for index, text in
                        enumerate(texts)}
            found = run.checked(root / "shards", programs, WARNINGS, list(programs), 2, root / "w1", False)
            again = run.checked_again(found, {}, list(programs))
            found.update(run.checked(root / "alone", programs, WARNINGS, again, 2, root / "w2", True))
        expected = {"type-mismatch": checked("reject", "type-mismatch"), "abort": verdicts.aborted(-6),
                    "slow": checked("accept"), "operand-type": checked("reject", "operand-type"),
                    "slow type-mismatch": checked("reject", "type-mismatch")}
        for program_key, entry in programs.items():
            text = entry["sources"][0]["text"].rsplit(" ", 1)[0]
            self.assertEqual(found[program_key][:2], expected[text], text)
            self.assertEqual(found[program_key][2], None if text == "abort" else 1.0, text)

    def test_each_probe_program_is_checked_under_the_vocabulary_its_tag_names_and_an_unknown_tag_is_reported(self):
        tagged = ("#signature\nClip c:\n    Note\n#expect\n----\n#weird\nlet a = 1\n----\n\nlet b = 2\n"
                  "#expect no-entry-instance\n----\n#bounded\nlet e = 5\n")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "corpora").mkdir()
            write_jsonl_gz(root / "corpora" / "probes.jsonl.gz",
                           [{"origin": "p/a.sym", "sources": [{"path": "main.sym", "text": tagged}]},
                            {"origin": "p/b.sym", "sources": [{"path": "main.sym", "text": "\nlet c = 3"}]}])
            write_jsonl_gz(root / "corpora" / "breakers.jsonl.gz",
                           [{"origin": "b.sym", "sources": [{"path": "main.sym", "text": "# Vocabulary: music\n"}]}])
            for name, text in (("evolve-mutants", "#!/bin/sh\n"), ("evolve-check", VOCABULARY_CHECK),
                               ("evolve-probe", FAKE_PROBE)):
                (root / name).write_text(text)
                (root / name).chmod((root / name).stat().st_mode | stat.S_IEXEC)
            programs = run.programs_at(root, root / "corpora")
            unknown = run.unknown_tags(programs)
            named = [program_key for program_key in programs if program_key not in unknown]
            found = run.checked(root, programs, WARNINGS, named, 2, root / "work", False)
        read = {each["origin"]: (each.get("vocabulary"), each.get("tag"), each["sources"][0]["text"])
                for each in programs.values()}
        self.assertEqual(read, {"p/a.sym:2": ("signature", None, "Clip c:\n    Note\n"),
                                "p/a.sym:7": (None, "weird", "let a = 1\n"),
                                "p/a.sym:10": ("main", None, "let b = 2\n"),
                                "p/a.sym:14": ("bounded", None, "let e = 5\n"),
                                "p/b.sym": (None, None, "\nlet c = 3"),
                                "b.sym": (None, None, "# Vocabulary: music\n")})
        by_origin = {programs[program_key]["origin"]: program_key for program_key in programs}
        self.assertEqual({origin: found[program_key][0]["errors"] for origin, program_key in by_origin.items()
                          if program_key in found},
                         {"p/a.sym:2": ["signature"], "p/a.sym:10": ["main"], "p/a.sym:14": ["bounded"],
                          "p/b.sym": ["main"], "b.sym": ["main"]})
        self.assertEqual(unknown, {by_origin["p/a.sym:7"]: (*verdicts.unknown_tag("weird"), None)})
        self.assertEqual(unknown[by_origin["p/a.sym:7"]][0]["outcome"], "unknown-tag")
        self.assertEqual(run.unknown_tag_note(programs, unknown),
                         "Probes whose tag names no vocabulary, reported and not checked: 1; p/a.sym:7 (#weird).")
        self.assertIn("- Tag: #weird", verdicts.program_lines(programs[by_origin["p/a.sym:7"]], {"tag": "weird"}))

    def test_the_probe_tag_table_is_read_from_the_tags_evolve_probe_prints(self):
        self.assertEqual(programs.tag_table(TAGS),
                         {"example": "main", "native": "native", "bounded": "bounded", "envelope": "envelope",
                          "bar": "bar", "signature": "signature", "to": "to"})
        with tempfile.TemporaryDirectory() as directory:
            fake = Path(directory) / "evolve-probe"
            fake.write_text(FAKE_PROBE)
            fake.chmod(fake.stat().st_mode | stat.S_IEXEC)
            self.assertEqual(programs.probe_vocabularies(Path(directory)), programs.tag_table(TAGS))

    def test_the_baseline_is_the_nearest_ancestor_with_one(self):
        with tempfile.TemporaryDirectory() as directory:
            repository, baselines = Path(directory) / "repository", Path(directory) / "baselines"
            baselines.mkdir()
            run.git(directory, "init", "-q", str(repository))
            shas = {}
            for name in ("first", "second", "third"):
                run.git(repository, *COMMIT, name)
                shas[name] = run.short_sha(repository, "HEAD")
            run.git(repository, "checkout", "-q", "-b", "side", shas["first"])
            run.git(repository, *COMMIT, "side")
            shas["side"] = run.short_sha(repository, "HEAD")
            for name in ("first", "second", "side", "third"):
                (baselines / f"{shas[name]}.jsonl.gz").write_bytes(b"")
            chosen = {name: run.previous_baseline(repository, shas[name], baselines) for name in shas}
        self.assertEqual(chosen, {"first": None, "second": baselines / f"{shas['first']}.jsonl.gz",
                                  "third": baselines / f"{shas['second']}.jsonl.gz",
                                  "side": baselines / f"{shas['first']}.jsonl.gz"})

    def test_the_shared_step_runs_each_command_alone_and_the_manifest_names_its_exit_load_time_and_log(self):
        with tempfile.TemporaryDirectory() as directory:
            results = Path(directory)
            waited = []
            planned = [run.step("first", ["sh", "-c", "echo one"], results),
                       run.step("failing", ["sh", "-c", "echo two; exit 3"], results, {"SEEN": "yes"}, 12.0),
                       run.step("missing", [str(results / "absent")], results, None, 6.0),
                       run.step("after", ["sh", "-c", "echo $SEEN"], results)]
            records = [run.performed(each, results, lambda below: waited.append(below) or 2.5) for each in planned]
            logs = {record["log"]: (results / record["log"]).read_text() for record in records}
            shape = run.manifest("f" * 40, results / "into", results, records, "2026-10-28T00:00:00+00:00", 4)
            asked = run.manifest("f" * 40, results / "into", results, records, "2026-10-28T00:00:00+00:00", 4, 6.0,
                                 True, True)
            unbounded = run.manifest("f" * 40, results / "into", results, records, "2026-10-28T00:00:00+00:00", 4,
                                     float("inf"))
        self.assertEqual(waited, [None, 12.0, 6.0, None])
        self.assertEqual([(r["name"], r["exit"], r["load"], r["log"]) for r in records],
                         [("first", 0, 2.5, "first.log"), ("failing", 3, 2.5, "failing.log"),
                          ("missing", 127, 2.5, "missing.log"), ("after", 0, 2.5, "after.log")])
        self.assertEqual((logs["first.log"], logs["failing.log"], logs["after.log"]), ("one\n", "two\n", "\n"))
        self.assertTrue(all(isinstance(r["seconds"], float) and r["started"].endswith("+00:00") for r in records))
        self.assertEqual(records[1]["env"], {"SEEN": "yes"})
        self.assertEqual(shape["tip"], "f" * 40)
        self.assertEqual(shape["results"], ["after.log", "failing.log", "first.log", "missing.log"])
        self.assertEqual(shape["commands"], records)
        self.assertEqual(set(shape), {"tip", "export", "target", "evolve_binaries", "profile", "jobs", "ignored_tests",
                                      "load_limits", "pipeline", "host", "started", "written", "results", "commands",
                                      "test_times"})
        self.assertEqual(shape["test_times"], {})
        self.assertEqual((shape["ignored_tests"], asked["ignored_tests"]),
                         ({"exhaustive": False, "stress": False}, {"exhaustive": True, "stress": True}))
        self.assertEqual((shape["load_limits"], asked["load_limits"], unbounded["load_limits"]),
                         ({"builds_and_checks": 12.0, "timed_steps": 6.0, "figures": 4.0},
                          {"builds_and_checks": 12.0, "timed_steps": 6.0, "figures": 6.0},
                          {"builds_and_checks": 12.0, "timed_steps": None, "figures": None}))

    def test_the_manifest_lists_every_tests_time_and_status_slowest_first_for_the_unit_and_exhaustive_runs(self):
        unit = ("   Compiling symphonyscript v0.1.0\n"
                "    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.00s\n"
                "    Starting 4 tests across 2 binaries (1 test skipped)\n"
                "        PASS [   0.004s] symphonyscript-core types::small\n"
                "        FAIL [   1.250s] symphonyscript typing::broken\n"
                "--- STDOUT:              symphonyscript typing::broken ---\n"
                "        PASS [   0.500s] printed\n"
                "        SLOW [> 60.000s] symphonyscript typing::slow\n"
                "        SKIP [         ] symphonyscript typing::exhaustive_forms\n"
                "        PASS [  62.500s] (  3/4) symphonyscript typing::slow\n"
                "------------\n"
                "     Summary [  63.800s] 3 tests run: 2 passed, 1 failed, 1 skipped\n"
                "        FAIL [   1.250s] symphonyscript typing::broken\n"
                "error: test run failed\n")
        exhaustive = "        PASS [  12.000s] symphonyscript typing::exhaustive_forms\n"
        self.assertEqual(run.test_times(unit),
                         [{"test": "symphonyscript typing::slow", "seconds": 62.5, "status": "pass"},
                          {"test": "symphonyscript typing::broken", "seconds": 1.25, "status": "fail"},
                          {"test": "symphonyscript-core types::small", "seconds": 0.004, "status": "pass"},
                          {"test": "symphonyscript typing::exhaustive_forms", "seconds": None, "status": "skip"}])
        with tempfile.TemporaryDirectory() as directory:
            results = Path(directory)
            records = []
            for name, log in (("nextest", unit), ("nextest-exhaustive", exhaustive), ("nextest-stress", exhaustive)):
                (results / f"{name}.log").write_text(log)
                records.append({"name": name, "log": f"{name}.log"})
            shape = run.manifest("f" * 40, results / "into", results, records, "2026-10-28T00:00:00+00:00", 4)
            unit_only = run.manifest("f" * 40, results / "into", results, records[:1], "2026-10-28T00:00:00+00:00", 4)
        forms = {"test": "symphonyscript typing::exhaustive_forms", "seconds": 12.0, "status": "pass"}
        self.assertEqual(shape["test_times"], {"nextest": run.test_times(unit), "nextest-exhaustive": [forms]})
        self.assertEqual(list(unit_only["test_times"]), ["nextest"])

    def test_the_shared_step_runs_the_four_checks_as_commands_of_their_own_in_the_dev_profile(self):
        into, results = Path("/into"), Path("/results")
        planned = run.shared_steps("abc1234", Path("/top"), into, results, 4, Path("/baselines"), None)
        commands = {each["name"]: each["command"] for each in planned}
        self.assertEqual(list(commands), ["archive", "extract", "rustc", "cargo-nextest", "build", "fmt", "clippy",
                                          "nextest", "doctest", "thumbv7em", "verdicts", "figures"])
        self.assertEqual(commands["archive"], ["git", "-C", "/top", "archive", "--output", "/into/export.tar",
                                               "abc1234", "packages/symphonyscript"])
        self.assertEqual(commands["build"], ["cargo", "build", "--workspace", "--all-targets"])
        self.assertEqual([commands[name] for name in ("fmt", "clippy", "nextest", "doctest", "thumbv7em")],
                         [["cargo", "fmt", "--check"],
                          ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
                          ["cargo", "nextest", "run", "--workspace"], ["cargo", "test", "--workspace", "--doc"],
                          ["cargo", "build", "-p", "symphonyscript-core", "--target", "thumbv7em-none-eabihf"]])
        self.assertEqual(commands["verdicts"][2:], ["check", "abc1234", "--profile", "dev", "--jobs", "4",
                                                    "--settle-below", "6.0", "--build", "/into/check", "--baselines",
                                                    "/baselines"])
        self.assertEqual(commands["figures"][2:], ["figures", "/into/check/target/debug",
                                                   "/results/check-times.json.gz", "/results/figures.json",
                                                   "--load-below", "4.0"])
        self.assertEqual({each["name"]: each["below"] for each in planned if each["below"]},
                         dict.fromkeys(["build", "fmt", "clippy", "nextest", "doctest", "thumbv7em", "verdicts"], 12.0))
        quiet = run.shared_steps("abc1234", Path("/top"), into, results, 4, Path("/baselines"), None, float("inf"),
                                 True, True)
        self.assertEqual([quiet[-2]["command"][9], quiet[-1]["name"], quiet[-1]["command"][-2:], quiet[-1]["below"]],
                         ["inf", "figures", ["--load-below", "inf"], None])
        given = run.shared_steps("abc1234", Path("/top"), into, results, 4, Path("/baselines"), None, 5.0)
        self.assertEqual([given[-2]["command"][9], given[-1]["command"][-1]], ["5.0", "5.0"])
        self.assertTrue(all(each["cwd"] == "/into/export/packages/symphonyscript"
                            for each in planned if each["command"][0] in ("cargo", "rustc")))
        self.assertTrue(all(each["env"]["CARGO_TARGET_DIR"] == "/into/target" for each in planned
                            if each["command"][0] == "cargo" and each["name"] != "cargo-nextest"))

    def test_the_shared_step_runs_the_exhaustive_tests_only_when_asked(self):
        into, results = Path("/into"), Path("/results")
        planned = run.shared_steps("abc1234", Path("/top"), into, results, 4, Path("/baselines"), None, 6.0, False,
                                   True)
        names = [each["name"] for each in planned]
        self.assertEqual(names[names.index("nextest"):names.index("doctest")], ["nextest", "nextest-exhaustive"])
        exhaustive = planned[names.index("nextest-exhaustive")]
        self.assertEqual(exhaustive["command"], ["cargo", "nextest", "run", "--workspace", "--run-ignored", "all",
                                                 "-E", "test(/(^|::)exhaustive_/)"])
        self.assertEqual((exhaustive["below"], exhaustive["cwd"], exhaustive["env"]["NEXTEST_TEST_THREADS"]),
                         (12.0, "/into/export/packages/symphonyscript", "4"))
        for default in (run.shared_steps("abc1234", Path("/top"), into, results, 4, Path("/baselines"), None),
                        run.shared_steps("abc1234", Path("/top"), into, results, 4, Path("/baselines"), None, 6.0,
                                         True)):
            self.assertNotIn("nextest-exhaustive", [each["name"] for each in default])

    def test_the_shared_step_runs_the_stress_tests_after_the_exhaustive_ones_on_a_quiet_host_when_asked(self):
        into, results = Path("/into"), Path("/results")
        planned = run.shared_steps("abc1234", Path("/top"), into, results, 4, Path("/baselines"), None, 5.0, True,
                                   True)
        names = [each["name"] for each in planned]
        self.assertEqual(names[names.index("nextest"):names.index("doctest")],
                         ["nextest", "nextest-exhaustive", "nextest-stress"])
        stress = planned[names.index("nextest-stress")]
        self.assertEqual(stress["command"], ["cargo", "nextest", "run", "--workspace", "--run-ignored", "all", "-E",
                                             "test(/(^|::)stress_/)"])
        self.assertEqual((stress["below"], stress["cwd"], stress["env"]["NEXTEST_TEST_THREADS"]),
                         (5.0, "/into/export/packages/symphonyscript", "4"))
        default = run.shared_steps("abc1234", Path("/top"), into, results, 4, Path("/baselines"), None)
        self.assertNotIn("nextest-stress", [each["name"] for each in default])


if __name__ == "__main__":
    unittest.main()
