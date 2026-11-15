"""Neutral sets of SPEC section 6.5's three places and of a type argument, from the productions.

Method: an Earley recogniser over BNF grammars hand-expanded from the SPEC's EBNF; bounded
generation of every sentence of the smaller grammar up to a length, each tested in the other
grammar; then targeted sequences for the proposal's prose claims.

Tokens are abstract kinds. For the children reading, a token written `~X` is adjacent to the
token before it (no whitespace), as the lexer sees it; the expression reading ignores that.
`E` stands for any one expression inside parentheses (a DefaultArg's `'(' Expr ')'`), kept
opaque so the enumeration stays small.
"""
import itertools, sys
from collections import defaultdict

# ---------------- Earley recogniser ----------------
def earley(grammar, start, toks):
    # grammar: dict nt -> list of alternatives (lists of symbols). Terminals are strings not in grammar.
    n = len(toks)
    chart = [set() for _ in range(n + 1)]
    # item: (nt, alt_index, dot, origin)
    def predict(i, nt):
        for ai, alt in enumerate(grammar[nt]):
            item = (nt, ai, 0, i)
            if item not in chart[i]:
                chart[i].add(item); agenda.append(item)
    for i in range(n + 1):
        agenda = list(chart[i])
        if i == 0:
            predict(0, start)
            agenda = list(chart[0])
        seen_nullable = {}
        while agenda:
            nt, ai, dot, origin = agenda.pop()
            alt = grammar[nt][ai]
            if dot < len(alt):
                sym = alt[dot]
                if sym in grammar:
                    predict(i, sym)
                    # nullable completion already in chart at i
                    for (nt2, ai2, dot2, o2) in list(chart[i]):
                        if nt2 == sym and o2 == i and dot2 == len(grammar[nt2][ai2]):
                            adv = (nt, ai, dot + 1, origin)
                            if adv not in chart[i]:
                                chart[i].add(adv); agenda.append(adv)
                else:
                    if i < n and toks[i] == sym:
                        adv = (nt, ai, dot + 1, origin)
                        if adv not in chart[i + 1]:
                            chart[i + 1].add(adv)
            else:
                # complete
                for (nt2, ai2, dot2, o2) in list(chart[origin]):
                    alt2 = grammar[nt2][ai2]
                    if dot2 < len(alt2) and alt2[dot2] == nt:
                        adv = (nt2, ai2, dot2 + 1, o2)
                        if adv not in chart[i]:
                            chart[i].add(adv); agenda.append(adv)
    return any(nt == start and dot == len(grammar[nt][ai]) and o == 0 for (nt, ai, dot, o) in chart[n])

# ---------------- bounded sentence generation ----------------
def min_lengths(grammar):
    ml = {nt: 10**9 for nt in grammar}
    changed = True
    while changed:
        changed = False
        for nt, alts in grammar.items():
            for alt in alts:
                l = 0
                for s in alt:
                    l += ml[s] if s in grammar else 1
                if l < ml[nt]:
                    ml[nt] = l; changed = True
    return ml

def sentences(grammar, start, maxlen):
    ml = min_lengths(grammar)
    out = set()
    stack = [((start,), ())]
    while stack:
        form, done = stack.pop()
        if not form:
            out.add(done); continue
        sym, rest = form[0], form[1:]
        if sym in grammar:
            for alt in grammar[sym]:
                new = tuple(alt) + rest
                need = len(done) + sum(ml[s] if s in grammar else 1 for s in new)
                if need <= maxlen:
                    stack.append((new, done))
        else:
            if len(done) + 1 + sum(ml[s] if s in grammar else 1 for s in rest) <= maxlen:
                stack.append((rest, done + (sym,)))
    return out

def strip_adj(seq):
    return tuple(t[1:] if t.startswith('~') else t for t in seq)

