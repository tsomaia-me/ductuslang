import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

import anthropic
import httpx

from run import Runner, read_keys, with_cache_control

USAGE = {"input_tokens": 10, "cache_creation_input_tokens": 100, "cache_read_input_tokens": 0, "output_tokens": 5}


def api_error(cls, status, message, error_type, headers=None):
    request = httpx.Request("POST", "https://api.anthropic.com/v1/messages/batches")
    response = httpx.Response(status, request=request, headers=headers or {})
    return cls(message, response=response, body={"error": {"type": error_type, "message": message}})


class Result:
    def __init__(self, data):
        self.data = data

    def to_dict(self, **_):
        return self.data


class FakeBatches:
    def __init__(self, name, log, errors=()):
        self.name = name
        self.log = log
        self.errors = list(errors)
        self.created = {}

    def create(self, requests):
        self.log.append(("create", self.name, [r["custom_id"] for r in requests]))
        if self.errors:
            raise self.errors.pop(0)
        batch_id = f"{self.name}-batch-{len(self.created) + 1}"
        self.created[batch_id] = requests
        return SimpleNamespace(id=batch_id)

    def retrieve(self, batch_id):
        return SimpleNamespace(processing_status="ended")

    def results(self, batch_id):
        self.log.append(("results", self.name, batch_id))
        message = {"model": "claude-opus-5-5", "usage": USAGE}
        return [SimpleNamespace(custom_id=r["custom_id"], result=Result({"type": "succeeded", "message": message}))
                for r in self.created[batch_id]]


def fake_client(name, log, errors=()):
    return SimpleNamespace(messages=SimpleNamespace(batches=FakeBatches(name, log, errors)))


def requests(n, system=None):
    params = {"model": "claude-opus-5-5", "max_tokens": 10, "messages": [{"role": "user", "content": "hi"}]}
    if system is not None:
        params["system"] = system
    return [{"custom_id": f"r{i}", "params": params} for i in range(n)]


class RunnerTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.run_dir = Path(self.dir.name) / "run"
        self.log = []
        self.sleeps = []

    def tearDown(self):
        self.dir.cleanup()

    def runner(self, clients, **kwargs):
        return Runner(clients, self.run_dir, poll_seconds=0, sleep=self.sleeps.append, log=lambda _: None, **kwargs)

    def results(self):
        return [json.loads(line) for line in (self.run_dir / "results.jsonl").read_text().splitlines()]

    def test_rotates_to_next_key_when_credit_is_exhausted(self):
        exhausted = api_error(anthropic.BadRequestError, 400,
                              "Your credit balance is too low to access the Anthropic API.", "invalid_request_error")
        clients = {1: fake_client("k1", self.log, [exhausted]), 2: fake_client("k2", self.log)}
        self.assertTrue(self.runner(clients).run(requests(2)))
        self.assertEqual([entry[1] for entry in self.log if entry[0] == "create"], ["k1", "k2"])
        self.assertEqual({entry["key"] for entry in self.results()}, {2})
        self.assertEqual(json.loads((self.run_dir / "state.json").read_text())["key"], 2)

    def test_stops_when_every_key_is_exhausted(self):
        exhausted = api_error(anthropic.APIStatusError, 402, "billing", "billing_error")
        clients = {1: fake_client("k1", self.log, [exhausted])}
        self.assertFalse(self.runner(clients).run(requests(1)))
        self.assertFalse((self.run_dir / "results.jsonl").exists())

    def test_waits_and_retries_same_key_on_rate_limit(self):
        limited = api_error(anthropic.RateLimitError, 429, "rate limited", "rate_limit_error", {"retry-after": "7"})
        clients = {1: fake_client("k1", self.log, [limited]), 2: fake_client("k2", self.log)}
        self.assertTrue(self.runner(clients).run(requests(2)))
        self.assertEqual([entry[1] for entry in self.log if entry[0] == "create"], ["k1", "k1"])
        self.assertIn(7.0, self.sleeps)
        self.assertEqual({entry["key"] for entry in self.results()}, {1})

    def test_resume_does_not_resubmit_and_reads_with_submitting_key(self):
        first = {1: fake_client("k1", self.log), 2: fake_client("k2", self.log)}
        self.runner(first, batch_size=2).run(requests(2))
        batches = first[1].messages.batches.created
        self.run_dir.joinpath("results.jsonl").unlink()
        state = json.loads((self.run_dir / "state.json").read_text())
        state["batches"][0]["done"] = False
        state["key"] = 2
        (self.run_dir / "state.json").write_text(json.dumps(state))

        self.log.clear()
        second = {1: fake_client("k1", self.log), 2: fake_client("k2", self.log)}
        second[1].messages.batches.created = batches
        self.assertTrue(self.runner(second, batch_size=2).run(requests(4)))
        self.assertEqual(self.log[0], ("results", "k1", "k1-batch-1"))
        creates = [entry for entry in self.log if entry[0] == "create"]
        self.assertEqual(creates, [("create", "k2", ["r2", "r3"])])
        self.assertEqual(sorted(entry["custom_id"] for entry in self.results()), ["r0", "r1", "r2", "r3"])

    def test_max_usd_stops_new_batches(self):
        clients = {1: fake_client("k1", self.log)}
        self.assertFalse(self.runner(clients, batch_size=1, max_usd=0.0).run(requests(3)))
        self.assertEqual(len([entry for entry in self.log if entry[0] == "create"]), 1)


