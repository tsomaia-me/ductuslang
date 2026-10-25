import json
import random
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import yaml

import fuzz4all_symphonyscript as breaker
from proxy_llm import ProxyClient
from spec_judge import JUDGE_MODEL, SpecJudge, novelty, spec_keywords
from test_fuzz4all import EVOLVE_BIN, HERE, RUNNER, RUNNER_PYTHON, REPLIES, Recorder, free_port

SPEC = (HERE.parents[2] / "docs" / "SPEC.md").read_text()
CLEAN = "Clip intro:\n    Note/60\n"
ERRORS = [{"code": "unknown-kind", "file": "main.sym", "span": [0, 4], "message": "no kind Clp"}]


def answer(verdict, rule="7.2: A top-level placement is named."):
    return json.dumps({"verdict": verdict, "rule": rule})


class Keeper:
    def __init__(self):
        self.kept = {}

    def __call__(self, name, record):
        self.kept[name] = record
        return True


class JudgeSamplingTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.root = Path(self.dir.name)

    def tearDown(self):
        self.dir.cleanup()

    def judge(self, client, worker="000", every=3, generated=0):
        return SpecJudge(client, SPEC, Keeper(), self.root / "judged", self.root / f"{worker}.jsonl", every, generated)

    def test_novelty_is_the_error_codes_or_the_top_level_kinds_and_keywords(self):
        keywords = spec_keywords(SPEC)
        self.assertIn("when", keywords)
        self.assertEqual(novelty("Clp a:\n", ERRORS + ERRORS, keywords), "codes:unknown-kind")
        self.assertEqual(novelty("let x = 1\nClip a:\n    Note/x when true\n", [], keywords), "clean:Clip,let|let,true,when")

    def test_one_novel_program_is_judged_per_k_generated_and_a_key_only_once(self):
        judge = self.judge(Recorder(answer("accept")))
        self.assertTrue(judge.consider("0", CLEAN, [], True))
        self.assertFalse(judge.consider("1", "Clip a:\n    Note/1 when true\n", [], True))
        self.assertFalse(judge.consider("2", "Clp a:\n", None, True))
        self.assertFalse(judge.consider("3", "Clip b:\n    Note/2\n", [], True))
        self.assertTrue(judge.consider("4", "Clp a:\n", ERRORS, True))
        self.assertFalse(judge.consider("5", "Clp b:\n", ERRORS, True))
        self.assertFalse(judge.consider("6", "Clp c:\n", None, True))
        self.assertTrue(judge.consider("7", "Clip a:\n    Note/1 when true\n", [], True))
        judge.close()
        self.assertEqual(len(judge.client.sent), 6)
        self.assertEqual([json.loads(line)["id"] for line in (self.root / "000.jsonl").read_text().splitlines()],
                         ["0", "4", "7"])

    def test_a_program_that_does_not_parse_is_never_judged_and_its_key_stays_open(self):
        judge = self.judge(Recorder(answer("accept")), every=1)
        self.assertFalse(judge.consider("0", "Clp a:\n", ERRORS, False))
        self.assertFalse(judge.consider("1", CLEAN, [], False))
        self.assertTrue(judge.consider("2", "Clp a:\n", ERRORS, True))
        judge.close()
        self.assertEqual(len(judge.client.sent), 2)
        self.assertEqual([json.loads(line)["id"] for line in (self.root / "000.jsonl").read_text().splitlines()],
                         ["2"])

    def test_workers_share_the_judged_keys_and_a_restart_keeps_its_budget(self):
        first = self.judge(Recorder(answer("accept")))
        self.assertTrue(first.consider("0", CLEAN, [], True))
        first.close()
        other = self.judge(Recorder(answer("accept")), worker="001")
        self.assertFalse(other.consider("0", "Clip other:\n    Note/9\n", [], True))
        restarted = self.judge(Recorder(answer("accept")), generated=1)
        self.assertEqual(restarted.judged, 1)
        self.assertFalse(restarted.consider("1", "Clp a:\n", ERRORS, True))
        restarted.close()

    def test_a_judge_call_sends_the_spec_alone_as_the_system_prompt(self):
        judge = self.judge(Recorder(answer("accept")))
        judge.consider("0", CLEAN, [], True)
        judge.close()
        first, second = judge.client.sent
        for sent in (first, second):
            self.assertEqual((sent["model"], sent["system"]), (JUDGE_MODEL, SPEC))
            self.assertTrue(sent["messages"][0]["content"].endswith(f"```sym\n{CLEAN}\n```"))
            self.assertEqual(sent["output_config"]["format"]["schema"]["required"], ["verdict", "rule"])
        self.assertEqual(sorted("argue for the verdict opposite" in sent["messages"][0]["content"]
                                for sent in (first, second)), [False, True])


