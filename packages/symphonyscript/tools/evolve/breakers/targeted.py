import argparse
import itertools
import json
import os
import random
import re
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import yaml

os.environ["OPENAI_API_KEY"] = "unused"
os.environ["OPENAI_BASE_URL"] = "http://127.0.0.1:9"

from oracle import SOURCE_PATH, Oracle
from proxy_llm import THINKING, ProxyClient, ProxyRefused, generation_system, text_of
from spec_judge import SpecJudge

HERE = Path(__file__).resolve().parent
ANALYSTS = ("claude-opus-5-5", "claude-fable-5-1")
WRITER = "claude-sonnet-5-5"
EFFORT = "high"
MAX_TOKENS = 32000
ORACLE_THREADS = os.cpu_count() or 4
STATED = re.compile(r"(accept|reject)( \S+)?")
REQUIREMENTS_SCHEMA = {
    "type": "object",
    "properties": {"requirements": {"type": "array", "items": {
        "type": "object",
        "properties": {"edge": {"type": "string"}, "verdict": {"type": "string"}, "rule": {"type": "string"}},
        "required": ["edge", "verdict", "rule"],
        "additionalProperties": False}}},
    "required": ["requirements"],
    "additionalProperties": False,
}
PROGRAM_SCHEMA = {
    "type": "object",
    "properties": {"program": {"type": "string"}, "verdict": {"type": "string", "enum": ["accept", "reject"]},
                   "rule": {"type": "string"}},
    "required": ["program", "verdict", "rule"],
    "additionalProperties": False,
}
FILE_NOTE = (f"A program is one file, {SOURCE_PATH}, compiled against the example vocabulary of section 1.3. "
             "It is accepted when it compiles with warnings at most, and rejected when the specification "
             "requires an error for it.")


def requirements_prompt(target, count, known):
    sections = ", ".join(f"section {section}" for section in target["sections"])
    prompt = (f"Target: {target['name']}\nSPEC sections: {sections}\n\n"
              f"Write {count} requirements for this target. Each is one tricky edge of its rules that a careless "
              "compiler gets wrong: a boundary or an interaction, not a plain case. For each give edge (one or two "
              "sentences), verdict (accept; or reject, followed by a space and the error code a correct compiler "
              "draws when the specification names one; or reject alone), and rule (the section and the sentence "
              f"that decides it). {FILE_NOTE} Reply with JSON only.")
    if known:
        prompt += "\n\nThese edges are already written; write different ones:\n" + "\n".join(f"- {edge}" for edge in known)
    return prompt


def program_prompt(requirement, variant):
    return ("Write one SymphonyScript program that exercises the edge below, so that a careless compiler gets it "
            f"wrong. {FILE_NOTE}\n\nEdge: {requirement['edge']}\nExpected verdict: {requirement['verdict']}\n"
            f"Rule: {requirement['rule']}\n\nThis is variant {variant}; make its shape differ from what other "
            "variants of the same edge would be. Reply with JSON only: program is the whole file, verdict is "
            "accept or reject as the specification decides for your program, and rule is the section and the "
            "sentence that decides it.")


def message(model, system, schema, prompt):
    return {"model": model, "max_tokens": MAX_TOKENS, "system": system, "thinking": THINKING,
            "output_config": {"effort": EFFORT, "format": {"type": "json_schema", "schema": schema}},
            "messages": [{"role": "user", "content": prompt}]}


def submit(client, calls):
    if not calls:
        return []
    with ThreadPoolExecutor(max_workers=len(calls)) as pool:
        return list(pool.map(client.create, calls))


def json_of(reply):
    try:
        value = json.loads(text_of(reply))
        return value if isinstance(value, dict) else None
    except ValueError:
        return None


def requirements_of(reply):
    found = json_of(reply)
    entries = found.get("requirements") if found else None
    if not isinstance(entries, list):
        return []
    return [{"edge": entry["edge"], "verdict": entry["verdict"].strip(), "rule": entry["rule"]}
            for entry in entries
            if isinstance(entry, dict) and all(isinstance(entry.get(key), str) for key in ("edge", "verdict", "rule"))
            and STATED.fullmatch(entry["verdict"].strip())]


