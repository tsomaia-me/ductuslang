"""Recognisers for SymphonyScript `Type`, `Expr`, `AltType`, `Fallback`,
`TypeArgs` and `Dim`, built from the productions of SPEC.md alone.

The productions below are transcribed from SPEC.md sections 3.2, 3.8, 3.10,
3.11, 5.1, 5.5, 5.8, 5.10, 6.7, 6.8, 7.9 and 15.3.  Three changes apply the
prose that fixes a reading by tokens (quoted in REPORT.md):

  * 3.11: after `json`, a `[` that `..` does not follow opens JSON text, never
    a dimension.  So `Base` and `AltType` take `json` as
    `'json' ('[' '..' Size ']' Dims?)?` instead of `JsonType Dims?`.
    (`json[2]` is the document `[2]`; `json[..64][2]` is still a type.)
  * 5.1: `{` after `json` opens a document literal, after a name a record
    literal, otherwise a map literal.  The productions already say this
    (no juxtaposition exists in `Expr`), so no rule changes; the recogniser
    follows the productions.
  * 5.3 / 6.8: the brace-group and first-bracket-list sentences agree with
    the productions as written (`MapType` always carries `[..`, the first
    list after a name is `TypeArgs` or a `Dim`, later lists are `Dims`);
    no rule changes.

`Result` (used by §6.7's `Lambda`) is defined in §6.3, which was not read;
it is taken as `'->' Type` from §6.7's example `(x: i32) -> bool: x > 60`.

Token kinds: every keyword and punctuator the productions name, plus
Name, Number, String, JsonString, JsonNumber.
"""

import re
from collections import namedtuple

EBNF = r"""
Type        ::= FnType | Base ('|' Base)*
FnType      ::= '(' (FnParam (',' FnParam)*)? ')' '->' Type
FnParam     ::= (Name ':')? Type
Base        ::= (ScalarType | StrType | MapType | NameRef TypeArgs? | 'none' | '(' Type ')') Dims? '?'?
              | 'json' ('[' '..' Size ']' Dims?)? '?'?
Dims        ::= Dim+
Dim         ::= '[' Size ']' | '[' '..' Size ']'
Size        ::= Expr
ScalarType  ::= 'u8' | 'i32' | 'i64' | 'f32' | 'f64' | 'bool'
StrType     ::= 'str' ('[' '..' Size ']')?
MapType     ::= '{' Type ':' Type '}' '[' '..' Size ']'
TypeArgs    ::= '[' TypeArg (',' TypeArg)* ']'
TypeArg     ::= Type | Expr
NameRef     ::= Name | QualifiedName
QualifiedName ::= Name '::' Name

Expr        ::= IfExpr | ForExpr | OrExpr
IfExpr      ::= 'if' Expr 'then' Expr 'else' Expr
OrExpr      ::= AndExpr ('or' AndExpr)*
AndExpr     ::= NotExpr ('and' NotExpr)*
NotExpr     ::= 'not' NotExpr | CmpExpr
CmpExpr     ::= Fallback (CmpOp Fallback | 'is' 'not'? AltType)?
CmpOp       ::= 'is' 'not'? | '<' | '<=' | '>' | '>='
AltType     ::= 'none' | (ScalarType | StrType | MapType | NameRef TypeArgs?) Dims?
              | 'json' ('[' '..' Size ']' Dims?)?
Fallback    ::= AddExpr ('??' Fallback)?
AddExpr     ::= MulExpr (('+' | '-' | 'without') MulExpr)*
MulExpr     ::= UnaryExpr (('*' | '/' | 'div' | '%') UnaryExpr)*
UnaryExpr   ::= '-' UnaryExpr | PowExpr
PowExpr     ::= Postfix ('**' UnaryExpr)?
Postfix     ::= Primary ('.' Name | '[' Index ']' | '(' Args? ')')*
Primary     ::= Number | 'true' | 'false' | String | RecordLit | MapLit | JsonLit | 'none' | Name | QualifiedName
              | 'to' | Conversion | ArrayLit | '(' Expr ')'
ArrayLit    ::= '[' (Expr (',' Expr)* ','?)? ']'
Args        ::= Arg (',' Arg)* | NamedArg (',' NamedArg)*
Arg         ::= Expr | Lambda
NamedArg    ::= Name '=' Arg
Lambda      ::= Name ':' Expr | '(' LambdaParams? ')' Result? ':' Expr
Result      ::= '->' Type
LambdaParams ::= LambdaParam (',' LambdaParam)*
LambdaParam ::= Name (':' Type)?
RecordLit   ::= NameRef TypeArgs? '{' Fields '}'
Fields      ::= (Name ':' Expr (',' Name ':' Expr)* ','?)?
MapLit      ::= '{' (Entry (',' Entry)* ','?)? '}'
Entry       ::= Expr ':' Expr
JsonLit     ::= 'json' JsonValue
JsonValue   ::= JsonObject | JsonArray | JsonString | '-'? JsonNumber | 'true' | 'false' | 'null'
              | NameRef ('.' Name)* | 'to' ('.' Name)+ | '(' Expr ')'
JsonObject  ::= '{' (JsonString ':' JsonValue (',' JsonString ':' JsonValue)*)? '}'
JsonArray   ::= '[' (JsonValue (',' JsonValue)*)? ']'
Conversion  ::= ('u8' | 'i32' | 'i64' | 'f32' | 'f64') '(' Expr ')'
Index       ::= Bound | Bound? '..' Bound?
Bound       ::= '^'? Expr
ForExpr     ::= ForExprHead Expr
ForExprHead ::= LoopHead 'into' Name (':' Type)? 'from' Expr ':'
LoopHead    ::= 'for' Name (':' Type)? (',' Name (':' Type)?)? 'in' Iterable
Iterable    ::= Expr '..' Expr | Expr
"""

