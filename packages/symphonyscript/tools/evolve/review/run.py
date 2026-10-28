import argparse
import json
import math
import os
import shutil
import subprocess
import time
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import verdicts
from programs import MAIN, key, read_jsonl, write_jsonl_gz

HERE = Path(__file__).resolve().parent
EVOLVE = HERE.parent
RUNS = EVOLVE / "review-runs"
CORPORA = EVOLVE / "corpora"
PACKAGE = "packages/symphonyscript"
OVERLAID = ("src", "Cargo.toml", "Cargo.lock")
SHARD = 2000


def git(repository, *arguments, check=True):
    return subprocess.run(["git", "-C", str(repository), *arguments], check=check, capture_output=True, text=True)


def short_sha(repository, revision):
    return git(repository, "rev-parse", "--short=7", "--verify", f"{revision}^{{commit}}").stdout.strip()


def default_baselines():
    common = Path(git(HERE, "rev-parse", "--path-format=absolute", "--git-common-dir").stdout.strip())
    return common.parent / ".memory" / "symphonyscript" / "baselines"


def previous_baseline(repository, sha, baselines):
    nearest = None
    for path in sorted(baselines.glob("*.jsonl.gz")):
        base = path.name.removesuffix(".jsonl.gz")
        if base == sha or git(repository, "merge-base", "--is-ancestor", base, sha, check=False).returncode != 0:
            continue
        distance = int(git(repository, "rev-list", "--count", f"{base}..{sha}").stdout)
        if nearest is None or distance < nearest[0]:
            nearest = (distance, path)
    return nearest[1] if nearest else None


def against(sha, baselines):
    recorded = baselines / f"{sha}.jsonl.gz"
    return recorded if recorded.exists() else RUNS / sha / "verdicts.jsonl.gz"


def built(sha, run):
    export = run / "export"
    archive = subprocess.run(["git", "-C", str(HERE), "archive", f"{sha}:{PACKAGE}"], check=True,
                             capture_output=True).stdout
    export.mkdir(parents=True)
    subprocess.run(["tar", "-x", "-C", str(export)], input=archive, check=True)
    tools = export / "tools" / "evolve"
    for name in OVERLAID:
        if (tools / name).is_dir():
            shutil.rmtree(tools / name)
        if (EVOLVE / name).is_dir():
            shutil.copytree(EVOLVE / name, tools / name)
        else:
            shutil.copy2(EVOLVE / name, tools / name)
    with open(run / "build.log", "w") as log:
        subprocess.run(["cargo", "build", "--release", "--bins"], cwd=tools, stdout=log, stderr=subprocess.STDOUT,
                       env={**os.environ, "CARGO_TARGET_DIR": str(run / "target")}, check=True)
    return run / "target" / "release"


def code_table(binaries):
    lines = subprocess.run([binaries / "evolve-codes"], check=True, capture_output=True, text=True).stdout
    return {row["code"]: row for row in map(json.loads, lines.splitlines())}


def programs_at(binaries, corpora):
    found = {}
    for corpus in sorted(corpora.glob("*.jsonl.gz")):
        for record in read_jsonl(corpus):
            found.setdefault(key(record), {**record, "set": corpus.name.removesuffix(".jsonl.gz")})
    mutants = subprocess.run([binaries / "evolve-mutants"], check=True, capture_output=True, text=True).stdout
    for program in map(json.loads, mutants.splitlines()):
        found.setdefault(key(program), {**program, "set": "mutants"})
    return found


