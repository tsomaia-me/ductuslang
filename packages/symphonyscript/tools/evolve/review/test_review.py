import json
import stat
import tempfile
import unittest
from pathlib import Path

import corpus
import run
import verdicts
from programs import key, read_jsonl

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
given, taken = sys.argv[3], sys.argv[4]
with open(taken, "w") as out:
    for line in open(given):
        program = json.loads(line)
        text = program["sources"][0]["text"]
        if "abort" in text:
            out.flush()
            os.abort()
        slow = "slow" in text and "alone" not in sys.argv[0]
        out.write(json.dumps({"id": program["id"], "diagnostics": [{"code": c, "message": ""} for c in text.split()
                  if c != "slow" and not c.isdigit()], "signals": ["slow"] if slow else [], "findings": [],
                  "elapsed_ms": None if slow else 1.0}) + "\\n")
"""


def verdict(outcome, errors=(), warnings=(), signals=(), program_set="mutants"):
    return {"outcome": outcome, "errors": list(errors), "warnings": list(warnings), "signals": list(signals),
            "set": program_set}


def checked(outcome, *errors):
    return {"outcome": outcome, "errors": list(errors), "warnings": [], "signals": []}, None


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
                ({"elapsed_ms": 1.0, "diagnostics": [{"code": "no-entry-instance"}]}, "accept"),
                ({"elapsed_ms": 1.0, "diagnostics": [{"code": "no-entry-instance"}, {"code": "type-mismatch"}]},
                 "reject")]
        for raw, outcome in rows:
            compacted = verdicts.compact({"signals": [], "findings": [], **raw}, WARNINGS)
            self.assertEqual(compacted["outcome"], outcome, raw)
        self.assertEqual(compacted["warnings"], ["no-entry-instance"])
        self.assertEqual(compacted["errors"], ["type-mismatch"])

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
                     "| 1 | 3 | +operand-type | §5.2 |", "| 2 | 1 | +operand-type -type-mismatch | §3.2, §5.2 |",
                     "| 3 | 1 | +signal order | no section |", "### 1. +operand-type: 3 · §5.2",
                     "- Shortest: `a3` · mutants",
                     "## Stated verdicts: 2 programs whose compiler verdict differs, 1 new",
                     "| `t2` | t#2 | reject | accept | new |",
                     "| `t1` | t#1 | accept | reject: type-mismatch |  |"]:
            self.assertIn(line, report.splitlines())


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
            self.assertEqual(found[program_key], expected[text], text)

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


if __name__ == "__main__":
    unittest.main()