CLASS_TERMINALS = ('Name', 'Number', 'String', 'JsonString', 'JsonNumber')

# ---------------------------------------------------------------- EBNF parse

_TOK = re.compile(r"\s*(?:('(?:[^']|'')*')|(::=)|([A-Za-z_][A-Za-z_0-9]*)|([|?*+()]))")


def _lex_ebnf(text):
    out = []
    for line in text.splitlines():
        if not line.strip():
            continue
        pos = 0
        if not line.startswith(' '):
            out.append(('NL', None))
        while pos < len(line):
            m = _TOK.match(line, pos)
            if not m or m.end() == pos:
                if line[pos:].strip() == '':
                    break
                raise SyntaxError('bad ebnf at %r' % line[pos:])
            pos = m.end()
            lit, defs, ident, op = m.groups()
            if lit is not None:
                out.append(('LIT', lit[1:-1]))
            elif defs is not None:
                out.append(('DEF', None))
            elif ident is not None:
                out.append(('ID', ident))
            else:
                out.append(('OP', op))
    out.append(('NL', None))
    return out


class _P:
    def __init__(self, toks):
        self.t = toks
        self.i = 0

    def peek(self):
        return self.t[self.i] if self.i < len(self.t) else ('EOF', None)

    def take(self):
        tok = self.peek()
        self.i += 1
        return tok

    def rules(self):
        rules = {}
        order = []
        while self.peek()[0] != 'EOF':
            if self.peek()[0] == 'NL':
                self.take()
                continue
            kind, name = self.take()
            assert kind == 'ID', (kind, name)
            assert self.take()[0] == 'DEF'
            rhs = self.alt()
            rules[name] = rhs
            order.append(name)
        return rules, order

    def alt(self):
        alts = [self.seq()]
        while self.peek() == ('OP', '|'):
            self.take()
            alts.append(self.seq())
        return ('alt', alts) if len(alts) > 1 else alts[0]

    def seq(self):
        items = []
        while True:
            k, v = self.peek()
            if k in ('NL', 'EOF') or (k == 'OP' and v in ('|', ')')):
                break
            items.append(self.postfix())
        return ('seq', items) if len(items) != 1 else items[0]

    def postfix(self):
        node = self.atom()
        while self.peek()[0] == 'OP' and self.peek()[1] in ('?', '*', '+'):
            node = ({'?': 'opt', '*': 'star', '+': 'plus'}[self.take()[1]], node)
        return node

    def atom(self):
        k, v = self.take()
        if k == 'LIT':
            return ('t', v)
        if k == 'ID':
            return ('t', v) if v in CLASS_TERMINALS else ('n', v)
        if k == 'OP' and v == '(':
            node = self.alt()
            assert self.take() == ('OP', ')')
            return node
        raise SyntaxError((k, v))


# --------------------------------------------------------- NFA -> DFA per rule

class _NFA:
    def __init__(self):
        self.edges = []  # (src, label or None, dst)
        self.n = 0

    def new(self):
        self.n += 1
        return self.n - 1

    def build(self, node, start, end):
        kind = node[0]
        if kind == 't' or kind == 'n':
            self.edges.append((start, (kind, node[1]), end))
        elif kind == 'seq':
            cur = start
            for i, item in enumerate(node[1]):
                nxt = end if i == len(node[1]) - 1 else self.new()
                self.build(item, cur, nxt)
                cur = nxt
        elif kind == 'alt':
            for a in node[1]:
                self.build(a, start, end)
        elif kind == 'opt':
            self.build(node[1], start, end)
            self.edges.append((start, None, end))
        elif kind == 'star':
            mid = self.new()
            self.edges.append((start, None, mid))
            self.build(node[1], mid, mid)
            self.edges.append((mid, None, end))
        elif kind == 'plus':
            mid = self.new()
            self.build(node[1], start, mid)
            self.build(node[1], mid, mid)
            self.edges.append((mid, None, end))
        else:
            raise ValueError(kind)