def shard(binaries, programs, warnings, work, alone, index, vocabulary, members):
    given, taken = work / f"{index}.in.jsonl", work / f"{index}.out.jsonl"
    with open(given, "w", encoding="utf-8") as out:
        for program_key in members:
            out.write(json.dumps({"id": program_key, "sources": programs[program_key]["sources"]}) + "\n")
    finished = subprocess.run([binaries / "evolve-check", "--vocabulary", vocabulary, given, taken],
                              stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    found = {}
    if taken.exists():
        for line in taken.read_text(encoding="utf-8").splitlines():
            try:
                raw = json.loads(line)
            except json.JSONDecodeError:
                continue
            found[raw["id"]] = verdicts.compact(raw, warnings), verdicts.detail(raw)
    if alone:
        for program_key in members:
            found.setdefault(program_key, verdicts.aborted(finished.returncode))
    return found


def checked(binaries, programs, warnings, keys, jobs, work, alone):
    work.mkdir(parents=True, exist_ok=True)
    by_vocabulary = defaultdict(list)
    for program_key in keys:
        by_vocabulary[programs[program_key].get("vocabulary", MAIN)].append(program_key)
    shards = []
    for vocabulary, members in sorted(by_vocabulary.items()):
        count = len(members) if alone else max(jobs, math.ceil(len(members) / SHARD))
        shards += [(vocabulary, members[start::count]) for start in range(min(count, len(members)))]
    found = {}
    with ThreadPoolExecutor(jobs) as pool:
        for part in pool.map(lambda numbered: shard(binaries, programs, warnings, work, alone, numbered[0],
                                                    *numbered[1]), enumerate(shards)):
            found.update(part)
    return found


def checked_again(found, previous, keys):
    return [program_key for program_key in keys if program_key not in found
            or "slow" in (found[program_key][0]["outcome"], previous.get(program_key, {}).get("outcome"))]


def check(arguments):
    sha = short_sha(HERE, arguments.revision)
    run = RUNS / sha
    shutil.rmtree(run, ignore_errors=True)
    run.mkdir(parents=True)
    load = os.getloadavg()[0]
    started = time.monotonic()
    binaries = built(sha, run)
    build_seconds = time.monotonic() - started
    table = code_table(binaries)
    warnings = {code for code, row in table.items() if row["warning"]}
    programs = programs_at(binaries, CORPORA)
    if arguments.against:
        base = short_sha(HERE, arguments.against)
        baseline = against(base, arguments.baselines)
    else:
        baseline = previous_baseline(HERE, sha, arguments.baselines)
        base = baseline.name.removesuffix(".jsonl.gz") if baseline else None
    previous = {record["key"]: record for record in read_jsonl(baseline)} if baseline else {}
    started = time.monotonic()
    found = checked(binaries, programs, warnings, list(programs), arguments.jobs, run / "work" / "shards", False)
    again = checked_again(found, previous, list(programs))
    found.update(checked(binaries, programs, warnings, again, arguments.jobs, run / "work" / "alone", True))
    check_seconds = time.monotonic() - started
    current = {program_key: {"key": program_key, "set": programs[program_key]["set"], **found[program_key][0]}
               for program_key in sorted(programs)}
    details = {program_key: found[program_key][1] for program_key in programs if found[program_key][1]}
    write_jsonl_gz(run / "verdicts.jsonl.gz", current.values())
    notes = [f"Machine time: build {build_seconds:.0f} s, check {check_seconds:.0f} s with {arguments.jobs} jobs; "
             f"1-minute load {load:.1f} before, {os.getloadavg()[0]:.1f} after.",
             f"Checked again alone, as slow now or at the baseline or without a verdict from their shard: "
             f"{len(again)}; slow after: {sum(found[k][0]['outcome'] == 'slow' for k in again)}."]
    heading = f"Verdicts at {sha} against {base}" if base else f"Verdicts at {sha}, no baseline"
    (run / "diff.md").write_text(verdicts.render(heading, programs, current, previous, details, table, notes))
    for name in ("export", "target", "work"):
        shutil.rmtree(run / name)
    print(run / "diff.md")


def record(arguments):
    sha = short_sha(HERE, arguments.revision)
    arguments.baselines.mkdir(parents=True, exist_ok=True)
    target = arguments.baselines / f"{sha}.jsonl.gz"
    shutil.copy2(RUNS / sha / "verdicts.jsonl.gz", target)
    print(target)


def main():
    parser = argparse.ArgumentParser(description="Check every corpus and the generated mutants at a commit and "
                                                 "diff the verdicts against the previous reviewed commit's baseline.")
    commands = parser.add_subparsers(dest="command", required=True)
    checking = commands.add_parser("check", help="write review-runs/<sha>/verdicts.jsonl.gz and diff.md")
    checking.add_argument("revision", nargs="?", default="HEAD")
    checking.add_argument("--against", help="a commit whose recorded baseline, or else whose last run, to diff "
                                           "against instead of the nearest ancestor's baseline")
    checking.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    recording = commands.add_parser("record", help="record review-runs/<sha>/verdicts.jsonl.gz as the baseline")
    recording.add_argument("revision", nargs="?", default="HEAD")
    for command in (checking, recording):
        command.add_argument("--baselines", type=Path, default=None)
    arguments = parser.parse_args()
    arguments.baselines = arguments.baselines or default_baselines()
    {"check": check, "record": record}[arguments.command](arguments)


if __name__ == "__main__":
    main()
