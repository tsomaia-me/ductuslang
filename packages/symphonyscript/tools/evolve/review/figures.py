import gzip
import json
import math
import statistics
import subprocess
import tempfile
from collections import defaultdict
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
PROFILES = ("dev", "release")
GUARD = 1.3
FRAME_TERMS = 8_000
VALUE_TERMS = 500
SITE_TERMS = 250
CONSTRUCTS = ("parentheses", "array and index", "call", "not", "interpolation", "negation", "pitch parentheses")
RECURSIVE = ("A compile-time recursive call", "a program making {calls:,} distinct calls")
INLINED = ("An inlined call", "a program inlining {calls:,} calls")
NESTING = ("A level of nesting", "the deep-expressions fixture at depth {depth:,}, each construct")
BREAKERS = ("A program of the breaker corpus", "Check over every program, median and 99th percentile")
QUICK_BREAKERS = ("A program of the breaker corpus",
                  "Check over the {breakers} slowest programs, median and 99th percentile of each one's median")
NEW_CORE = ("A program the new core takes",
            "Check over every breaker program the verdict check found the new core taking, median and 99th "
            "percentile, and the median time a syntax node")
QUICK_NEW_CORE = ("A program the new core takes",
                  "Check over the {breakers} slowest breaker programs the verdict check found the new core taking, "
                  "median and 99th percentile of each one's median, and the median time a syntax node")
NEW_CORE_HEADING = "New core"
NEW = "new"
ENGINES = (NEW, "old")
UNKNOWN_ENGINE = "unknown"
MIXED_ENGINES = "mixed"
EXPANSION = ("An expanded item", "`Rig r:` / `for i in 0..n:` / `Lamp/(1 + i % 12) | hue=0.5` at {sizes}")
REFERENCE_USES = ("A use of a pattern with a reference parameter", "a pattern with a reference parameter used {sizes} "
                                                                   "times")
RINGS = ("A cell of a ring of history reads",
         "X9's rings of {sizes} cells, each cell reading its neighbours' history both ways, beside the ring read "
         "one way")
FRAMES = ("A frame of a compile-time recursion", "X8's frame program, {terms:,} terms at depths {sizes}")
PLACEHOLDER = ("A placeholder beside a recursion", "a placeholder beside a recursion of {sizes} distinct calls")
VALUE_SHAPED = ("A value-shaped body",
                "a {body:,}-node body whose parameter bounds a slice, called at {sizes} distinct values")
DERIVED_SITES = ("A site passing an unannotated derived cell", "a {body:,}-node function called at {sizes} sites, each "
                                                               "passing its own unannotated derived cell")
RECOMPILE = ("A whole-project recompile", "the 10,000-line fixture project, its syntax nodes and expanded items "
                                          "counted")
MEMORY = ("Peak memory", "the fixture project and the expansion program at {sizes}, the compile's own allocations")
LIMIT_WARNING = "limit-warning"
NO_ENTRY = "no-entry-instance"
UNKNOWN_NAME = "unknown-name"
FIXTURE_CODES = (NO_ENTRY,)
FULL = {"name": "full", "calls": CALLS, "depth": DEPTH, "breakers": None,
        "called": (LIMIT_WARNING, NO_ENTRY), "nested": (NO_ENTRY,),
        "expansion": (50_000, 100_000, 200_000), "shape": (1_000, 2_000, 4_000), "frames": (200, 400, 800)}
QUICK = {"name": "quick", "calls": CALLS // 10, "depth": DEPTH // 10, "breakers": 200,
         "called": (NO_ENTRY,), "nested": (NO_ENTRY,),
         "expansion": (5_000, 10_000, 20_000), "shape": (100, 200, 400), "frames": (20, 40, 80)}
