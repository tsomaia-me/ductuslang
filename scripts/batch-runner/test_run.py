import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

import anthropic
import httpx

from run import Runner, prefix_of, read_keys, with_cache_control

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
        self.retrieve_errors = []
        self.result_errors = []

    def create(self, requests):
        self.log.append(("create", self.name, [r["custom_id"] for r in requests]))
        if self.errors:
            raise self.errors.pop(0)
        batch_id = f"{self.name}-batch-{len(self.created) + 1}"
        self.created[batch_id] = requests
        return SimpleNamespace(id=batch_id)

    def retrieve(self, batch_id):
        if self.retrieve_errors:
            raise self.retrieve_errors.pop(0)
        return SimpleNamespace(processing_status="ended")

    def results(self, batch_id):
        self.log.append(("results", self.name, batch_id))
        message = {"model": "claude-opus-5-5", "usage": USAGE}
        items = [SimpleNamespace(custom_id=r["custom_id"], result=Result({"type": "succeeded", "message": message}))
                 for r in self.created[batch_id]]
        return self.stream(items)

    def stream(self, items):
        for index, item in enumerate(items):
            if self.result_errors and index == 1:
                raise self.result_errors.pop(0)
            yield item


def fake_client(name, log, errors=()):
    return SimpleNamespace(messages=SimpleNamespace(batches=FakeBatches(name, log, errors)))


def requests(n, system=None, model="claude-opus-5-5", start=0):
    params = {"model": model, "max_tokens": 10, "messages": [{"role": "user", "content": "hi"}]}
    if system is not None:
        params["system"] = system
    return [{"custom_id": f"r{i}", "params": params} for i in range(start, start + n)]


class RunnerTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.TemporaryDirectory()
        self.run_dir = Path(self.dir.name) / "run"
        self.log = []
        self.sleeps = []
        self.lines = []
        self.now = 1000.0

    def tearDown(self):
        self.dir.cleanup()

    def runner(self, clients, **kwargs):
        kwargs.setdefault("clock", lambda: self.now)
        kwargs.setdefault("log", self.lines.append)
        return Runner(clients, self.run_dir, poll_seconds=0, sleep=self.sleeps.append, **kwargs)

    def creates(self):
        return [entry[2] for entry in self.log if entry[0] == "create"]

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

    def timeout(self):
        return anthropic.APITimeoutError(httpx.Request("GET", "https://api.anthropic.com/v1/messages/batches"))

    def test_retries_retrieve_on_transient_errors(self):
        clients = {1: fake_client("k1", self.log)}
        runner = self.runner(clients)
        self.assertTrue(runner.run(requests(2)))
        self.run_dir.joinpath("results.jsonl").unlink()
        batch = runner.state["batches"][0]
        batch["done"] = False
        clients[1].messages.batches.retrieve_errors = [self.timeout(), self.timeout()]
        self.sleeps.clear()
        runner.collect(batch)
        self.assertEqual(sorted(entry["custom_id"] for entry in self.results()), ["r0", "r1"])
        self.assertEqual(len(self.sleeps), 2)

    def test_retries_results_stream_without_duplicates(self):
        clients = {1: fake_client("k1", self.log)}
        runner = self.runner(clients, batch_size=3)
        self.assertTrue(runner.run(requests(3)))
        self.run_dir.joinpath("results.jsonl").unlink()
        batch = runner.state["batches"][0]
        batch["done"] = False
        clients[1].messages.batches.result_errors = [
            anthropic.APIConnectionError(request=httpx.Request("GET", "https://api.anthropic.com"))]
        runner.collect(batch)
        self.assertEqual(sorted(entry["custom_id"] for entry in self.results()), ["r0", "r1", "r2"])

    def test_non_transient_error_while_collecting_raises(self):
        clients = {1: fake_client("k1", self.log)}
        runner = self.runner(clients)
        self.assertTrue(runner.run(requests(1)))
        batch = runner.state["batches"][0]
        batch["done"] = False
        clients[1].messages.batches.retrieve_errors = [
            api_error(anthropic.BadRequestError, 400, "bad", "invalid_request_error")]
        with self.assertRaises(anthropic.BadRequestError):
            runner.collect(batch)

    def test_max_usd_stops_new_batches(self):
        clients = {1: fake_client("k1", self.log)}
        runner = self.runner(clients, batch_size=1, max_usd=0.0005)
        self.assertFalse(runner.run(requests(3)))
        self.assertEqual(self.creates(), [["r0"]])
        self.assertEqual(runner.held, {"r1", "r2"})

    def test_a_shared_system_prompt_is_warmed_by_one_request_before_the_rest(self):
        clients = {1: fake_client("k1", self.log)}
        runner = self.runner(clients)
        self.assertTrue(runner.run(requests(5, system="long prompt")))
        self.assertEqual(self.creates(), [["r0"], ["r1", "r2", "r3", "r4"]])
        self.assertEqual(sorted(entry["custom_id"] for entry in self.results()), ["r0", "r1", "r2", "r3", "r4"])
        self.assertEqual(self.log.index(("results", "k1", "k1-batch-1")), 1)
        warmings = json.loads((self.run_dir / "state.json").read_text())["warmings"]
        self.assertEqual([(w["key"], w["model"], w["time"]) for w in warmings], [(1, "claude-opus-5-5", 1000.0)])

    def test_a_second_group_within_the_window_warms_nothing(self):
        clients = {1: fake_client("k1", self.log)}
        runner = self.runner(clients)
        runner.run(requests(2, system="long prompt"))
        self.now += 54 * 60
        runner.run(requests(2, system="long prompt") + requests(2, system="long prompt", start=2))
        self.assertEqual(self.creates(), [["r0"], ["r1"], ["r2", "r3"]])

    def test_a_prefix_warmed_more_than_55_minutes_ago_is_warmed_again(self):
        clients = {1: fake_client("k1", self.log)}
        runner = self.runner(clients)
        runner.run(requests(2, system="long prompt"))
        self.now += 56 * 60
        runner.run(requests(2, system="long prompt", start=2))
        self.assertEqual(self.creates(), [["r0"], ["r1"], ["r2"], ["r3"]])

    def test_each_model_and_each_system_text_is_warmed_once(self):
        clients = {1: fake_client("k1", self.log)}
        group = (requests(2, system="a") + requests(2, system="b", start=2)
                 + requests(2, system="a", model="claude-haiku-5-5", start=4) + requests(2, start=6))
        self.assertTrue(self.runner(clients).run(group))
        self.assertEqual(self.creates()[0], ["r0", "r2", "r4"])
        self.assertEqual(self.creates()[1], ["r1", "r3", "r5", "r6", "r7"])
        self.assertEqual(len(self.creates()), 2)

    def test_a_group_without_a_system_prompt_is_submitted_whole(self):
        clients = {1: fake_client("k1", self.log)}
        self.assertTrue(self.runner(clients).run(requests(3)))
        self.assertEqual(self.creates(), [["r0", "r1", "r2"]])

    def test_a_resumed_runner_reads_its_warmings_from_state(self):
        first = {1: fake_client("k1", self.log)}
        self.runner(first).run(requests(2, system="long prompt"))
        self.log.clear()
        self.now += 60
        second = {1: fake_client("k1", self.log)}
        self.assertTrue(self.runner(second).run(requests(4, system="long prompt")))
        self.assertEqual(self.creates(), [["r2", "r3"]])

    def test_a_batch_over_the_cap_submits_only_the_requests_that_fit(self):
        clients = {1: fake_client("k1", self.log)}
        probe = self.runner(clients)
        one = probe.estimate(requests(1)[0])
        runner = self.runner(clients, max_usd=one * 2.5)
        self.assertFalse(runner.run(requests(5)))
        self.assertEqual(self.creates(), [["r0", "r1"]])
        self.assertEqual(runner.held, {"r2", "r3", "r4"})
        self.assertEqual(len([line for line in self.lines if "holding back 3 of 5" in line]), 1)

    def test_nothing_is_submitted_when_not_even_one_request_fits(self):
        clients = {1: fake_client("k1", self.log)}
        runner = self.runner(clients, max_usd=1e-9)
        self.assertFalse(runner.run(requests(2)))
        self.assertEqual(self.creates(), [])
        self.assertEqual(runner.held, {"r0", "r1"})

    def test_a_warm_prompt_is_priced_as_a_cache_read_and_a_cold_one_as_a_write(self):
        runner = self.runner({1: fake_client("k1", self.log)})
        self.run_dir.mkdir()
        request = {"custom_id": "x", "params": with_cache_control(requests(1, system="s" * 3000)[0]["params"])}
        cold = runner.estimate(request)
        runner.record_warming(1, prefix_of(request["params"]))
        self.assertLess(runner.estimate(request) * 5, cold)

    def test_no_cap_changes_nothing(self):
        runner = self.runner({1: fake_client("k1", self.log)})
        self.assertEqual(runner.affordable(requests(50)), 50)


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
