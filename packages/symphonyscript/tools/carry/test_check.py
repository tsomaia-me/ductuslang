import io
import os
import re
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import check

BRIEF_WITH_LEDGER = "# Brief\n\n## Ledger\n\n- 1: taken\n- 2: carried, to M3\n\n## Hand-back\n"
BRIEF_WITHOUT_LEDGER = "# Brief\n\n## Scope\n"


def git(path, *args):
    subprocess.run(
        ["git", "-C", path, "-c", "user.name=t", "-c", "user.email=t@t", *args],
        check=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def init_repo(path):
    os.makedirs(path, exist_ok=True)
    git(path, "init", "-q", "-b", "main")
    with open(os.path.join(path, "file.txt"), "w") as handle:
        handle.write("x")
    git(path, "add", "file.txt")
    git(path, "commit", "-q", "-m", "one")
    out = subprocess.run(
        ["git", "-C", path, "rev-parse", "HEAD"], capture_output=True, text=True, check=True
    )
    return out.stdout.strip()


def entry(entry_id, section_state_lines):
    return "### %s. Title (A: x)\n- What: w\n- Verdict: v\n- Route: r\n%s\n" % (
        entry_id,
        "\n".join(section_state_lines),
    )


class CheckTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = self.tmp.name
        self.repo = os.path.join(self.root, "repo")
        self.sha = init_repo(self.repo)
        self.briefs = os.path.join(self.root, "briefs")
        os.makedirs(self.briefs)
        self.write_brief("m2-fix-round-1-brief.md", BRIEF_WITH_LEDGER)
        self.ledger = os.path.join(self.root, "CARRY.md")

    def write_brief(self, name, text):
        with open(os.path.join(self.briefs, name), "w") as handle:
            handle.write(text)

    def write_ledger(self, sections):
        text = "# Ledger\n\n"
        for heading, entries in sections:
            text += "## %s\n\n%s\n" % (heading, "".join(entries))
        with open(self.ledger, "w") as handle:
            handle.write(text)

    def run_check(self):
        return check.run(self.ledger, self.briefs, self.repo)

    def test_clean_ledger(self):
        self.write_ledger(
            [
                ("Open (2)", [entry("1", []), entry("2", ["- State: open, carried"])]),
                ("Done (1)", [entry("3", ["- State: done, built in `%s`." % self.sha[:9]])]),
                ("Decided not to do (1)", [entry("4", [])]),
            ]
        )
        failures, summary = self.run_check()
        self.assertEqual(failures, [])
        self.assertEqual(summary, "carry check: 4 entries, 2 open, 0 taken, 0 owner, 0 failures")

    def test_open_entry_missing_from_ledger(self):
        self.write_ledger([("Open (1)", [entry("1", []), entry("9", [])])])
        failures, _ = self.run_check()
        self.assertEqual(len(failures), 1)
        self.assertIn("entry 9", failures[0])
        self.assertIn("missing", failures[0])

    def test_taken_entry_missing_from_ledger(self):
        self.write_ledger([("Open (1)", [entry("7", ["- State: taken, by round 2."])])])
        failures, summary = self.run_check()
        self.assertEqual(len(failures), 1)
        self.assertIn("entry 7", failures[0])
        self.assertIn("1 taken", summary)

    def test_brief_without_ledger_section(self):
        self.write_brief("m2-fix-round-2-brief.md", BRIEF_WITHOUT_LEDGER)
        self.write_ledger([("Open (1)", [entry("1", [])])])
        failures, _ = self.run_check()
        self.assertEqual(len(failures), 1)
        self.assertIn("m2-fix-round-2-brief.md", failures[0])

    def test_newest_brief_is_by_round_number(self):
        self.write_brief("m2-fix-round-10-brief.md", BRIEF_WITHOUT_LEDGER)
        self.write_brief("m2-fix-round-9-brief.md", BRIEF_WITH_LEDGER)
        self.write_ledger([("Open (1)", [entry("1", [])])])
        failures, _ = self.run_check()
        self.assertIn("m2-fix-round-10-brief.md", failures[0])

    def test_newest_brief_takes_a_later_step_of_the_milestone(self):
        self.write_brief("m2e-step-1-brief.md", BRIEF_WITH_LEDGER)
        self.write_brief("m2e-step-2-brief.md", BRIEF_WITHOUT_LEDGER)
        self.write_ledger([("Open (1)", [entry("1", [])])])
        failures, _ = self.run_check()
        self.assertIn("m2e-step-2-brief.md", failures[0])

    def test_newest_brief_takes_a_later_milestone_step_over_an_earlier_fix_round(self):
        self.write_brief("m2b-fix-round-15-brief.md", BRIEF_WITH_LEDGER)
        self.write_brief("m2e-step-1-brief.md", BRIEF_WITHOUT_LEDGER)
        self.write_ledger([("Open (1)", [entry("1", [])])])
        failures, _ = self.run_check()
        self.assertIn("m2e-step-1-brief.md", failures[0])

    def test_newest_brief_takes_a_fix_round_over_the_steps_of_its_milestone(self):
        self.write_brief("m2e-step-7-brief.md", BRIEF_WITH_LEDGER)
        self.write_brief("m2e-fix-round-1-brief.md", BRIEF_WITHOUT_LEDGER)
        self.write_ledger([("Open (1)", [entry("1", [])])])
        failures, _ = self.run_check()
        self.assertIn("m2e-fix-round-1-brief.md", failures[0])

    def test_newest_brief_falls_back_to_modification_time(self):
        for name in os.listdir(self.briefs):
            os.remove(os.path.join(self.briefs, name))
        self.write_brief("a-brief.md", BRIEF_WITH_LEDGER)
        self.write_brief("b-brief.md", BRIEF_WITHOUT_LEDGER)
        os.utime(os.path.join(self.briefs, "a-brief.md"), (2000000000, 2000000000))
        os.utime(os.path.join(self.briefs, "b-brief.md"), (1000000000, 1000000000))
        self.write_ledger([("Open (1)", [entry("1", [])])])
        failures, _ = self.run_check()
        self.assertEqual(failures, [])

    def test_done_without_commit(self):
        self.write_ledger([("Done (1)", [entry("5", ["- State: done, it was built."])])])
        failures, _ = self.run_check()
        self.assertEqual(len(failures), 1)
        self.assertIn("names no commit", failures[0])

    def test_done_with_commit_not_on_main(self):
        git(self.repo, "checkout", "-q", "-b", "side")
        with open(os.path.join(self.repo, "side.txt"), "w") as handle:
            handle.write("y")
        git(self.repo, "add", "side.txt")
        git(self.repo, "commit", "-q", "-m", "side")
        out = subprocess.run(
            ["git", "-C", self.repo, "rev-parse", "HEAD"], capture_output=True, text=True
        )
        side = out.stdout.strip()
        git(self.repo, "checkout", "-q", "main")
        self.write_ledger([("Done (1)", [entry("5", ["- State: done, built in `%s`." % side[:9]])])])
        failures, _ = self.run_check()
        self.assertEqual(len(failures), 1)
        self.assertIn("not an ancestor of main", failures[0])

    def test_done_with_memory_commit_checks_the_memory_head(self):
        memory = os.path.join(self.repo, ".memory")
        memory_sha = init_repo(memory)
        self.write_ledger(
            [
                (
                    "Done (1)",
                    [entry("5", ["- State: done, the memory commit `%s`." % memory_sha[:9]])],
                )
            ]
        )
        failures, _ = self.run_check()
        self.assertEqual(failures, [])
        self.write_ledger(
            [("Done (1)", [entry("5", ["- State: done, the memory commit `%s`." % ("1" * 9)])])]
        )
        failures, _ = self.run_check()
        self.assertEqual(len(failures), 1)
        self.assertIn("memory HEAD", failures[0])

    def test_cancelled_without_ruling(self):
        self.write_ledger([("Open (1)", [entry("6", ["- State: cancelled, not wanted."])])])
        failures, _ = self.run_check()
        self.assertEqual(len(failures), 1)
        self.assertIn("names no ruling", failures[0])

    def test_cancelled_with_ruling(self):
        self.write_ledger(
            [
                (
                    "Open (2)",
                    [
                        entry("6", ['- State: cancelled, on the owner\'s "6 - cancel".']),
                        entry("8", ["- State: cancelled, by ruling M2b R4."]),
                    ],
                )
            ]
        )
        failures, _ = self.run_check()
        self.assertEqual(failures, [])

    def test_unknown_state_word(self):
        self.write_ledger([("Open (1)", [entry("1", ["- State: finished, yes."])])])
        failures, _ = self.run_check()
        self.assertEqual(len(failures), 1)
        self.assertIn("'finished'", failures[0])

    def test_state_moves_backwards(self):
        lines = [
            "- State: done, built in `%s`." % self.sha[:9],
            "- State: open, reopened.",
        ]
        self.write_ledger([("Open (1)", [entry("1", lines)])])
        failures, _ = self.run_check()
        self.assertEqual(len(failures), 1)
        self.assertIn("backwards", failures[0])

    def test_newest_state_line_overrides_the_section(self):
        self.write_ledger(
            [
                ("Done (1)", [entry("1", ["- State: owner, asked."]), entry("2", [])]),
                ("Full text of the open findings", ["### 99. Copy\n- State: open\n"]),
            ]
        )
        failures, summary = self.run_check()
        self.assertEqual(failures, [])
        self.assertEqual(summary, "carry check: 2 entries, 0 open, 0 taken, 1 owner, 0 failures")

    def test_exit_code_and_output(self):
        self.write_ledger([("Open (1)", [entry("9", [])])])
        out = io.StringIO()
        with redirect_stdout(out):
            code = check.main(
                ["--ledger", self.ledger, "--briefs", self.briefs, "--repo", self.repo]
            )
        lines = out.getvalue().splitlines()
        self.assertEqual(code, 1)
        self.assertEqual(len(lines), 2)
        self.assertTrue(lines[1].startswith("carry check: 1 entries"))
        self.write_ledger([("Open (1)", [entry("1", [])])])
        out = io.StringIO()
        with redirect_stdout(out):
            code = check.main(
                ["--ledger", self.ledger, "--briefs", self.briefs, "--repo", self.repo]
            )
        self.assertEqual(code, 0)


class RealLedgerTest(unittest.TestCase):
    def test_every_entry_gets_a_state(self):
        here = os.path.dirname(os.path.abspath(__file__))
        common = subprocess.run(
            ["git", "-C", here, "rev-parse", "--path-format=absolute", "--git-common-dir"],
            capture_output=True,
            text=True,
        )
        root = os.path.dirname(common.stdout.strip())
        ledger = os.path.join(root, ".memory", "symphonyscript", "CARRY.md")
        if not os.path.isfile(ledger):
            self.skipTest("no ledger at %s" % ledger)
        with open(ledger, encoding="utf-8") as handle:
            text = handle.read()
        body = text.split("\n## Full text", 1)[0]
        headings = len(re.findall(r"^### ", body, re.M))
        entries = check.parse_ledger(ledger)
        self.assertEqual(len(entries), headings)
        for item in entries:
            self.assertIn(
                item.state(),
                ("open", "taken", "done", "cancelled", "owner", "running", "superseded", "decided-no"),
            )
        briefs = os.path.join(root, ".memory", "symphonyscript", "work")
        failures, summary = check.run(ledger, briefs, root)
        self.assertTrue(summary.startswith("carry check: %d entries," % headings))


if __name__ == "__main__":
    unittest.main()