# ---------------- the expression grammar (SPEC 5.1, 5.5, 3.8, 3.10, 3.11, 6.7, 6.8, 3.2) ----------------
# Terminals: N Q NUM STR TRUE TO NONE MINUS PLUS STAR SLASH LP RP LB RB LBR RBR COMMA COLON DOTDOT CARET DOT
#            IS NOT IF THEN ELSE FLAG I32 STRT JSON QMARK ARROW BAR E
EXPR = {
 'Expr':     [['IfExpr'], ['OrExpr']],                       # ForExpr left out: a destination's for stands in parentheses (5.10 prose, ruled)
 'IfExpr':   [['IF', 'Expr', 'THEN', 'Expr', 'ELSE', 'Expr']],
 'OrExpr':   [['AndExpr'], ['OrExpr', 'OR', 'AndExpr']],
 'AndExpr':  [['NotExpr'], ['AndExpr', 'AND', 'NotExpr']],
 'NotExpr':  [['NOT', 'NotExpr'], ['CmpExpr']],
 'CmpExpr':  [['Fallback'], ['Fallback', 'CmpOp', 'Fallback'], ['Fallback', 'IS', 'AltType'], ['Fallback', 'IS', 'NOT', 'AltType']],
 'CmpOp':    [['IS'], ['IS', 'NOT'], ['LT']],
 'Fallback': [['AddExpr'], ['AddExpr', 'QQ', 'Fallback']],
 'AddExpr':  [['MulExpr'], ['AddExpr', 'PLUS', 'MulExpr'], ['AddExpr', 'MINUS', 'MulExpr']],
 'MulExpr':  [['UnaryExpr'], ['MulExpr', 'STAR', 'UnaryExpr'], ['MulExpr', 'SLASH', 'UnaryExpr']],
 'UnaryExpr':[['MINUS', 'UnaryExpr'], ['PowExpr']],
 'PowExpr':  [['Postfix'], ['Postfix', 'POW', 'UnaryExpr']],
 'Postfix':  [['Primary'], ['Postfix', 'DOT', 'N'], ['Postfix', 'LB', 'Index', 'RB'], ['Postfix', 'LP', 'RP'], ['Postfix', 'LP', 'Args', 'RP']],
 'Primary':  [['NUM'], ['TRUE'], ['STR'], ['RecordLit'], ['MapLit'], ['JsonLit'], ['NONE'], ['N'], ['Q'], ['TO'], ['Conversion'], ['ArrayLit'], ['LP', 'Expr', 'RP'], ['LP', 'E', 'RP']],
 'ArrayLit': [['LB', 'RB'], ['LB', 'ExprList', 'RB'], ['LB', 'ExprList', 'COMMA', 'RB']],
 'ExprList': [['Expr'], ['ExprList', 'COMMA', 'Expr']],
 'Args':     [['ArgList'], ['NamedList']],
 'ArgList':  [['Arg'], ['ArgList', 'COMMA', 'Arg']],
 'Arg':      [['Expr'], ['Lambda']],
 'NamedList':[['NamedArg'], ['NamedList', 'COMMA', 'NamedArg']],
 'NamedArg': [['N', 'EQ', 'Arg']],
 'Lambda':   [['N', 'COLON', 'Expr'], ['LP', 'RP', 'COLON', 'Expr'], ['LP', 'LParams', 'RP', 'COLON', 'Expr'],
              ['LP', 'RP', 'ARROW', 'Type', 'COLON', 'Expr'], ['LP', 'LParams', 'RP', 'ARROW', 'Type', 'COLON', 'Expr']],
 'LParams':  [['LParam'], ['LParams', 'COMMA', 'LParam']],
 'LParam':   [['N'], ['N', 'COLON', 'Type']],
 'Index':    [['Bound'], ['DOTDOT'], ['Bound', 'DOTDOT'], ['DOTDOT', 'Bound'], ['Bound', 'DOTDOT', 'Bound']],
 'Bound':    [['Expr'], ['CARET', 'Expr']],
 'Conversion':[['I32', 'LP', 'Expr', 'RP']],
 'RecordLit':[['N', 'LBR', 'Fields', 'RBR'], ['Q', 'LBR', 'Fields', 'RBR'], ['N', 'TypeArgs', 'LBR', 'Fields', 'RBR'], ['Q', 'TypeArgs', 'LBR', 'Fields', 'RBR']],
 'Fields':   [[], ['FieldList'], ['FieldList', 'COMMA']],
 'FieldList':[['N', 'COLON', 'Expr'], ['FieldList', 'COMMA', 'N', 'COLON', 'Expr']],
 'MapLit':   [['LBR', 'RBR'], ['LBR', 'EntryList', 'RBR'], ['LBR', 'EntryList', 'COMMA', 'RBR']],
 'EntryList':[['Entry'], ['EntryList', 'COMMA', 'Entry']],
 'Entry':    [['Expr', 'COLON', 'Expr']],
 'JsonLit':  [['JSON', 'JsonValue']],
 'JsonValue':[['LBR', 'RBR'], ['LB', 'RB'], ['LB', 'JsonValue', 'RB'], ['STR'], ['NUM'], ['MINUS', 'NUM'], ['TRUE'], ['NONE_J'], ['N'], ['N', 'DOT', 'N'], ['Q'], ['TO', 'DOT', 'N'], ['LP', 'Expr', 'RP']],
 # types (3.2, 6.8) as the expression grammar needs them
 'Type':     [['FnType'], ['Sum']],
 'Sum':      [['Base'], ['Sum', 'BAR', 'Base']],
 'FnType':   [['LP', 'RP', 'ARROW', 'Type'], ['LP', 'FnParams', 'RP', 'ARROW', 'Type']],
 'FnParams': [['FnParam'], ['FnParams', 'COMMA', 'FnParam']],
 'FnParam':  [['Type'], ['N', 'COLON', 'Type']],
 'Base':     [['Base0'], ['Base0', 'Dims'], ['Base0', 'QMARK'], ['Base0', 'Dims', 'QMARK']],
 'Base0':    [['I32'], ['StrType'], ['MapType'], ['JsonType'], ['N'], ['Q'], ['N', 'TypeArgs'], ['Q', 'TypeArgs'], ['NONE'], ['LP', 'Type', 'RP']],
 'Dims':     [['Dim'], ['Dims', 'Dim']],
 'Dim':      [['LB', 'Expr', 'RB'], ['LB', 'DOTDOT', 'Expr', 'RB']],
 'StrType':  [['STRT'], ['STRT', 'LB', 'DOTDOT', 'Expr', 'RB']],
 'MapType':  [['LBR', 'Type', 'COLON', 'Type', 'RBR', 'LB', 'DOTDOT', 'Expr', 'RB']],
 'JsonType': [['JSON'], ['JSON', 'LB', 'DOTDOT', 'Expr', 'RB']],
 'TypeArgs': [['LB', 'TypeArgList', 'RB']],
 'TypeArgList':[['TypeArg'], ['TypeArgList', 'COMMA', 'TypeArg']],
 'TypeArg':  [['Type'], ['Expr']],
 'AltType':  [['NONE'], ['AltBase'], ['AltBase', 'Dims']],
 'AltBase':  [['I32'], ['StrType'], ['MapType'], ['JsonType'], ['N'], ['Q'], ['N', 'TypeArgs'], ['Q', 'TypeArgs']],
}

