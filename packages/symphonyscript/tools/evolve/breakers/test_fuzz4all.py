import itertools
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
from proxy_llm import ProxyClient, ProxyModel, ProxyRefused, generation_system, request_engine

HERE = Path(__file__).resolve().parent
RUNNER = HERE.parents[4] / "scripts" / "batch-runner"
RUNNER_PYTHON = RUNNER / ".venv" / "bin" / "python"
EVOLVE_BIN = HERE.parent / "target" / "release"
SPEC = (HERE.parents[2] / "docs" / "SPEC.md").read_text()
RECORDING_BACKEND = (
    "import json, sys, threading, fake_backend\n"
    "out, replies = open(sys.argv[1], 'a'), json.load(open(sys.argv[2]))\n"
    "lock, count = threading.Lock(), iter(range(1 << 62))\n"
    "def respond(params):\n"
    "    with lock:\n"
    "        out.write(json.dumps(params) + '\\n')\n"
    "        out.flush()\n"
    "        return replies[next(count) % len(replies)]\n"
    "started = fake_backend.start(sys.argv[3], respond, port=int(sys.argv[4]), batch_requests=8, batch_seconds=0.2)\n"
    "print(started.url, flush=True)\n"
    "threading.Event().wait()\n")
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
    def test_autoprompting_goes_to_each_model_in_turn_with_the_system_message_lifted_and_adaptive_thinking_at_high_effort(self):
        client = Recorder("summary")
        models = itertools.cycle(["claude-opus-5-5", "claude-fable-5-1"])
        config = {"model": "gpt-4", "max_tokens": 500, "temperature": 0.0,
                  "messages": [{"role": "system", "content": "You are an auto-prompting tool"},
                               {"role": "user", "content": "doc\nsummarize"}]}
        for _ in range(3):
            reply = request_engine(client, models, config)
            self.assertEqual(reply.choices[0].message.content, "summary")
        self.assertEqual([sent["model"] for sent in client.sent],
                         ["claude-opus-5-5", "claude-fable-5-1", "claude-opus-5-5"])
        self.assertEqual(client.sent[0], {"model": "claude-opus-5-5", "max_tokens": 32000,
                                          "system": "You are an auto-prompting tool",
                                          "messages": [{"role": "user", "content": "doc\nsummarize"}],
                                          "thinking": {"type": "adaptive"}, "output_config": {"effort": "high"}})

    def test_generation_sends_one_call_per_sample_with_effort_high_and_the_eos_as_stop_sequences(self):
        client = Recorder("```sym\nClip a:\n    Note/60\n```")
        model = ProxyModel(client, "claude-sonnet-5-5", ["// sep", "<eom>", "// sep", ""], "the system", 32000)
        self.assertEqual(model.generate("prompt", batch_size=3, max_length=1024), ["Clip a:\n    Note/60\n"] * 3)
        self.assertEqual(len(client.sent), 3)
        self.assertEqual(client.sent[0], {
            "model": "claude-sonnet-5-5", "max_tokens": 32000, "system": "the system",
            "thinking": {"type": "adaptive"}, "output_config": {"effort": "high"}, "stop_sequences": ["// sep", "<eom>"],
            "messages": [{"role": "user", "content": "prompt"}]})
        self.assertEqual(client.sent[1], client.sent[0])

    def test_the_generation_system_is_the_spec_then_each_sample_in_sorted_order(self):
        with tempfile.TemporaryDirectory() as folder:
            (Path(folder) / "b.sym").write_text("Clip b:\n")
            (Path(folder) / "a.sym").write_text("Clip a:\n")
            (Path(folder) / "c.txt").write_text("ignored")
            system = generation_system("SPEC TEXT", folder)
        self.assertEqual(system, "SPEC TEXT\n\nValid program a.sym:\n\nClip a:\n\n\nValid program b.sym:\n\nClip b:\n")

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
        self.requests = self.root / "requests.jsonl"
        self.proxy = subprocess.Popen(
            [str(RUNNER_PYTHON), "-c", RECORDING_BACKEND, str(self.requests), str(self.root / "replies.json"),
             str(self.root / "proxy"), self.url.rsplit(":", 1)[1]],
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
        samples = self.root / "samples"
        samples.mkdir()
        (samples / "b.sym").write_text("Clip beta:\n    Note/62\n")
        (samples / "a.sym").write_text("Clip alpha:\n    Note/60\n")
        started = subprocess.run(
            [sys.executable, str(HERE / "fuzz4all_symphonyscript.py"), "start", "--workers", "2", "--proxy", self.url,
             "--run-dir", str(run), "--config", str(self.root / "config.yaml"), "--evolve-bin", str(EVOLVE_BIN),
             "--samples", str(samples)],
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
        self.assertEqual(models.count("claude-fable-5-1"), 2)
        self.assertEqual(models.count("claude-sonnet-5-5"), 4 * 2 + 2 * 6)
        sent = [json.loads(line) for line in self.requests.read_text().splitlines()]
        autoprompts = [params["model"] for params in sent if params["model"] in ("claude-opus-5-5", "claude-fable-5-1")
                       and "format" not in params.get("output_config", {})]
        self.assertEqual(sorted(autoprompts), ["claude-fable-5-1"] * 2 + ["claude-opus-5-5"] * 2)
        generations = [params for params in sent if params["model"] == "claude-sonnet-5-5"]
        self.assertEqual(len(generations), 4 * 2 + 2 * 6)
        expected = SPEC + "\n\nValid program a.sym:\n\nClip alpha:\n    Note/60\n" \
                          "\n\nValid program b.sym:\n\nClip beta:\n    Note/62\n"
        for params in generations:
            self.assertEqual([block["text"] for block in params["system"]], [expected])
            self.assertEqual(params["system"][-1]["cache_control"], {"type": "ephemeral", "ttl": "1h"})
            self.assertEqual(params["output_config"], {"effort": "high"})
            self.assertEqual(params["max_tokens"], 32000)


if __name__ == "__main__":
    unittest.main()