def program_of(reply):
    found = json_of(reply)
    if found and isinstance(found.get("program"), str) and found.get("verdict") in ("accept", "reject") \
            and isinstance(found.get("rule"), str):
        return found
    return None


def yield_of(requirement):
    return requirement["confirmed"] + requirement["disagreements"] / 2


def allocate(requirements, per, rng):
    if all(requirement["programs"] == 0 for requirement in requirements):
        return [per] * len(requirements)
    draws = [rng.betavariate(1 + yield_of(r), 1 + max(0, r["programs"] - yield_of(r))) for r in requirements]
    spare = per * len(requirements) - len(requirements)
    shares = [spare * draw / sum(draws) for draw in draws]
    counts = [1 + int(share) for share in shares]
    order = sorted(range(len(shares)), key=lambda i: shares[i] - int(shares[i]), reverse=True)
    for index in order[:spare - sum(int(share) for share in shares)]:
        counts[index] += 1
    return counts


class Loop:
    def __init__(self, client, spec, system, targets, oracle, run_dir, requirements=6, programs=4, seed=None):
        self.client = client
        self.spec = spec
        self.system = system
        self.targets = targets
        self.oracle = oracle
        self.run_dir = Path(run_dir)
        self.count = requirements
        self.per = programs
        self.rng = random.Random(seed)
        self.models = itertools.cycle(ANALYSTS)
        self.requirements = []
        self.judge = SpecJudge(client, spec, oracle.keep, self.run_dir / "judged", self.run_dir / "judged.jsonl", 1)

    def write(self, name, value):
        (self.run_dir / name).parent.mkdir(parents=True, exist_ok=True)
        (self.run_dir / name).write_text(json.dumps(value, indent=1))

    def fresh_targets(self):
        return [target for target in self.targets
                if sum(yield_of(r) for r in self.requirements if r["target"] == target["name"]) == 0]

    def write_requirements(self, number, targets):
        calls, models = [], []
        for target in targets:
            known = [r["edge"] for r in self.requirements if r["target"] == target["name"]]
            models.append(next(self.models))
            calls.append(message(models[-1], self.spec, REQUIREMENTS_SCHEMA,
                                 requirements_prompt(target, self.count, known)))
        for target, model, reply in zip(targets, models, submit(self.client, calls)):
            for entry in requirements_of(reply):
                self.requirements.append({
                    "id": f"r{len(self.requirements):03d}", "target": target["name"], "round": number,
                    "model": model, **entry, "programs": 0, "disagreements": 0, "confirmed": 0})

    def write_programs(self, number):
        jobs = []
        for requirement, count in zip(self.requirements, allocate(self.requirements, self.per, self.rng)):
            jobs += [(requirement, variant) for variant in range(count)]
        calls = [message(WRITER, self.system, PROGRAM_SCHEMA, program_prompt(requirement, variant))
                 for requirement, variant in jobs]
        records = []
        for (requirement, variant), reply in zip(jobs, submit(self.client, calls)):
            written = program_of(reply)
            records.append({"id": f"{requirement['id']}-{number}-{variant}", "requirement": requirement["id"],
                            "target": requirement["target"], "round": number,
                            "program": written["program"] if written else None,
                            "stated": written["verdict"] if written else None,
                            "stated_rule": written["rule"] if written else None})
        return records

    def check(self, records):
        written = [record for record in records if record["program"] is not None]
        with ThreadPoolExecutor(max_workers=ORACLE_THREADS) as pool:
            checked = list(pool.map(lambda r: self.oracle.verdict_of(r["id"], r["program"]), written))
        for record in records:
            record.update(compiler=None, codes=[], signal=False, disagreement=False, judges=None, confirmed=False)
        errors = {}
        for record, (result, _, found, _) in zip(written, checked):
            if found is None:
                record["signal"] = result.name == "ERROR"
                continue
            errors[record["id"]] = found
            record["compiler"] = "reject" if found else "accept"
            record["codes"] = sorted({error["code"] for error in found})
            record["disagreement"] = record["compiler"] != record["stated"]
        return errors

    def judge_disagreements(self, records, errors):
        disputed = [record for record in records if record["disagreement"]]
        if not disputed:
            return
        with ThreadPoolExecutor(max_workers=len(disputed)) as pool:
            answers = list(pool.map(lambda r: self.judge.answers_for(r["program"]), disputed))
        for record, given in zip(disputed, answers):
            if given is not None:
                record["judges"] = given
                record["confirmed"] = self.judge.conclude(record["id"], record["program"], errors[record["id"]],
                                                          f"targeted|{record['id']}", given)

    def settle(self, records):
        by_id = {requirement["id"]: requirement for requirement in self.requirements}
        for record in records:
            requirement = by_id[record["requirement"]]
            requirement["programs"] += 1
            requirement["disagreements"] += record["disagreement"]
            requirement["confirmed"] += record["confirmed"] + record["signal"]
        for requirement in self.requirements:
            requirement["yield"] = yield_of(requirement)

    def summarize(self, number, records):
        def tally(rows):
            return {"programs": len(rows), "disagreements": sum(r["disagreement"] for r in rows),
                    "judged": sum(r["judges"] is not None for r in rows),
                    "confirmed": sum(r["confirmed"] + r["signal"] for r in rows),
                    "signals": sum(r["signal"] for r in rows)}
        return {"round": number, **tally(records),
                "by_target": {target["name"]: tally([r for r in records if r["target"] == target["name"]])
                              for target in self.targets}}

    def play(self, number):
        self.write_requirements(number, self.fresh_targets())
        self.write("requirements.json", self.requirements)
        records = self.write_programs(number)
        errors = self.check(records)
        self.judge_disagreements(records, errors)
        self.settle(records)
        self.write("requirements.json", self.requirements)
        folder = self.run_dir / "rounds" / f"{number:02d}"
        folder.mkdir(parents=True, exist_ok=True)
        (folder / "programs.jsonl").write_text("".join(json.dumps(record) + "\n" for record in records))
        summary = self.summarize(number, records)
        self.write(f"rounds/{number:02d}/summary.json", summary)
        return summary

    def run(self, rounds):
        self.write("targets.json", self.targets)
        summaries = []
        for number in range(1, rounds + 1):
            summaries.append(self.play(number))
            print(json.dumps({key: value for key, value in summaries[-1].items() if key != "by_target"}), flush=True)
        self.judge.close()
        return summaries


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--proxy", default="http://127.0.0.1:8787")
    parser.add_argument("--run-dir", default=str(HERE / "runs" / "targeted"))
    parser.add_argument("--targets", default=str(HERE / "targets.yaml"))
    parser.add_argument("--samples", default=str(HERE / "samples"))
    parser.add_argument("--spec", default=str(HERE.parents[2] / "docs" / "SPEC.md"))
    parser.add_argument("--evolve-bin", default=str(HERE.parent / "target" / "release"))
    parser.add_argument("--requirements", type=int, default=6)
    parser.add_argument("--programs", type=int, default=4)
    parser.add_argument("--rounds", type=int, default=3)
    parser.add_argument("--seed", type=int)
    args = parser.parse_args(argv)
    run_dir = Path(args.run_dir)
    if (run_dir / "requirements.json").exists():
        print(f"{run_dir} already holds a run; name a new --run-dir", file=sys.stderr)
        return 1
    spec = Path(args.spec).read_text()
    targets = yaml.safe_load(Path(args.targets).read_text())["targets"]
    oracle = Oracle(Path(args.evolve_bin) / "evolve-check", Path(args.evolve_bin) / "evolve-shrink",
                    run_dir / "findings")
    loop = Loop(ProxyClient(args.proxy), spec, generation_system(spec, args.samples), targets, oracle, run_dir,
                args.requirements, args.programs, args.seed)
    try:
        loop.run(args.rounds)
    except ProxyRefused as refused:
        print(f"the proxy refused a call: {refused}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
