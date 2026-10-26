import json
import random
import subprocess
import tempfile
import threading
import unittest
from pathlib import Path

import targeted
from oracle import Oracle
from proxy_llm import ProxyClient, generation_system
from targeted import Loop, allocate
from test_fuzz4all import EVOLVE_BIN, HERE, RUNNER, RUNNER_PYTHON, free_port

SPEC = (HERE.parents[2] / "docs" / "SPEC.md").read_text()
TARGETS = [{"name": "alpha", "sections": ["6.6"]}, {"name": "beta", "sections": ["3.7", "5.5"]}]
BACKEND = (
    "import json, sys, threading, fake_backend\n"
    "out, rules = open(sys.argv[1], 'a'), json.load(open(sys.argv[2]))\n"
    "lock = threading.Lock()\n"
    "def respond(params):\n"
    "    with lock:\n"
    "        out.write(json.dumps(params) + '\\n')\n"
    "        out.flush()\n"
    "    content = params['messages'][0]['content']\n"
    "    for rule in rules:\n"
    "        if all(needle in content for needle in rule['all']):\n"
    "            return rule['reply']\n"
    "    return '{}'\n"
    "started = fake_backend.start(sys.argv[3], respond, port=int(sys.argv[4]), batch_requests=8, batch_seconds=0.2)\n"
    "print(started.url, flush=True)\n"
    "threading.Event().wait()\n")


def requirements(name):
    return json.dumps({"requirements": [
        {"edge": f"{name} one", "verdict": "reject unknown-attribute", "rule": "6.6: one"},
        {"edge": f"{name} two", "verdict": "accept", "rule": "6.6: two"}]})


def program(text, verdict):
    return json.dumps({"program": text, "verdict": verdict, "rule": "7.2: a placement is named"})


def judgement(verdict):
    return json.dumps({"verdict": verdict, "rule": "7.2: a placement is named"})


RULES = [
    {"all": ["Judge the SymphonyScript program", "Clip b", "argue for the verdict opposite"],
     "reply": judgement("accept")},
    {"all": ["Judge the SymphonyScript program"], "reply": judgement("reject")},
    {"all": ["Target: alpha"], "reply": requirements("alpha")},
    {"all": ["Target: beta"], "reply": requirements("beta")},
    {"all": ["Edge: alpha one"], "reply": program("Clip a:\n    Note/60\n", "reject")},
    {"all": ["Edge: alpha two"], "reply": program("Clip b:\n    Note/62\n", "reject")},
    {"all": ["Edge: beta one"], "reply": program("Clp c:\n", "reject")},
    {"all": ["Edge: beta two"], "reply": program("Clip d:\n    Note/64\n", "accept")},
]


def reply_to(params):
    content = params["messages"][0]["content"]
    for rule in RULES:
        if all(needle in content for needle in rule["all"]):
            return rule["reply"]
    return "{}"


class Scripted:
    def __init__(self):
        self.sent = []
        self.lock = threading.Lock()

    def create(self, params):
        with self.lock:
            self.sent.append(params)
        return {"content": [{"type": "text", "text": reply_to(params)}]}

    def where(self, model=None, starts=None):
        return [params for params in self.sent
                if (model is None or params["model"] == model)
                and (starts is None or params["messages"][0]["content"].startswith(starts))]


def user_of(params):
    return params["messages"][0]["content"]


class AllocationTest(unittest.TestCase):
    def requirements(self, stats):
        return [{"programs": programs, "confirmed": confirmed, "disagreements": disagreements}
                for programs, confirmed, disagreements in stats]

    def test_the_first_round_gives_every_requirement_the_same_count(self):
        self.assertEqual(allocate(self.requirements([(0, 0, 0)] * 3), 4, random.Random(0)), [4, 4, 4])

    def test_a_yielding_requirement_gets_more_and_none_gets_fewer_than_one(self):
        stats = self.requirements([(10, 8, 0), (10, 0, 0), (10, 0, 0), (10, 0, 0)])
        rng = random.Random(0)
        totals = [0] * 4
        for _ in range(300):
            counts = allocate(stats, 4, rng)
            self.assertEqual(sum(counts), 16)
            self.assertGreaterEqual(min(counts), 1)
            totals = [total + count for total, count in zip(totals, counts)]
        self.assertGreater(totals[0], 2 * max(totals[1:]))

    def test_half_a_disagreement_counts_toward_the_yield(self):
        stats = self.requirements([(10, 0, 8), (10, 0, 0), (10, 0, 0)])
        rng = random.Random(0)
        totals = [0] * 3
        for _ in range(300):
            totals = [total + count for total, count in zip(totals, allocate(stats, 4, rng))]
        self.assertGreater(totals[0], 2 * max(totals[1:]))

    def test_one_program_per_requirement_leaves_nothing_to_share(self):
        self.assertEqual(allocate(self.requirements([(5, 5, 0), (5, 0, 0)]), 1, random.Random(0)), [1, 1])