# ---------------- the children reading (7.1 Body's InlineChildren, 7.7, 7.8, 7.5, 7.4, 9.1, 7.6) ----------------
# Adjacency: a flag is `~FLAG`/`~STAR`/`~PLUS` (directly after the kind); every member after the first starts spaced
# (ruled: placements with no whitespace between them are refused). `/` is free (7.1). Inside a bundle the same.
CHILD = {
 'InlineChildren': [['First'], ['First', 'Rest']],
 'Rest':     [['SelfDelimitedSp'], ['Rest', 'SelfDelimitedSp']],
 'First':    [['Member'], ['Bundle']],
 'SelfDelimitedSp': [['Member'], ['Bundle']],   # first token spaced: all our member-start terminals are spaced forms
 'Member':   [['Kind'], ['Kind', 'Flags'], ['Kind', 'AsName'], ['Kind', 'Flags', 'AsName'],
              ['Kind', 'DefaultArg'], ['Kind', 'Flags', 'DefaultArg'], ['Kind', 'AsName', 'DefaultArg'], ['Kind', 'Flags', 'AsName', 'DefaultArg'],
              ['LP', 'Placement', 'RP']],
 'Kind':     [['N'], ['Q']],
 'Flags':    [['Flag'], ['Flags', 'Flag']],
 'Flag':     [['~FLAG'], ['~STAR'], ['~PLUS']],
 'AsName':   [['AS', 'N']],
 'DefaultArg':[['SLASH', 'Atom'], ['SLASH', 'LP', 'E', 'RP']],
 'Atom':     [['NUM'], ['MINUS', 'NUM'], ['TRUE'], ['STR'], ['Path'], ['TO', 'DotNames']],
 'Path':     [['N'], ['Q'], ['Path', 'DOT', 'N']],
 'DotNames': [['DOT', 'N'], ['DotNames', 'DOT', 'N']],
 'Placement':[['PHead'], ['PHead', 'Gate'], ['PHead', 'Attrs'], ['PHead', 'Gate', 'Attrs']],
 'PHead':    [['Member0']],
 'Member0':  [['Kind'], ['Kind', 'Flags'], ['Kind', 'AsName'], ['Kind', 'Flags', 'AsName'],
              ['Kind', 'DefaultArg'], ['Kind', 'Flags', 'DefaultArg'], ['Kind', 'AsName', 'DefaultArg'], ['Kind', 'Flags', 'AsName', 'DefaultArg']],
 'Gate':     [['WHEN', 'E']],
 'Attrs':    [['BAR', 'N', 'EQ', 'E'], ['BAR', 'N']],
 'Bundle':   [['LB', 'Members', 'RB'], ['LB', 'Members', 'BundleFor', 'RB'], ['LB', 'BundleFor', 'RB']],
 'Members':  [['Member'], ['Members', 'Member']],
 'BundleFor':[['FOR', 'N', 'IN', 'E', 'COLON', 'Members']],
}

