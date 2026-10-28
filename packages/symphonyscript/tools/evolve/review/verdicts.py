import re
from collections import Counter, defaultdict

from programs import size

INTERNAL_ERROR = "internal-error"
CRASHED = ("panic", "abort")
FIELDS = (("errors", ""), ("warnings", "warning "), ("signals", "signal "))
SHOWN_TEXT = 6000


def compact(raw, warnings):
    codes = {diagnostic["code"] for diagnostic in raw["diagnostics"]}
    errors = sorted(codes - warnings)
    if raw.get("panic") is not None:
        outcome = "panic"
    elif raw["elapsed_ms"] is None:
        outcome = "slow"
    elif errors:
        outcome = "reject"
    else:
        outcome = "accept"
    return {"outcome": outcome, "errors": errors, "warnings": sorted(codes & warnings),
            "signals": sorted(raw["signals"])}


def detail(raw):
    internal = [diagnostic["message"] for diagnostic in raw["diagnostics"] if diagnostic["code"] == INTERNAL_ERROR]
    if raw.get("panic") is None and not internal and not raw["findings"]:
        return None
    return {"panic": raw.get("panic"), "internal": internal, "findings": raw["findings"]}


def aborted(exit_code):
    return {"outcome": "abort", "errors": [], "warnings": [], "signals": []}, {"exit": exit_code}


def change(before, after):
    parts = [] if before["outcome"] == after["outcome"] else [f"{before['outcome']} → {after['outcome']}"]
    for field, label in FIELDS:
        old, new = set(before[field]), set(after[field])
        parts += [f"+{label}{item}" for item in sorted(new - old)]
        parts += [f"-{label}{item}" for item in sorted(old - new)]
    return tuple(parts)


def moved(previous, current):
    classes = defaultdict(list)
    for program_key in sorted(current.keys() & previous.keys()):
        delta = change(previous[program_key], current[program_key])
        if delta:
            classes[delta].append(program_key)
    return sorted(classes.items(), key=lambda item: (-len(item[1]), item[0]))


def sections(codes, table):
    found = {table[code]["section"] for code in codes if code in table and table[code]["section"]}
    ordered = sorted(found, key=lambda section: [int(part) for part in section.split(".")])
    return ", ".join(f"§{section}" for section in ordered) or "no section"


def class_sections(before, after, table):
    delta = set(before["errors"]) ^ set(after["errors"]) | set(before["warnings"]) ^ set(after["warnings"])
    return sections(delta or {*before["errors"], *after["errors"], *before["warnings"], *after["warnings"]}, table)


def shown(verdict):
    head = verdict["outcome"] + (": " + ", ".join(verdict["errors"]) if verdict["errors"] else "")
    tail = [f"{label}{', '.join(verdict[field])}" for field, label in FIELDS[1:] if verdict[field]]
    return "; ".join([head, *tail])


def counted(counter):
    return ", ".join(f"{name} {count}" for name, count in sorted(counter.items())) or "none"


def fenced(text):
    fence = "`" * max(3, 1 + max((len(run) for run in re.findall("`+", text)), default=0))
    return [f"{fence}sym", text.rstrip("\n"), fence]


def title(program_key, program):
    return f"`{program_key}` · {program['set']}" + (f" · {program['origin']}" if "origin" in program else "")


def program_lines(program, extra):
    lines = []
    if extra:
        lines += [f"- Exit: {extra['exit']}"] if "exit" in extra else []
        lines += [f"- Panic: {extra['panic']}"] if extra.get("panic") else []
        lines += [f"- Internal error: {message}" for message in extra.get("internal", [])]
        lines += [f"- Finding {finding['signal']}: {', '.join(finding['codes']) or '-'}"
                  for finding in extra.get("findings", [])]
    for source in program["sources"]:
        text = source["text"]
        cut = f", the first {SHOWN_TEXT} shown" if len(text) > SHOWN_TEXT else ""
        lines += ["", f"`{source['path']}`, {len(text)} characters{cut}:", *fenced(text[:SHOWN_TEXT])]
    return lines


def program_block(program_key, program, verdict, extra):
    return ["", f"### {title(program_key, program)}", "", f"- Verdict: {shown(verdict)}",
            *program_lines(program, extra)]


def render(heading, programs, current, previous, details, table, notes):
    added = Counter(current[program_key]["set"] for program_key in current.keys() - previous.keys())
    removed = Counter(previous[program_key]["set"] for program_key in previous.keys() - current.keys())
    lines = [f"# {heading}", "", f"Programs: {len(current)} ({counted(Counter(v['set'] for v in current.values()))}).",
             f"Added since the baseline: {counted(added)}. Removed: {counted(removed)}.", *notes]

    crashed = sorted(program_key for program_key, verdict in current.items()
                     if verdict["outcome"] in CRASHED or INTERNAL_ERROR in verdict["errors"])
    lines += ["", f"## Internal errors, panics and aborts: {len(crashed)}"]
    for program_key in crashed:
        lines += program_block(program_key, programs[program_key], current[program_key], details.get(program_key))

    fresh = sorted(program_key for program_key, verdict in current.items()
                   if set(verdict["signals"]) - set(previous.get(program_key, {}).get("signals", [])))
    standing = Counter(signal for program_key, verdict in current.items() if program_key not in fresh
                       for signal in verdict["signals"])
    lines += ["", f"## New signals: {len(fresh)}", "", f"Standing signals: {counted(standing)}."]
    for program_key in fresh:
        lines += program_block(program_key, programs[program_key], current[program_key], details.get(program_key))

    classes = moved(previous, current)
    lines += ["", f"## Moved: {sum(len(keys) for _, keys in classes)} programs in {len(classes)} classes"]
    if classes:
        lines += ["", "| Class | Programs | Change | SPEC |", "|---|---|---|---|"]
    rows = []
    for number, (delta, keys) in enumerate(classes, 1):
        example = min(keys, key=lambda program_key: (size(programs[program_key]), program_key))
        tag = class_sections(previous[example], current[example], table)
        lines.append(f"| {number} | {len(keys)} | {' '.join(delta)} | {tag} |")
        rows += ["", f"### {number}. {' '.join(delta)}: {len(keys)} · {tag}", "",
                 f"- Shortest: {title(example, programs[example])}", f"- Before: {shown(previous[example])}",
                 f"- After: {shown(current[example])}", *program_lines(programs[example], details.get(example))]
    lines += rows

    disagreeing = [program_key for program_key, program in programs.items()
                   if "stated" in program and current[program_key]["outcome"] != program["stated"]]
    new = {program_key for program_key in disagreeing if previous.get(program_key, {}).get(
        "outcome", programs[program_key]["stated"]) == programs[program_key]["stated"]}
    lines += ["", f"## Stated verdicts: {len(disagreeing)} programs whose compiler verdict differs, {len(new)} new"]
    if disagreeing:
        lines += ["", "| Program | Origin | Stated | Compiler | New |", "|---|---|---|---|---|"]
    for program_key in sorted(disagreeing, key=lambda each: (each not in new, programs[each]["origin"])):
        program = programs[program_key]
        lines.append(f"| `{program_key}` | {program['origin']} | {program['stated']} | "
                     f"{shown(current[program_key])} | {'new' if program_key in new else ''} |")
    return "\n".join(lines) + "\n"