class RoundTest(unittest.TestCase):
    def setUp(self):
        self.assertTrue((EVOLVE_BIN / "evolve-check").exists(), "run cargo build --release first")
        self.dir = tempfile.TemporaryDirectory()
        self.root = Path(self.dir.name)
        self.run_dir = self.root / "run"
        self.samples = self.root / "samples"
        self.samples.mkdir()
        (self.samples / "a.sym").write_text("Clip alpha:\n    Note/60\n")
        (self.samples / "b.sym").write_text("Clip beta:\n    Note/62\n")
        self.system = generation_system(SPEC, self.samples)

    def tearDown(self):
        self.dir.cleanup()

    def loop(self, client, targets=TARGETS, per=1):
        oracle = Oracle(EVOLVE_BIN / "evolve-check", EVOLVE_BIN / "evolve-shrink", self.run_dir / "findings")
        return Loop(client, SPEC, self.system, targets, oracle, self.run_dir, 2, per, seed=0)

    def read(self, name):
        return json.loads((self.run_dir / name).read_text())

    def test_a_round_writes_requirements_programs_verdicts_and_only_a_confirmed_finding(self):
        client = Scripted()
        self.loop(client).run(1)
        found = {r["edge"]: r for r in self.read("requirements.json")}
        self.assertEqual(sorted(found), ["alpha one", "alpha two", "beta one", "beta two"])
        self.assertEqual([(r["programs"], r["disagreements"], r["confirmed"], r["yield"]) for r in found.values()],
                         [(1, 1, 1, 1.5), (1, 1, 0, 0.5), (1, 0, 0, 0.0), (1, 0, 0, 0.0)])
        self.assertEqual(found["alpha one"]["model"], "claude-opus-5-5")
        self.assertEqual(found["beta two"]["model"], "claude-fable-5-1")
        self.assertEqual(self.read("targets.json"), TARGETS)
        records = [json.loads(line) for line in (self.run_dir / "rounds/01/programs.jsonl").read_text().splitlines()]
        self.assertEqual([(r["id"], r["stated"], r["compiler"], r["disagreement"], r["confirmed"]) for r in records],
                         [("r000-1-0", "reject", "accept", True, True), ("r001-1-0", "reject", "accept", True, False),
                          ("r002-1-0", "reject", "reject", False, False), ("r003-1-0", "accept", "accept", False, False)])
        self.assertEqual(records[0]["program"], "Clip a:\n    Note/60\n")
        self.assertIn("unknown-kind", records[2]["codes"])
        self.assertEqual([r["code_mismatch"] for r in records], [False, False, True, False])
        findings = list((self.run_dir / "findings").glob("*.json"))
        self.assertEqual([path.name.split("-")[0] for path in findings], ["verdict"])
        self.assertEqual(json.loads(findings[0].read_text())["id"], "r000-1-0")
        counts = {"programs": 4, "disagreements": 2, "judged": 2, "confirmed": 1, "signals": 0}
        summary = self.read("rounds/01/summary.json")
        self.assertEqual({key: summary[key] for key in counts}, counts)
        self.assertEqual(summary["round"], 1)
        self.assertEqual(summary["by_target"]["alpha"],
                         {"programs": 2, "disagreements": 2, "judged": 2, "confirmed": 1, "signals": 0,
                          "code_mismatches": 0})
        self.assertEqual(summary["by_target"]["beta"],
                         {"programs": 2, "disagreements": 0, "judged": 0, "confirmed": 0, "signals": 0,
                          "code_mismatches": 1})

    def test_only_disagreements_reach_the_judges(self):
        client = Scripted()
        self.loop(client).run(1)
        judged = [user_of(params) for params in client.where(starts="Judge the SymphonyScript program")]
        self.assertEqual(len(judged), 4)
        self.assertEqual(sorted(text.count("Clip a:") + text.count("Clip b:") for text in judged), [1, 1, 1, 1])
        self.assertFalse(any("Clp c" in text or "Clip d" in text for text in judged))

    def test_requirement_calls_alternate_opus_and_fable_with_the_spec_as_the_whole_system(self):
        client = Scripted()
        targets = TARGETS + [{"name": "gamma", "sections": ["4.1"]}, {"name": "delta", "sections": ["3.6"]}]
        self.loop(client, targets).run(1)
        asked = client.where(starts="Target:")
        by_target = {user_of(params).split("\n")[0]: params["model"] for params in asked}
        self.assertEqual(by_target, {"Target: alpha": "claude-opus-5-5", "Target: beta": "claude-fable-5-1",
                                     "Target: gamma": "claude-opus-5-5", "Target: delta": "claude-fable-5-1"})
        for params in asked:
            self.assertEqual(params["system"], SPEC)
            self.assertEqual(params["max_tokens"], 32000)
            self.assertEqual(params["thinking"], {"type": "adaptive"})
            self.assertEqual(params["output_config"]["effort"], "high")
            self.assertEqual(params["output_config"]["format"]["schema"], targeted.REQUIREMENTS_SCHEMA)
        self.assertIn("section 3.7, section 5.5", user_of(client.where(starts="Target: beta")[0]))
        self.assertIn("Write 2 requirements", user_of(asked[0]))

    def test_program_calls_carry_the_spec_and_samples_and_the_requirement_in_the_user_turn(self):
        client = Scripted()
        self.loop(client).run(1)
        written = client.where(model="claude-sonnet-5-5")
        self.assertEqual(len(written), 4)
        for params in written:
            self.assertEqual(params["system"], self.system)
            self.assertEqual(params["max_tokens"], 32000)
            self.assertEqual(params["thinking"], {"type": "adaptive"})
            self.assertEqual(params["output_config"]["effort"], "high")
            self.assertEqual(params["output_config"]["format"]["schema"], targeted.PROGRAM_SCHEMA)
        self.assertTrue(self.system.startswith(SPEC + "\n\nValid program a.sym:"))
        prompt = user_of(client.where(starts="Write one SymphonyScript program")[0])
        self.assertTrue(any(f"Edge: {edge}" in prompt for edge in ("alpha one", "alpha two", "beta one", "beta two")))
        self.assertIn("Expected verdict:", prompt)
        self.assertIn("Rule: 6.6:", prompt)

    def test_the_next_round_writes_requirements_only_for_targets_that_yielded_nothing_and_keeps_every_one_funded(self):
        client = Scripted()
        self.loop(client, per=2).run(2)
        self.assertEqual(len(client.where(starts="Target: alpha")), 1)
        self.assertEqual(len(client.where(starts="Target: beta")), 2)
        again = user_of(client.where(starts="Target: beta")[1])
        self.assertIn("- beta one\n- beta two", again)
        found = self.read("requirements.json")
        self.assertEqual([(r["id"], r["target"], r["round"]) for r in found[4:]],
                         [("r004", "beta", 2), ("r005", "beta", 2)])
        second = [json.loads(line) for line in (self.run_dir / "rounds/02/programs.jsonl").read_text().splitlines()]
        self.assertEqual(len(second), 12)
        funded = {r["id"]: sum(p["requirement"] == r["id"] for p in second) for r in found}
        self.assertEqual(sorted(funded), [f"r{n:03d}" for n in range(6)])
        self.assertGreaterEqual(min(funded.values()), 1)
        self.assertEqual(self.read("rounds/02/summary.json")["round"], 2)
        self.assertEqual(sorted(path.name for path in (self.run_dir / "rounds").iterdir()), ["01", "02"])