def fits_children(seq):
    return earley(CHILD, 'InlineChildren', list(seq))

def fits_expr(seq):
    return earley(EXPR, 'Expr', list(strip_adj(seq)))

def show(seq):
    return ' '.join(seq)

# ---- the proposal's stated both-fit set for place 1 (section (2), Place 1) ----
def proposal_place1_both(seq):
    s = strip_adj(seq)
    if len(s) == 1 and s[0] in ('N', 'Q'):
        return True
    def kind_atom(t):
        # Kind/Atom with atom a number, a name, a path or a parenthesised expression; `Lamp/-1` among them
        if len(t) >= 3 and t[0] in ('N', 'Q') and t[1] == 'SLASH':
            a = t[2:]
            if a == ('NUM',) or a == ('MINUS', 'NUM'): return True
            if a == ('LP', 'E', 'RP'): return True
            if a[0] in ('N', 'Q') and all(a[i] == 'DOT' if i % 2 == 1 else a[i] == 'N' for i in range(1, len(a))): return True
        return False
    if kind_atom(s): return True
    if s == ('LP', 'N', 'RP'): return True
    if len(s) >= 3 and s[0] == 'LB' and s[-1] == 'RB':
        inner = s[1:-1]
        if inner in (('N',), ('Q',)) or kind_atom(inner): return True
    return False