class JudgeAgreementOnFakeProxyTest(unittest.TestCase):
    def setUp(self):
        self.assertTrue(RUNNER_PYTHON.exists(), f"set up {RUNNER} first")
        self.dir = tempfile.TemporaryDirectory()
        self.root = Path(self.dir.name)
        self.proxies = []

    def tearDown(self):
        for proxy in self.proxies:
            proxy.terminate()
            proxy.wait()
            proxy.stdout.close()
        self.dir.cleanup()

    def proxy(self, replies):
        name = f"proxy-{len(self.proxies)}"
        (self.root / f"{name}.json").write_text(json.dumps(replies))
        port = free_port()
        proxy = subprocess.Popen(
            [str(RUNNER_PYTHON), "fake_backend.py", str(self.root / name), str(self.root / f"{name}.json"),
             "--port", str(port), "--batch-requests", "2", "--batch-seconds", "0.2"],
            cwd=RUNNER, stdout=subprocess.PIPE, text=True)
        proxy.stdout.readline()
        self.proxies.append(proxy)
        return ProxyClient(f"http://127.0.0.1:{port}")

    def judged(self, replies, text, errors):
        findings = self.root / "findings"
        findings.mkdir(exist_ok=True)
        oracle = breaker.Oracle(EVOLVE_BIN / "evolve-check", EVOLVE_BIN / "evolve-shrink", findings)
        judge = SpecJudge(self.proxy(replies), SPEC, oracle.keep, self.root / f"judged-{len(self.proxies)}",
                          self.root / f"log-{len(self.proxies)}.jsonl", 20)
        self.assertTrue(judge.consider("p", text, errors, True))
        judge.close()
        return [json.loads(path.read_text()) for path in sorted(findings.glob("verdict-*.json"))]

    def test_judges_that_agree_and_differ_from_the_compiler_make_a_finding(self):
        kept = self.judged([answer("reject", "7.2: rule one"), answer("reject", "7.2: rule two")], CLEAN, [])
        self.assertEqual(len(kept), 1)
        self.assertEqual((kept[0]["signal"], kept[0]["compiler"], kept[0]["judges"]), ("verdict", "accept", "reject"))
        self.assertEqual(sorted(kept[0]["rules"]), ["7.2: rule one", "7.2: rule two"])
        self.assertEqual((kept[0]["diagnostics"], kept[0]["sources"][0]["text"]), ([], CLEAN))

    def test_judges_that_accept_what_the_compiler_rejects_make_a_finding_with_its_diagnostics(self):
        kept = self.judged([answer("accept")], "Clp a:\n", ERRORS)
        self.assertEqual(len(kept), 1)
        self.assertEqual((kept[0]["compiler"], kept[0]["codes"], kept[0]["diagnostics"]),
                         ("reject", ["unknown-kind"], ERRORS))

    def test_judges_that_disagree_with_each_other_make_none(self):
        self.assertEqual(self.judged([answer("accept"), answer("reject")], CLEAN, []), [])

    def test_judges_that_agree_with_the_compiler_make_none(self):
        self.assertEqual(self.judged([answer("accept")], CLEAN, []), [])
        self.assertEqual(self.judged([answer("reject")], "Clp a:\n", ERRORS), [])

    def test_a_reply_that_is_not_the_answer_makes_none(self):
        self.assertEqual(self.judged(["not json"], CLEAN, []), [])


def finding(signal, codes, text):
    return {"signal": signal, "codes": codes, "sources": [{"path": "main.sym", "text": text}]}


