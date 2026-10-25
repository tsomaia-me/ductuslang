import argparse
import itertools
import json
import os
import random
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
from collections import Counter
from functools import partial
from pathlib import Path

os.environ["OPENAI_API_KEY"] = "unused"
os.environ["OPENAI_BASE_URL"] = "http://127.0.0.1:9"

import Fuzz4All.target.target as fuzz4all_target
from Fuzz4All.fuzz import fuzz
from Fuzz4All.target.target import FResult, Target
from Fuzz4All.util.util import load_config_file

from oracle import Oracle
from proxy_llm import ProxyClient, ProxyRefused, generation_system, make_model_factory, request_engine
from spec_judge import SpecJudge

HERE = Path(__file__).resolve().parent
EXAMPLE = re.compile(r"^```sym\n(.*?)^```", re.MULTILINE | re.DOTALL)
COMMENT = re.compile(r'"(?:\\.|[^"\\])*"|//[^\n]*')


def without_comments(code):
    return COMMENT.sub(lambda match: match.group(0) if match.group(0).startswith('"') else "", code)


class SymphonyScriptTarget(Target):
    def __init__(self, oracle, **kwargs):
        super().__init__(**kwargs)
        self.config_dict = kwargs["config_dict"]
        self.prompt_used = self._create_prompt_from_config(self.config_dict)
        self.oracle = oracle

    def initialize(self):
        super().initialize()
        if self.prompt_used["example_code"]:
            self.update(prev=[(FResult.SAFE, self.prompt_used["example_code"])])

    def wrap_prompt(self, prompt):
        return f"// {prompt}\n{self.prompt_used['separator']}\n{self.prompt_used['begin']}"

    def wrap_in_comment(self, prompt):
        return f"// {prompt}"

    def filter(self, code):
        code = without_comments(code.replace(self.prompt_used["begin"], "").strip())
        return self.prompt_used["target_api"] in code

    def clean(self, code):
        return without_comments(code)

    def clean_code(self, code):
        code = without_comments(code.replace(self.prompt_used["begin"], "").strip())
        return "\n".join(line for line in code.split("\n") if line.strip() != "")

    def write_back_file(self, code):
        path = os.path.join(self.folder, "prompt_check.sym")
        with open(path, "w", encoding="utf-8") as out:
            out.write(code)
        return path

    def validate_individual(self, filename):
        with open(filename, encoding="utf-8") as source:
            return self.oracle.judge(filename, source.read())


def configured(config_path, folder, example=None):
    config = load_config_file(config_path)
    base = Path(config_path).resolve().parent
    target = config["target"]
    target["path_documentation"] = str(base / target["path_documentation"])
    target["path_example_code"] = str(example) if example else None
    config["fuzzing"]["output_folder"] = str(folder)
    return config


def make_target(config, oracle, proxy_url, samples):
    client = ProxyClient(proxy_url)
    spec = Path(config["target"]["path_documentation"]).read_text()
    fuzz4all_target.make_model = make_model_factory(client, generation_system(spec, samples))
    fuzz4all_target.request_engine = partial(
        request_engine, client, itertools.cycle(config["llm"]["autoprompt_model_names"]))
    llm, fuzzing = config["llm"], config["fuzzing"]
    os.makedirs(fuzzing["output_folder"], exist_ok=True)
    return SymphonyScriptTarget(
        oracle,
        language=config["target"]["language"],
        folder=fuzzing["output_folder"],
        bs=llm["batch_size"],
        temperature=llm["temperature"],
        device=llm["device"],
        model_name=llm["model_name"],
        max_length=llm["max_length"],
        use_hw=fuzzing["use_hand_written_prompt"],
        no_input_prompt=fuzzing["no_input_prompt"],
        prompt_strategy=fuzzing["prompt_strategy"],
        level=fuzzing["log_level"],
        template="fuzzing_with_config_file",
        config_dict=config,
        target_name="evolve-check",
    )


def oracle_for(args):
    return Oracle(Path(args.evolve_bin) / "evolve-check", Path(args.evolve_bin) / "evolve-shrink",
                  Path(args.run_dir) / "findings")


def seeds(args, oracle):
    folder = Path(args.run_dir) / "seeds"
    if folder.exists():
        return sorted(folder.glob("*.sym"))
    config = load_config_file(args.config)
    documentation = (Path(args.config).resolve().parent / config["target"]["path_documentation"]).read_text()
    kept = []
    for index, example in enumerate(EXAMPLE.findall(documentation)):
        result, _ = oracle.judge(f"spec-example-{index}", example)
        if result == FResult.SAFE:
            kept.append(example)
    folder.mkdir(parents=True)
    for index, example in enumerate(kept):
        (folder / f"{index:03}.sym").write_text(example)
    return sorted(folder.glob("*.sym"))


def autoprompt(args, oracle):
    folder = Path(args.run_dir) / "autoprompt"
    best = folder / "prompts" / "best_prompt.txt"
    if not best.exists():
        make_target(configured(args.config, folder), oracle, args.proxy, args.samples).initialize()
    return best


def programs_done(folder):
    return max((int(path.stem) for path in Path(folder).glob("*.fuzz")), default=-1) + 1


def archive(findings):
    records = []
    for path in sorted(Path(findings).glob("*.json")):
        try:
            records.append(json.loads(path.read_text()))
        except ValueError:
            continue
    return records


