"""Rebuild the production count and the cycles from SPEC.md by my own method:
tokenize each ebnf block's right-hand side (quoted literals, ranges, operators, names),
build the reference graph, then compute the transitive closure by repeated squaring
(Warshall) and call a production cyclic when it reaches itself."""
import sys, re
text = open(sys.argv[1], encoding='utf-8').read()
lines = text.split('\n')
in_ebnf = False
prods = []          # (name, rhs, lineno)
cur = None
for i, ln in enumerate(lines, 1):
    if ln.startswith('```'):
        if ln.strip() == '```ebnf':
            in_ebnf = True
        else:
            in_ebnf = False
            cur = None
        continue
    if not in_ebnf:
        continue
    if '::=' in ln:
        lhs, rhs = ln.split('::=', 1)
        cur = [lhs.strip(), rhs.strip(), i]
        prods.append(cur)
    elif cur is not None and ln.strip():
        cur[1] += ' ' + ln.strip()
names = [p[0] for p in prods]
dups = [n for n in set(names) if names.count(n) > 1]
print('productions:', len(prods), 'distinct:', len(set(names)), 'duplicates:', dups)
defined = set(names)
# tokenizer for the rhs
def toks(rhs):
    out = []
    j = 0
    while j < len(rhs):
        c = rhs[j]
        if c == "'":
            k = j + 1
            while k < len(rhs) and rhs[k] != "'":
                k += 1
            out.append(('lit', rhs[j+1:k])); j = k + 1
        elif c == '"':
            k = j + 1
            while k < len(rhs) and rhs[k] != '"':
                k += 1
            out.append(('lit', rhs[j+1:k])); j = k + 1
        elif c.isalpha() or c == '_':
            k = j
            while k < len(rhs) and (rhs[k].isalnum() or rhs[k] == '_'):
                k += 1
            out.append(('name', rhs[j:k])); j = k
        elif c.isspace():
            j += 1
        else:
            out.append(('op', c)); j += 1
    return out
refs = {}
unknown = {}
for name, rhs, ln in prods:
    if rhs.startswith('any character'):
        refs[name] = set(); continue
    rs = set()
    for kind, v in toks(rhs):
        if kind == 'name':
            if v in defined:
                rs.add(v)
            elif v not in ('NEWLINE', 'INDENT', 'DEDENT', 'Name', 'any', 'character', 'except', 'and', 'a', 'line', 'break', 'below'):
                unknown.setdefault(v, []).append(name)
    refs[name] = rs
print('names referenced but not defined (tokens/prose):', {k: v for k, v in unknown.items()})
idx = {n: i for i, n in enumerate(names)}
N = len(names)
reach = [[False]*N for _ in range(N)]
for n in names:
    for r in refs[n]:
        reach[idx[n]][idx[r]] = True
# Warshall
for k in range(N):
    rk = reach[k]
    for i in range(N):
        if reach[i][k]:
            ri = reach[i]
            for j in range(N):
                if rk[j]:
                    ri[j] = True
cyclic = [n for n in names if reach[idx[n]][idx[n]]]
# group cyclic ones into components: same component iff mutually reachable
comps = []
seen = set()
for n in cyclic:
    if n in seen: continue
    comp = [m for m in cyclic if reach[idx[n]][idx[m]] and reach[idx[m]][idx[n]]]
    seen.update(comp)
    comps.append(comp)
# lexical by my own criterion: productions in §2.6 (lines before '## 3.') plus the Json character/number ones
sec26 = [p[0] for p in prods if p[2] < next(i for i, l in enumerate(lines, 1) if l.startswith('## 3.'))]
json_lex = [n for n in names if n in ('JsonString', 'JsonChar', 'JsonEscape', 'JsonNumber')]
lexical = [n for n in sec26 if n != 'StringLit'] + json_lex
print('lexical by my criterion (section 2.6 minus StringLit, plus the four JSON text productions):', len(lexical))
print('   ', lexical)
print('on a cycle:', len(cyclic))
for c in comps:
    print('  component of', len(c), ':', c)
plain = [n for n in names if n not in cyclic and n not in lexical]
print('plain (not cyclic, not lexical):', len(plain))
print('   ', plain)
cyc_nonlex = [n for n in cyclic if n not in lexical]
print('cyclic non-lexical:', len(cyc_nonlex), ' lexical on a cycle:', [n for n in cyclic if n in lexical])
# also: which productions reach Expr, and which Expr reaches
e = idx['Expr']
print('Expr reaches Body?', reach[e][idx['Body']], ' Body reaches Expr?', reach[idx['Body']][e])
print('Expr reaches MatchBlock?', reach[e][idx['MatchBlock']], ' ForExprBlock?', reach[e][idx['ForExprBlock']], ' LocalLet?', reach[e][idx['LocalLet']])
print('WirePlacement reaches Body?', reach[idx['WirePlacement']][idx['Body']])
print('Member reaches Body?', reach[idx['Member']][idx['Body']], ' Placement reaches Body?', reach[idx['Placement']][idx['Body']])
# productions that only list alternatives of single names (choosers) by my own test
chooser = []
for name, rhs, ln in prods:
    alts = [a.strip() for a in rhs.split('|')]
    if len(alts) > 1 and all(len(toks(a)) == 1 and toks(a)[0][0] == 'name' for a in alts):
        chooser.append(name)
print('productions whose every alternative is one bare name:', chooser)
print()
for name, rhs, ln in prods:
    print(f'{ln:5d} {name:16s} ::= {rhs}')