class SeedDrawTest(unittest.TestCase):
    def test_the_archive_share_rises_with_its_size_up_to_half(self):
        self.assertEqual(breaker.archive_share(0, 20), 0.0)
        self.assertAlmostEqual(breaker.archive_share(5, 20), 0.2)
        self.assertEqual(breaker.archive_share(20, 20), 0.5)
        self.assertEqual(breaker.archive_share(500, 20), 0.5)

    def test_a_finding_weighs_by_the_rarity_of_its_signal_and_codes(self):
        records = [finding("crash", ["a"], "c1"), finding("crash", ["a"], "c2"), finding("slow", ["a"], "s"),
                   finding("verdict", ["b"], "v"), finding("process exited on signal 6", [], "p")]
        self.assertEqual(breaker.rarity_weights(records), [1 / 6, 1 / 6, 1 / 3, 1.0, 1.0])

    def test_draws_follow_the_share_and_the_weights(self):
        examples = [f"example {n}" for n in range(3)]
        records = [finding("crash", ["a"], "common")] * 2 + [finding("verdict", ["b"], "rare")]
        rng = random.Random(0)
        drawn = [breaker.draw_seed(rng, examples, records) for _ in range(20000)]
        self.assertAlmostEqual(sum(text in examples for text in drawn) / len(drawn), 0.5, delta=0.02)
        self.assertAlmostEqual(drawn.count("rare") / drawn.count("common"), 2.0, delta=0.15)
        self.assertEqual({breaker.draw_seed(rng, examples, []) for _ in range(200)}, set(examples))

    def test_the_archive_reads_every_finding_record(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            (folder / "crash-1.json").write_text(json.dumps(finding("crash", [], "x")))
            (folder / "partial.tmp").write_text("{")
            self.assertEqual([record["sources"][0]["text"] for record in breaker.archive(folder)], ["x"])


class WorkerRestartTest(unittest.TestCase):
    def setUp(self):
        self.assertTrue(RUNNER_PYTHON.exists(), f"set up {RUNNER} first")
        self.assertTrue((EVOLVE_BIN / "evolve-check").exists(), "run cargo build --release first")
        self.dir = tempfile.TemporaryDirectory()
        self.root = Path(self.dir.name)
        (self.root / "replies.json").write_text(json.dumps(REPLIES))
        self.url = f"http://127.0.0.1:{free_port()}"
        self.proxy = subprocess.Popen(
            [str(RUNNER_PYTHON), "fake_backend.py", str(self.root / "proxy"), str(self.root / "replies.json"),
             "--port", self.url.rsplit(":", 1)[1], "--batch-requests", "8", "--batch-seconds", "0.2"],
            cwd=RUNNER, stdout=subprocess.PIPE, text=True)
        self.proxy.stdout.readline()

    def tearDown(self):
        self.proxy.terminate()
        self.proxy.wait()
        self.proxy.stdout.close()
        self.dir.cleanup()

    def test_a_worker_restarts_every_n_programs_on_fuzz4alls_resume_and_loses_nothing(self):
        config = yaml.safe_load((HERE / "symphonyscript.yaml").read_text())
        config["fuzzing"]["num"] = 6
        config["llm"]["batch_size"] = 2
        config["target"]["path_documentation"] = str((HERE / config["target"]["path_documentation"]).resolve())
        (self.root / "config.yaml").write_text(yaml.safe_dump(config))
        run = self.root / "run"
        started = subprocess.run(
            [sys.executable, str(HERE / "fuzz4all_symphonyscript.py"), "start", "--workers", "1", "--proxy", self.url,
             "--run-dir", str(run), "--config", str(self.root / "config.yaml"), "--evolve-bin", str(EVOLVE_BIN),
             "--restart-every", "2"],
            capture_output=True, text=True, timeout=600)
        self.assertEqual(started.returncode, 0, started.stderr[-2000:])
        self.assertIn("workers exited: [0]", started.stdout)
        folder = run / "workers" / "000"
        self.assertEqual(sorted(p.name for p in folder.glob("*.fuzz")), [f"{n}.fuzz" for n in range(6)])
        log = (folder / "worker.log").read_text()
        self.assertEqual([log.count(f"resuming from {n})") for n in (0, 2, 4, 6)], [1, 1, 1, 0])
        models = [json.loads(line)["result"]["message"]["model"]
                  for line in (self.root / "proxy" / "results.jsonl").read_text().splitlines()]
        self.assertEqual(models.count("claude-sonnet-5-5"), 4 * 2 + 6)
        self.assertGreater(models.count(JUDGE_MODEL), 0)
        self.assertTrue((folder / "seed.sym").read_text())


if __name__ == "__main__":
    unittest.main()
