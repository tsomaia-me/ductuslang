"""Step 3b: compare the single-pass reader with the recognisers on every token
sequence up to length N, in every position.

For a position with recogniser pair (A, B), every sequence of length <= N is
one of:
  * a viable prefix of A or B: visited explicitly below; the reader's answer
    is compared with the recognisers' at that point;
  * not viable: the recognisers answer neither for it and all its extensions.
    When the reader is still alive there, the reader is run alone until it
    dies or the length bound is reached, and must answer neither throughout.

Both machines are deterministic, so the pair (reader state, recogniser state)
determines every future; a pair already verified to depth >= n is not
re-expanded.  That makes the check exhaustive over all 58^N sequences while
visiting each distinct pair once per depth.
"""
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from grammar import Grammar, Recogniser, render  # noqa: E402
import reader  # noqa: E402

PAIRS = {'free': ('Type', 'Expr'), 'typearg': ('Type', 'Expr'),
         'is': ('AltType', 'Fallback'), 'bracket': ('TypeArgs', 'Dim')}
LABEL = {'free': ('type-only', 'expression-only'), 'typearg': ('type-only', 'expression-only'),
         'is': ('type-only', 'expression-only'), 'bracket': ('type-only', 'expression-only')}


def rec_answer(RA, RB, a, b):
    ta = RA.accept[a] if a else False
    tb = RB.accept[b] if b else False
    if ta and tb:
        return 'both'
    if ta:
        return 'type-only'
    if tb:
        return 'expression-only'
    return 'neither'


class Compare:
    def __init__(self, g, position, N, max_report=40):
        self.g = g
        self.alpha = g.alphabet()
        self.position = position
        a, b = PAIRS[position]
        self.RA = Recogniser(g, a)
        self.RB = Recogniser(g, b)
        self.N = N
        self.visited = {}
        self.alone = {}
        self.disagreements = []
        self.n_dis = 0
        self.n_pairs = 0
        self.n_alone = 0
        self.max_report = max_report
        self.per_len = {}  # length -> [agree, disagree]

    def note(self, seq, want, got):
        self.n_dis += 1
        if len(self.disagreements) < self.max_report:
            self.disagreements.append((seq, want, got))

    def check(self, rs, a, b, n, seq):
        key = (rs, a, b)
        if self.visited.get(key, -1) >= n:
            return
        self.visited[key] = n
        self.n_pairs += 1
        want = rec_answer(self.RA, self.RB, a, b)
        got = reader.answer(rs)
        L = len(seq)
        c = self.per_len.setdefault(L, [0, 0])
        if want != got:
            c[1] += 1
            self.note(seq, want, got)
        else:
            c[0] += 1
        if n == 0:
            return
        for t in self.alpha:
            a2 = self.RA.step(a, t) if a else 0
            b2 = self.RB.step(b, t) if b else 0
            rs2 = reader.step(rs, t)
            if a2 == 0 and b2 == 0:
                if rs2 is not None:
                    self.check_alone(rs2, n - 1, seq + (t,))
                continue
            if rs2 is None:
                # recognisers alive, reader dead: the reader answers neither for every
                # extension; a disagreement arises where a recogniser accepts one.
                self.check_dead_reader(a2, b2, n - 1, seq + (t,))
                continue
            self.check(rs2, a2, b2, n - 1, seq + (t,))

    def check_alone(self, rs, n, seq):
        """Recognisers dead, reader alive: the reader must say neither throughout."""
        key = rs
        if self.alone.get(key, -1) >= n:
            return
        self.alone[key] = n
        self.n_alone += 1
        got = reader.answer(rs)
        if got != 'neither':
            self.note(seq, 'neither', got)
        if n == 0:
            return
        for t in self.alpha:
            rs2 = reader.step(rs, t)
            if rs2 is not None:
                self.check_alone(rs2, n - 1, seq + (t,))

    def check_dead_reader(self, a, b, n, seq):
        key = (None, a, b)
        if self.visited.get(key, -1) >= n:
            return
        self.visited[key] = n
        want = rec_answer(self.RA, self.RB, a, b)
        if want != 'neither':
            self.note(seq, want, 'neither')
        if n == 0:
            return
        for t in self.alpha:
            a2 = self.RA.step(a, t) if a else 0
            b2 = self.RB.step(b, t) if b else 0
            if a2 == 0 and b2 == 0:
                continue
            self.check_dead_reader(a2, b2, n - 1, seq + (t,))

    def run(self):
        t0 = time.time()
        self.check(reader.init(self.position), self.RA.init, self.RB.init, self.N, ())
        self.time = time.time() - t0
        return self


def main(N, outdir):
    os.makedirs(outdir, exist_ok=True)
    g = Grammar()
    lines = []
    total = 0
    for position in ('free', 'is', 'typearg', 'bracket'):
        c = Compare(g, position, N).run()
        total += c.n_dis
        lines.append('position %-8s N=%d: %d disagreements; %d distinct (reader, recogniser) pairs, '
                     '%d reader-alone states, %.1fs' % (position, N, c.n_dis, c.n_pairs, c.n_alone, c.time))
        lines.append('   sequences compared, by length (distinct state pairs reached, agree/disagree): ' +
                     ' '.join('n%d:%d/%d' % (L, v[0], v[1]) for L, v in sorted(c.per_len.items())))
        for seq, want, got in c.disagreements:
            lines.append('   DISAGREE %-45s  recognisers=%-16s reader=%-16s  %s' % (' '.join(seq), want, got, render(seq)))
        print('\n'.join(lines[-2 - len(c.disagreements):]))
        sys.stdout.flush()
    lines.append('total disagreements: %d' % total)
    print(lines[-1])
    with open(os.path.join(outdir, 'compare.txt'), 'w') as f:
        f.write('\n'.join(lines) + '\n')
    return total


if __name__ == '__main__':
    N = int(sys.argv[1]) if len(sys.argv) > 1 else 4
    outdir = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(os.path.abspath(__file__)), 'out')
    sys.exit(1 if main(N, outdir) else 0)
