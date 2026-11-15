"""Step 2: enumerate token sequences up to length N over the alphabet, pruned
to viable prefixes of Type, Expr or AltType; count what each recogniser
accepts per length and the intersections Type ∩ Expr, AltType ∩ Fallback and
TypeArgs ∩ Dim; list the intersections grouped by shape.

The six recognisers run in lock step as one state tuple.  Counts come from a
memoised recursion over (state tuple, remaining length): the recognisers are
deterministic on their interned state sets, so two prefixes that reach the
same tuple have identical futures, and each distinct tuple is expanded once
per remaining length.  Every sequence of length <= N is covered.  The
intersection lists are then enumerated explicitly, descending only into
prefixes whose joint-acceptance count is non-zero.
"""
import os
import sys
import time
from collections import defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from grammar import Grammar, Recogniser, render  # noqa: E402

START = ('Type', 'Expr', 'AltType', 'Fallback', 'TypeArgs', 'Dim')
IT, IE, IA, IF, IG, ID = range(6)
# columns of a count vector
COLS = ['Type', 'Expr', 'AltType', 'Fallback', 'TypeArgs', 'Dim',
        'Type∩Expr', 'AltType∩Fallback', 'TypeArgs∩Dim', 'viable(T|E|A)']
NCOL = len(COLS)
ZERO = (0,) * NCOL


class Lockstep:
    def __init__(self):
        self.g = Grammar()
        self.alpha = self.g.alphabet()
        self.R = [Recogniser(self.g, s) for s in START]
        self.init = tuple(r.init for r in self.R)
        self.memo = {}
        self.tmemo = {}

    def step(self, st, tok):
        key = (st, tok)
        r = self.tmemo.get(key)
        if r is None:
            r = tuple(0 if s == 0 else R.step(s, tok) for s, R in zip(st, self.R))
            self.tmemo[key] = r
        return r

    def alive_tea(self, st):
        return st[IT] or st[IE] or st[IA]

    def accepts(self, st):
        return tuple(R.accept[s] for s, R in zip(st, self.R))

    def leaf(self, st):
        a = self.accepts(st)
        return (int(a[IT]), int(a[IE]), int(a[IA]), int(a[IF]), int(a[IG]), int(a[ID]),
                int(a[IT] and a[IE]), int(a[IA] and a[IF]), int(a[IG] and a[ID]),
                1)

    def count(self, st, n):
        """Count vector over sequences of exactly n more tokens that keep at
        least one of Type/Expr/AltType alive (viable prefixes)."""
        if n == 0:
            return self.leaf(st)
        key = (st, n)
        r = self.memo.get(key)
        if r is not None:
            return r
        acc = [0] * NCOL
        for tok in self.alpha:
            nst = self.step(st, tok)
            if not self.alive_tea(nst):
                continue
            v = self.count(nst, n - 1)
            for i in range(NCOL):
                acc[i] += v[i]
        r = tuple(acc)
        self.memo[key] = r
        return r

    def enumerate_joint(self, col, n, st=None, prefix=()):
        """Yield every sequence of exactly n tokens whose count column `col`
        is accepted (joint acceptance), pruning by the memoised counts."""
        if st is None:
            st = self.init
        if n == 0:
            if self.leaf(st)[col]:
                yield prefix
            return
        for tok in self.alpha:
            nst = self.step(st, tok)
            if not self.alive_tea(nst):
                continue
            if self.count(nst, n - 1)[col] == 0:
                continue
            yield from self.enumerate_joint(col, n - 1, nst, prefix + (tok,))


def shape(toks):
    """Abstract a sequence to its shape: the contents of each top-level
    bracket, paren or brace group become '…' (a leading '..' or '^' inside a
    bracket group is kept), everything else stays."""
    out = []
    depth = 0
    i = 0
    opener = {'(': ')', '[': ']', '{': '}'}
    while i < len(toks):
        t = toks[i]
        if depth == 0:
            if t in opener:
                depth = 1
                out.append(t)
                if i + 1 < len(toks) and toks[i + 1] in ('..', '^') and t == '[':
                    out.append(toks[i + 1])
                if i + 1 < len(toks) and toks[i + 1] == opener[t]:
                    pass
                else:
                    out.append('…')
            else:
                out.append(t)
        else:
            if t in opener:
                depth += 1
            elif t in (')', ']', '}'):
                depth -= 1
                if depth == 0:
                    out.append(t)
        i += 1
    return tuple(out)


def main(N, outdir):
    os.makedirs(outdir, exist_ok=True)
    L = Lockstep()
    t0 = time.time()
    lines = []
    lines.append('alphabet (%d token kinds): %s' % (len(L.alpha), ' '.join(L.alpha)))
    lines.append('')
    lines.append('counts per length n (sequences of exactly n tokens):')
    header = '%4s ' % 'n' + ' '.join('%17s' % c for c in COLS)
    lines.append(header)
    totals = [0] * NCOL
    per_len = {}
    for n in range(1, N + 1):
        tn = time.time()
        v = L.count(L.init, n)
        per_len[n] = v
        for i in range(NCOL):
            totals[i] += v[i]
        lines.append('%4d ' % n + ' '.join('%17d' % x for x in v) + '   (%.1fs, %d memo states)' % (time.time() - tn, len(L.memo)))
    lines.append('%4s ' % 'sum' + ' '.join('%17d' % x for x in totals))
    lines.append('')
    print('\n'.join(lines))
    sys.stdout.flush()

    # intersections by shape
    inter = [('Type∩Expr', 6), ('AltType∩Fallback', 7), ('TypeArgs∩Dim', 8)]
    for name, col in inter:
        groups = defaultdict(lambda: defaultdict(int))
        example = {}
        all_seqs = []
        for n in range(1, N + 1):
            for seq in L.enumerate_joint(col, n):
                sh = shape(seq)
                groups[sh][n] += 1
                if sh not in example:
                    example[sh] = seq
                all_seqs.append(seq)
        lines.append('intersection %s: %d sequences up to length %d, %d shapes' % (name, len(all_seqs), N, len(groups)))
        lines.append('%-40s %-8s %-40s %s' % ('shape', 'count', 'by length', 'example (source text)'))
        for sh in sorted(groups, key=lambda s: (min(groups[s]), len(s), s)):
            bylen = ' '.join('n%d:%d' % (n, groups[sh][n]) for n in sorted(groups[sh]))
            lines.append('%-40s %-8d %-40s %s' % (' '.join(sh), sum(groups[sh].values()), bylen, render(example[sh])))
        lines.append('')
        with open(os.path.join(outdir, 'intersection_%s.txt' % name.replace('∩', '_')), 'w') as f:
            for seq in all_seqs:
                f.write('%s\t%s\n' % (' '.join(seq), render(seq)))
    lines.append('time %.1fs; distinct (state,n) memo entries %d; distinct state tuples %d'
                 % (time.time() - t0, len(L.memo), len({k[0] for k in L.memo})))
    text = '\n'.join(lines)
    with open(os.path.join(outdir, 'counts.txt'), 'w') as f:
        f.write(text + '\n')
    print('\n'.join(lines[len(lines) - (len(text.splitlines()) - header.count('') - 0):]) if False else '')
    print(text[text.index('intersection'):])
    return L


if __name__ == '__main__':
    N = int(sys.argv[1]) if len(sys.argv) > 1 else 5
    outdir = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(os.path.abspath(__file__)), 'out')
    main(N, outdir)