NESTING_WARNED = 512
RAISED_LIMITS = {"inlined_calls": CALLS, "inlined_calls_warning": CALLS // 2}
QUICK_NAME = "figures-quick.json"
VOID_EXIT = 3


def guarded(release):
    return round(release * GUARD, 6)


TARGETS_US = {RECURSIVE[0]: {"release": 1.7, "dev": 2.24}, INLINED[0]: {"release": 1.5, "dev": 2.0},
              NESTING[0]: {"release": 1.5, "dev": 2.0}, EXPANSION[0]: {"release": 0.7, "dev": guarded(0.7)}}
BREAKER_TARGETS_MS = {"release": {"median": 0.38, "p99": 1.5}, "dev": {"median": 0.5, "p99": 2.0}}
RECOMPILE_TARGETS_US = {"release": {"node": 1.0, "item": 0.7}, "dev": {"node": guarded(1.0), "item": guarded(0.7)}}
MEMORY_TARGETS_B = {"node": 256, "item": 112}
NEW_CORE_NODE_TARGETS_US = {profile: RECOMPILE_TARGETS_US[profile]["node"] for profile in PROFILES}


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
    return "state i(1)\nfn f(x, k): x + k\nClip c:\n" + note * (total // CALLS_PER_NOTE)


def nested(opening, middle, closing, depth):
    return opening * depth + middle + closing * depth


def deep_expressions(depths):
    depth = {construct: depths.get(construct, 1) for construct in CONSTRUCTS}
    interpolated = nested('"{', "1", '}"', depth["interpolation"])
    return (f"state x(true)\nfn f(v): v\n"
            f"let a = {nested('(', '1', ')', depth['parentheses'])}\n"
            f"let b = {nested('[', '1', ']', depth['array and index'])}\n"
            f"let c = {nested('f(', '1', ')', depth['call'])}\n"
            f"state d = {'not ' * depth['not']}x\n"
            f"let e = {interpolated}\n"
            f"let g = {'-' * depth['negation']}1\n"
            f"Clip top:\n"
            f"    Note/{nested('(', '60', ')', depth['pitch parentheses'])} when d\n"
            f"    Note/(f(a) + c + g + b{'[0]' * depth['array and index']})\n")


def expansion(iterations):
    return f"Rig r:\n    for i in 0..{iterations}:\n        Lamp/(1 + i % 12) | hue=0.5\n"


def reference_uses(uses):
    return ("Clip base | gain=0.5\npattern Echo(default source):\n    Note | velocity=source.gain\nClip c:\n"
            + "    Echo/base\n" * uses)


def ring(cells, start, read):
    last = cells - 1
    lines = ["state s(true)\n", f"state c0 = if s then c{last}.past(1, {start}) else {start}\n"]
    lines += [f"state c{k} = if s then c{k - 1}.past(1, {start}) else c{k + 1}.past(1, {start})\n"
              for k in range(1, last)]
    lines.append(f"state c{last} = if s then c{last - 1}.past(1, {start}) else c0.past(1, {start})\n")
    return "".join(lines) + f"Clip c:\n    Note/({read})\n"


def one_way_ring(cells, start, read):
    lines = ["state s(true)\n", f"state c0 = if s then c{cells - 1}.past(1, {start}) else {start}\n"]
    lines += [f"state c{k} = if s then c{k - 1}.past(1, {start}) else {start}\n" for k in range(1, cells)]
    return "".join(lines) + f"Clip c:\n    Note/({read})\n"


RINGED = (("scalar ring", ring, "5i64", "i32(c0)"), ("fixed-array ring", ring, "[7]", "c0[0]"),
          ("variable-array ring", ring, "[]", "c0.size"), ("one-way ring", one_way_ring, "5i64", "i32(c0)"))


def frames(depth, terms=FRAME_TERMS):
    return (f"fn f(n): if n < 1 then 0 else f(n - 1){' + n' * terms}\nlet x = f({depth})\n"
            f"Clip c:\n    Note/(x % 100)\n")


def placeholder_beside(calls):
    return recursing(calls) + "    Note/(zz)\n"


def value_shaped(values, terms=VALUE_TERMS):
    calls = "".join(f"    Note/(f({value}))\n" for value in range(1, values + 1))
    return f"state xs([]): i32[..8]\nfn f(k): xs[..k].size{' + k' * terms}\nClip c:\n{calls}"


def derived_sites(sites, terms=SITE_TERMS):
    body = " + ".join(f"x * 0.{index:03d}" for index in range(1, terms + 1))
    cells = "".join(f"state v{site} = a * {site + 2}\n" for site in range(sites))
    uses = "".join(f"    Note | velocity=shape(v{site})\n" for site in range(sites))
    return f"state a(1.0)\nfn shape(x): {body}\n{cells}Clip c:\n{uses}"


def stated(figure, scale):
    return figure[0], figure[1].format(**scale)


def listed_sizes(sizes):
    return f"{sizes[0]:,}, {sizes[1]:,} and {sizes[2]:,}"


def stated_shape(figure, sizes, **more):
    return figure[0], figure[1].format(sizes=listed_sizes(sizes), **more)


def measured_programs(scale=FULL):
    calls, depth, called, nested = scale["calls"], scale["depth"], scale["called"], scale["nested"]
    programs = [(stated(RECURSIVE, scale), "recursive", recursing(calls), calls, called),
                (stated(INLINED, scale), "inlined", inlining(calls), calls, called)]
    programs += [(stated(NESTING, scale), construct, deep_expressions({construct: depth}), depth, nested)
                 for construct in CONSTRUCTS]
    programs.append((stated(NESTING, scale), "every construct", deep_expressions(dict.fromkeys(CONSTRUCTS, depth)),
                     depth * len(CONSTRUCTS), nested))
    return programs


def frame_codes(depth):
    return (LIMIT_WARNING, NO_ENTRY) if depth > NESTING_WARNED else (NO_ENTRY,)


def measured_shapes(scale=FULL):
    expanded, shaped, depths = scale["expansion"], scale["shape"], scale["frames"]
    entry = (NO_ENTRY,)
    shapes = [(stated_shape(EXPANSION, expanded), "expansion", [(size, expansion(size), size, entry)
                                                                for size in expanded]),
              (stated_shape(REFERENCE_USES, shaped), "reference uses",
               [(size, reference_uses(size), size, entry) for size in shaped])]
    shapes += [(stated_shape(RINGS, shaped), label, [(size, shape(size, start, read), size, entry) for size in shaped])
               for label, shape, start, read in RINGED]
    shapes += [(stated_shape(FRAMES, depths, terms=FRAME_TERMS), "frames",
                [(depth, frames(depth), depth, frame_codes(depth)) for depth in depths]),
               (stated_shape(PLACEHOLDER, shaped), "placeholder beside a recursion",
                [(size, placeholder_beside(size), size, (NO_ENTRY, UNKNOWN_NAME)) for size in shaped]),
               (stated_shape(VALUE_SHAPED, shaped, body=2 * VALUE_TERMS), "value-shaped body",
                [(size, value_shaped(size), size, entry) for size in shaped]),
               (stated_shape(DERIVED_SITES, shaped, body=4 * SITE_TERMS), "derived-cell sites",
                [(size, derived_sites(size), size, entry) for size in shaped])]
    return shapes


def ranked(finished, count, quantile):
    index = max(0, math.ceil(quantile * count) - 1)
    return finished[index] if index < len(finished) else None


def waited(below):
    if not math.isfinite(below):
        return "at any load"
    return f"once the 1-minute load is below {below} or the figures have waited {WAIT_SECONDS // 60} minutes"


def ratio(release, dev):
    return round(release / dev, 2) if release is not None and dev else None


def engine_of(runs):
    engines = sorted({run.get("engine") for run in runs} - {None})
    if len(engines) > 1:
        return MIXED_ENGINES
    return engines[0] if engines else None


def engine_counts(engines):
    counted = defaultdict(int)
    for engine in engines:
        counted[engine if engine in ENGINES else UNKNOWN_ENGINE] += 1
    return {name: counted[name] for name in (*ENGINES, UNKNOWN_ENGINE)}


def listed_engines(counts):
    return ", ".join(f"{name} {counts[name]}" for name in (*ENGINES, UNKNOWN_ENGINE))


def breaker_profile(elapsed, load, targets, engines=None):
    finished = sorted(ms for ms in elapsed.values() if ms is not None)
    median, p99 = ranked(finished, len(elapsed), 0.5), ranked(finished, len(elapsed), 0.99)
    return {"programs": len(elapsed), "past_budget": len(elapsed) - len(finished), "median_ms": median,
            "p99_ms": p99, "max_timed_ms": finished[-1] if finished else None, "load": round(load, 2),
            "targets_ms": targets,
            "engines": engine_counts(engines.values()) if engines is not None else None,
            "past_target": {"median": median is None or median > targets["median"],
                            "p99": p99 is None or p99 > targets["p99"]}}


def per_node_times(elapsed, nodes):
    return {program_key: ms * 1000 / nodes[program_key] if ms is not None else None
            for program_key, ms in elapsed.items() if nodes.get(program_key)}


def new_core_profile(elapsed, nodes, load, profile, engines):
    row = breaker_profile(elapsed, load, BREAKER_TARGETS_MS[profile], engines)
    per_node = per_node_times(elapsed, nodes)
    finished = sorted(us for us in per_node.values() if us is not None)
    median = ranked(finished, len(per_node), 0.5)
    target = NEW_CORE_NODE_TARGETS_US[profile]
    past = {**row["past_target"], "node": median is None or median > target}
    return {**row, "counted_programs": len(per_node), "per_node_us": median, "target_node_us": target,
            "past_target": past if elapsed else dict.fromkeys(past, None)}


def new_core_figure(times, elapsed, nodes, loads, below, scale=FULL, engines=None):
    counted = len(elapsed["dev"])
    judged = (f"each program's time divided by its syntax nodes as evolve-count counts them, the median of those "
              f"over the programs it counts against {NEW_CORE_NODE_TARGETS_US['release']} us a syntax node release "
              f"and {NEW_CORE_NODE_TARGETS_US['dev']} us dev; beside it, how many {BREAKER_SET} programs each engine "
              f"takes in the verdict check, by the engine its check-times.json.gz row names")
    if scale["breakers"] is None:
        figure = NEW_CORE
        method = (f"every {BREAKER_SET} program whose check-times.json.gz row names the new core, timed in the "
                  f"breaker figure's own run, each once in one process per profile, one job, after every other "
                  f"command of the step and {waited(below)}; a program past the budget here ranks above every "
                  f"timed one and reads as null; nearest-rank percentiles over all {counted} programs; {judged}")
    else:
        figure = stated(QUICK_NEW_CORE, scale)
        method = (f"the {scale['breakers']} {BREAKER_SET} programs whose check-times.json.gz row names the new "
                  f"core slowest in the verdict check's times ({times['profile']} profile) among those it timed "
                  f"within its {times['budget_s']} s budget, or all of them when fewer, {RUNS} runs each in one "
                  f"process per profile, one job, {waited(below)}; each program's median, a program past the budget "
                  f"in any run ranking above every timed one and reading as null; nearest-rank percentiles over the "
                  f"{counted} programs; {judged}")
    profiles = {profile: new_core_profile(elapsed[profile], nodes, loads[profile], profile,
                                          (engines or {}).get(profile))
                for profile in PROFILES}
    return {"work": figure[0], "measured_by": figure[1], "program": "new-core breakers",
            "taken_in_check": engine_counts(row.get("engine") for row in breaker_rows(times)), **profiles,
            "ratio": {quantile: ratio(profiles["release"][field], profiles["dev"][field])
                      for quantile, field in (("median", "median_ms"), ("p99", "p99_ms"), ("node", "per_node_us"))},
            "method": f"Compiler::check as evolve-time times it on evolve-check's compile thread, counting "
                      f"allocations, in the dev and the release profile built from one export, {method}"}


def breaker_figure(times, elapsed, loads, below, scale=FULL, engines=None):
    counted = len(elapsed["dev"])
    if scale["breakers"] is None:
        figure = BREAKERS
        method = (f"each {BREAKER_SET} program once in one process per profile, one job, after every other command "
                  f"of the step and {waited(below)}; a program the verdict check ({times['profile']} profile) found "
                  f"past its {times['budget_s']} s budget is not run again, and it and a program past the budget "
                  f"here rank above every timed one and read as null; nearest-rank percentiles over all {counted} "
                  f"programs")
    else:
        figure = stated(QUICK_BREAKERS, scale)
        method = (f"the {scale['breakers']} {BREAKER_SET} programs slowest in the verdict check's times "
                  f"({times['profile']} profile) among those it timed within its {times['budget_s']} s budget, "
                  f"{RUNS} runs each in one process per profile, one job, {waited(below)}; each program's median, a "
                  f"program past the budget in any run ranking above every timed one and reading as null; "
                  f"nearest-rank percentiles over the {counted} programs")
    profiles = {profile: breaker_profile(elapsed[profile], loads[profile], BREAKER_TARGETS_MS[profile],
                                         (engines or {}).get(profile))
                for profile in PROFILES}
    return {"work": figure[0], "measured_by": figure[1], **profiles,
            "ratio": {quantile: ratio(profiles["release"][f"{quantile}_ms"], profiles["dev"][f"{quantile}_ms"])
                      for quantile in ("median", "p99")},
            "method": f"Compiler::check as evolve-time times it on evolve-check's compile thread, counting "
                      f"allocations, in the dev and the release profile built from one export, {method}"}


def timed(binaries, programs, work, budget, limits=None):
    if not programs:
        return []
    given, taken = work / "given.jsonl", work / "taken.jsonl"
    with open(given, "w", encoding="utf-8") as out:
        for program in programs:
            out.write(json.dumps(program) + "\n")
    flags = [word for name, value in (limits or {}).items() for word in ("--limit", f"{name}={value}")]
    subprocess.run([binaries / "evolve-time", "--budget", str(budget), *flags, given, taken], check=True)
    return [json.loads(line) for line in taken.read_text(encoding="utf-8").splitlines() if line.strip()]


def counted(binaries, programs, work, nodes_only=False):
    if not programs:
        return {}
    given, taken = work / "counted.jsonl", work / "counts.jsonl"
    with open(given, "w", encoding="utf-8") as out:
        for program in programs:
            out.write(json.dumps(program) + "\n")
    subprocess.run([binaries / "evolve-count", *(["--nodes"] if nodes_only else []), given, taken], check=True)
    return {row["id"]: row for row in map(json.loads, taken.read_text(encoding="utf-8").splitlines())}


def fixture_program(binaries):
    printed = subprocess.run([binaries / "evolve-count", "--fixture"], check=True, capture_output=True, text=True)
    return json.loads(printed.stdout)


def breaker_rows(times, engine=None):
    return [row for row in times["programs"]
            if row["set"] == BREAKER_SET and (engine is None or row.get("engine") == engine)]


def breaker_elapsed(binaries, work, times, sources):
    rows = breaker_rows(times)
    again = [{"id": row["key"], "sources": sources[row["key"]]} for row in rows if row["elapsed_ms"] is not None]
    taken = {run["id"]: run for run in timed(binaries, again, work, times["budget_s"])}
    return ({row["key"]: taken.get(row["key"], {}).get("elapsed_ms") for row in rows},
            {row["key"]: taken.get(row["key"], {}).get("engine") for row in rows})


def slowest_breakers(times, count, engine=None):
    rows = [row for row in breaker_rows(times, engine) if row["elapsed_ms"] is not None]
    return sorted(rows, key=lambda row: (-row["elapsed_ms"], row["key"]))[:count]


def quick_breaker_elapsed(binaries, work, times, sources, count, engine=None):
    rows = slowest_breakers(times, count, engine)
    runs = [{"id": f"{row['key']}#{number}", "sources": sources[row["key"]]} for row in rows for number in range(RUNS)]
    taken = defaultdict(list)
    for run in timed(binaries, runs, work, times["budget_s"]):
        taken[run["id"].rsplit("#", 1)[0]].append(run)
    elapsed = {}
    for row in rows:
        ms = [run["elapsed_ms"] for run in taken[row["key"]]]
        elapsed[row["key"]] = statistics.median(ms) if len(ms) == RUNS and None not in ms else None
    return elapsed, {row["key"]: engine_of(taken[row["key"]]) for row in rows}


def new_core_breakers(binaries, work, times, sources, timings, settle, below, scale):
    elapsed, engines, loads = {}, {}, {}
    for profile in PROFILES:
        if scale["breakers"] is None:
            chosen = [row["key"] for row in breaker_rows(times, NEW)]
            elapsed[profile] = {program_key: timings["elapsed"][profile][program_key] for program_key in chosen}
            engines[profile] = {program_key: timings["engines"][profile][program_key] for program_key in chosen}
            loads[profile] = timings["loads"][profile]
        else:
            loads[profile] = settle()
            elapsed[profile], engines[profile] = quick_breaker_elapsed(binaries[profile], work, times, sources,
                                                                       scale["breakers"], NEW)
    counts = counted(binaries["dev"], [{"id": program_key, "sources": sources[program_key]}
                                       for program_key in elapsed["dev"]], work, True)
    nodes = {program_key: row["syntax_nodes"] for program_key, row in counts.items()}
    return new_core_figure(times, elapsed, nodes, loads, below, scale, engines)


def raised(limits):
    if not limits:
        return ""
    return "with " + " and ".join(f"{name} at {value:,}" for name, value in limits.items()) + ", "


def runs_of(binaries, work, label, sources, limits):
    return timed(binaries, [{"id": f"{label}#{number}", "sources": sources} for number in range(RUNS)], work,
                 BUDGET_SECONDS, limits)


def main_sym(text):
    return [{"path": "main.sym", "text": text}]


def taken(runs, units, expected, target, load):
    elapsed = [run["elapsed_ms"] for run in runs]
    median = statistics.median(elapsed) if None not in elapsed else None
    panics = [run["panic"] for run in runs if run.get("panic") is not None]
    drawn = sorted({code for run in runs for code in run.get("codes") or ()})
    void = bool(panics) or any("codes" in run and sorted(run["codes"]) != sorted(expected) for run in runs)
    per_unit = median * 1000 / units if median is not None else None
    peaks = [run["peak_bytes"] for run in runs if run.get("peak_bytes") is not None]
    return {"load": round(load, 2), "engine": engine_of(runs), "runs_ms": elapsed, "median_ms": median,
            "per_unit_us": per_unit,
            "target_us": target, "past_target": target is not None and (per_unit is None or per_unit > target),
            "peak_bytes": max(peaks) if peaks else None, "diagnostics": runs[0].get("diagnostics") if runs else None,
            "codes": drawn, "void": void, "panic": panics[0] if panics else None}


def method_of(limits, below, units, must):
    return (f"Compiler::check as evolve-time times it on evolve-check's compile thread, counting allocations, "
            f"{raised(limits)}{RUNS} runs in one process per profile, the dev and the release profile built from "
            f"one export, one job, after every other command of the step and {waited(below)}, the median divided "
            f"by the {units}; the program's other text is counted in its time; void when a run panics or draws a "
            f"code set other than {must}")


def program_figure(binaries, work, figure, label, text, units, expected, settle, below):
    target = TARGETS_US.get(figure[0], {})
    profiles = {}
    for profile in PROFILES:
        load = settle()
        runs = runs_of(binaries[profile], work, label, main_sym(text), RAISED_LIMITS)
        profiles[profile] = taken(runs, units, expected, target.get(profile), load)
    return {"work": figure[0], "measured_by": figure[1], "program": label, "units": units, "characters": len(text),
            "expected_codes": sorted(expected), "void": any(profiles[profile]["void"] for profile in PROFILES),
            "limits": RAISED_LIMITS, **profiles,
            "ratio": ratio(profiles["release"]["median_ms"], profiles["dev"]["median_ms"]),
            "method": method_of(RAISED_LIMITS, below, f"{units} units", listed(sorted(expected)))}


def growth(sized):
    return [ratio(later["median_ms"], earlier["median_ms"]) for earlier, later in zip(sized, sized[1:])]


def marginal(sized):
    smallest, largest = min(sized, key=lambda row: row["size"]), max(sized, key=lambda row: row["size"])
    if smallest["median_ms"] is None or largest["median_ms"] is None or largest["units"] == smallest["units"]:
        return None
    return (largest["median_ms"] - smallest["median_ms"]) * 1000 / (largest["units"] - smallest["units"])


def marginal_profile(sized, target):
    marginal_us = marginal(sized)
    return {"sizes": sized, "growth": growth(sized), "marginal_us": marginal_us, "target_us": target,
            "past_target_marginal": target is not None and (marginal_us is None or marginal_us > target)}


def shape_figure(binaries, work, figure, label, sized, settle, below):
    target = TARGETS_US.get(figure[0], {})
    profiles = {}
    for profile in PROFILES:
        rows = []
        for size, text, units, expected in sized:
            load = settle()
            runs = runs_of(binaries[profile], work, f"{label}@{size}", main_sym(text), None)
            rows.append({"size": size, "units": units, "characters": len(text), "expected_codes": sorted(expected),
                         **taken(runs, units, expected, target.get(profile), load)})
        profiles[profile] = marginal_profile(rows, target.get(profile))
    return {"work": figure[0], "measured_by": figure[1], "program": label, "sizes": [size for size, *_ in sized],
            "void": any(row["void"] for profile in PROFILES for row in profiles[profile]["sizes"]), "limits": {},
            **profiles,
            "ratio": [ratio(release["median_ms"], dev["median_ms"])
                      for dev, release in zip(profiles["dev"]["sizes"], profiles["release"]["sizes"])],
            "method": method_of(None, below, "units at each size", "the one stated for its size")
                      + "; at three doubling sizes, each size's time per unit and the ratio of each size's median to "
                        "the one before it (growth); the marginal cost the difference between the largest and the "
                        "smallest size's medians divided by the difference in their units, judged against the "
                        "per-unit target where the row has one"}


def recompile_profile(runs, nodes, items, targets, load):
    row = taken(runs, nodes, FIXTURE_CODES, None, load)
    target_ms = (nodes * targets["node"] + items * targets["item"]) / 1000 if items is not None else None
    median = row["median_ms"]
    per_node = row.pop("per_unit_us")
    return {**row, "per_node_us": per_node, "targets_us": targets, "target_ms": target_ms,
            "past_target": median is None or (target_ms is not None and median > target_ms)}


def recompile_figure(binaries, work, settle, below):
    program = fixture_program(binaries["dev"])
    counts = counted(binaries["dev"], [program], work)[program["id"]]
    nodes, items = counts["syntax_nodes"], counts["expanded_items"]
    profiles = {}
    for profile in PROFILES:
        load = settle()
        runs = runs_of(binaries[profile], work, "fixture", program["sources"], None)
        profiles[profile] = recompile_profile(runs, nodes, items, RECOMPILE_TARGETS_US[profile], load)
    lines = sum(len(source["text"].splitlines()) for source in program["sources"])
    return {"work": RECOMPILE[0], "measured_by": RECOMPILE[1], "program": "fixture project", "files":
            len(program["sources"]), "lines": lines, "syntax_nodes": nodes, "expanded_items": items,
            "expected_codes": sorted(FIXTURE_CODES), "void": any(profiles[profile]["void"] for profile in PROFILES),
            "limits": {}, **profiles, "ratio": ratio(profiles["release"]["median_ms"], profiles["dev"]["median_ms"]),
            "method": method_of(None, below, "syntax nodes", listed(sorted(FIXTURE_CODES)))
                      + "; the syntax nodes counted by evolve-count over the parse of every file, the expanded "
                        "items, the instances and wires the expansion yields, by the least expanded_items_warning "
                        "that draws no expansion warning; the target 1 us a node plus 0.7 us an item in release "
                        "and 1.3 times that in dev"}


def memory_entry(program, nodes, items, peak, engine):
    target = (nodes * MEMORY_TARGETS_B["node"] + items * MEMORY_TARGETS_B["item"]) if items is not None else None
    return {"program": program, "engine": engine, "syntax_nodes": nodes, "expanded_items": items, "peak_bytes": peak,
            "per_node_b": peak / nodes if peak is not None and nodes else None,
            "per_item_b": peak / items if peak is not None and items else None,
            "target_bytes": target, "past_target": peak is None or (target is not None and peak > target)}


def memory_figure(recompile, expansion_figure, nodes):
    sizes = expansion_figure["sizes"]
    profiles = {}
    for profile in PROFILES:
        entries = [memory_entry("fixture project", recompile["syntax_nodes"], recompile["expanded_items"],
                                recompile[profile]["peak_bytes"], recompile[profile].get("engine"))]
        entries += [memory_entry(f"expansion at {row['size']:,}", nodes, row["units"], row["peak_bytes"],
                                 row.get("engine"))
                    for row in expansion_figure[profile]["sizes"]]
        profiles[profile] = {"entries": entries}
    return {"work": MEMORY[0], "measured_by": MEMORY[1].format(sizes=listed_sizes(sizes)), "program": "peak memory",
            "targets_b": MEMORY_TARGETS_B, "void": recompile["void"] or expansion_figure["void"], **profiles,
            "method": "the most bytes held at once by the compile's own allocations, as allocation-counter's measure "
                      "reports them on evolve-time's compile thread, the most over the runs of the recompile and the "
                      "expanded-item figures; each with its bytes per syntax node and per expanded item; the target "
                      "256 B a syntax node plus 112 B an expanded item in either profile, the expansion program's "
                      "items its iterations"}


def project_figures(binaries, work, settle, below, expansion_figure):
    recompile = recompile_figure(binaries, work, settle, below)
    figures = [recompile]
    if expansion_figure is not None:
        nodes = counted(binaries["dev"], [{"id": "expansion", "sources": main_sym(expansion(1))}], work,
                        True)["expansion"]["syntax_nodes"]
        figures.append(memory_figure(recompile, expansion_figure, nodes))
    return figures


def listed(codes):
    return ", ".join(codes) or "nothing"


def per_unit(row):
    return "past the budget" if row["per_unit_us"] is None else f"{row['per_unit_us']:.3f} us"


def past(row, unit):
    return f"PAST TARGET {row['target_us']} {unit}" if row.get("past_target") else ""


def void_note(row, expected):
    panic = f", panicked: {row['panic']}" if row["panic"] is not None else ""
    return f"VOID: drew {listed(row['codes'])}{panic}, must draw {listed(expected)}"


def notes(*parts):
    kept = [part for part in parts if part]
    return "; " + "; ".join(kept) if kept else ""


def shown_program(figure):
    dev, release = figure["dev"], figure["release"]
    line = (f"{figure['work']}, {figure['program']}: dev {per_unit(dev)}, release {per_unit(release)} per unit, "
            f"release : dev {figure['ratio']}, load {dev['load']} / {release['load']}")
    marks = [f"{profile} {past(figure[profile], 'us')}" for profile in PROFILES if figure[profile]["past_target"]]
    marks += [f"{profile} {void_note(figure[profile], figure['expected_codes'])}" for profile in PROFILES
              if figure[profile]["void"]]
    return line + notes(*marks)


def shown_shape(figure):
    lines = []
    for profile in PROFILES:
        rows = figure[profile]["sizes"]
        sized = ", ".join(f"{row['size']:,}: {per_unit(row)}" for row in rows)
        marks = [f"{row['size']:,} {past(row, 'us')}" for row in rows if row["past_target"]]
        marks += [f"{row['size']:,} {void_note(row, row['expected_codes'])}" for row in rows if row["void"]]
        lines.append(f"{figure['work']}, {figure['program']}, {profile}: per unit at {sized}; growth "
                     f"{figure[profile]['growth']}{notes(*marks)}")
        lines.append(shown_marginal(figure, profile))
    lines.append(f"{figure['work']}, {figure['program']}: release : dev {figure['ratio']}")
    return "\n".join(lines)


def cost(us):
    return "none" if us is None else f"{us:.3f} us"


def shown_marginal(figure, profile):
    row = figure[profile]
    sizes = [each["size"] for each in row["sizes"]]
    target = "no target" if row["target_us"] is None else f"target {row['target_us']} us"
    mark = f"PAST TARGET {row['target_us']} us" if row["past_target_marginal"] else ""
    return (f"{figure['work']}, {figure['program']}, {profile}: marginal {cost(row['marginal_us'])} a unit between "
            f"{min(sizes):,} and {max(sizes):,}, {target}{notes(mark)}")


def counted_items(items):
    return "uncounted" if items is None else f"{items:,}"


def shown_recompile(figure):
    lines = [f"{figure['work']}: {figure['files']} files, {figure['lines']:,} lines, {figure['syntax_nodes']:,} "
             f"syntax nodes, {counted_items(figure['expanded_items'])} expanded items"]
    for profile in PROFILES:
        row = figure[profile]
        target = "no target" if row["target_ms"] is None else f"target {row['target_ms']:.1f} ms"
        marks = ["PAST TARGET" if row["past_target"] else "",
                 void_note(row, figure["expected_codes"]) if row["void"] else ""]
        lines.append(f"{figure['work']}, {profile}: median {row['median_ms']} ms, {target}, load {row['load']}"
                     f"{notes(*marks)}")
    lines.append(f"{figure['work']}: release : dev {figure['ratio']}")
    return "\n".join(lines)


def per_entry(bytes_per):
    return "?" if bytes_per is None else f"{bytes_per:.0f}"


def shown_memory(figure):
    lines = []
    for profile in PROFILES:
        for entry in figure[profile]["entries"]:
            lines.append(f"{figure['work']}, {profile}, {entry['program']}: {entry['peak_bytes']} bytes, "
                         f"{per_entry(entry['per_node_b'])} B a syntax node, {per_entry(entry['per_item_b'])} B an "
                         f"expanded item, target {entry['target_bytes']} bytes"
                         f"{notes('PAST TARGET' if entry['past_target'] else '')}")
    return "\n".join(lines)


def shown_breakers(figure):
    lines = []
    for profile in PROFILES:
        row = figure[profile]
        marks = [f"PAST TARGET {quantile} {row['targets_ms'][quantile]} ms"
                 for quantile in ("median", "p99") if row["past_target"][quantile]]
        lines.append(f"{figure['work']}, {profile}: median {row['median_ms']} ms, p99 {row['p99_ms']} ms over "
                     f"{row['programs']} programs, {row['past_budget']} past the budget, load {row['load']:.2f}"
                     f"{notes(*marks)}")
    lines.append(f"{figure['work']}: release : dev median {figure['ratio']['median']}, p99 {figure['ratio']['p99']}")
    lines += [f"{figure['work']}, {profile}: engines at this run {listed_engines(figure[profile]['engines'])}"
              for profile in PROFILES if figure[profile].get("engines") is not None]
    return "\n".join(lines)


def shown_new_core(figure):
    lines = []
    for profile in PROFILES:
        row = figure[profile]
        marks = [f"PAST TARGET {quantile} {row['targets_ms'][quantile]} ms"
                 for quantile in ("median", "p99") if row["past_target"][quantile]]
        if row["past_target"]["node"]:
            marks.append(f"PAST TARGET node {row['target_node_us']} us")
        lines.append(f"{figure['work']}, {profile}: median {row['median_ms']} ms, p99 {row['p99_ms']} ms over "
                     f"{row['programs']} programs, {row['past_budget']} past the budget, median "
                     f"{cost(row['per_node_us'])} a syntax node over {row['counted_programs']} programs, target "
                     f"{row['target_node_us']} us, load {row['load']:.2f}{notes(*marks)}")
    lines.append(f"{figure['work']}: release : dev median {figure['ratio']['median']}, p99 {figure['ratio']['p99']}, "
                 f"a syntax node {figure['ratio']['node']}")
    lines += [f"{figure['work']}, {profile}: engines at this run {listed_engines(figure[profile]['engines'])}"
              for profile in PROFILES if figure[profile].get("engines") is not None]
    lines.append(f"{figure['work']}: breaker programs each engine takes in the verdict check: "
                 f"{listed_engines(figure['taken_in_check'])}")
    return "\n".join(lines)


def shown(figure):
    if "taken_in_check" in figure:
        return shown_new_core(figure)
    if "entries" in figure.get("dev", {}):
        return shown_memory(figure)
    if "syntax_nodes" in figure:
        return shown_recompile(figure)
    if "sizes" in figure:
        return shown_shape(figure)
    if "units" in figure:
        return shown_program(figure)
    return shown_breakers(figure)


def past_targets(figure):
    found = []
    for profile in PROFILES:
        row = figure[profile]
        if "entries" in row:
            found += [f"{figure['program']}, {entry['program']} ({profile})" for entry in row["entries"]
                      if entry["past_target"]]
        elif "sizes" in row:
            found += [f"{figure['program']} at {each['size']:,} ({profile})" for each in row["sizes"]
                      if each["past_target"]]
            if row.get("past_target_marginal"):
                found.append(f"{figure['program']} marginal ({profile})")
        elif isinstance(row["past_target"], dict):
            found += [f"{figure['work']} {quantile} ({profile})" for quantile, past_it in row["past_target"].items()
                      if past_it]
        elif row["past_target"]:
            found.append(f"{figure['program']} ({profile})")
    return found


def on_new_core(figure):
    if "taken_in_check" in figure:
        return True
    rows = []
    for profile in PROFILES:
        row = figure.get(profile, {})
        if "entries" in row:
            return False
        rows += row["sizes"] if "sizes" in row else [row]
    return bool(rows) and all(each.get("engine") == NEW for each in rows)


def entry_profile(per_unit_us, marginal_us, target_us, past_target, past_target_marginal):
    return {"per_unit_us": per_unit_us, "marginal_us": marginal_us, "target_us": target_us,
            "past_target": past_target, "past_target_marginal": past_target_marginal}


def recompile_node_target(figure, profile):
    target_ms = figure[profile]["target_ms"]
    return target_ms * 1000 / figure["syntax_nodes"] if target_ms is not None and figure["syntax_nodes"] else None


def new_core_entry(figure):
    size, unit = None, "unit"
    if "taken_in_check" in figure:
        unit, ratio_of = "syntax node", figure["ratio"]["node"]
        profiles = {profile: entry_profile(figure[profile]["per_node_us"], None, figure[profile]["target_node_us"],
                                           figure[profile]["past_target"]["node"], None) for profile in PROFILES}
    elif "syntax_nodes" in figure:
        unit, ratio_of = "syntax node, the items' share in its target", figure["ratio"]
        profiles = {profile: entry_profile(figure[profile]["per_node_us"], None,
                                           recompile_node_target(figure, profile), figure[profile]["past_target"],
                                           None) for profile in PROFILES}
    elif "sizes" in figure:
        size = max(figure["sizes"])
        ratio_of = figure["ratio"][figure["sizes"].index(size)]
        profiles = {}
        for profile in PROFILES:
            row = figure[profile]
            largest = max(row["sizes"], key=lambda each: each["size"])
            profiles[profile] = entry_profile(largest["per_unit_us"], row["marginal_us"], row["target_us"],
                                              any(each["past_target"] for each in row["sizes"]),
                                              row["past_target_marginal"])
    else:
        ratio_of = figure["ratio"]
        profiles = {profile: entry_profile(figure[profile]["per_unit_us"], None, figure[profile]["target_us"],
                                           figure[profile]["past_target"], None) for profile in PROFILES}
    return {"work": figure["work"], "program": figure["program"], "unit": unit, "size": size, **profiles,
            "ratio": ratio_of}


def judged_against(row):
    if row["target_us"] is None:
        return "no target"
    marks = ["PAST TARGET" if row["past_target"] else "within target"]
    if row["past_target_marginal"] is not None:
        marks.append("marginal PAST TARGET" if row["past_target_marginal"] else "marginal within target")
    return ", ".join(marks)


def shown_entry(entry):
    at = "" if entry["size"] is None else f" at {entry['size']:,}"
    parts = []
    for profile in PROFILES:
        row = entry[profile]
        spread = "one size" if entry["size"] is None else cost(row["marginal_us"])
        parts.append(f"{profile} {cost(row['per_unit_us'])} a {entry['unit']}{at}, marginal {spread}, target "
                     f"{cost(row['target_us'])}, {judged_against(row)}")
    return f"{entry['work']}, {entry['program']}: {'; '.join(parts)}; release : dev {entry['ratio']}"


def measure(binaries, times_path, corpus, out_path, below, settle, scale=FULL):
    with gzip.open(times_path, "rt", encoding="utf-8") as raw:
        times = json.load(raw)
    sources = {key(record): record["sources"] for record in read_jsonl(corpus)}
    with tempfile.TemporaryDirectory() as directory:
        work = Path(directory)
        elapsed, engines, loads = {}, {}, {}
        for profile in PROFILES:
            loads[profile] = settle()
            elapsed[profile], engines[profile] = (
                breaker_elapsed(binaries[profile], work, times, sources) if scale["breakers"] is None
                else quick_breaker_elapsed(binaries[profile], work, times, sources, scale["breakers"]))
        figures = [breaker_figure(times, elapsed, loads, below, scale, engines)]
        print(shown(figures[0]), flush=True)
        figures.append(new_core_breakers(binaries, work, times, sources,
                                         {"elapsed": elapsed, "engines": engines, "loads": loads}, settle, below,
                                         scale))
        print(shown(figures[-1]), flush=True)
        for figure, label, text, units, expected in measured_programs(scale):
            figures.append(program_figure(binaries, work, figure, label, text, units, expected, settle, below))
            print(shown(figures[-1]), flush=True)
        for figure, label, sized in measured_shapes(scale):
            figures.append(shape_figure(binaries, work, figure, label, sized, settle, below))
            print(shown(figures[-1]), flush=True)
        expanded = next((figure for figure in figures if figure.get("program") == "expansion"), None)
        for figure in project_figures(binaries, work, settle, below, expanded):
            figures.append(figure)
            print(shown(figure), flush=True)
    void = [figure["program"] for figure in figures if figure.get("void")]
    past = [named for figure in figures for named in past_targets(figure)]
    new_core = [new_core_entry(figure) for figure in figures if on_new_core(figure)]
    print(NEW_CORE_HEADING, flush=True)
    for entry in new_core:
        print(shown_entry(entry), flush=True)
    if past:
        print(f"figures past their target: {len(past)} ({'; '.join(past)})", flush=True)
    if void:
        print(f"void figures: {len(void)} ({', '.join(void)}); exit {VOID_EXIT}", flush=True)
    out_path.write_text(json.dumps({"profiles": list(PROFILES), "binaries": {profile: str(binaries[profile])
                                                                             for profile in PROFILES},
                                    "check_profile": times["profile"], "scale": scale["name"],
                                    "load_below": below if math.isfinite(below) else None, "void": void,
                                    "past_target": past, "figures": figures, "new_core": new_core}, indent=2) + "\n")
    return void
