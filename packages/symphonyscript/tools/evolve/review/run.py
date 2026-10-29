import argparse
import gzip
import json
import math
import os
import platform
import shutil
import subprocess
import sys
import tempfile
import time
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
from pathlib import Path

import figures
import verdicts
from programs import MAIN, PROBES, key, probe_records, read_jsonl, write_jsonl_gz

HERE = Path(__file__).resolve().parent
EVOLVE = HERE.parent
RUNS = EVOLVE / "review-runs"
CORPORA = EVOLVE / "corpora"
PACKAGE = "packages/symphonyscript"
OVERLAID = ("src", "Cargo.toml", "Cargo.lock")
SHARD = 2000
BUDGET_SECONDS = 5
PROFILES = {"dev": "debug", "release": "release"}
CHECK_OUTPUTS = ("verdicts.jsonl.gz", "diff.md", "diff-full.md", "check-times.json.gz", "build.log")
BUSY_LOAD = 12.0
QUIET_LOAD = 6.0
POLL_SECONDS = 15
THUMB = "thumbv7em-none-eabihf"
EXHAUSTIVE = "test(/(^|::)exhaustive_/)"
STRESS = "test(/(^|::)stress_/)"


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


def built(sha, run, build, profile):
    export = build / "export"
    top = git(HERE, "rev-parse", "--show-toplevel").stdout.strip()
    archive = subprocess.run(["git", "-C", top, "archive", f"{sha}:{PACKAGE}"], check=True, capture_output=True).stdout
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
        subprocess.run(["cargo", "build", "--profile", profile, "--bins"], cwd=tools, stdout=log,
                       stderr=subprocess.STDOUT, env={**os.environ, "CARGO_TARGET_DIR": str(build / "target")},
                       check=True)
    return build / "target" / PROFILES[profile]


def code_table(binaries):
    lines = subprocess.run([binaries / "evolve-codes"], check=True, capture_output=True, text=True).stdout
    return {row["code"]: row for row in map(json.loads, lines.splitlines())}


def programs_at(binaries, corpora):
    found = {}
    for corpus in sorted(corpora.glob("*.jsonl.gz")):
        name = corpus.name.removesuffix(".jsonl.gz")
        for record in read_jsonl(corpus):
            for program in probe_records(record) if name == PROBES else [record]:
                found.setdefault(key(program), {**program, "set": name})
    mutants = subprocess.run([binaries / "evolve-mutants"], check=True, capture_output=True, text=True).stdout
    for program in map(json.loads, mutants.splitlines()):
        found.setdefault(key(program), {**program, "set": "mutants"})
    return found


