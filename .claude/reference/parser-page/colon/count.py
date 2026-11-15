#!/usr/bin/env python3
"""The peer's rough count for the parser's page: on body lines `Kind ...: REST`, how often does
REST fit the reading as inline children, the reading as a wire's destination, or both, by its
tokens alone. A rough classifier over valid programs; no parser is used."""
import re, sys, glob, collections

KEY = set("let state stream channel fn pattern type default const cell out children if then else match when for in into from as on without is not and or div to true false none u8 i32 i64 f32 f64 bool str json node wire require use publish".split())
TOK = re.compile(r'\s*(?:(//.*)|("(?:\\.|[^"\\])*")|(\d[\w.]*)|([^\W\d][\w#]*|#[\w#]*)|(::|\.\.|\*\*|\?\?|<=|>=|=>|->|.))')

def toks(text):
    out, pos = [], 0
    while pos < len(text):
        m = TOK.match(text, pos)
        if not m: break
        pos = m.end()
        if m.group(1): break
        kind = 'str' if m.group(2) else 'num' if m.group(3) else 'name' if m.group(4) else 'op'
        out.append((kind, m.group(m.lastindex)))
    return out

def split_colon(ts):
    depth = 0
    for i, (k, t) in enumerate(ts):
        if t in '([{': depth += 1
        elif t in ')]}': depth -= 1
        elif t == ':' and depth == 0: return ts[:i], ts[i + 1:]
    return ts, None

def group_end(ts, i):
    depth = 0
    for j in range(i, len(ts)):
        if ts[j][1] in '([{': depth += 1
        elif ts[j][1] in ')]}':
            depth -= 1
            if depth == 0: return j + 1
    return len(ts)

FLAGS = set("'!?*+~@$")

def atom(ts, i):
    if i < len(ts) and ts[i][1] == '-' and i + 1 < len(ts) and ts[i + 1][0] == 'num': return i + 2
    if i >= len(ts): return None
    k, t = ts[i]
    if k in ('num', 'str'): return i + 1
    if t == '(': return group_end(ts, i)
    if k == 'name' and (t not in KEY or t in ('true', 'false', 'to')):
        i += 1
        if i + 1 < len(ts) and ts[i][1] == '::' and ts[i + 1][0] == 'name': i += 2
        while i + 1 < len(ts) and ts[i][1] == '.' and ts[i + 1][0] == 'name': i += 2
        return i
    return None

def members(ts):
    """The number of self-delimited members if ts is nothing but members, else None; and whether
    a member carries a flag or an `as` name."""
    i, n, marked, later_group = 0, 0, False, False
    while i < len(ts):
        k, t = ts[i]
        if t in '([':
            if n > 0: later_group = True
            i = group_end(ts, i); n += 1; continue
        if k != 'name' or t in KEY: return None
        i += 1
        if i + 1 < len(ts) and ts[i][1] == '::' and ts[i + 1][0] == 'name': i += 2
        while i < len(ts) and ts[i][1] in FLAGS and ts[i][1] != '/':
            # a flag stands directly after the kind; the rough tokens do not keep whitespace
            if ts[i][1] == '*' and i + 1 < len(ts) and ts[i + 1][1] not in '/' and ts[i + 1][0] != 'name': return None
            marked = True; i += 1
        if i + 1 < len(ts) and ts[i][1] == 'as' and ts[i + 1][0] == 'name': marked = True; i += 2
        if i < len(ts) and ts[i][1] == '/':
            j = atom(ts, i + 1)
            if j is None: return None
            i = j
        n += 1
    return n, marked, later_group

def classify(rest):
    if rest[0][1] == 'match': return 'destination only'
    m = members(rest)
    if m is None: return 'destination only'
    n, marked, later_group = m
    if marked: return 'children only'
    if n == 1:
        if rest[0][1] == '[' and any(t == ',' for _, t in rest): return 'destination only'
        return 'both'
    return 'both' if later_group and n == 2 else 'children only'

counts = collections.Counter(); both_tokens = 0; all_tokens = 0; examples = collections.defaultdict(list)
for path in sys.argv[1:]:
    stack = []  # (indent, kind of block)
    for raw in open(path, encoding='utf-8').read().split('\n'):
        if not raw.strip() or raw.strip().startswith('//'): continue
        indent = len(raw) - len(raw.lstrip(' '))
        ts = toks(raw)
        if not ts: continue
        all_tokens += len(ts)
        while stack and stack[-1][0] >= indent: stack.pop()
        inside = stack[-1][1] if stack else 'top'
        first = ts[0][1]
        opens = ts[-1][1] == ':'
        if opens:
            if any(t == 'match' for _, t in ts): kind = 'match'
            elif first in ('fn', 'type', 'let', 'state', 'stream', 'channel') or (first == 'for' and any(t == 'into' for _, t in ts)): kind = 'other'
            elif first == 'when' and len(ts) == 2: kind = 'guards'
            elif inside in ('match', 'other'): kind = 'other'
            else: kind = 'body'
            stack.append((indent, kind))
        if inside != 'body' or ts[0][0] != 'name' or first in KEY: continue
        head, rest = split_colon(ts)
        if rest is None: counts['no colon'] += 1; continue
        if not rest: counts['colon, then a block'] += 1; continue
        c = classify(rest)
        counts[c] += 1
        if c == 'both': both_tokens += len(rest)
        if len(examples[c]) < 6: examples[c].append(raw.strip())
print('files', len(sys.argv) - 1, 'tokens', all_tokens)
for k, v in counts.most_common(): print('%6d  %s' % (v, k))
print('tokens after the colon on the lines that fit both:', both_tokens)
for k, v in examples.items():
    print(k); [print('     ', e[:100]) for e in v]
