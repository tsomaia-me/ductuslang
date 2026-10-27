import argparse
import os
import re
import subprocess
import sys

STATES = ("open", "taken", "done", "cancelled", "owner")
SECTION_STATES = (
    ("running", "running"),
    ("done", "done"),
    ("superseded", "superseded"),
    ("decided", "decided-no"),
)
FORWARD_ONLY = ("done", "cancelled")
BACKWARD_TARGETS = ("open", "taken", "owner")
COMMIT = re.compile(r"`([0-9a-f]{7,40})`")
MEMORY_COMMIT = re.compile(r"memory commit\s+`?([0-9a-f]{7,40})`?")
QUOTED = re.compile(r'"[^"\n]+"|“[^”\n]+”')
ROUND_BRIEF = re.compile(r"^(.*?)round-(\d+)([a-z]?)-brief\.md$")
LEDGER_HEADING = re.compile(r"^(#{1,6})\s+(?:§\s*)?Ledger\b")
LEDGER_LINE = re.compile(r"^\s*(?:[-*]\s+)?`?([^\s`:,]+?)`?\s*:\s*(\S.*)$")
ENTRY_HEADING = re.compile(r"^###\s+([^\s.]+)\.?\s*(.*)$")
STATE_LINE = re.compile(r"^-\s+State:\s*(.*)$")
FULL_TEXT = "## Full text"


class Entry:
    def __init__(self, entry_id, title, section_state):
        self.id = entry_id
        self.title = title
        self.section_state = section_state
        self.states = []

    def state(self):
        for word, _ in reversed(self.states):
            if word in STATES:
                return word
        return self.section_state

    def newest_line(self):
        return self.states[-1][1] if self.states else ""


def section_state(heading):
    name = heading[3:].strip().lower()
    for prefix, state in SECTION_STATES:
        if name.startswith(prefix):
            return state
    return "open"


def parse_ledger(path):
    entries = []
    current_section = "open"
    entry = None
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            line = line.rstrip("\n")
            if line.startswith(FULL_TEXT):
                break
            if line.startswith("## "):
                current_section = section_state(line)
                entry = None
                continue
            match = ENTRY_HEADING.match(line)
            if match:
                entry = Entry(match.group(1), match.group(2), current_section)
                entries.append(entry)
                continue
            match = STATE_LINE.match(line)
            if match and entry is not None:
                text = match.group(1)
                word = re.match(r"[A-Za-z-]*", text).group(0).lower()
                entry.states.append((word, text))
    return entries


def natural_key(text):
    parts = re.split(r"(\d+)", text)
    return tuple((0, int(p)) if p.isdigit() else (1, p) for p in parts)


def newest_brief(directory):
    names = [n for n in os.listdir(directory) if n.endswith("-brief.md")]
    rounds = []
    for name in names:
        match = ROUND_BRIEF.match(name)
        if match:
            key = (natural_key(match.group(1)), int(match.group(2)), match.group(3))
            rounds.append((key, name))
    if rounds:
        return os.path.join(directory, max(rounds)[1])
    if not names:
        return None
    return os.path.join(
        directory,
        max(names, key=lambda n: os.path.getmtime(os.path.join(directory, n))),
    )


def brief_ledger(path):
    ids = None
    level = 0
    with open(path, encoding="utf-8") as handle:
        for line in handle:
            line = line.rstrip("\n")
            heading = re.match(r"^(#{1,6})\s", line)
            if heading:
                if ids is not None and len(heading.group(1)) <= level:
                    break
                match = LEDGER_HEADING.match(line)
                if match and ids is None:
                    ids = {}
                    level = len(match.group(1))
                continue
            if ids is not None:
                match = LEDGER_LINE.match(line)
                if match:
                    ids[match.group(1)] = match.group(2)
    return ids


def is_ancestor(repo, commit, ref):
    result = subprocess.run(
        ["git", "-C", repo, "merge-base", "--is-ancestor", commit, ref],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return result.returncode == 0


def check_commits(entry, line, repo, failures):
    memory = os.path.join(repo, ".memory")
    memory_commits = MEMORY_COMMIT.findall(line)
    commits = [c for c in COMMIT.findall(line) if c not in memory_commits]
    if not commits and not memory_commits:
        failures.append("entry %s: done, its State line names no commit" % entry.id)
        return
    for commit in commits:
        if not is_ancestor(repo, commit, "main"):
            failures.append(
                "entry %s: done, commit %s is not an ancestor of main" % (entry.id, commit)
            )
    for commit in memory_commits:
        if not is_ancestor(memory, commit, "HEAD"):
            failures.append(
                "entry %s: done, memory commit %s is not an ancestor of the memory HEAD"
                % (entry.id, commit)
            )


def check_cancelled(entry, line, failures):
    lowered = line.lower()
    if QUOTED.search(line) or "ruling" in lowered or "owner's" in lowered:
        return
    failures.append("entry %s: cancelled, its State line names no ruling" % entry.id)


def check_moves(entry, failures):
    seen_final = None
    for word, _ in entry.states:
        if seen_final and word in BACKWARD_TARGETS:
            failures.append(
                "entry %s: State moves backwards from %s to %s" % (entry.id, seen_final, word)
            )
            return
        if word in FORWARD_ONLY:
            seen_final = word


def run(ledger, briefs, repo):
    entries = parse_ledger(ledger)
    failures = []
    brief = newest_brief(briefs) if os.path.isdir(briefs) else None
    ids = None
    if brief is None:
        failures.append("no brief under %s" % briefs)
    else:
        ids = brief_ledger(brief)
        if ids is None:
            failures.append("brief %s has no Ledger section" % os.path.basename(brief))
    for entry in entries:
        for word, _ in entry.states:
            if word not in STATES:
                failures.append(
                    "entry %s: State word %r is not one of %s"
                    % (entry.id, word, ", ".join(STATES))
                )
        state = entry.state()
        if state in ("open", "taken") and ids is not None and entry.id not in ids:
            failures.append(
                "entry %s: %s, missing from the Ledger of %s"
                % (entry.id, state, os.path.basename(brief))
            )
        if entry.states and entry.states[-1][0] == "done":
            check_commits(entry, entry.newest_line(), repo, failures)
        if entry.states and entry.states[-1][0] == "cancelled":
            check_cancelled(entry, entry.newest_line(), failures)
        check_moves(entry, failures)
    states = [e.state() for e in entries]
    summary = "carry check: %d entries, %d open, %d taken, %d owner, %d failures" % (
        len(entries),
        states.count("open"),
        states.count("taken"),
        states.count("owner"),
        len(failures),
    )
    return failures, summary


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--ledger", required=True)
    parser.add_argument("--briefs", required=True)
    parser.add_argument("--repo", required=True)
    args = parser.parse_args(argv)
    failures, summary = run(args.ledger, args.briefs, args.repo)
    for failure in failures:
        print(failure)
    print(summary)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
