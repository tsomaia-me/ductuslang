#!/usr/bin/env python3
"""assemble_lint.py - gate, arrange, assemble, and lint the Ductus DECISION_LOG production.

Modes:
  gate FILE...   C1 per-file lint on draft/ or reviewed/ chunk files (exit 1 on error)
  assemble       C2+C3: arrange reviewed/ per outline.tsv, renumber 1..N, global lint,
                 write assembled.md (exit 1 on error)
  final FILE     header-aware lint of the assembled/delivered log (skips to <!-- BEGIN LOG -->)

File formats expected (chunk files):
  <entries: 'NN-k. text (§x.y.z)' optionally + ' <!-- ooo -->'>
  [HANDOFF: §x.y]
  COVERAGE:
  §<num>: <int>          (every owned §, parents included; 0 allowed)
  FINDINGS: none  |  FINDINGS:\n- kind: ... | sections: §a, §b | ...
  <!-- END NN n=K -->

Reports (report/NN.md) consumed by C3 coverage net:
  NO-NORMATIVE-CONTENT: block with '§<num>: reason' lines; FINDINGS lines ( § tokens ).
"""
import sys, os, re, collections

WORK = os.path.dirname(os.path.abspath(__file__))
SPEC = os.path.normpath(os.path.join(WORK, '..', 'SPEC.md'))

SEC = r'\d+(?:\.\d+){0,3}'
OOO = '<!-- ooo -->'
REF_RE = re.compile(r'\(§(' + SEC + r')\)\s*(?:' + re.escape(OOO) + r')?\s*$')
SENTINEL_RE = re.compile(r'^<!-- END (\d{2}) n=(\d+) -->\s*$')
COV_RE = re.compile(r'^§(' + SEC + r'):\s*(\d+)\s*$')
SECTOK_RE = re.compile(r'§(' + SEC + r')')
FINAL_ENTRY_RE = re.compile(r'^(\d+)\. \S.* \(§(' + SEC + r')\)$')


def load_headings():
    hs, fence = set(), False
    pat = re.compile(r'^#{1,6}\s+(' + SEC + r'(?:\.\d+)*)\.?(?:\s|$)')
    for line in open(SPEC, encoding='utf-8'):
        if line.lstrip().startswith('```'):
            fence = not fence
            continue
        if fence or not line.startswith('#'):
            continue
        m = pat.match(line)
        if m:
            hs.add(m.group(1))
    return hs


def load_manifest():
    man = {}
    with open(os.path.join(WORK, 'manifest.tsv'), encoding='utf-8') as f:
        next(f)
        for row in f:
            cid, a, b, owned = row.rstrip('\n').split('\t')
            man[cid] = (int(a), int(b), owned.split(','))
    return man


def sec_key(s):
    return tuple(int(p) for p in s.split('.'))


