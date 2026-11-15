#!/usr/bin/env python3
"""Which of the SPEC's productions lie on a cycle of the grammar. Reads the `ebnf` blocks of
SPEC.md, builds the graph of which production names which, and prints the strongly connected
components with more than one member or a self-loop. Run: python3 -I cycles.py <path to SPEC.md>."""
import re, sys
text = open(sys.argv[1], encoding='utf-8').read()
blocks = re.findall(r'```ebnf\n(.*?)```', text, re.S)
prods, order = {}, []
for b in blocks:
    cur = None
    for line in b.split('\n'):
        m = re.match(r'^([A-Za-z]+)\s*::=\s*(.*)$', line)
        if m:
            cur = m.group(1); prods[cur] = m.group(2); order.append(cur)
        elif cur and line.strip():
            prods[cur] += ' ' + line.strip()
print('productions', len(order), 'distinct', len(prods))
LEX = set('NumberLit PrefixedInt FloatLit Exponent Digits HexDigits BinDigits OctDigits Digit HexDigit BinDigit OctDigit Suffix TypeSuffix UnitSuffix BoolLit Char Escape JsonString JsonChar JsonEscape JsonNumber'.split())
def refs(name):
    rhs = prods[name]
    if rhs.startswith('any character'): return []
    s = re.sub(r"'(?:[^'\\]|\\.)*'", ' ', rhs)
    return [w for w in re.findall(r'[A-Za-z][A-Za-z0-9]*', s) if w in prods and w not in ('NEWLINE', 'INDENT', 'DEDENT')]
graph = {n: set(refs(n)) for n in prods}
index, low, onstack, stack, sccs, counter = {}, {}, set(), [], [], [0]
sys.setrecursionlimit(10000)
def strong(v):
    index[v] = low[v] = counter[0]; counter[0] += 1
    stack.append(v); onstack.add(v)
    for w in graph[v]:
        if w not in index:
            strong(w); low[v] = min(low[v], low[w])
        elif w in onstack:
            low[v] = min(low[v], index[w])
    if low[v] == index[v]:
        comp = []
        while True:
            w = stack.pop(); onstack.discard(w); comp.append(w)
            if w == v: break
        sccs.append(comp)
for v in order:
    if v not in index: strong(v)
oncycle = set()
for comp in sccs:
    if len(comp) > 1 or comp[0] in graph[comp[0]]: oncycle.update(comp)
lex = [n for n in order if n in LEX]
cyc = [n for n in order if n in oncycle and n not in LEX]
plain = [n for n in order if n not in oncycle and n not in LEX]
print('lexical', len(lex)); print('  ', ' '.join(lex))
print('on a cycle', len(cyc)); print('  ', ' '.join(cyc))
print('plain', len(plain)); print('  ', ' '.join(plain))
for comp in sccs:
    if len(comp) > 1 or comp[0] in graph[comp[0]]:
        print('cycle of', len(comp), sorted(comp, key=order.index))