def shard(binaries, programs, warnings, work, alone, index, vocabulary, members):
    given, taken = work / f"{index}.in.jsonl", work / f"{index}.out.jsonl"
    with open(given, "w", encoding="utf-8") as out:
        for program_key in members:
            out.write(json.dumps({"id": program_key, "sources": programs[program_key]["sources"]}) + "\n")
    finished = subprocess.run([binaries / "evolve-check", "--budget", str(BUDGET_SECONDS), "--vocabulary", vocabulary,
                               given, taken], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    found = {}
    if taken.exists():
        for line in taken.read_text(encoding="utf-8").splitlines():
            try:
                raw = json.loads(line)
            except json.JSONDecodeError:
                continue
            found[raw["id"]] = verdicts.compact(raw, warnings), verdicts.detail(raw), raw["elapsed_ms"]
    if alone:
        for program_key in members:
            found.setdefault(program_key, (*verdicts.aborted(finished.returncode), None))
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


def unknown_tags(programs):
    return {program_key: (*verdicts.unknown_tag(program["tag"]), None) for program_key, program in programs.items()
            if program.get("vocabulary", MAIN) is None}


def unknown_tag_note(programs, keys):
    named = "".join(f"; {programs[program_key]['origin']} (#{programs[program_key]['tag']})"
                    for program_key in sorted(keys, key=lambda program_key: programs[program_key]["origin"]))
    return f"Probes whose tag names no vocabulary, reported and not checked: {len(keys)}{named}."


def checked_again(found, previous, keys):
    return [program_key for program_key in keys if program_key not in found
            or "slow" in (found[program_key][0]["outcome"], previous.get(program_key, {}).get("outcome"))]


def check(arguments):
    sha = short_sha(HERE, arguments.revision)
    run = RUNS / sha
    run.mkdir(parents=True, exist_ok=True)
    for name in CHECK_OUTPUTS:
        (run / name).unlink(missing_ok=True)
    build = arguments.build or run / "build"
    if not arguments.build:
        shutil.rmtree(build, ignore_errors=True)
    load = os.getloadavg()[0]
    started = time.monotonic()
    binaries = built(sha, run, build, arguments.profile)
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
    unknown = unknown_tags(programs)
    named = [program_key for program_key in programs if program_key not in unknown]
    check_load, started = settled(arguments.settle_below), time.monotonic()
    found = checked(binaries, programs, warnings, named, arguments.jobs, build / "work" / "shards", False)
    again = checked_again(found, previous, named)
    found.update(checked(binaries, programs, warnings, again, arguments.jobs, build / "work" / "alone", True))
    found.update(unknown)
    check_seconds = time.monotonic() - started
    current = {program_key: {"key": program_key, "set": programs[program_key]["set"], **found[program_key][0]}
               for program_key in sorted(programs)}
    details = {program_key: found[program_key][1] for program_key in programs if found[program_key][1]}
    write_jsonl_gz(run / "verdicts.jsonl.gz", current.values())
    times = {"profile": arguments.profile, "jobs": arguments.jobs, "budget_s": BUDGET_SECONDS,
             "load": round(check_load, 2),
             "programs": [{"key": program_key, "set": programs[program_key]["set"],
                           "elapsed_ms": found[program_key][2]} for program_key in sorted(programs)]}
    with gzip.open(run / "check-times.json.gz", "wt", encoding="utf-8") as out:
        json.dump(times, out)
    notes = [f"Machine time: build {build_seconds:.0f} s in the {arguments.profile} profile, check "
             f"{check_seconds:.0f} s with {arguments.jobs} jobs; "
             f"1-minute load {load:.1f} before, {os.getloadavg()[0]:.1f} after.",
             f"Checked again alone, as slow now or at the baseline or without a verdict from their shard: "
             f"{len(again)}; slow after: {sum(found[k][0]['outcome'] == 'slow' for k in again)}.",
             unknown_tag_note(programs, unknown)]
    heading = f"Verdicts at {sha} against {base}" if base else f"Verdicts at {sha}, no baseline"
    (run / "diff-full.md").write_text(verdicts.render(heading, programs, current, previous, details, table, notes))
    (run / "diff.md").write_text(verdicts.summary(heading, programs, current, previous, details, table, notes,
                                                  "diff-full.md"))
    shutil.rmtree(build / "work")
    if not arguments.build:
        shutil.rmtree(build)
    print(run / "diff.md")


def record(arguments):
    sha = short_sha(HERE, arguments.revision)
    arguments.baselines.mkdir(parents=True, exist_ok=True)
    target = arguments.baselines / f"{sha}.jsonl.gz"
    shutil.copy2(RUNS / sha / "verdicts.jsonl.gz", target)
    print(target)


def settled(below, poll=POLL_SECONDS):
    while True:
        load = os.getloadavg()[0]
        if below is None or load < below:
            return load
        time.sleep(poll)


def limit(below):
    return below if below is not None and math.isfinite(below) else None


def now():
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def step(name, command, cwd, env=None, below=None):
    return {"name": name, "command": [str(part) for part in command], "cwd": str(cwd), "env": env or {},
            "below": below}


def performed(planned, results, settle=settled):
    load = settle(planned["below"])
    log = results / f"{planned['name']}.log"
    started, clock = now(), time.monotonic()
    with open(log, "w") as out:
        try:
            code = subprocess.run(planned["command"], cwd=planned["cwd"], env={**os.environ, **planned["env"]},
                                  stdout=out, stderr=subprocess.STDOUT).returncode
        except OSError as error:
            out.write(f"{error}\n")
            code = 127
    return {"name": planned["name"], "command": planned["command"], "cwd": planned["cwd"], "env": planned["env"],
            "waited_for_load_below": limit(planned["below"]), "load": round(load, 2), "started": started,
            "seconds": round(time.monotonic() - clock, 1), "exit": code, "log": log.name}


def ignored_tests(name, expression, package, cargo, below):
    return step(name, ["cargo", "nextest", "run", "--workspace", "--run-ignored", "all", "-E", expression], package,
                cargo, below)


def shared_steps(sha, top, into, run, jobs, baselines, against, quiet=QUIET_LOAD, stress=False, exhaustive=False):
    export, package = into / "export", into / "export" / PACKAGE
    cargo = {"CARGO_TARGET_DIR": str(into / "target"), "CARGO_BUILD_JOBS": str(jobs),
             "NEXTEST_TEST_THREADS": str(jobs), "RUST_TEST_THREADS": str(jobs)}
    verdict_diff = [sys.executable, HERE / "run.py", "check", sha, "--profile", "dev", "--jobs", jobs,
                    "--settle-below", quiet, "--build", into / "check", "--baselines", baselines, *(["--against", against] if against else [])]
    return [step("archive", ["git", "-C", top, "archive", "--output", into / "export.tar", sha, PACKAGE], top),
            step("extract", ["tar", "-x", "-f", into / "export.tar", "-C", export], into),
            step("rustc", ["rustc", "--version", "--verbose"], package),
            step("cargo-nextest", ["cargo", "nextest", "--version"], package),
            step("build", ["cargo", "build", "--workspace", "--all-targets"], package, cargo, BUSY_LOAD),
            step("fmt", ["cargo", "fmt", "--check"], package, cargo, BUSY_LOAD),
            step("clippy", ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"], package,
                 cargo, BUSY_LOAD),
            step("nextest", ["cargo", "nextest", "run", "--workspace"], package, cargo, BUSY_LOAD),
            *([ignored_tests("nextest-exhaustive", EXHAUSTIVE, package, cargo, BUSY_LOAD)] if exhaustive else []),
            *([ignored_tests("nextest-stress", STRESS, package, cargo, quiet)] if stress else []),
            step("doctest", ["cargo", "test", "--workspace", "--doc"], package, cargo, BUSY_LOAD),
            step("thumbv7em", ["cargo", "build", "-p", "symphonyscript-core", "--target", THUMB], package, cargo,
                 BUSY_LOAD),
            step("verdicts", verdict_diff, HERE, None, BUSY_LOAD),
            step("figures", [sys.executable, HERE / "run.py", "figures", into / "check" / "target" / PROFILES["dev"],
                             run / "check-times.json.gz", run / "figures.json"], HERE, None, figures.LOAD_BELOW)]


def manifest(tip, into, run, records, started, jobs, quiet=QUIET_LOAD, stress=False, exhaustive=False):
    pipeline = git(HERE, "rev-parse", "HEAD").stdout.strip()
    edited = git(HERE, "status", "--porcelain", "--", str(EVOLVE)).stdout.strip()
    return {"tip": tip, "export": str(into / "export" / PACKAGE), "target": str(into / "target"),
            "evolve_binaries": str(into / "check" / "target" / PROFILES["dev"]), "profile": "dev", "jobs": jobs,
            "ignored_tests": {"exhaustive": exhaustive, "stress": stress},
            "load_limits": {"builds_and_checks": BUSY_LOAD, "timed_steps": limit(quiet),
                            "figures": figures.LOAD_BELOW},
            "pipeline": {"commit": pipeline, "uncommitted_changes": bool(edited)},
            "host": {"name": platform.node(), "system": platform.platform(), "cpus": os.cpu_count()},
            "started": started, "written": now(),
            "results": sorted(path.name for path in run.iterdir() if path.name != "manifest.json"),
            "commands": records}


def shared(arguments):
    sha = short_sha(HERE, arguments.revision)
    tip = git(HERE, "rev-parse", "--verify", f"{sha}^{{commit}}").stdout.strip()
    top = Path(git(HERE, "rev-parse", "--show-toplevel").stdout.strip())
    into = (arguments.into or Path(tempfile.mkdtemp(prefix=f"symphonyscript-{sha}-"))).resolve()
    into.mkdir(parents=True, exist_ok=True)
    if any(into.iterdir()):
        sys.exit(f"{into} is not empty; the shared step exports into a new directory")
    (into / "export").mkdir()
    run = RUNS / sha
    shutil.rmtree(run, ignore_errors=True)
    run.mkdir(parents=True)
    started, records = now(), []
    for planned in shared_steps(sha, top, into, run, arguments.jobs, arguments.baselines, arguments.against,
                                arguments.quiet_below, arguments.stress, arguments.exhaustive):
        records.append(performed(planned, run))
        print(f"{planned['name']}: exit {records[-1]['exit']}, {records[-1]['seconds']} s, "
              f"load {records[-1]['load']}", flush=True)
        shape = manifest(tip, into, run, records, started, arguments.jobs, arguments.quiet_below, arguments.stress,
                         arguments.exhaustive)
        (run / "manifest.json").write_text(json.dumps(shape, indent=2) + "\n")
    print(run / "manifest.json")
    sys.exit(0 if all(record["exit"] == 0 for record in records) else 1)


def measured(arguments):
    figures.measure(arguments.binaries, arguments.times, CORPORA / f"{figures.BREAKER_SET}.jsonl.gz", arguments.out,
                    lambda: settled(figures.LOAD_BELOW))


def main():
    parser = argparse.ArgumentParser(description="Check every corpus and the generated mutants at a commit and "
                                                 "diff the verdicts against the previous reviewed commit's baseline.")
    commands = parser.add_subparsers(dest="command", required=True)
    checking = commands.add_parser("check", help="write review-runs/<sha>/verdicts.jsonl.gz and diff.md")
    checking.add_argument("revision", nargs="?", default="HEAD")
    checking.add_argument("--against", help="a commit whose recorded baseline, or else whose last run, to diff "
                                           "against instead of the nearest ancestor's baseline")
    checking.add_argument("--profile", choices=sorted(PROFILES), default="release",
                          help="the cargo profile the evolve tools build in")
    checking.add_argument("--build", type=Path, default=None,
                          help="a new directory to export and build in, kept after the run; by default "
                               "review-runs/<sha>/build, deleted after it")
    checking.add_argument("--settle-below", type=float, default=None,
                          help="wait after the build until the 1-minute load is below this before checking, "
                               "since the check times every program")
    sharing = commands.add_parser("shared", help="the shared step before a review: export the tip, build it in the "
                                                 "dev profile, run the four checks, the verdict diff and the speed "
                                                 "figures, and write review-runs/<sha>/manifest.json beside every "
                                                 "log and result; cargo-nextest must be on PATH")
    sharing.add_argument("revision", nargs="?", default="HEAD")
    sharing.add_argument("--into", type=Path, default=None,
                         help="a new directory for the export and its builds, kept for the reviewers' probes; "
                              "by default a new temporary directory")
    sharing.add_argument("--against", help="as for check")
    sharing.add_argument("--exhaustive", action="store_true",
                         help="also run the exhaustive tests after the unit tests")
    sharing.add_argument("--stress", action="store_true",
                         help="also run the stress tests after the unit and exhaustive tests, once the load is below "
                              "--quiet-below")
    sharing.add_argument("--quiet-below", type=float, default=QUIET_LOAD,
                         help="the 1-minute load the stress tests and the verdict check wait to fall below; inf "
                              "takes them at any load, and the manifest records the load each began at; the figures "
                              f"always wait for a load below {figures.LOAD_BELOW:g}")
    for command in (checking, sharing):
        command.add_argument("--jobs", type=int, default=max(1, (os.cpu_count() or 2) // 2))
    recording = commands.add_parser("record", help="record review-runs/<sha>/verdicts.jsonl.gz as the baseline")
    recording.add_argument("revision", nargs="?", default="HEAD")
    for command in (checking, sharing, recording):
        command.add_argument("--baselines", type=Path, default=None)
    measuring = commands.add_parser("figures", help="take the speed figures with the evolve tools' binaries and "
                                                    "a check's check-times.json.gz")
    measuring.add_argument("binaries", type=Path)
    measuring.add_argument("times", type=Path)
    measuring.add_argument("out", type=Path)
    arguments = parser.parse_args()
    if arguments.command != "figures":
        arguments.baselines = arguments.baselines or default_baselines()
    {"check": check, "record": record, "shared": shared, "figures": measured}[arguments.command](arguments)


if __name__ == "__main__":
    main()