def parse_chunk_file(path, cid, errors, warnings):
    """Returns (entries, coverage, findings_lines, handoff). Entry = (idx, text_line)."""
    entry_re = re.compile(r'^' + re.escape(cid) + r'-(\d+)\. (\S.*) \(§(' + SEC + r')\)\s*(' + re.escape(OOO) + r')?\s*$')
    lines = open(path, encoding='utf-8').read().split('\n')
    entries, coverage, findings, handoff = [], {}, [], None
    sentinel = None
    mode = 'entries'
    for i, line in enumerate(lines, 1):
        if not line.strip():
            continue
        m = SENTINEL_RE.match(line)
        if m:
            sentinel = (m.group(1), int(m.group(2)))
            continue
        if line.startswith('HANDOFF:'):
            handoff = line.strip()
            continue
        if line.strip() == 'COVERAGE:':
            mode = 'coverage'
            continue
        if line.strip() == 'FINDINGS: none':
            mode = 'findings'
            continue
        if line.strip() == 'FINDINGS:':
            mode = 'findings'
            continue
        if mode == 'coverage':
            m = COV_RE.match(line.strip())
            if m:
                coverage[m.group(1)] = int(m.group(2))
                continue
            errors.append(f"{path}:{i}: bad COVERAGE line: {line[:80]!r}")
            continue
        if mode == 'findings':
            if line.startswith('- '):
                findings.append(line)
                continue
            errors.append(f"{path}:{i}: bad FINDINGS line: {line[:80]!r}")
            continue
        # entry territory
        if line.startswith('```'):
            errors.append(f"{path}:{i}: code fence in chunk file")
            continue
        if '\t' in line:
            errors.append(f"{path}:{i}: tab character")
        if re.match(r'^\d+\. ', line):
            errors.append(f"{path}:{i}: bare global number (forgot 'NN-' placeholder): {line[:60]!r}")
            continue
        m = entry_re.match(line)
        if not m:
            errors.append(f"{path}:{i}: malformed entry: {line[:100]!r}")
            continue
        entries.append((int(m.group(1)), line.rstrip()))
        if len(line) > 300:
            warnings.append(f"{path}:{i}: entry > 300 chars")
        body = m.group(2)
        if '; ' in body:
            warnings.append(f"{path}:{i}: atomicity smell '; '")
        if ' and also ' in body:
            warnings.append(f"{path}:{i}: atomicity smell ' and also '")
    return entries, coverage, findings, handoff, sentinel


def gate(paths):
    headings = load_headings()
    man = load_manifest()
    any_err = False
    for path in paths:
        errors, warnings = [], []
        cid = os.path.basename(path)[:2]
        if cid not in man:
            print(f"ERROR {path}: unknown chunk id {cid!r}")
            any_err = True
            continue
        _, _, owned = man[cid]
        owned_set = set(owned)
        if not os.path.exists(path) or os.path.getsize(path) == 0:
            print(f"ERROR {path}: missing or empty")
            any_err = True
            continue
        entries, coverage, findings, handoff, sentinel = parse_chunk_file(path, cid, errors, warnings)
        if sentinel is None:
            errors.append(f"{path}: missing END sentinel (truncation?)")
        else:
            sid, k = sentinel
            if sid != cid:
                errors.append(f"{path}: sentinel id {sid} != chunk {cid}")
            if k != len(entries):
                errors.append(f"{path}: sentinel n={k} but {len(entries)} entries parsed")
        idxs = [n for n, _ in entries]
        if idxs != list(range(1, len(idxs) + 1)):
            errors.append(f"{path}: placeholder indices not dense 1..K")
        if not coverage:
            errors.append(f"{path}: missing COVERAGE block")
        else:
            missing_cov = owned_set - set(coverage)
            if missing_cov:
                errors.append(f"{path}: COVERAGE missing owned §: {sorted(missing_cov, key=sec_key)}")
        # findings block presence: parse_chunk_file only fills on block; check raw text
        raw = open(path, encoding='utf-8').read()
        if 'FINDINGS:' not in raw:
            errors.append(f"{path}: missing FINDINGS block/attestation")
        prev = None
        for n, line in entries:
            ref = REF_RE.search(line).group(1)
            if ref not in headings:
                errors.append(f"{path}: {cid}-{n}: ref §{ref} not a SPEC heading")
            if ref not in owned_set:
                errors.append(f"{path}: {cid}-{n}: ref §{ref} outside owned set")
            is_ooo = line.endswith(OOO)
            if prev is not None and not is_ooo and ref in owned_set and prev in owned_set:
                if sec_key(ref) < sec_key(prev):
                    errors.append(f"{path}: {cid}-{n}: § order regression §{prev} -> §{ref} (annotate ' {OOO}' if intentional)")
            if not is_ooo:
                prev = ref
        # per-§ coverage consistency: counted refs vs declared
        ref_counts = collections.Counter(REF_RE.search(l).group(1) for _, l in entries)
        for s, declared in coverage.items():
            actual = ref_counts.get(s, 0)
            if declared != actual:
                warnings.append(f"{path}: COVERAGE says §{s}: {declared}, actual {actual}")
        status = 'ERROR' if errors else 'OK'
        print(f"[{status}] {path}: {len(entries)} entries, {len(findings)} findings"
              + (f", {handoff}" if handoff else ""))
        for e in errors:
            print(f"  E: {e}")
        for w in warnings:
            print(f"  W: {w}")
        if errors:
            any_err = True
    return 1 if any_err else 0


