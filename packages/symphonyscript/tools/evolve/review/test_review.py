import json
import stat
import tempfile
import unittest
from pathlib import Path

import corpus
import figures
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
given, taken = sys.argv[-2], sys.argv[-1]
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
        self.assertEqual(headings, ["## Moved: 5 programs in 4 classes", "## New signals: 6 programs in 2 classes",
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
        rows = [{"key": f"b{index}", "set": "breakers", "elapsed_ms": float(index + 1)} for index in range(98)]
        rows += [{"key": "slow1", "set": "breakers", "elapsed_ms": None},
                 {"key": "slow2", "set": "breakers", "elapsed_ms": None},
                 {"key": "p1", "set": "probes", "elapsed_ms": 999.0}]
        figure = figures.breaker_figure({"profile": "dev", "jobs": 4, "budget_s": 5, "load": 3.5, "programs": rows})
        self.assertEqual({name: figure[name] for name in ("programs", "past_budget", "median_ms", "p99_ms",
                                                          "max_timed_ms", "load")},
                         {"programs": 100, "past_budget": 2, "median_ms": 50.0, "p99_ms": None,
                          "max_timed_ms": 98.0, "load": 3.5})

    def test_the_measured_programs_hold_a_million_calls_and_the_fixtures_depth_per_construct(self):
        self.assertEqual(sum((1 << (level + 1)) - 1 for level in figures.sizes_totalling(1_000_000)), 1_000_000)
        expressions = figures.deep_expressions(dict.fromkeys(figures.CONSTRUCTS, 3))
        self.assertEqual(expressions, 'input x = true\nfn f(v): v\nlet a = (((1)))\nlet b = [[[1]]]\n'
                                      'let c = f(f(f(1)))\nlet d = not not not x\nlet e = "{"{"{1}"}"}"\n'
                                      'let g = ---1\nClip top:\n    Note/(((60))) when d\n'
                                      '    Note/(f(a) + c + g + b[0][0][0])\n')
        self.assertIn("let a = (1)\n", figures.deep_expressions({"call": 3}))
        self.assertEqual(len(figures.measured_programs()), 2 + len(figures.CONSTRUCTS) + 1)


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
        self.assertEqual(set(shape), {"tip", "export", "target", "evolve_binaries", "profile", "jobs", "pipeline",
                                      "host", "started", "written", "results", "commands"})

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
                                                   "/results/check-times.json.gz", "/results/figures.json"])
        self.assertEqual({each["name"]: each["below"] for each in planned if each["below"]},
                         {**dict.fromkeys(["build", "fmt", "clippy", "nextest", "doctest", "thumbv7em", "verdicts"],
                                          12.0), "figures": 6.0})
        self.assertTrue(all(each["cwd"] == "/into/export/packages/symphonyscript"
                            for each in planned if each["command"][0] in ("cargo", "rustc")))
        self.assertTrue(all(each["env"]["CARGO_TARGET_DIR"] == "/into/target" for each in planned
                            if each["command"][0] == "cargo" and each["name"] != "cargo-nextest"))


if __name__ == "__main__":
    unittest.main()
