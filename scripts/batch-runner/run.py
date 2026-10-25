import argparse
import copy
import json
import re
import sys
import time
from pathlib import Path

import anthropic

MAX_BATCH_REQUESTS = 100_000
MAX_BATCH_BYTES = 250 * 1024 * 1024
POLL_SECONDS = 60
RATE_LIMIT_SECONDS = 60
CACHE_CONTROL = {"type": "ephemeral", "ttl": "1h"}

PRICES = {
    "claude-fable-5-1": (10.0, 20.0, 0.25, 50.0),
    "claude-mythos-5-1": (10.0, 20.0, 0.25, 50.0),
    "claude-fable-5": (10.0, 20.0, 1.0, 50.0),
    "claude-mythos-5": (10.0, 20.0, 1.0, 50.0),
    "claude-opus-5-5": (4.0, 8.0, 0.20, 20.0),
    "claude-opus-5": (5.0, 10.0, 0.50, 25.0),
    "claude-opus-4-8": (5.0, 10.0, 0.50, 25.0),
    "claude-opus-4-7": (5.0, 10.0, 0.50, 25.0),
    "claude-opus-4-6": (5.0, 10.0, 0.50, 25.0),
    "claude-opus-4-5": (5.0, 10.0, 0.50, 25.0),
    "claude-sonnet-5-5": (2.0, 4.0, 0.10, 10.0),
    "claude-sonnet-5": (2.0, 4.0, 0.20, 10.0),
    "claude-sonnet-4-6": (3.0, 6.0, 0.30, 15.0),
    "claude-sonnet-4-5": (3.0, 6.0, 0.30, 15.0),
    "claude-haiku-5-5": (0.10, 0.20, 0.01, 0.50),
    "claude-haiku-4-5": (1.0, 2.0, 0.10, 5.0),
}
LONG_PROMPT_PRICES = {"claude-haiku-5-5": (100_000, (0.50, 1.0, 0.05, 2.50))}
BATCH_DISCOUNT = 0.5
TOKEN_FIELDS = ("input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens", "output_tokens")


def read_keys(env_path):
    if not env_path.is_file():
        sys.exit("no ~/.env file: create it with ANTHROPIC_API_KEYS=<key 1>,<key 2>,...")
    value = None
    for line in env_path.read_text().splitlines():
        match = re.match(r"\s*(?:export\s+)?ANTHROPIC_API_KEYS?\s*=\s*(.*)$", line)
        if match:
            value = match.group(1).strip().strip("'\"")
    if value is None:
        sys.exit("no ANTHROPIC_API_KEYS in ~/.env: add ANTHROPIC_API_KEYS=<key 1>,<key 2>,...")
    keys = [key.strip() for key in value.split(",") if key.strip()]
    if not keys:
        sys.exit("ANTHROPIC_API_KEYS in ~/.env lists no key: set it to <key 1>,<key 2>,...")
    return keys


def with_cache_control(params):
    system = params.get("system")
    if not system:
        return params
    blocks = [{"type": "text", "text": system}] if isinstance(system, str) else copy.deepcopy(system)
    if any("cache_control" in block for block in blocks):
        return params
    blocks[-1]["cache_control"] = dict(CACHE_CONTROL)
    return {**params, "system": blocks}


def chunk(requests, max_requests):
    batches, current, size = [], [], 0
    for request in requests:
        request_size = len(json.dumps(request).encode())
        if current and (len(current) >= max_requests or size + request_size > MAX_BATCH_BYTES):
            batches.append(current)
            current, size = [], 0
        current.append(request)
        size += request_size
    if current:
        batches.append(current)
    return batches


def credit_exhausted(error):
    return isinstance(error, anthropic.APIStatusError) and (
        error.status_code == 402
        or error.type == "billing_error"
        or "credit balance" in str(error.message).lower()
    )


def retry_after(error):
    try:
        return float(error.response.headers.get("retry-after", RATE_LIMIT_SECONDS))
    except ValueError:
        return RATE_LIMIT_SECONDS


def cost(model, usage):
    tokens = [usage.get(field) or 0 for field in TOKEN_FIELDS]
    prices = PRICES[model]
    if model in LONG_PROMPT_PRICES and sum(tokens[:3]) > LONG_PROMPT_PRICES[model][0]:
        prices = LONG_PROMPT_PRICES[model][1]
    return sum(count * price for count, price in zip(tokens, prices)) * BATCH_DISCOUNT / 1_000_000