def load_outline():
    """outline.tsv: order\theader\tprefixes(comma). exceptions.tsv: section\torder (optional)."""
    topics = []
    with open(os.path.join(WORK, 'outline.tsv'), encoding='utf-8') as f:
        next(f)
        for row in f:
            order, header, prefixes = row.rstrip('\n').split('\t')
            topics.append((int(order), header, prefixes.split(',')))
    topics.sort()
    exceptions = {}
    exc_path = os.path.join(WORK, 'exceptions.tsv')
    if os.path.exists(exc_path):
        with open(exc_path, encoding='utf-8') as f:
            next(f)
            for row in f:
                sec, order = row.rstrip('\n').split('\t')
                exceptions[sec] = int(order)
    return topics, exceptions


def topic_for(ref, topics, exceptions):
    if ref in exceptions:
        return exceptions[ref]
    best, best_len = None, -1
    for order, _, prefixes in topics:
        for p in prefixes:
            if ref == p or ref.startswith(p + '.'):
                if len(p) > best_len:
                    best, best_len = order, len(p)
    return best


def assemble():
    headings = load_headings()
    man = load_manifest()
    topics, exceptions = load_outline()
    errors, warnings = [], []

    # collect all reviewed entries in manifest order
    expected = sorted(man.keys())
    present = sorted(f[:2] for f in os.listdir(os.path.join(WORK, 'reviewed')) if f.endswith('.md'))
    if present != expected:
        print(f"ERROR: reviewed/ chunk set mismatch. expected {expected}, present {present}")
        return 1

    all_entries = []  # (spec_order_idx, canonical_text, ref, ooo)
    per_chunk = {}
    order_idx = 0
    for cid in expected:
        path = os.path.join(WORK, 'reviewed', f'{cid}.md')
        entries, _, _, handoff, _ = parse_chunk_file(path, cid, errors, warnings)
        if handoff:
            errors.append(f"{path}: unresolved {handoff}")
        per_chunk[cid] = len(entries)
        for n, line in entries:
            ref = REF_RE.search(line).group(1)
            ooo = line.endswith(OOO)
            text = re.sub(r'^' + re.escape(cid) + r'-\d+\. ', '', line)
            if ooo:
                text = text[: -len(OOO)].rstrip()
            all_entries.append((order_idx, text, ref, ooo))
            order_idx += 1
    if errors:
        for e in errors:
            print(f"E: {e}")
        return 1

    pre_multiset = sorted(t for _, t, _, _ in all_entries)

    # arrange: stable sort by topic order
    placed = []
    for idx, text, ref, ooo in all_entries:
        t = topic_for(ref, topics, exceptions)
        if t is None:
            errors.append(f"no topic for §{ref}: {text[:60]!r}")
            continue
        placed.append((t, idx, text, ref))
    if errors:
        for e in errors:
            print(f"E: {e}")
        return 1
    placed.sort(key=lambda x: (x[0], x[1]))

    post_multiset = sorted(t for _, _, t, _ in placed)
    if pre_multiset != post_multiset:
        print("ERROR: arrangement is NOT a pure permutation (multiset mismatch)")
        return 1

    # emit with topic headers + renumber
    out_lines, n = [], 0
    cur_topic = None
    header_of = {order: header for order, header, _ in topics}
    for t, _, text, ref in placed:
        if t != cur_topic:
            if out_lines:
                out_lines.append('')
            out_lines.append(f'## {header_of[t]}')
            out_lines.append('')
            cur_topic = t
        n += 1
        out_lines.append(f'{n}. {text}')

    # C3 global lint on emitted body
    for line in out_lines:
        if line and not line.startswith('## ') and not FINAL_ENTRY_RE.match(line):
            errors.append(f"final-shape violation: {line[:100]!r}")
    cited = set()
    for line in out_lines:
        m = FINAL_ENTRY_RE.match(line)
        if m:
            cited.add(m.group(2))

    # coverage net: cited | no-normative-content rows | findings sections
    excused = set()
    rep_dir = os.path.join(WORK, 'report')
    for fn in os.listdir(rep_dir):
        if not fn.endswith('.md'):
            continue
        raw = open(os.path.join(rep_dir, fn), encoding='utf-8').read()
        in_nnc = False
        for line in raw.split('\n'):
            stripped = line.strip()
            if stripped.startswith('NO-NORMATIVE-CONTENT'):
                in_nnc = True
                continue
            if re.match(r'^(REMOVALS|UNRESOLVED|FINDINGS|ORCHESTRATOR|#|\|)', stripped):
                in_nnc = False
            if in_nnc:
                m = re.match(r'^[-\s]*§(' + SEC + r')\b', stripped)
                if m:
                    excused.add(m.group(1))
                    continue
            if 'FINDINGS' in line or stripped.startswith('- kind:'):
                for m in SECTOK_RE.finditer(line):
                    excused.add(m.group(1))
    uncovered = load_headings() - cited - excused
    if uncovered:
        errors.append(f"coverage net: {len(uncovered)} §s neither cited nor excused: "
                      + ', '.join('§' + s for s in sorted(uncovered, key=sec_key)))

    # duplicates (normalized)
    norm = collections.Counter()
    for line in out_lines:
        m = FINAL_ENTRY_RE.match(line)
        if m:
            t = re.sub(r'\s*\(§' + SEC + r'\)$', '', line.split('. ', 1)[1])
            norm[re.sub(r'\s+', ' ', t.lower())] += 1
    for t, c in norm.items():
        if c > 1:
            warnings.append(f"possible duplicate x{c}: {t[:80]!r}")

    with open(os.path.join(WORK, 'assembled.md'), 'w', encoding='utf-8') as f:
        f.write('\n'.join(out_lines) + '\n')

    report = [f"entries: {n}", "per-chunk: " + ', '.join(f"{c}={k}" for c, k in sorted(per_chunk.items()))]
    report += [f"E: {e}" for e in errors] + [f"W: {w}" for w in warnings]
    open(os.path.join(WORK, 'lint.txt'), 'w', encoding='utf-8').write('\n'.join(report) + '\n')
    print('\n'.join(report))
    return 1 if errors else 0