def archive_share(archived, examples):
    return min(0.5, archived / (archived + examples)) if archived else 0.0


def rarity_weights(records):
    signals = Counter(record["signal"] for record in records)
    codes = Counter(code for record in records for code in set(record.get("codes", [])))
    weights = []
    for record in records:
        own = set(record.get("codes", []))
        code_rarity = sum(1 / codes[code] for code in own) / len(own) if own else 1.0
        weights.append(code_rarity / signals[record["signal"]])
    return weights


def draw_seed(rng, examples, records):
    if rng.random() < archive_share(len(records), len(examples)):
        return rng.choices(records, weights=rarity_weights(records))[0]["sources"][0]["text"]
    return rng.choice(examples)


class Workers:
    def __init__(self, args, examples):
        self.args = args
        self.examples = examples
        self.deadline = time.time() + load_config_file(args.config)["fuzzing"]["total_time"] * 3600
        self.lock = threading.Lock()
        self.stopping = False
        self.running = {}

    def run(self, index):
        args = self.args
        folder = Path(args.run_dir) / "workers" / f"{index:03}"
        total = load_config_file(args.config)["fuzzing"]["num"]
        code = 0
        while (done := programs_done(folder)) < total and time.time() < self.deadline:
            seed = folder / "seed.sym"
            seed.write_text(draw_seed(random.Random(f"{index}:{done}"), self.examples,
                                      archive(Path(args.run_dir) / "findings")))
            with self.lock, open(folder / "worker.log", "a") as log:
                if self.stopping:
                    return code
                self.running[index] = subprocess.Popen(
                    [sys.executable, __file__, "worker", "--index", str(index), "--seed", str(seed),
                     "--until", str(done + args.restart_every), "--hours", str((self.deadline - time.time()) / 3600),
                     "--judge-every", str(args.judge_every), "--proxy", args.proxy, "--run-dir", args.run_dir,
                     "--config", args.config, "--evolve-bin", args.evolve_bin, "--samples", args.samples],
                    stdout=log, stderr=subprocess.STDOUT)
            code = self.running[index].wait()
            if code != 0 or programs_done(folder) == done:
                return code
        return code

    def stop(self):
        with self.lock:
            self.stopping = True
            for running in self.running.values():
                running.send_signal(signal.SIGTERM)


def start(args):
    oracle = oracle_for(args)
    examples = [path.read_text() for path in seeds(args, oracle)]
    if not examples:
        sys.exit("no SPEC example compiles cleanly to seed the workers")
    best = autoprompt(args, oracle)
    for index in range(args.workers):
        folder = Path(args.run_dir) / "workers" / f"{index:03}"
        (folder / "prompts").mkdir(parents=True, exist_ok=True)
        shutil.copyfile(best, folder / "prompts" / "best_prompt.txt")
    workers = Workers(args, examples)
    codes = [None] * args.workers
    threads = [threading.Thread(target=lambda index=index: codes.__setitem__(index, workers.run(index)))
               for index in range(args.workers)]
    for thread in threads:
        thread.start()
    print(f"{args.workers} workers running; findings in {Path(args.run_dir) / 'findings'}", flush=True)
    try:
        for thread in threads:
            thread.join()
    except KeyboardInterrupt:
        workers.stop()
        for thread in threads:
            thread.join()
    print(f"workers exited: {codes}")


def worker(args):
    folder = Path(args.run_dir) / "workers" / f"{args.index:03}"
    done = programs_done(folder)
    random.seed(f"{args.index}:{done}")
    config = configured(args.config, folder, Path(args.seed))
    oracle = oracle_for(args)
    spec = Path(config["target"]["path_documentation"]).read_text()
    oracle.spec_judge = SpecJudge(ProxyClient(args.proxy), spec, oracle.keep, Path(args.run_dir) / "judged",
                                  folder / "judged.jsonl", args.judge_every, done)
    target = make_target(config, oracle, args.proxy, args.samples)
    fuzzing = config["fuzzing"]
    try:
        fuzz(target=target, number_of_iterations=min(fuzzing["num"], args.until),
             total_time=min(fuzzing["total_time"], args.hours), output_folder=str(folder),
             resume=fuzzing["resume"], otf=fuzzing["otf"])
    except ProxyRefused as refused:
        sys.exit(f"the proxy refused a call: {refused}")
    finally:
        oracle.spec_judge.close()


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["start", "worker"])
    parser.add_argument("--proxy", default="http://127.0.0.1:8787")
    parser.add_argument("--run-dir", default=str(HERE / "runs" / "fuzz4all"))
    parser.add_argument("--config", default=str(HERE / "symphonyscript.yaml"))
    parser.add_argument("--samples", default=str(HERE / "samples"))
    parser.add_argument("--evolve-bin", default=str(HERE.parent / "target" / "release"))
    parser.add_argument("--workers", type=int, default=32)
    parser.add_argument("--index", type=int)
    parser.add_argument("--seed")
    parser.add_argument("--until", type=int)
    parser.add_argument("--hours", type=float)
    parser.add_argument("--restart-every", type=int, default=200)
    parser.add_argument("--judge-every", type=int, default=20)
    args = parser.parse_args(argv)
    args.run_dir = str(Path(args.run_dir).resolve())
    args.config = str(Path(args.config).resolve())
    args.samples = str(Path(args.samples).resolve())
    if args.command == "start":
        start(args)
    else:
        worker(args)


if __name__ == "__main__":
    main()