def main():
    L = int(sys.argv[1]) if len(sys.argv) > 1 else 5
    print(f'=== Place 1: body-line tails of up to {L} tokens that fit the children reading ===')
    C = sentences(CHILD, 'InlineChildren', L)
    print('children sentences:', len(C))
    both = sorted((s for s in C if fits_expr(s)), key=lambda s: (len(s), s))
    print('fit both readings:', len(both))
    missed = [s for s in both if not proposal_place1_both(s)]
    print('of these, NOT in the proposal\'s stated both-fit set:', len(missed))
    for s in missed:
        print('   ', show(s))
    print('claimed both by the proposal but fit one reading only:')
    for s in both:
        pass
    # test the proposal's claimed-both forms explicitly
    claims = [('N',), ('Q',), ('N','SLASH','NUM'), ('N','SLASH','MINUS','NUM'), ('N','SLASH','N'), ('N','SLASH','N','DOT','N'),
              ('N','SLASH','LP','E','RP'), ('LP','N','RP'), ('LB','N','RB'), ('LB','N','SLASH','NUM','RB'), ('Q','SLASH','Q')]
    for c in claims:
        ch, ex = fits_children(c), fits_expr(c)
        if not (ch and ex):
            print('   ', show(c), 'children' if ch else '', 'expr' if ex else '')
    print('   (none printed above means every claimed form fits both)')
    print()
    print('=== the proposal\'s children-only claims, checked ===')
    for c, why in [(('N','N'), 'two members'), (('N','~STAR','N'), 'a flag `*` then a member'), (('N','~PLUS','N'), 'a flag `+` then a member'),
                   (('N','~FLAG','N'), 'a non-arithmetic flag then a member'), (('N','LP','N','RP'), 'two members, the second parenthesised'),
                   (('N','LB','N','RB'), 'a member then a bundle'), (('LB','N','RB','LB','N','RB'), 'two bundles'),
                   (('N','SLASH','NUM','LP','N','RP'), 'Kind/Atom then a parenthesised member'), (('LB','N','LP','N','RP','RB'), 'a bundle of two members'),
                   (('N','AS','N'), 'an as name'), (('LP','N','WHEN','E','RP'), 'a gate'), (('LP','N','BAR','N','EQ','E','RP'), 'an attribute clause'),
                   (('LB','N','SLASH','NUM','N','SLASH','NUM','RB'), 'a bundle of two'), (('N','SLASH','STR'), 'Kind/"str"'), (('N','SLASH','TRUE'), 'Kind/true'),
                   (('N','SLASH','TO','DOT','N'), 'Kind/to.name'), (('LP','N','SLASH','NUM','RP'), '(Kind/Atom)'), (('LB','LP','N','RP','RB'), '[(Name)]'),
                   (('N','~STAR','SLASH','NUM'), 'Kind*/Atom'), (('N','SLASH','NUM','N','SLASH','NUM'), 'two Kind/Atom members')]:
        ch, ex = fits_children(c), fits_expr(c)
        print(f'    {show(c):40s} children={ch!s:5s} expr={ex!s:5s}  ({why})')
    print()
    print('=== the proposal\'s destination-only claims, checked ===')
    for c, why in [(('LB','N','COMMA','N','RB','LB','N','RB'), '`[a, b][i]`'), (('N','SLASH','NUM','SLASH','NUM'), 'a `/` whose right operand is no atom? here two slashes'),
                   (('N','LB','NUM','RB'), 'an index after the first member (adjacent not modelled: spaced here)'), (('N','~LB','NUM','RB'), 'index, adjacent'),
                   (('N','LP','N','RP'), 'a call after the first member (spaced)'), (('N','~LP','N','RP'), 'a call, adjacent'),
                   (('NUM',), 'a literal'), (('N','PLUS','N'), 'an operator'), (('IF','E','THEN','N','ELSE','N'), 'if'), (('TO',), 'to')]:
        ch, ex = fits_children(c), fits_expr(c)
        print(f'    {show(c):40s} children={ch!s:5s} expr={ex!s:5s}  ({why})')

    # ---------------- Place 3: the right operand of `is` ----------------
    print()
    print('=== Place 3: right operands of `is` ===')
    def fits_alt(seq): return earley(EXPR, 'AltType', list(seq))
    def fits_val(seq): return earley(EXPR, 'Fallback', list(seq))
    A = sentences(EXPR, 'AltType', 4)
    print('AltType sentences up to 4 tokens:', len(A))
    both3 = sorted((s for s in A if fits_val(s)), key=lambda s: (len(s), s))
    print('that also fit a value (Fallback):', len(both3))
    for s in both3: print('   ', show(s))
    print('targeted:')
    for c, why in [(('I32',), 'scalar word'), (('I32','LP','N','RP'), 'scalar word then ( ): a conversion'), (('N','LB','N','COMMA','N','RB'), 'comma list'),
                   (('N','LB','N','COMMA','N','RB','LBR','RBR'), 'comma list then {}: a record literal'), (('N','LBR','RBR'), 'Name {}'),
                   (('N','LB','NUM','RB','LBR','RBR'), 'Name[4] {}'), (('NONE',), 'none'), (('LBR','N','COLON','N','RBR','LB','DOTDOT','NUM','RB'), '{a: b}[..4]'),
                   (('LBR','NUM','COLON','NUM','RBR','LB','DOTDOT','NUM','RB'), '{1: 2}[..4]'), (('STRT',), 'str'), (('STRT','LB','DOTDOT','NUM','RB'), 'str[..4]'),
                   (('N','LB','DOTDOT','NUM','RB'), 'Name[..4]'), (('N','LB','NUM','RB','LB','NUM','RB'), 'Name[4][2]'), (('N','QMARK'), 'Name?'),
                   (('JSON',), 'json'), (('JSON','LB','NUM','RB'), 'json[2]'), (('N','LB','I32','RB'), 'Name[i32]'), (('N','LB','N','QMARK','RB'), 'Name[T?]'),
                   (('N','LB','LP','N','RP','RB'), 'Name[(x)]'), (('N','LB','CARET','NUM','RB'), 'Name[^1]'), (('N','DOT','N'), 'a.b'), (('N','LB','NUM','RB','DOT','N'), 'a[0].b')]:
        print(f'    {show(c):40s} type={fits_alt(c)!s:5s} value={fits_val(c)!s:5s}  ({why})')

    # ---------------- Place 2: the first bracket list after a name in a type ----------------
    print()
    print('=== Place 2: contents of the first bracket list after a name in a type ===')
    def fits_targs(inner): return earley(EXPR, 'TypeArgList', list(inner))
    def fits_dim(inner): return earley(EXPR, 'Expr', list(inner)) or (len(inner) > 0 and inner[0] == 'DOTDOT' and earley(EXPR, 'Expr', list(inner[1:])))
    T = sentences(EXPR, 'TypeArgList', 3)
    print('TypeArgList contents up to 3 tokens:', len(T))
    neutral2 = sorted((s for s in T if fits_dim(s)), key=lambda s: (len(s), s))
    print('that also fit a Size (Dims):', len(neutral2))
    for s in neutral2: print('   ', show(s))
    # the proposal's rule: neutral iff exactly one item and it fits an expression; check equivalence
    def one_item_expr(inner): return earley(EXPR, 'Expr', list(inner))
    viol = [s for s in T if fits_dim(s) != one_item_expr(s)]
    print('TypeArgList contents where "fits Dims" differs from "one item fitting an expression":', [show(s) for s in viol])
    for c, why in [(('DOTDOT','NUM'), '..4'), (('N','COMMA','N'), 'a, b'), (('I32',), 'i32'), (('N',), 'x'), (('N','LB','NUM','RB'), 'Pair[2]'), (('NONE',), 'none'),
                   (('LBR','RBR'), '{}'), (('JSON','LB','NUM','RB'), 'json[2]'), (('N','COLON','I32'), 'x: i32')]:
        print(f'    [{show(c):30s}] TypeArgs={fits_targs(c)!s:5s} Dims={fits_dim(c)!s:5s}  ({why})')

    # ---------------- a type argument ----------------
    print()
    print('=== A type argument: sequences up to 4 tokens that fit both Type and Expr ===')
    def fits_type(s): return earley(EXPR, 'Type', list(s))
    def fits_e(s): return earley(EXPR, 'Expr', list(s))
    TY = sentences(EXPR, 'Type', 4)
    print('Type sentences up to 4 tokens:', len(TY))
    nt = sorted((s for s in TY if fits_e(s)), key=lambda s: (len(s), s))
    print('that also fit an expression:', len(nt))
    for s in nt: print('   ', show(s))
    for c, why in [(('LP','N','RP','LB','NUM','RB'), '(Name)[4]'), (('LBR','N','COLON','N','RBR','LB','DOTDOT','NUM','RB'), '{K: V}[..N]'),
                   (('LBR','N','COLON','N','RBR','LB','DOTDOT','N','RB','LB','NUM','RB'), '{K: V}[..N][2]'), (('NONE',), 'none'), (('LP','LP','N','RP','RP'), '((Name))'),
                   (('N','QMARK'), 'T?'), (('LP','N','RP','QMARK'), '(x)?')]:
        print(f'    {show(c):50s} type={fits_type(c)!s:5s} expr={fits_e(c)!s:5s}  ({why})')

main()
