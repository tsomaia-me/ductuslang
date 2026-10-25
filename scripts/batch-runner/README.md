# batch-runner

Runs a JSONL file of Messages API requests through Anthropic's Message Batches API.

## Setup

```sh
cd scripts/batch-runner
python3 -m venv .venv
.venv/bin/pip install -r requirements.txt
```

Put the keys in `~/.env`, numbered from 1:

```
ANTHROPIC_API_KEY_1=...
ANTHROPIC_API_KEY_2=...
```

## Usage

```sh
.venv/bin/python run.py requests.jsonl runs/first [--max-usd 50] [--batch-size 1000]
```

Each line of `requests.jsonl` is `{"custom_id": "...", "params": {...Messages API params...}}`.

- Batches are submitted one at a time (at most 100,000 requests or 250 MB each, or `--batch-size`), each polled until it ends and its results appended to `<run-dir>/results.jsonl` as `{"custom_id", "key", "result"}`.
- Key 1 is used until the API reports its credit exhausted, then key 2, and so on. A rate limit waits and retries on the same key. Keys appear only as key-1, key-2, ...
- `<run-dir>/state.json` records each batch, the key that submitted it and whether its results are in. Re-running the same command collects unfinished batches with their own key and submits only requests that were never submitted.
- A system prompt with no `cache_control` gets `{"type": "ephemeral", "ttl": "1h"}` on its last block.
- `--max-usd` stops submitting new batches once the spend from the results so far passes the limit; it is checked between batches, so set `--batch-size` to bound the overshoot. Spend uses the batch prices in `PRICES` (half the standard prices), with every cache write priced at the 1-hour rate.
- The summary prints requests done, failed and not run, tokens per key (uncached input, cache writes, cache reads, output) and the spend.

## Tests

```sh
.venv/bin/python -m unittest -v
```
