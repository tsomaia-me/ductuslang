"""Step 2b: list the intersections Type ∩ Expr, AltType ∩ Fallback and
TypeArgs ∩ Dim to a longer length than the full enumeration reaches.

Only prefixes viable for BOTH recognisers of a pair can lead to a sequence in
the intersection, so the search keeps the pair alive and prunes everything
else; that state space is far smaller than the one of all viable prefixes.
Counts for lengths the full enumeration (counts.py) also covers must agree
with it: a cross-check between the two searches.
"""
import os
import sys
import time
from collections import defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from grammar import Grammar, Recogniser, render  # noqa: E402
from counts import shape  # noqa: E402

PAIRS = [('Type∩Expr', 'Type', 'Expr'), ('AltType∩Fallback', 'AltType', 'Fallback'),
         ('TypeArgs∩Dim', 'TypeArgs', 'Dim')]


class Pair:
    def __init__(self, g, a, b):
        self.alpha = g.alphabet()
        self.A = Recogniser(g, a)
        self.B = Recogniser(g, b)
        self.memo = {}

    def count(self, a, b, n):
        if n == 0:
            return int(self.A.accept[a] and self.B.accept[b])
        key = (a, b, n)
        r = self.memo.get(key)
        if r is not None:
            return r
        tot = 0
        for t in self.alpha:
            a2 = self.A.step(a, t)
            if a2 == 0:
                continue
            b2 = self.B.step(b, t)
            if b2 == 0:
                continue
            tot += self.count(a2, b2, n - 1)
        self.memo[key] = tot
        return tot

    def enumerate(self, n, a=None, b=None, prefix=()):
        if a is None:
            a, b = self.A.init, self.B.init
        if n == 0:
            if self.A.accept[a] and self.B.accept[b]:
                yield prefix
            return
        for t in self.alpha:
            a2 = self.A.step(a, t)
            if a2 == 0:
                continue
            b2 = self.B.step(b, t)
            if b2 == 0 or self.count(a2, b2, n - 1) == 0:
                continue
            yield from self.enumerate(n - 1, a2, b2, prefix + (t,))


def main(N, outdir):
    os.makedirs(outdir, exist_ok=True)
    g = Grammar()
    lines = []
    t0 = time.time()
    for name, a, b in PAIRS:
        P = Pair(g, a, b)
        groups = defaultdict(lambda: defaultdict(int))
        example = {}
        per_len = {}
        total = 0
        tn = time.time()
        with open(os.path.join(outdir, 'intersection_%s_N%d.txt' % (name.replace('∩', '_'), N)), 'w') as f:
            for n in range(1, N + 1):
                tl = time.time()
                per_len[n] = P.count(P.A.init, P.B.init, n)
                print('  %s n=%d: %d sequences in the intersection (%.1fs, %d memo entries)'
                      % (name, n, per_len[n], time.time() - tl, len(P.memo)), file=sys.stderr)
                sys.stderr.flush()
                for seq in P.enumerate(n):
                    total += 1
                    sh = shape(seq)
                    groups[sh][n] += 1
                    if sh not in example:
                        example[sh] = seq
                    f.write('%s\t%s\n' % (' '.join(seq), render(seq)))
        lines.append('intersection %s up to length %d: %d sequences, %d shapes (%.1fs, %d memo entries)'
                     % (name, N, total, len(groups), time.time() - tn, len(P.memo)))
        lines.append('   by length: ' + ' '.join('n%d:%d' % (n, per_len[n]) for n in range(1, N + 1)))
        lines.append('   %-44s %-7s %-48s %s' % ('shape', 'count', 'by length', 'example (source text)'))
        for sh in sorted(groups, key=lambda s: (min(groups[s]), len(s), s)):
            bylen = ' '.join('n%d:%d' % (n, groups[sh][n]) for n in sorted(groups[sh]))
            lines.append('   %-44s %-7d %-48s %s' % (' '.join(sh), sum(groups[sh].values()), bylen, render(example[sh])))
        lines.append('')
        print('\n'.join(lines[-(4 + len(groups)):]))
        sys.stdout.flush()
    lines.append('time %.1fs' % (time.time() - t0))
    with open(os.path.join(outdir, 'intersections.txt'), 'w') as f:
        f.write('\n'.join(lines) + '\n')


if __name__ == '__main__':
    N = int(sys.argv[1]) if len(sys.argv) > 1 else 8
    outdir = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(os.path.abspath(__file__)), 'out')
    main(N, outdir)