def _rule_dfa(node):
    nfa = _NFA()
    s, e = nfa.new(), nfa.new()
    nfa.build(node, s, e)
    eps = {i: set() for i in range(nfa.n)}
    lab = {i: [] for i in range(nfa.n)}
    for a, l, b in nfa.edges:
        (eps[a].add(b) if l is None else lab[a].append((l, b)))

    def closure(states):
        st = set(states)
        work = list(states)
        while work:
            x = work.pop()
            for y in eps[x]:
                if y not in st:
                    st.add(y)
                    work.append(y)
        return frozenset(st)

    start = closure({s})
    ids = {start: 0}
    order = [start]
    trans = [dict()]
    final = [e in start]
    work = [start]
    while work:
        cur = work.pop()
        moves = {}
        for x in cur:
            for l, b in lab[x]:
                moves.setdefault(l, set()).add(b)
        for l, tgt in moves.items():
            c = closure(tgt)
            if c not in ids:
                ids[c] = len(order)
                order.append(c)
                trans.append(dict())
                final.append(e in c)
                work.append(c)
            trans[ids[cur]][l] = ids[c]
    return trans, final


class Grammar:
    def __init__(self, text=EBNF):
        rules, order = _P(_lex_ebnf(text)).rules()
        self.order = order
        self.dfa = {}    # nt -> (trans, final)
        for nt, node in rules.items():
            self.dfa[nt] = _rule_dfa(node)
        self.terminals = set()
        for nt, (trans, final) in self.dfa.items():
            for d in trans:
                for (k, v) in d:
                    if k == 't':
                        self.terminals.add(v)
                    elif v not in self.dfa:
                        raise KeyError('undefined nonterminal %s in %s' % (v, nt))

    def alphabet(self):
        return sorted(self.terminals)


# -------------------------------------------------- pushdown recogniser (NPDA)

class Recogniser:
    """Nondeterministic pushdown recogniser for one start symbol.

    A configuration is a tuple of (nonterminal, state) frames; the parent
    frames already hold their return states.  A state of the recogniser is a
    frozenset of configurations closed under push (predict) and pop
    (complete).  States are interned to ints; transitions are memoised.
    State 0 is DEAD (empty set)."""

    def __init__(self, grammar, start):
        self.g = grammar
        self.start = start
        self.sets = [frozenset()]
        self.ids = {frozenset(): 0}
        self.trans = [dict()]
        self.accept = [False]
        s0 = self._close({((start, 0),)})
        self.init = self._intern(s0)

    def _intern(self, s):
        if s in self.ids:
            return self.ids[s]
        i = len(self.sets)
        self.ids[s] = i
        self.sets.append(s)
        self.trans.append(dict())
        acc = any(len(c) == 1 and self.g.dfa[c[0][0]][1][c[0][1]] for c in s)
        self.accept.append(acc)
        return i

    def _close(self, configs):
        dfa = self.g.dfa
        seen = set(configs)
        work = list(configs)
        while work:
            c = work.pop()
            nt, st = c[-1]
            trans, final = dfa[nt]
            for (k, v), nxt in trans[st].items():
                if k == 'n':
                    c2 = c[:-1] + ((nt, nxt), (v, 0))
                    if c2 not in seen:
                        seen.add(c2)
                        work.append(c2)
            if final[st] and len(c) > 1:
                c2 = c[:-1]
                if c2 not in seen:
                    seen.add(c2)
                    work.append(c2)
        return frozenset(seen)

    def step(self, sid, tok):
        d = self.trans[sid]
        if tok in d:
            return d[tok]
        dfa = self.g.dfa
        out = set()
        key = ('t', tok)
        for c in self.sets[sid]:
            nt, st = c[-1]
            nxt = dfa[nt][0][st].get(key)
            if nxt is not None:
                out.add(c[:-1] + ((nt, nxt),))
        res = self._intern(self._close(out)) if out else 0
        d[tok] = res
        return res

    def run(self, toks):
        s = self.init
        for t in toks:
            s = self.step(s, t)
            if s == 0:
                return 0
        return s

    def accepts(self, toks):
        return self.accept[self.run(toks)]


# ------------------------------------------------- source text <-> token kinds

KEYWORDS = {'if', 'then', 'else', 'or', 'and', 'not', 'is', 'without', 'div',
            'none', 'to', 'true', 'false', 'null', 'json', 'for', 'in', 'into',
            'from', 'u8', 'i32', 'i64', 'f32', 'f64', 'bool', 'str'}
PUNCT = sorted(['|', '(', ')', '->', ',', ':', '[', ']', '..', '?', '{', '}',
                '::', '??', '+', '-', '*', '/', '%', '**', '<', '<=', '>', '>=',
                '.', '=', '^'], key=len, reverse=True)


