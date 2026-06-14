#!/usr/bin/env python3
"""lint_nnm.py — validate the section-based (NNN-M) Ductus DECISION_LOG.md.

From <!-- BEGIN LOG --> onward, checks:
  * every entry is 'NNN-M. <text> (§ref)'
  * each entry's NNN prefix matches its enclosing '## NNN. Title' section
  * M is unique within its section (gaps allowed; never reused)
  * section header numbers (## NNN.) are unique
  * every (§ref) resolves to a real SPEC.md heading

Usage: lint_nnm.py DECISION_LOG.md [SPEC.md]
       (SPEC.md defaults to the file next to DECISION_LOG.md)
Exit status is non-zero if any error is reported.
"""
import sys, os, re, collections

SEC = r'\d+(?:\.\d+){0,3}'
HEAD_RE = re.compile(r'^#{1,6}\s+(' + SEC + r'(?:\.\d+)*)\.?(?:\s|$)')
SECTION_RE = re.compile(r'^##\s+(\d+)\.\s+\S')
ENTRY_RE = re.compile(r'^(\d+)-(\d+)\. \S.* \(§(' + SEC + r')\)$')


def load_headings(spec):
    hs, fence = set(), False
    for line in open(spec, encoding='utf-8'):
        if line.lstrip().startswith('```'):
            fence = not fence
            continue
        if fence or not line.startswith('#'):
            continue
        m = HEAD_RE.match(line)
        if m:
            hs.add(m.group(1))
    return hs


def lint(path, spec):
    headings = load_headings(spec)
    lines = open(path, encoding='utf-8').read().split('\n')
    try:
        start = lines.index('<!-- BEGIN LOG -->') + 1
    except ValueError:
        print('ERROR: no <!-- BEGIN LOG --> marker')
        return 1
    errs, n, cur_sec = [], 0, None
    seen_sections = set()
    per_section_m = collections.defaultdict(set)
    for i, line in enumerate(lines[start:], start + 1):
        if not line.strip():
            continue
        ms = SECTION_RE.match(line)
        if ms:
            sec = ms.group(1)
            if sec in seen_sections:
                errs.append(f"line {i}: duplicate section header {sec}")
            seen_sections.add(sec)
            cur_sec = sec
            continue
        if line.startswith('#'):
            continue
        m = ENTRY_RE.match(line)
        if not m:
            errs.append(f"line {i}: bad entry shape: {line[:80]!r}")
            continue
        n += 1
        nnn, mm, ref = m.group(1), int(m.group(2)), m.group(3)
        if cur_sec is None:
            errs.append(f"line {i}: entry {nnn}-{mm} before any section header")
        elif nnn != cur_sec:
            errs.append(f"line {i}: entry {nnn}-{mm} under section {cur_sec} (prefix mismatch)")
        if mm in per_section_m[nnn]:
            errs.append(f"line {i}: duplicate id {nnn}-{mm}")
        per_section_m[nnn].add(mm)
        if ref not in headings:
            errs.append(f"line {i}: ref §{ref} not a SPEC heading")
    print(f"nnm lint: {n} entries, {len(seen_sections)} sections, {len(errs)} errors")
    for e in errs[:50]:
        print(f"  E: {e}")
    return 1 if errs else 0


if __name__ == '__main__':
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    p = sys.argv[1]
    s = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(os.path.abspath(p)), 'SPEC.md')
    sys.exit(lint(p, s))
