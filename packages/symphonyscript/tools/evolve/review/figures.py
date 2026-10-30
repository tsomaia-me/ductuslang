import gzip
import json
import math
import statistics
import subprocess
import tempfile
from pathlib import Path

from programs import key, read_jsonl

CALLS = 1_000_000
CALLS_PER_NOTE = 100
DEPTH = 100_000
LEVELS = 20
RUNS = 3
LOAD_BELOW = 4.0
WAIT_SECONDS = 600
BREAKER_SET = "breakers"
BUDGET_SECONDS = 600
TARGET_US = 2
BREAKER_TARGETS_MS = {"median": 0.5, "p99": 2}
CONSTRUCTS = ("parentheses", "array and index", "call", "not", "interpolation", "negation", "pitch parentheses")
RECURSIVE = ("A compile-time recursive call", "a program making 1,000,000 distinct calls")
INLINED = ("An inlined call", "a program inlining 1,000,000 calls")
NESTING = ("A level of nesting", "the deep-expressions fixture at depth 100,000, each construct")
BREAKERS = ("A program of the breaker corpus", "Check over every program, median and 99th percentile")


def sizes_totalling(total):
    remaining, levels = total, []
    for level in reversed(range(LEVELS)):
        size = (1 << (level + 1)) - 1
        while remaining >= size:
            remaining -= size
            levels.append(level)
    return levels


def recursing(total):
    head = "fn g(t, a, b): if a < 1 then 0 else g(t, a - 1, 2 * b) + g(t, a - 1, 2 * b + 1)\nClip c:\n"
    return head + "".join(f"    Note/(g({tree}, {level}, 0))\n"
                          for tree, level in enumerate(sizes_totalling(total)))


