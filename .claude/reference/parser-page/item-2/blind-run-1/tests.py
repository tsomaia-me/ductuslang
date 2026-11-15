"""Sanity tests of the recognisers against examples taken from the spec text
(positive and negative), plus hand cases for the prose rules."""
import sys
import time, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from grammar import Grammar, Recogniser, tokenize, render

g = Grammar()
R = {s: Recogniser(g, s) for s in ('Type', 'Expr', 'AltType', 'Fallback', 'TypeArgs', 'Dim')}

# (source, expected set of start symbols that accept it)
CASES = [
    # §3.2 types
    ('i32[3][2]', {'Type', 'AltType'}),
    ('str[..32][4]', {'Type', 'AltType'}),
    ('(i32?)[..4]', {'Type'}),
    ('i32[..4]?', {'Type'}),
    ('T[..N + 1]', {'Type', 'AltType', 'Expr', 'Fallback'}),
    ('T[N * 2]', {'Type', 'AltType', 'Expr', 'Fallback'}),
    ('i32[3][..10]', {'Type', 'AltType'}),
    ('(i32, i32) -> bool', {'Type'}),
    ('(x: f64) -> f64', {'Type'}),
    ('() -> i32', {'Type'}),
    ('Circle | Square', {'Type'}),
    ('none', {'Type', 'AltType', 'Expr', 'Fallback'}),
    ('{str[..32]: str[..256]}[..32]', {'Type', 'AltType'}),
    ('{str[..64]: i32}[..64]', {'Type', 'AltType'}),
    ('json[..65536]', {'Type', 'AltType'}),
    ('json', {'Type', 'AltType'}),
    ('json[..256]?', {'Type'}),
    ('Stack[i32, 8][4]', {'Type', 'AltType'}),
    ('Band[4]', {'Type', 'AltType', 'Expr', 'Fallback'}),
    ('Stack[T, N]', {'Type', 'AltType'}),
    ('gradients::warm', {'Type', 'AltType', 'Expr', 'Fallback'}),
    ('math::abs', {'Type', 'AltType', 'Expr', 'Fallback'}),
    # §3.11 prose: json[2] is a document, (json)[2] a type with a dimension
    ('json[2]', {'Expr', 'Fallback'}),
    ('(json)[2]', {'Type'}),
    ('json[..64][2]', {'Type', 'AltType'}),
    ('json {"a": 1}', {'Expr', 'Fallback'}),
    ('json [1, "b"]', {'Expr', 'Fallback'}),
    ('json "text"', {'Expr', 'Fallback'}),
    ('json 12.5', {'Expr', 'Fallback'}),
    ('json true', {'Expr', 'Fallback'}),
    ('json null', {'Expr', 'Fallback'}),
    ('json {"count": count, "msg": ("hi {name}"), "echo": items}', {'Expr', 'Fallback'}),
    ('json {"ok": true, "count": count, "echo": items, "order": order}', {'Expr', 'Fallback'}),
    ('json {}', {'Expr', 'Fallback'}),
    ('json -3', {'Expr', 'Fallback'}),
    ('json [..3]', {'Type', 'AltType'}),
    # §3.8, §6.8 record literals
    ('Band {low: 0.2, high: 0.8}', {'Expr', 'Fallback'}),
    ('Band {}', {'Expr', 'Fallback'}),
    ('Stack[i32, 8] {}', {'Expr', 'Fallback'}),
    ('Stack[i32, 4 * 2] {}', {'Expr', 'Fallback'}),
    ('Pair {left: 1i32, right: true}', {'Expr', 'Fallback'}),
    ('Stack[T, N] {items: (s.items + [x])[..N], top: s.top + 1}', {'Expr', 'Fallback'}),
    # §3.10 maps
    ('{}', {'Expr', 'Fallback'}),
    ('headers["host"] ?? "localhost"', {'Expr', 'Fallback'}),
    ('headers without ["cookie", "set-cookie"]', {'Expr', 'Fallback'}),
    ('headers + {"host": host}', {'Expr', 'Fallback'}),
    ('(hits.past(1) + {req.path: (hits.past(1)[req.path] ?? 0) + 1})[^64..]', {'Expr', 'Fallback'}),
    ('(m + n)[^64..]', {'Expr', 'Fallback'}),
    # §5.1
    ('-2 ** 2', {'Expr', 'Fallback'}),
    ('n-1', {'Expr', 'Fallback'}),
    ('port ?? 80 > 1024', {'Expr'}),
    ('x ?? base + 1', {'Expr', 'Fallback'}),
    ('x ?? -1', {'Expr', 'Fallback'}),
    ('-x ?? 1', {'Expr', 'Fallback'}),       # type error, not a syntax fault
    ('(x ?? 0) + 1', {'Expr', 'Fallback'}),
    ('base + (if high then 12 else 0)', {'Expr', 'Fallback'}),
    ('if high then 12 else 0', {'Expr'}),
    ('a < b < c', set()),
    ('a < b and c < d', {'Expr'}),
    ('not a < b', {'Expr'}),
    ('a + not b', set()),
    ('a is not b', {'Expr'}),
    ('a is not not b', set()),
    ('x is none', {'Expr'}),
    ('x is Circle', {'Expr'}),
    ('x is i32', {'Expr'}),
    ('x is Band[4]', {'Expr'}),
    ('x is levels[0]', {'Expr'}),
    ('x is i32 + 1', set()),
    ('x is Name + 1', {'Expr'}),
    ('x is str[..4] and y', {'Expr'}),
    ('x is {a: b}[..4]', {'Expr'}),
    ('x is {a: b}[0]', {'Expr'}),
    ('x is json {"a": 1}', {'Expr'}),
    ('x is json[..4]', {'Expr'}),
    ('v is "bob"', {'Expr'}),
    ('v is [1, 2]', {'Expr'}),
    ('items.kind is Array', {'Expr'}),
    ('body["customer"]["name"] is none', {'Expr'}),
    ('flag ?? true', {'Expr', 'Fallback'}),
    # §5.5
    ('xs[^1]', {'Expr', 'Fallback'}),
    ('xs[^2..]', {'Expr', 'Fallback'}),
    ('xs[1..^1]', {'Expr', 'Fallback'}),
    ('xs[..i] + xs[i + 1..]', {'Expr', 'Fallback'}),
    ('xs[..]', {'Expr', 'Fallback'}),
    ('"a日b"[1..]', {'Expr', 'Fallback'}),
    ('[a, b, a + b]', {'Expr', 'Fallback', 'TypeArgs'}),
    ('[]', {'Expr', 'Fallback'}),
    ('[1, 2] + [3]', {'Expr', 'Fallback'}),
    ('played[step % played.size]', {'Expr', 'Fallback', 'Type', 'AltType'}),
    ('if flag then [60, 64] else [62, 65]', {'Expr'}),
    # §5.8
    ('u8(x)', {'Expr', 'Fallback'}),
    ('f64(a) / 2', {'Expr', 'Fallback'}),
    ('i32', {'Type', 'AltType'}),
    ('i32 x', set()),
    # §5.10
    ('for x in readings into kept from []: if keep then kept + [x] else kept', {'Expr'}),
    ('1 + (for i, x in readings into sum from 0: sum + i * x)', {'Expr', 'Fallback'}),
    ('for x in 0..n into s from 0: s + x', {'Expr'}),
    ('for k, v: i32 in m into s: i32 from 0: s + v', {'Expr'}),
    ('for x in xs into total from 0.0: total + x', {'Expr'}),
    # §6.7 lambdas as arguments
    ('count(readings, p: p > 60)', {'Expr', 'Fallback'}),
    ('count(readings, (p: i32) -> bool: p < 48)', {'Expr', 'Fallback'}),
    ('count(readings, is_high)', {'Expr', 'Fallback'}),
    ('bar(1.0, inc)', {'Expr', 'Fallback'}),
    ('f(x=seed) * 2', {'Expr', 'Fallback'}),
    ('apply((a, b): a + b, 60)', {'Expr', 'Fallback'}),
    ('x: x * 2', set()),                 # a lambda is no expression on its own
    ('f(x: x * 2)', {'Expr', 'Fallback'}),
    ('f(g = x: x + 1)', {'Expr', 'Fallback'}),
    ('f(a, b = 1)', set()),             # positional and named do not mix
    ('f(() : 1)', {'Expr', 'Fallback'}),
    # §6.8
    ('push(start, total)', {'Expr', 'Fallback'}),
    ('(a + b) / 2', {'Expr', 'Fallback'}),
    ('xs[..N div 2]', {'Expr', 'Fallback', 'Type', 'AltType'}),
    ('shorter([60]).size', {'Expr', 'Fallback'}),
    # §5.7-ish reads
    ('hits.past(1)[req.path] ?? 0', {'Expr', 'Fallback'}),
    ('to', {'Expr', 'Fallback'}),
    ('to.lamp', {'Expr', 'Fallback'}),
    ('json to.lamp', {'Expr', 'Fallback'}),
    ('json to', set()),
    ('json a::b.c', {'Expr', 'Fallback'}),
    ('a::b::c', set()),
    # TypeArgs / Dim position
    ('[i32, 8]', {'TypeArgs'}),
    ('[4]', {'TypeArgs', 'Dim', 'Expr', 'Fallback'}),
    ('[..4]', {'Dim'}),
    ('[i32]', {'TypeArgs'}),
    ('[N + 1]', {'TypeArgs', 'Dim', 'Expr', 'Fallback'}),
    ('[^1]', set()),
    ('[a..b]', set()),
    ('[]', {'Expr', 'Fallback'}),
    ('json {"a": (x + 1), "b": [1, 2]}', {'Expr', 'Fallback'}),
    ('a.b', {'Expr', 'Fallback'}),
    ('a.b.c(1)[2]', {'Expr', 'Fallback'}),
]


def main():
    bad = 0
    for src, expect in CASES:
        toks = tokenize(src)
        got = {s for s, r in R.items() if r.accepts(toks)}
        # TypeArgs / Dim only apply to bracket lists; Expr `[]` etc. are listed explicitly
        if got != expect:
            bad += 1
            print('MISMATCH %-60s tokens=%s\n   expected %s\n   got      %s' % (src, ' '.join(toks), sorted(expect), sorted(got)))
    print('%d cases, %d mismatches' % (len(CASES), bad))
    # round trip of render on a few
    for src in ('Stack[i32, 8] {}', 'x is {a: b}[..4]', 'json {"a": 1}', '(x: f64) -> f64', 'xs[^2..]'):
        print('  %-28s -> %-40s -> %s' % (src, ' '.join(tokenize(src)), render(tokenize(src))))
    return bad


if __name__ == '__main__':
    sys.exit(1 if main() else 0)