class Runner:
    def __init__(self, clients, run_dir, max_usd=None, batch_size=MAX_BATCH_REQUESTS,
                 poll_seconds=POLL_SECONDS, sleep=time.sleep, log=print):
        self.clients = clients
        self.run_dir = Path(run_dir)
        self.max_usd = max_usd
        self.batch_size = batch_size
        self.poll_seconds = poll_seconds
        self.sleep = sleep
        self.log = log
        self.state_path = self.run_dir / "state.json"
        self.results_path = self.run_dir / "results.jsonl"
        self.state = json.loads(self.state_path.read_text()) if self.state_path.exists() else {"key": 1, "batches": []}

    def save(self):
        tmp = self.state_path.with_suffix(".tmp")
        tmp.write_text(json.dumps(self.state, indent=1))
        tmp.replace(self.state_path)

    def read_results(self):
        if not self.results_path.exists():
            return []
        return [json.loads(line) for line in self.results_path.read_text().splitlines() if line.strip()]

    def spend(self):
        return sum(
            cost(entry["result"]["message"]["model"], entry["result"]["message"]["usage"])
            for entry in self.read_results()
            if entry["result"]["type"] == "succeeded"
        )

    def collect(self, batch):
        client = self.clients[batch["key"]]
        while client.messages.batches.retrieve(batch["id"]).processing_status != "ended":
            self.sleep(self.poll_seconds)
        written = {entry["custom_id"] for entry in self.read_results()}
        with self.results_path.open("a") as out:
            for entry in client.messages.batches.results(batch["id"]):
                if entry.custom_id not in written:
                    out.write(json.dumps({"custom_id": entry.custom_id, "key": batch["key"],
                                          "result": entry.result.to_dict(mode="json")}) + "\n")
        batch["done"] = True
        self.save()
        self.log(f"batch {batch['id']} done (key-{batch['key']})")

    def submit(self, requests):
        while True:
            key = self.state["key"]
            if key not in self.clients:
                return None
            try:
                created = self.clients[key].messages.batches.create(requests=requests)
            except anthropic.RateLimitError as error:
                self.log(f"key-{key} rate limited; waiting")
                self.sleep(retry_after(error))
                continue
            except anthropic.APIStatusError as error:
                if not credit_exhausted(error):
                    raise
                self.log(f"key-{key} credit exhausted; moving to key-{key + 1}")
                self.state["key"] = key + 1
                self.save()
                continue
            batch = {"id": created.id, "key": key, "custom_ids": [r["custom_id"] for r in requests], "done": False}
            self.state["batches"].append(batch)
            self.save()
            self.log(f"submitted batch {created.id} with {len(requests)} requests (key-{key})")
            return batch

    def run(self, requests):
        self.run_dir.mkdir(parents=True, exist_ok=True)
        for batch in self.state["batches"]:
            if not batch["done"]:
                self.collect(batch)
        submitted = {cid for batch in self.state["batches"] for cid in batch["custom_ids"]}
        pending = [
            {"custom_id": r["custom_id"], "params": with_cache_control(r["params"])}
            for r in requests if r["custom_id"] not in submitted
        ]
        for group in chunk(pending, self.batch_size):
            if self.max_usd is not None and self.spend() > self.max_usd:
                self.log(f"spend passed ${self.max_usd}; submitting no more batches")
                return False
            batch = self.submit(group)
            if batch is None:
                self.log("every key's credit is exhausted; submitting no more batches")
                return False
            self.collect(batch)
        return True

    def summary(self, total):
        results = self.read_results()
        done = sum(1 for entry in results if entry["result"]["type"] == "succeeded")
        lines = [f"requests: {total}, done: {done}, failed: {len(results) - done}, not run: {total - len(results)}"]
        per_key = {}
        for entry in results:
            if entry["result"]["type"] == "succeeded":
                usage = entry["result"]["message"]["usage"]
                sums = per_key.setdefault(entry["key"], [0, 0, 0, 0])
                for i, field in enumerate(TOKEN_FIELDS):
                    sums[i] += usage.get(field) or 0
        for key in sorted(per_key):
            uncached, writes, reads, output = per_key[key]
            lines.append(f"key-{key}: uncached input {uncached}, cache writes {writes}, cache reads {reads}, output {output}")
        if all(entry["result"]["message"]["model"] in PRICES for entry in results if entry["result"]["type"] == "succeeded"):
            lines.append(f"spend: ${self.spend():.2f}")
        return "\n".join(lines)


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("requests")
    parser.add_argument("run_dir")
    parser.add_argument("--max-usd", type=float)
    parser.add_argument("--batch-size", type=int, default=MAX_BATCH_REQUESTS)
    args = parser.parse_args(argv)

    requests = [json.loads(line) for line in Path(args.requests).read_text().splitlines() if line.strip()]
    if args.max_usd is not None:
        unknown = sorted({r["params"]["model"] for r in requests} - set(PRICES))
        if unknown:
            sys.exit(f"--max-usd needs a price for: {', '.join(unknown)}")
    keys = read_keys(Path.home() / ".env")
    clients = {n: anthropic.Anthropic(api_key=key) for n, key in enumerate(keys, start=1)}
    runner = Runner(clients, args.run_dir, max_usd=args.max_usd, batch_size=min(args.batch_size, MAX_BATCH_REQUESTS))
    runner.run(requests)
    print(runner.summary(len(requests)))


if __name__ == "__main__":
    main()
