import json
import socket
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import yaml

import fuzz4all_symphonyscript as breaker
from Fuzz4All.target.target import FResult
from oracle import Oracle
from proxy_llm import ProxyClient, ProxyModel, ProxyRefused, request_engine

HERE = Path(__file__).resolve().parent
RUNNER = HERE.parents[4] / "scripts" / "batch-runner"
RUNNER_PYTHON = RUNNER / ".venv" / "bin" / "python"
EVOLVE_BIN = HERE.parent / "target" / "release"
REPLIES = ["```\nClip intro:\n    Note/60 Note/64\n```", "Clip verse:\n    Note/62 | velocity=0.5\n", "Clip (("]


def free_port():
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        return probe.getsockname()[1]


class Recorder:
    def __init__(self, text="reply"):
        self.text = text
        self.sent = []

    def create(self, params):
        self.sent.append(params)
        return {"content": [{"type": "text", "text": self.text}]}


class AdapterTest(unittest.TestCase):
    def test_autoprompting_goes_to_its_model_with_the_system_message_lifted_and_thinking_off(self):
        client = Recorder("summary")
        config = {"model": "gpt-4", "max_tokens": 500, "temperature": 0.0,
                  "messages": [{"role": "system", "content": "You are an auto-prompting tool"},
                               {"role": "user", "content": "doc\nsummarize"}]}
        reply = request_engine(client, "claude-sonnet-5-5", config)
        self.assertEqual(reply.choices[0].message.content, "summary")
        self.assertEqual(client.sent, [{"model": "claude-sonnet-5-5", "max_tokens": 500,
                                        "system": "You are an auto-prompting tool",
                                        "messages": [{"role": "user", "content": "doc\nsummarize"}],
                                        "thinking": {"type": "between_tools"}}])

    def test_generation_sends_one_call_per_sample_with_the_eos_as_stop_sequences(self):
        client = Recorder("```sym\nClip a:\n    Note/60\n```")
        model = ProxyModel(client, "claude-haiku-5-5", ["// sep", "<eom>", "// sep", ""])
        self.assertEqual(model.generate("prompt", batch_size=3, max_length=1024), ["Clip a:\n    Note/60\n"] * 3)
        self.assertEqual(len(client.sent), 3)
        self.assertEqual(client.sent[0]["stop_sequences"], ["// sep", "<eom>"])
        self.assertEqual(client.sent[0]["thinking"], {"type": "disabled"})
        self.assertEqual(client.sent[0]["max_tokens"], 1024)
        self.assertEqual(client.sent[0]["messages"], [{"role": "user", "content": "prompt"}])

    def test_comments_are_removed_outside_strings(self):
        self.assertEqual(breaker.without_comments('Track a | label="x // y" // note\n'), 'Track a | label="x // y" \n')


def fake_tool(path, body):
    path.write_text(f"#!{sys.executable}\nimport json, sys\n{body}\n")
    path.chmod(0o755)
    return path


class OracleTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.root = Path(self.dir.name)

    def tearDown(self):
        self.dir.cleanup()

    def test_the_real_checker_judges_a_clean_program_safe_and_a_broken_one_failed(self):
        oracle = Oracle(EVOLVE_BIN / "evolve-check", EVOLVE_BIN / "evolve-shrink", self.root / "findings")
        self.assertEqual(oracle.judge("a", "Clip intro:\n    Note/60\n")[0], FResult.SAFE)
        self.assertEqual(oracle.judge("b", "Clip ((")[0], FResult.FAILURE)
        self.assertEqual(list((self.root / "findings").iterdir()), [])

    def test_findings_are_shrunk_and_kept_once_per_key(self):
        check = fake_tool(self.root / "check", (
            "program = json.loads(open(sys.argv[1]).read())\n"
            "open(sys.argv[2], 'w').write(json.dumps({'id': program['id'], 'diagnostics': [], 'signals': ['crash'],"
            " 'findings': [{'signal': 'crash', 'codes': []}]}) + '\\n')"))
        shrink = fake_tool(self.root / "shrink", (
            "program = json.loads(open(sys.argv[1]).read())\n"
            "print(json.dumps({'id': program['id'], 'signal': sys.argv[2], 'key': 'crash||00ff', 'codes': [],"
            " 'sources': [{'path': 'main.sym', 'text': 'Clip a:'}]}))"))
        oracle = Oracle(check, shrink, self.root / "findings")
        self.assertEqual(oracle.judge("one", "Clip a:\n    Note/1\n")[0], FResult.ERROR)
        self.assertEqual(oracle.judge("two", "Clip b:\n    Note/2\n")[0], FResult.ERROR)
        kept = list((self.root / "findings").iterdir())
        self.assertEqual(len(kept), 1)
        record = json.loads(kept[0].read_text())
        self.assertEqual((record["key"], record["id"]), ("crash||00ff", "one"))
        self.assertEqual(record["original_sources"][0]["text"], "Clip a:\n    Note/1\n")


class LoopOnFakeProxyTest(unittest.TestCase):
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

    def test_a_refused_call_raises(self):
        with self.assertRaises(ProxyRefused):
            ProxyClient(self.url).create({"model": "claude-haiku-5-5", "max_tokens": 1, "stream": True,
                                          "messages": [{"role": "user", "content": "hi"}]})

    def test_workers_run_fuzz4alls_loop_through_the_proxy(self):
        config = yaml.safe_load((HERE / "symphonyscript.yaml").read_text())
        config["fuzzing"]["num"] = 6
        config["llm"]["batch_size"] = 2
        config["target"]["path_documentation"] = str((HERE / config["target"]["path_documentation"]).resolve())
        (self.root / "config.yaml").write_text(yaml.safe_dump(config))
        run = self.root / "run"
        started = subprocess.run(
            [sys.executable, str(HERE / "fuzz4all_symphonyscript.py"), "start", "--workers", "2", "--proxy", self.url,
             "--run-dir", str(run), "--config", str(self.root / "config.yaml"), "--evolve-bin", str(EVOLVE_BIN)],
            capture_output=True, text=True, timeout=600)
        self.assertEqual(started.returncode, 0, started.stderr[-2000:])
        self.assertIn("workers exited: [0, 0]", started.stdout)
        self.assertTrue((run / "autoprompt" / "prompts" / "best_prompt.txt").exists())
        self.assertEqual(len(list((run / "autoprompt" / "prompts").glob("prompt_*.txt"))), 3)
        self.assertGreater(len(list((run / "seeds").glob("*.sym"))), 0)
        for worker in ("000", "001"):
            self.assertEqual(sorted(p.name for p in (run / "workers" / worker).glob("*.fuzz")),
                             [f"{n}.fuzz" for n in range(6)])
        models = [json.loads(line)["result"]["message"]["model"]
                  for line in (self.root / "proxy" / "results.jsonl").read_text().splitlines()]
        self.assertEqual(models.count("claude-sonnet-5-5"), 4)
        self.assertEqual(models.count("claude-haiku-5-5"), 4 * 2 + 2 * 6)


if __name__ == "__main__":
    unittest.main()
