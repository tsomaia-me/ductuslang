import hashlib
import json
import re
from pathlib import Path

ROW = "pinned-row "
RECORD = Path(__file__).resolve().parent / "pinned.json"
TEST_LINE = re.compile(r"\s*(?P<status>[A-Z][A-Z-]*) \[\s*(?P<seconds>[0-9.]+s)?\s*\]\s+(?:\(\s*\d+/\d+\)\s+)?"
                       r"(?P<test>\S+ \S+)\s*")
SUMMARY_LINE = re.compile(r"\s*Summary \[")
PASSING = ("pass", "leak", "flaky")
NOT_RUN = ("skip",)
KINDS = ("exhaustive", "stress")
SHOWN = 160
FORMAT = (f"one line per failing row, `{ROW}` followed by a JSON object on that line, "
          "{\"program\": <the row's source text>, \"diagnostics\": [<each diagnostic whole, in the compiler's order, "
          "as {\"code\", \"file\", \"span\": [start, end], \"summary\", \"help\", \"related\": [{\"file\", \"span\", "
          "\"label\"}]}>]}, printed to the test's standard output or error or in its panic message")


def identity(program):
    return json.dumps(program, ensure_ascii=False, sort_keys=True)


def named(program):
    digest = hashlib.sha256(identity(program).encode()).hexdigest()[:12]
    shown = json.dumps(program if isinstance(program, str) else identity(program), ensure_ascii=False)
    return f"{digest} {shown if len(shown) <= SHOWN else shown[:SHOWN - 1] + '…'}"


def kind(test):
    return test.split(" ", 1)[-1].rsplit("::", 1)[-1].split("_", 1)[0]


def outcomes(log):
    found, current = {}, None
    for line in log.splitlines():
        if SUMMARY_LINE.match(line):
            break
        matched = TEST_LINE.fullmatch(line)
        if matched:
            current = matched["test"]
            entry = found.setdefault(current, {"status": None, "rows": {}, "unreadable": 0})
            entry["status"] = matched["status"].lower()
            continue
        stripped = line.strip()
        if current is None or not stripped.startswith(ROW):
            continue
        try:
            row = json.loads(stripped[len(ROW):])
        except json.JSONDecodeError:
            found[current]["unreadable"] += 1
            continue
        if not isinstance(row, dict) or "program" not in row or "diagnostics" not in row:
            found[current]["unreadable"] += 1
            continue
        found[current]["rows"].setdefault(identity(row["program"]), row)
    return found


def failing(outcome):
    return outcome is not None and outcome["status"] not in PASSING + NOT_RUN


def ran(outcome):
    return outcome is not None and outcome["status"] not in NOT_RUN


def read_record(path=RECORD):
    return json.loads(Path(path).read_text(encoding="utf-8")) if Path(path).exists() else {"tests": {}}


def write_record(path, record):
    Path(path).write_text(json.dumps(record, indent=2, ensure_ascii=False, sort_keys=True) + "\n", encoding="utf-8")


def recorded_rows(entry):
    return {identity(row["program"]): row for row in entry["rows"]}


def pinned_entry(rows):
    ordered = [rows[program] for program in sorted(rows)]
    return {"count": len(ordered), "rows": [{"program": row["program"], "diagnostics": row["diagnostics"]}
                                            for row in ordered]}


def pinning(record, found, tests):
    tests_record, lines, refused = dict(record["tests"]), [], []
    for test in tests:
        outcome = found.get(test)
        if not ran(outcome):
            refused.append(f"{test}: not in the run")
        elif not failing(outcome):
            tests_record.pop(test, None)
            lines.append(f"{test}: passes, unpinned")
        elif outcome["unreadable"]:
            refused.append(f"{test}: {outcome['unreadable']} rows the tool cannot read; it reads {FORMAT}")
        elif not outcome["rows"]:
            refused.append(f"{test}: fails with no row the tool reads; it reads {FORMAT}")
        else:
            tests_record[test] = pinned_entry(outcome["rows"])
            lines.append(f"{test}: pinned: {len(outcome['rows'])} programs")
    if refused:
        raise ValueError("\n".join(refused))
    return {**record, "tests": dict(sorted(tests_record.items()))}, lines


def judgement(test, outcome, entry):
    if entry is None:
        return {"test": test, "state": "failed, not pinned"}
    state = {"test": test, "pinned": entry["count"]}
    if not ran(outcome):
        return {**state, "state": "not run"}
    if outcome["unreadable"]:
        return {**state, "state": "unreadable", "unreadable": outcome["unreadable"]}
    current = outcome["rows"] if failing(outcome) else {}
    if failing(outcome) and not current:
        return {**state, "state": "failed with no row"}
    before = recorded_rows(entry)
    joined = [current[program] for program in sorted(current) if program not in before]
    left = [before[program] for program in sorted(before) if program not in current]
    changed = [(before[program], current[program]) for program in sorted(current)
               if program in before and before[program]["diagnostics"] != current[program]["diagnostics"]]
    state.update({"now": len(current), "joined": [named(row["program"]) for row in joined],
                  "left": [named(row["program"]) for row in left],
                  "changed": [{"program": named(old["program"]), "before": old["diagnostics"],
                               "after": new["diagnostics"]} for old, new in changed]})
    return {**state, "state": "moved" if joined or left or changed else "unchanged"}


def judgement_lines(judged):
    test, state = judged["test"], judged["state"]
    if state == "failed, not pinned":
        return [f"{test}: failed, not pinned"]
    head = f"{test}: pinned: {judged['pinned']} programs"
    if state == "unchanged":
        return [f"{head}, unchanged"]
    if state == "not run":
        return [f"{head}; not run"]
    if state == "unreadable":
        return [f"{head}; {judged['unreadable']} rows the tool cannot read; it reads {FORMAT}"]
    if state == "failed with no row":
        return [f"{head}; failed with no row the tool reads; it reads {FORMAT}"]
    lines = [f"{head}; now {judged['now']}: {len(judged['joined'])} joined, {len(judged['left'])} left, "
             f"{len(judged['changed'])} changed"]
    lines += [f"  joined: {program}" for program in judged["joined"]]
    lines += [f"  left: {program}" for program in judged["left"]]
    lines += [f"  changed: {each['program']}; diagnostics before: {identity(each['before'])}; "
              f"after: {identity(each['after'])}" for each in judged["changed"]]
    return lines


def judged(log, record, step_kind, nextest_exit):
    found = outcomes(log)
    pins = {test: entry for test, entry in record["tests"].items() if kind(test) == step_kind}
    names = sorted(set(pins) | {test for test, outcome in found.items() if failing(outcome)})
    judgements = [judgement(test, found.get(test), pins.get(test)) for test in names]
    problems = [each for each in judgements if each["state"] != "unchanged"]
    lines = [line for each in judgements for line in judgement_lines(each)]
    if nextest_exit != 0 and not any(failing(outcome) for outcome in found.values()):
        lines.append(f"nextest exited {nextest_exit} with no failing test its log names")
        problems.append(None)
    return {"exit": 1 if problems else 0, "nextest_exit": nextest_exit, "tests": judgements, "lines": lines}