class CacheControlTest(unittest.TestCase):
    def test_string_system_gets_cache_control(self):
        params = with_cache_control({"system": "be brief"})
        self.assertEqual(params["system"], [{"type": "text", "text": "be brief",
                                             "cache_control": {"type": "ephemeral", "ttl": "1h"}}])

    def test_last_block_gets_cache_control_without_mutating_input(self):
        original = {"system": [{"type": "text", "text": "a"}, {"type": "text", "text": "b"}]}
        params = with_cache_control(original)
        self.assertNotIn("cache_control", params["system"][0])
        self.assertEqual(params["system"][1]["cache_control"], {"type": "ephemeral", "ttl": "1h"})
        self.assertNotIn("cache_control", original["system"][1])

    def test_existing_cache_control_is_kept(self):
        original = {"system": [{"type": "text", "text": "a", "cache_control": {"type": "ephemeral"}},
                               {"type": "text", "text": "b"}]}
        self.assertIs(with_cache_control(original), original)

    def test_no_system_is_left_alone(self):
        self.assertEqual(with_cache_control({"model": "m"}), {"model": "m"})


class ReadKeysTest(unittest.TestCase):
    def read(self, content):
        with tempfile.TemporaryDirectory() as directory:
            env = Path(directory) / ".env"
            if content is not None:
                env.write_text(content)
            return read_keys(env)

    def test_reads_comma_separated_keys_in_order(self):
        self.assertEqual(self.read("OTHER=x\nANTHROPIC_API_KEYS= one , two,,three ,\nANTHROPIC_API_KEY_1=old\n"), ["one", "two", "three"])

    def test_missing_file_exits_with_one_line(self):
        with self.assertRaises(SystemExit) as raised:
            self.read(None)
        self.assertIn("no ~/.env file", str(raised.exception))

    def test_missing_variable_exits_with_one_line(self):
        with self.assertRaises(SystemExit) as raised:
            self.read("ANTHROPIC_API_KEY_1=old\n")
        self.assertIn("no ANTHROPIC_API_KEYS", str(raised.exception))

    def test_empty_list_exits_with_one_line(self):
        with self.assertRaises(SystemExit) as raised:
            self.read("ANTHROPIC_API_KEYS= , ,\n")
        self.assertIn("lists no key", str(raised.exception))


if __name__ == "__main__":
    unittest.main()