class RoundOnFakeProxyTest(unittest.TestCase):
    def setUp(self):
        self.assertTrue(RUNNER_PYTHON.exists(), f"set up {RUNNER} first")
        self.assertTrue((EVOLVE_BIN / "evolve-check").exists(), "run cargo build --release first")
        self.dir = tempfile.TemporaryDirectory()
        self.root = Path(self.dir.name)
        (self.root / "rules.json").write_text(json.dumps(RULES))
        port = free_port()
        self.requests = self.root / "requests.jsonl"
        self.proxy = subprocess.Popen(
            [str(RUNNER_PYTHON), "-c", BACKEND, str(self.requests), str(self.root / "rules.json"),
             str(self.root / "proxy"), str(port)],
            cwd=RUNNER, stdout=subprocess.PIPE, text=True)
        self.proxy.stdout.readline()
        self.url = f"http://127.0.0.1:{port}"

    def tearDown(self):
        self.proxy.terminate()
        self.proxy.wait()
        self.proxy.stdout.close()
        self.dir.cleanup()

    def test_a_round_through_the_proxy_sends_the_spec_byte_identical_and_finds_what_both_judges_confirm(self):
        samples = self.root / "samples"
        samples.mkdir()
        (samples / "a.sym").write_text("Clip alpha:\n    Note/60\n")
        run = self.root / "run"
        oracle = Oracle(EVOLVE_BIN / "evolve-check", EVOLVE_BIN / "evolve-shrink", run / "findings")
        system = generation_system(SPEC, samples)
        Loop(ProxyClient(self.url), SPEC, system, TARGETS, oracle, run, 2, 1, seed=0).run(1)
        sent = [json.loads(line) for line in self.requests.read_text().splitlines()]
        asked = [params for params in sent if params["messages"][0]["content"].startswith("Target:")]
        self.assertEqual(sorted(params["model"] for params in asked), ["claude-fable-5-1", "claude-opus-5-5"])
        for params in asked:
            self.assertEqual([block["text"] for block in params["system"]], [SPEC])
            self.assertEqual(params["system"][-1]["cache_control"], {"type": "ephemeral", "ttl": "1h"})
        written = [params for params in sent if params["model"] == "claude-sonnet-5-5"]
        self.assertEqual(len(written), 4)
        for params in written:
            self.assertEqual([block["text"] for block in params["system"]], [system])
        self.assertEqual(len(sent), 2 + 4 + 4)
        findings = list((run / "findings").glob("verdict-*.json"))
        self.assertEqual([json.loads(path.read_text())["id"] for path in findings], ["r000-1-0"])


if __name__ == "__main__":
    unittest.main()