def tokenize(src):
    """Tokenise source text into token kinds for tests.  Inside `json` text,
    strings and numbers are JsonString / JsonNumber.  The lexer tracks the
    extent of the JSON text by its brackets, re-entering the language inside
    a `( Expr )` of the document, as 3.11 describes; a `[` that `..` follows
    does not open JSON text (3.11)."""
    toks = []
    i = 0
    n = len(src)
    modes = []          # stack of ['json', depth] / ['lang', depth]
    json_pending = False

    def in_json():
        return bool(modes) and modes[-1][0] == 'json'

    while i < n:
        c = src[i]
        if c.isspace():
            i += 1
            continue
        if c == '"':
            j = i + 1
            depth = 0
            jm = in_json() or json_pending
            while j < n:
                if src[j] == '\\':
                    j += 2
                    continue
                if jm:
                    if src[j] == '"':
                        break
                else:
                    if src[j] == '{':
                        depth += 1
                    elif src[j] == '}' and depth:
                        depth -= 1
                    elif src[j] == '"' and depth == 0:
                        break
                j += 1
            toks.append('JsonString' if jm else 'String')
            json_pending = False
            i = j + 1
            continue
        if c.isdigit():
            j = i
            while j < n and (src[j].isalnum() or (src[j] in '._' and src[j:j + 2] != '..')):
                j += 1
            toks.append('JsonNumber' if (in_json() or json_pending) else 'Number')
            json_pending = False
            i = j
            continue
        if c.isalpha() or c in '_#':
            j = i
            while j < n and (src[j].isalnum() or src[j] in '_#'):
                j += 1
            w = src[i:j]
            if w == 'json':
                json_pending = True
                toks.append('json')
            else:
                toks.append(w if w in KEYWORDS else 'Name')
                json_pending = False
            i = j
            continue
        for p in PUNCT:
            if src.startswith(p, i):
                toks.append(p)
                if json_pending:
                    if p == '{' or (p == '[' and not src[i + 1:].lstrip().startswith('..')):
                        modes.append(['json', 1])
                    if p != '-':
                        json_pending = False
                elif in_json():
                    if p in '{[':
                        modes[-1][1] += 1
                    elif p in '}]':
                        modes[-1][1] -= 1
                        if modes[-1][1] == 0:
                            modes.pop()
                    elif p == '(':
                        modes.append(['lang', 1])
                elif modes:
                    if p in '([{':
                        modes[-1][1] += 1
                    elif p in ')]}':
                        modes[-1][1] -= 1
                        if modes[-1][1] == 0:
                            modes.pop()
                i += len(p)
                break
        else:
            raise SyntaxError('cannot tokenise %r' % src[i:])
    return toks


_RENDER_SPACE_BEFORE = {'is', 'not', 'and', 'or', 'without', 'div', 'then',
                        'else', 'in', 'into', 'from', '->', '+', '-', '*', '/',
                        '%', '**', '<', '<=', '>', '>=', '??', '|', '='}
_RENDER_SPACE_AFTER = _RENDER_SPACE_BEFORE | {'if', 'for', 'json', ',', ':',
                                              'not', 'to'}


def render(toks):
    """Render token kinds as plausible source text."""
    names = 'abcdefgh'
    out = []
    ni = 0
    num = 0
    prev = None
    for t in toks:
        if t == 'Name':
            s = names[ni % len(names)]
            ni += 1
        elif t == 'Number':
            num += 1
            s = str(num)
        elif t == 'String':
            s = '"s"'
        elif t == 'JsonString':
            s = '"k"'
        elif t == 'JsonNumber':
            num += 1
            s = str(num)
        else:
            s = t
        if out:
            space = (t in _RENDER_SPACE_BEFORE or prev in _RENDER_SPACE_AFTER
                     or (prev not in ('(', '[', '{', '.', '::', '^', '-', '..')
                         and t not in (')', ']', '}', ',', '.', '::', '?', '[', '(', '..')))
            if prev == '-' and t not in ('Number', 'JsonNumber', 'Name', '(', 'to', 'none', 'true', 'false'):
                space = True
            if t == '[' and prev in ('json', 'str', 'is', 'not', '|', ':', '->', ',', '..'):
                space = True
            if t == '(' and prev in ('is', 'not', '|', ':', '->', ',', 'json', '..', 'if', 'then', 'else', 'in', 'from', 'into', 'and', 'or'):
                space = True
            if space:
                out.append(' ')
        out.append(s)
        prev = t
    return ''.join(out)


if __name__ == '__main__':
    g = Grammar()
    print(len(g.alphabet()), 'token kinds:', ' '.join(g.alphabet()))
    for nt in g.order:
        print('%-14s %3d states' % (nt, len(g.dfa[nt][0])))
