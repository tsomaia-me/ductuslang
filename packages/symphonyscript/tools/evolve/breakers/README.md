# breakers

Fuzz4All (https://github.com/fuzz4all/fuzz4all, ICSE 2024) generating SymphonyScript programs, with `evolve-check` as its oracle, its LLM calls answered from batches by `scripts/batch-runner/proxy.py`.

## Setup

```sh
(cd ../../../../../scripts/batch-runner && python3 -m venv .venv && .venv/bin/pip install -r requirements.txt)
./setup.sh
```

`setup.sh` makes the git-ignored `.venv` with `requirements.txt`, clones Fuzz4All into `.venv/src/fuzz4all` at commit `6d1169382e5470e8d83e86f327ce4215f307a12b` (the initial release; later commits disable autoprompting through an API) and builds `evolve-check` and `evolve-shrink` in release.

## Run

From the repository root, in two terminals:

```sh
cd scripts/batch-runner
.venv/bin/python proxy.py runs/fuzz4all --max-usd 20 --batch-seconds 60

cd packages/symphonyscript/tools/evolve/breakers
.venv/bin/python fuzz4all_symphonyscript.py start [--workers 32] [--run-dir runs/fuzz4all] [--proxy http://127.0.0.1:8787] [--restart-every 200] [--judge-every 20]
```

`start` does three things in `<run-dir>`:

1. `seeds/`: every `sym` example of `docs/SPEC.md` that `evolve-check` passes with no error and no signal.
2. `autoprompt/`: Fuzz4All's autoprompting, once: a greedy and three sampled summaries of the SPEC from Claude Sonnet 5.5, each scored by generating `batch_size` programs with Claude Haiku 5.5; the best goes to `prompts/best_prompt.txt`.
3. `workers/NNN/`: the workers, each running Fuzz4All's loop with its first example drawn as § Seeds from findings says and written to `seed.sym`; its programs are `N.fuzz`, its output `worker.log` and Fuzz4All's logs, and its judged programs `judged.jsonl`.

Each round of a worker sends `batch_size` (30) calls at once, so 32 workers hold up to 960 calls in the proxy.

The configuration is `symphonyscript.yaml` in Fuzz4All's format, with `llm.autoprompt_model_name` added.

## Findings

A program is a finding when its verdict holds a signal. Each signal is shrunk with `evolve-shrink` and kept once per key in `<run-dir>/findings/<signal>-<hash of key>.json` (the shrink output plus `original_sources`). A finding `evolve-shrink` cannot reproduce is kept as `unshrunk-<signal>-<hash>.json`, a program that kills `evolve-check` as `process-<signal number>-<hash>.json`, and a SPEC judge finding as `verdict-<hash of key>.json`.

## SPEC judge

A program whose verdict holds no signal may still be wrong: the compiler accepts what the SPEC rejects, or the reverse. `spec_judge.py` asks Claude Opus 5.5 for the SPEC's answer on a sample of them.

- Sample: a worker judges at most one program per `--judge-every` programs it generated, and only a novel one that parses (the verdict's `parses`, true when lexing and parsing draw no diagnostic): its set of error codes, or for a clean program its top-level kinds and the SPEC keywords (§2.5) it uses, not judged before in the run. `<run-dir>/judged/` holds the claimed keys, shared by every worker; `judged.jsonl` keeps the worker's count across restarts.
- Call: the whole of `docs/SPEC.md` as the system prompt, alone, so the proxy caches it for an hour; the program in the user message; effort `high`; the reply constrained to `{"verdict": "accept" | "reject", "rule": "<section and sentence>"}`. A program is accepted when it compiles with warnings at most.
- Two independent calls per program, the second told to argue the opposite verdict first. A finding needs both verdicts equal and different from the compiler's (error diagnostics only); a reply that is not that JSON makes none.
- The calls run on a worker's background threads, so the worker does not wait a batch for them; a worker stopped with Ctrl-C loses the judgements still in flight.
- `verdict-<hash of key>.json` holds the program, both rules, the compiler's verdict and diagnostics. It is not shrunk, since a smaller program can have a different answer.

## Seeds from findings

A worker restarts every `--restart-every` programs on Fuzz4All's own resume, and before each start its first example is drawn: from the findings archive with probability `min(1/2, findings / (findings + SPEC examples))`, else uniformly from the SPEC examples. A finding is drawn with weight `1 / n(signal)` times the mean of `1 / n(code)` over its codes (1 with none), the counts taken over the archive. The draw seeds the loop through Fuzz4All's `update`, as a SPEC example does.

## Stop

Ctrl-C on `start` stops the workers; Ctrl-C on the proxy stops it and prints the spend. Running the same commands again resumes: workers continue from their last program and reuse the seeds and the prompt, and the proxy collects a batch left in flight. When the proxy's cap is passed every call is refused and each worker exits.

## Dry run and tests

```sh
(cd ../../../../../scripts/batch-runner && .venv/bin/python fake_backend.py runs/fake <replies.json>)
.venv/bin/python -m unittest -v
```

`fake_backend.py` serves the proxy on a fake batch backend answering each call with the next string of `replies.json`. The tests run the adapter, the oracle, the SPEC judge, the seed draw and the workers end to end on it, and need the batch runner's `.venv`.

## What the adapter changes

No file of Fuzz4All changes. `fuzz4all_symphonyscript.py` builds the target itself, since `make_target` knows only Fuzz4All's languages, and calls Fuzz4All's own `fuzz` loop. It replaces two names in `Fuzz4All.target.target`, the module's only LLM calls:

- `request_engine` (autoprompting, written for OpenAI's chat API) by `proxy_llm.request_engine`: the system message becomes the Messages API `system`, the model is `llm.autoprompt_model_name` in place of the hard-coded `gpt-4`, and `max_tokens` stays.
- `make_model` (a local StarCoder) by `proxy_llm.ProxyModel`, whose `generate` sends `batch_size` concurrent calls of the prompt with the stop strings Fuzz4All gives StarCoder as `stop_sequences`, a system prompt that asks for a completion as source only, and takes the code block of the reply as Fuzz4All's Ollama path does.

Forced by the models: Claude Sonnet 5.5 and Claude Haiku 5.5 refuse a temperature other than the default 1, so no temperature is sent and the greedy autoprompt is sampled too; thinking is off (`between_tools` for Sonnet, `disabled` for Haiku) so `max_tokens` bounds the text as it did for the original models. `OPENAI_API_KEY` is set to a placeholder only because Fuzz4All builds an OpenAI client on import; it never calls it.

Two choices at the edge: `SymphonyScriptTarget.initialize` starts the loop from the worker's seed through Fuzz4All's own `update`, as if that seed were its first valid program, where Fuzz4All reads `path_example_code` and never uses it; and a program is valid (`SAFE`) when it has no error diagnostic, ignoring warnings. A worker's random seed is its index and the count it resumes from, so a restart does not replay the previous run's choices.