def inlining(total):
    note = "    Note/(" + " + ".join(f"f(i, {index})" for index in range(CALLS_PER_NOTE)) + ")\n"
    return "input i = 1\nfn f(x, k): x + k\nClip c:\n" + note * (total // CALLS_PER_NOTE)


def nested(opening, middle, closing, depth):
    return opening * depth + middle + closing * depth


def deep_expressions(depths):
    depth = {construct: depths.get(construct, 1) for construct in CONSTRUCTS}
    interpolated = nested('"{', "1", '}"', depth["interpolation"])
    return (f"input x = true\nfn f(v): v\n"
            f"let a = {nested('(', '1', ')', depth['parentheses'])}\n"
            f"let b = {nested('[', '1', ']', depth['array and index'])}\n"
            f"let c = {nested('f(', '1', ')', depth['call'])}\n"
            f"let d = {'not ' * depth['not']}x\n"
            f"let e = {interpolated}\n"
            f"let g = {'-' * depth['negation']}1\n"
            f"Clip top:\n"
            f"    Note/{nested('(', '60', ')', depth['pitch parentheses'])} when d\n"
            f"    Note/(f(a) + c + g + b{'[0]' * depth['array and index']})\n")


def measured_programs():
    programs = [(RECURSIVE, "recursive", recursing(CALLS), CALLS),
                (INLINED, "inlined", inlining(CALLS), CALLS)]
    programs += [(NESTING, construct, deep_expressions({construct: DEPTH}), DEPTH) for construct in CONSTRUCTS]
    programs.append((NESTING, "every construct", deep_expressions(dict.fromkeys(CONSTRUCTS, DEPTH)),
                     DEPTH * len(CONSTRUCTS)))
    return programs


def ranked(finished, count, quantile):
    index = max(0, math.ceil(quantile * count) - 1)
    return finished[index] if index < len(finished) else None


def waited(below):
    if not math.isfinite(below):
        return "at any load"
    return f"once the 1-minute load is below {below} or the figures have waited {WAIT_SECONDS // 60} minutes"


def breaker_figure(times, elapsed, load, below):
    finished = sorted(ms for ms in elapsed.values() if ms is not None)
    return {"work": BREAKERS[0], "measured_by": BREAKERS[1], "targets_ms": BREAKER_TARGETS_MS,
            "programs": len(elapsed), "past_budget": len(elapsed) - len(finished),
            "median_ms": ranked(finished, len(elapsed), 0.5), "p99_ms": ranked(finished, len(elapsed), 0.99),
            "max_timed_ms": finished[-1] if finished else None, "load": round(load, 2),
            "method": f"Compiler::check as evolve-time times it on evolve-check's compile thread, counting "
                      f"allocations, each {BREAKER_SET} program once in one process, one job, {times['profile']} "
                      f"profile, after every other command of the step and {waited(below)}; a program the verdict "
                      f"check found past its {times['budget_s']} s budget is not run again, and it and a program "
                      f"past the budget here rank above every timed one and read as null; nearest-rank percentiles "
                      f"over all {len(elapsed)} programs"}


def timed(binaries, programs, work, budget):
    given, taken = work / "given.jsonl", work / "taken.jsonl"
    with open(given, "w", encoding="utf-8") as out:
        for program in programs:
            out.write(json.dumps(program) + "\n")
    subprocess.run([binaries / "evolve-time", "--budget", str(budget), given, taken], check=True)
    return [json.loads(line) for line in taken.read_text(encoding="utf-8").splitlines() if line.strip()]


def breaker_elapsed(binaries, work, times, corpus):
    sources = {key(record): record["sources"] for record in read_jsonl(corpus)}
    rows = [row for row in times["programs"] if row["set"] == BREAKER_SET]
    again = [{"id": row["key"], "sources": sources[row["key"]]} for row in rows if row["elapsed_ms"] is not None]
    taken = {run["id"]: run["elapsed_ms"] for run in timed(binaries, again, work, times["budget_s"])}
    return {row["key"]: taken.get(row["key"]) for row in rows}


def program_figure(binaries, work, figure, label, text, units, load, below):
    runs = timed(binaries, [{"id": f"{label}#{number}", "sources": [{"path": "main.sym", "text": text}]}
                            for number in range(RUNS)], work, BUDGET_SECONDS)
    elapsed = [run["elapsed_ms"] for run in runs]
    median = statistics.median(elapsed) if None not in elapsed else None
    return {"work": figure[0], "measured_by": figure[1], "program": label, "target_us": TARGET_US,
            "units": units, "characters": len(text), "load": round(load, 2), "runs_ms": elapsed,
            "median_ms": median, "per_unit_us": median * 1000 / units if median is not None else None,
            "diagnostics": runs[0].get("diagnostics"), "codes": runs[0].get("codes"), "panic": runs[0].get("panic"),
            "method": f"Compiler::check as evolve-time times it on evolve-check's compile thread, counting "
                      f"allocations, {RUNS} runs in one process, one job, after every other command of the step "
                      f"and {waited(below)}, the median divided by the {units} units; "
                      f"the program's other text is counted in its time"}


def shown(figure):
    if "per_unit_us" in figure:
        per_unit = "past the budget" if figure["per_unit_us"] is None else f"{figure['per_unit_us']:.3f} us"
        return f"{figure['work']}, {figure['program']}: {per_unit} per unit, load {figure['load']}"
    return (f"{figure['work']}: median {figure['median_ms']} ms, p99 {figure['p99_ms']} ms over "
            f"{figure['programs']} programs, {figure['past_budget']} past the budget, load {figure['load']:.2f}")


def measure(binaries, times_path, corpus, out_path, below, settle):
    with gzip.open(times_path, "rt", encoding="utf-8") as raw:
        times = json.load(raw)
    with tempfile.TemporaryDirectory() as directory:
        work = Path(directory)
        load = settle()
        figures = [breaker_figure(times, breaker_elapsed(binaries, work, times, corpus), load, below)]
        print(shown(figures[0]), flush=True)
        for figure, label, text, units in measured_programs():
            load = settle()
            figures.append(program_figure(binaries, work, figure, label, text, units, load, below))
            print(shown(figures[-1]), flush=True)
    out_path.write_text(json.dumps({"profile": times["profile"], "load_below": below if math.isfinite(below) else None,
                                    "figures": figures}, indent=2) + "\n")