def final(path):
    headings = load_headings()
    lines = open(path, encoding='utf-8').read().split('\n')
    try:
        start = lines.index('<!-- BEGIN LOG -->') + 1
    except ValueError:
        print('ERROR: no <!-- BEGIN LOG --> marker')
        return 1
    n, errs = 0, []
    for i, line in enumerate(lines[start:], start + 1):
        if not line.strip() or line.startswith('## '):
            continue
        m = FINAL_ENTRY_RE.match(line)
        if not m:
            errs.append(f"line {i}: bad shape: {line[:80]!r}")
            continue
        n += 1
        if int(m.group(1)) != n:
            errs.append(f"line {i}: expected number {n}, got {m.group(1)}")
            n = int(m.group(1))
        if m.group(2) not in headings:
            errs.append(f"line {i}: ref §{m.group(2)} not a SPEC heading")
    print(f"final lint: {n} entries, {len(errs)} errors")
    for e in errs:
        print(f"  E: {e}")
    return 1 if errs else 0


if __name__ == '__main__':
    mode = sys.argv[1] if len(sys.argv) > 1 else ''
    if mode == 'gate':
        sys.exit(gate(sys.argv[2:]))
    elif mode == 'assemble':
        sys.exit(assemble())
    elif mode == 'final':
        sys.exit(final(sys.argv[2]))
    else:
        print(__doc__)
        sys.exit(2)
