"""Step 3: the single-pass reader.

One left-to-right pass, no backtracking, over a token sequence; answers
type-only, expression-only, both or neither for one of four positions:

  free      a free sequence: Type against Expr
  typearg   a type argument: Type against Expr (the same two languages)
  is        the right operand after `is` / `is not`: AltType against Fallback
  bracket   the first bracket list after a name in a type: TypeArgs against Dim

The reader is a deterministic stack machine.  Its state is a stack of
frames; each frame is an immutable record (so a state is hashable and the
exhaustive comparison can memoise on it).  What it remembers is exactly the
fields of the frames below; REPORT.md lists them in prose.

Each token is handed to the top frame.  A frame either consumes it, or
"closes" (pops itself, hands its result to the frame below, and the same
token is handed to that frame).  A token is never re-read from the input:
no backtracking.  The END marker closes every frame; the root's result is the
answer.
"""
from collections import namedtuple

END = '<END>'

SCALAR_KW = ('u8', 'i32', 'i64', 'f32', 'f64')
ARITH = ('+', '-', 'without', '*', '/', 'div', '%', '**', '??')
CMP = ('<', '<=', '>', '>=')
JSON_VALUE_START = ('{', '[', 'JsonString', 'JsonNumber', 'true', 'false', 'null', '-', 'Name', 'to', '(')

# ---------------------------------------------------------------- frames
# Item: one Type-and/or-Expr item (an operand sequence or a type).
#   T, E      the type reading / the expression reading is still viable
#   tsub      'full' = Type, 'alt' = AltType (no FnType, no `(T)`, no `|`, no `?`, `none` takes no Dims)
#   esub      'full' = Expr, 'fb' = Fallback (no if/for/not, no and/or/comparison/is)
#   tstart    what type form may start at pos 'start': 'type' (FnType or Base), 'base' (after `|`), 'alt'
#   lam       a Lambda may stand here (argument position)
#   pos       where the reading is (see handler)
#   td        type detail after a base: head qname list scalar none str json0 group mapcap q bar
#   last      expr detail after an operand: name qname targs targs_only scalar_kw other
#   notok     `not` may start the next operand
#   ifok      `if` / `for` may start the next operand (a whole Expr is expected)
#   cmp       a comparison operator was seen in the current CmpExpr
#   fresh     nothing consumed yet
#   bare      the item so far is exactly one Name
#   pf pg pp pl   after a paren group: FnType-params / grouped-type / paren-expr / lambda-params still fit
Item = namedtuple('Item', 'T E tsub esub tstart lam pos td last notok ifok cmp fresh bare pf pg pp pl')
Item.__new__.__defaults__ = (True, True, 'full', 'full', 'type', False, 'start', None, None, True, True,
                             False, True, False, False, False, False, False)

# Bracket: a `[ ... ]` after an operand/base.  targs/dim/capdim/index say which forms the
# context allows; phase tracks the list; it1T/it1E remember the first item's readings.
Bracket = namedtuple('Bracket', 'targs dim capdim index phase it1T it1E n')
Bracket.__new__.__defaults__ = (False, False, False, False, 'first', False, False, 0)

# Paren: a `( ... )` at operand start.  fn (FnType params), grp (grouped type), pe (paren
# expression), lp (lambda params) still fit; itT/itE/bare remember the last item.
Paren = namedtuple('Paren', 'fn grp pe lp phase itT itE bare n')
Paren.__new__.__defaults__ = (False, False, False, False, 'first', False, False, False, 0)

Args = namedtuple('Args', 'phase mode itE bare')
Args.__new__.__defaults__ = ('first', None, False, False)

# Brace: mode 'fields' (record literal), 'json' (document object), 'map' (map literal and/or map type)
Brace = namedtuple('Brace', 'mode phase mt ml kT kE')
Brace.__new__.__defaults__ = ('map', 'first', False, False, False, False)

JsonVal = namedtuple('JsonVal', 'phase')
JsonArr = namedtuple('JsonArr', 'phase')
# after `json [`: the token after `[` decides between a budget `[..Size]` (type) and a JSON array
JsonArrOrCap = namedtuple('JsonArrOrCap', 'T E phase')
Array = namedtuple('Array', 'phase')
If = namedtuple('If', 'phase')
For = namedtuple('For', 'phase')
Root4 = namedtuple('Root4', 'phase targs dim')
Root4.__new__.__defaults__ = ('open', False, False)

# results handed to the parent on close
Res = namedtuple('Res', 'kind a b c d')
Res.__new__.__defaults__ = ('item', False, False, False, False)


def new_item(T, E, tsub='full', esub='full', tstart='type', lam=False, ifok=True, notok=True):
    return Item(T=T, E=E, tsub=tsub, esub=esub, tstart=tstart, lam=lam, pos='start',
                notok=notok, ifok=ifok, fresh=True)


def expr_item(lam=False, ifok=True):
    return new_item(False, True, lam=lam, ifok=ifok, notok=True)


def type_item():
    return new_item(True, False)


# actions: ('c', top, pushes) consume; ('r', top, pushes) retry token on new top;
# ('x', res) close, token goes to parent; ('d', res) consume and close; None dead.

def item_close_result(f):
    """The readings an Item has completed, when a token it cannot take arrives."""
    if f.pos == 'after':
        T = f.T and f.td not in ('bar', 'mapcap', None)
        E = f.E and f.last not in ('scalar_kw', 'targs_only')
    elif f.pos in ('closed', 'cmpclosed'):
        T, E = False, True
    elif f.pos == 'json':
        T, E = f.T, False
    elif f.pos == 'paren':
        T, E = f.T and f.pg, f.E and f.pp
    elif f.pos == 'res':
        T, E = f.pf, False
    else:
        return None
    if not (T or E):
        return None
    return Res('item', T, E, f.bare)


def item_lambda_body(f):
    return f._replace(T=False, E=True, esub='full', pos='start', lam=False, notok=True, ifok=True,
                      fresh=False, bare=False, td=None, last=None, cmp=False)


def h_item(f, t):
    pos = f.pos
    if pos == 'start':
        T, E = f.T, f.E
        if t == 'Name':
            if not (T or E):
                return None
            return ('c', f._replace(pos='after', td='head' if T else None, last='name' if E else None,
                                    bare=f.fresh, fresh=False), ())
        if t == 'none':
            if not (T or E):
                return None
            return ('c', f._replace(pos='after', td='none' if T else None, last='other' if E else None,
                                    fresh=False, bare=False), ())
        if t in SCALAR_KW:
            if not (T or E):
                return None
            return ('c', f._replace(pos='after', td='scalar' if T else None, last='scalar_kw' if E else None,
                                    fresh=False, bare=False), ())
        if t == 'bool':
            return ('c', f._replace(pos='after', E=False, td='scalar', fresh=False, bare=False), ()) if T else None
        if t == 'str':
            return ('c', f._replace(pos='after', E=False, td='str', fresh=False, bare=False), ()) if T else None
        if t == 'json':
            if not (T or E):
                return None
            return ('c', f._replace(pos='json', td='json0' if T else None, last='json' if E else None,
                                    fresh=False, bare=False), ())
        if t == '{':
            mt = T  # a map type may start wherever a Base may (tstart type/base/alt)
            ml = E
            if not (mt or ml):
                return None
            return ('c', f._replace(pos='w_brace', fresh=False, bare=False), (Brace(mode='map', mt=mt, ml=ml),))
        if t == '(':
            fn = T and f.tstart == 'type'
            grp = T and f.tstart in ('type', 'base')
            pe = E
            lp = E and f.lam and f.fresh
            if not (fn or grp or pe or lp):
                return None
            return ('c', f._replace(pos='w_paren', fresh=False, bare=False), (Paren(fn=fn, grp=grp, pe=pe, lp=lp),))
        if t == '[':
            return ('c', f._replace(pos='w_array', T=False, fresh=False, bare=False), (Array('first'),)) if E else None
        if t in ('Number', 'String', 'true', 'false', 'to'):
            return ('c', f._replace(pos='after', T=False, td=None, last='other', fresh=False, bare=False), ()) if E else None
        if t == '-':
            return ('c', f._replace(T=False, notok=False, ifok=False, fresh=False, bare=False), ()) if E else None
        if t == 'not':
            if E and f.esub == 'full' and f.notok:
                return ('c', f._replace(T=False, ifok=False, fresh=False, bare=False), ())
            return None
        if t == 'if':
            if E and f.esub == 'full' and f.ifok:
                return ('c', f._replace(pos='w_if', T=False, fresh=False, bare=False), (If('cond'), expr_item()))
            return None
        if t == 'for':
            if E and f.esub == 'full' and f.ifok:
                return ('c', f._replace(pos='w_for', T=False, fresh=False, bare=False), (For('name1'),))
            return None
        return None
    if pos == 'after':
        T, E = f.T, f.E
        e_live = E and f.last not in ('scalar_kw', 'targs_only')
        if t == '::':
            nT = T and f.td == 'head'
            nE = E and f.last == 'name'
            if nT or nE:
                return ('c', f._replace(pos='qual', T=nT, E=nE, bare=False), ())
            return ('x', item_close_result(f))
        if t == '[':
            targs = T and f.td in ('head', 'qname')
            dim = T and f.td in ('head', 'qname', 'list', 'scalar', 'str', 'group') or (T and f.td == 'none' and f.tsub == 'full')
            capdim = T and f.td in ('json0', 'mapcap')
            index = e_live
            if not (targs or dim or capdim or index):
                return ('x', item_close_result(f))
            return ('c', f._replace(pos='w_bracket', bare=False, T=bool(targs or dim or capdim), E=bool(index)),
                    (Bracket(targs=bool(targs), dim=bool(dim), capdim=bool(capdim), index=bool(index)),))
        if t == '?':
            if T and f.tsub == 'full' and f.td not in ('mapcap', 'bar', 'q', None):
                return ('c', f._replace(E=False, td='q', last=None, bare=False), ())
            return ('x', item_close_result(f))
        if t == '|':
            if T and f.tsub == 'full' and f.td not in ('mapcap', 'bar', None):
                return ('c', f._replace(pos='start', E=False, tstart='base', td=None, last=None, fresh=False, bare=False), ())
            return ('x', item_close_result(f))
        if t == '.':
            if e_live:
                return ('c', f._replace(pos='dot', T=False, td=None, bare=False), ())
            return ('x', item_close_result(f))
        if t == '(':
            if E and f.last == 'scalar_kw':
                return ('c', f._replace(pos='w_conv', T=False, td=None, bare=False), (Paren(pe=True),))
            if e_live:
                return ('c', f._replace(pos='w_args', T=False, td=None, bare=False), (Args(),))
            return ('x', item_close_result(f))
        if t == '{':
            if E and f.last in ('name', 'qname', 'targs', 'targs_only'):
                return ('c', f._replace(pos='w_fields', T=False, td=None, bare=False), (Brace(mode='fields'),))
            return ('x', item_close_result(f))
        if t == ':':
            if E and f.lam and f.bare:
                return ('c', item_lambda_body(f), ())
            return ('x', item_close_result(f))
        if t in ARITH:
            if e_live:
                return ('c', f._replace(pos='start', T=False, td=None, last=None, notok=False, ifok=False, bare=False), ())
            return ('x', item_close_result(f))
        if t in CMP:
            if e_live and f.esub == 'full' and not f.cmp:
                return ('c', f._replace(pos='start', T=False, td=None, last=None, notok=False, ifok=False, cmp=True, bare=False), ())
            return ('x', item_close_result(f))
        if t == 'is':
            if e_live and f.esub == 'full' and not f.cmp:
                return ('c', f._replace(pos='is', T=False, td=None, last=None, bare=False), ())
            return ('x', item_close_result(f))
        if t in ('and', 'or'):
            if e_live and f.esub == 'full':
                return ('c', f._replace(pos='start', T=False, td=None, last=None, notok=True, ifok=False, cmp=False, bare=False), ())
            return ('x', item_close_result(f))
        return ('x', item_close_result(f))
    if pos == 'qual':
        if t == 'Name':
            return ('c', f._replace(pos='after', td='qname' if f.T else None, last='qname' if f.E else None), ())
        return None
    if pos == 'dot':
        if t == 'Name':
            return ('c', f._replace(pos='after', last='other'), ())
        return None
    if pos == 'is':
        if t == 'not':
            return ('c', f._replace(pos='is2'), ())
        return ('r', f._replace(pos='w_is'), (new_item(True, True, tsub='alt', esub='fb', tstart='alt', ifok=False, notok=False),))
    if pos == 'is2':
        return ('r', f._replace(pos='w_is'), (new_item(True, True, tsub='alt', esub='fb', tstart='alt', ifok=False, notok=False),))
    if pos == 'json':
        T, E = f.T, f.E
        if t == '[':
            return ('c', f._replace(pos='w_jcap'), (JsonArrOrCap(T=T, E=E, phase='first'),))
        if t == '?':
            if T and f.tsub == 'full':
                return ('c', f._replace(pos='after', E=False, last=None, td='q'), ())
            return ('x', item_close_result(f))
        if t == '|':
            if T and f.tsub == 'full':
                return ('c', f._replace(pos='start', E=False, tstart='base', td=None, last=None), ())
            return ('x', item_close_result(f))
        if E and t in JSON_VALUE_START:
            return ('r', f._replace(pos='w_jval', T=False, td=None), (JsonVal('first'),))
        return ('x', item_close_result(f))
    if pos == 'paren':
        if t == '->':
            if f.pf or f.pl:
                return ('c', f._replace(pos='w_res', T=f.pf, E=f.pl), (type_item(),))
            return ('x', item_close_result(f))
        if t == ':':
            if f.pl:
                return ('c', item_lambda_body(f), ())
            return ('x', item_close_result(f))
        nT = f.T and f.pg
        nE = f.E and f.pp
        if not (nT or nE):
            return ('x', item_close_result(f))
        return ('r', f._replace(pos='after', T=nT, E=nE, td='group' if nT else None, last='other' if nE else None), ())
    if pos == 'res':
        if t == ':' and f.pl:
            return ('c', item_lambda_body(f), ())
        return ('x', item_close_result(f))
    if pos == 'closed':
        return ('x', Res('item', False, True, False))
    if pos == 'cmpclosed':
        if t in ('and', 'or'):
            return ('c', f._replace(pos='start', notok=True, ifok=False, cmp=False), ())
        return ('x', Res('item', False, True, False))
    # waiting positions never see a token directly
    return None


def r_item(f, res):
    """Resume an Item after a child closed with `res`."""
    pos = f.pos
    if pos == 'w_brace':
        nT = f.T and res.a
        nE = f.E and res.b
        if not (nT or nE):
            return None
        return f._replace(pos='after', T=nT, E=nE, td='mapcap' if nT else None, last='other' if nE else None)
    if pos == 'w_paren':
        return f._replace(pos='paren', pf=res.a, pg=res.b, pp=res.c, pl=res.d)
    if pos == 'w_res':
        if not res.a:
            return None
        return f._replace(pos='res')
    if pos == 'w_bracket':
        targs_ok, dim_ok, index_ok = res.a, res.b, res.c
        nT = f.T and (targs_ok or dim_ok)
        nE = f.E and (index_ok or targs_ok)
        if not (nT or nE):
            return None
        last = None
        if nE:
            last = 'targs' if (targs_ok and index_ok) else ('targs_only' if targs_ok else 'other')
        return f._replace(pos='after', T=nT, E=nE, td='list' if nT else None, last=last)
    if pos == 'w_array':
        return f._replace(pos='after', last='other')
    if pos == 'w_conv':
        return f._replace(pos='after', last='other')
    if pos == 'w_args':
        return f._replace(pos='after', last='other')
    if pos == 'w_fields':
        return f._replace(pos='after', last='other')
    if pos == 'w_jcap':
        nT = f.T and res.a
        nE = f.E and res.b
        if not (nT or nE):
            return None
        return f._replace(pos='after', T=nT, E=nE, td='list' if nT else None, last='other' if nE else None)
    if pos == 'w_jval':
        return f._replace(pos='after', last='other')
    if pos == 'w_is':
        if not (res.a or res.b):
            return None
        if res.b:
            return f._replace(pos='after', cmp=True, last='other', E=True)
        return f._replace(pos='cmpclosed', E=True)
    if pos in ('w_if', 'w_for'):
        return f._replace(pos='closed', E=True)
    return None


def h_bracket(f, t):
    ph = f.phase
    if ph == 'first':
        if t == '..':
            if f.index or f.dim or f.capdim:
                return ('c', f._replace(phase='dotdot0', targs=False), ())
            return None
        if t == '^':
            return ('c', f._replace(phase='caret', targs=False, dim=False, capdim=False), ()) if f.index else None
        if t == ']':
            return None
        if f.targs or f.dim or f.index:
            return ('r', f._replace(phase='item1'), (new_item(f.targs, f.dim or f.index),))
        return None
    if ph == 'after1':
        if t == ']':
            targs_ok = f.targs and (f.it1T or f.it1E)
            dim_ok = f.dim and f.it1E
            index_ok = f.index and f.it1E
            if not (targs_ok or dim_ok or index_ok):
                return None
            return ('d', Res('bracket', targs_ok, dim_ok, index_ok))
        if t == ',':
            if f.targs and (f.it1T or f.it1E):
                return ('c', f._replace(phase='comma', dim=False, index=False, n=1), ())
            return None
        if t == '..':
            if f.index and f.it1E:
                return ('c', f._replace(phase='dotdot', targs=False, dim=False), ())
            return None
        return None
    if ph == 'comma':
        if t == ']':
            return None
        return ('r', f._replace(phase='itemN'), (new_item(True, True),))
    if ph == 'afterN':
        if t == ']':
            return ('d', Res('bracket', True, False, False))
        if t == ',':
            return ('c', f._replace(phase='comma'), ())
        return None
    if ph == 'dotdot0':
        if t == ']':
            return ('d', Res('bracket', False, False, True)) if f.index else None
        if t == '^':
            return ('c', f._replace(phase='caret_hi', dim=False, capdim=False), ()) if f.index else None
        return ('r', f._replace(phase='hi0'), (expr_item(),))
    if ph == 'afterhi0':
        if t == ']':
            dim_ok = (f.dim or f.capdim) and f.it1E
            index_ok = f.index and f.it1E
            if not (dim_ok or index_ok):
                return None
            return ('d', Res('bracket', False, dim_ok, index_ok))
        return None
    if ph == 'caret':
        return ('r', f._replace(phase='citem'), (expr_item(),))
    if ph == 'afterc':
        if t == ']':
            return ('d', Res('bracket', False, False, True)) if f.it1E else None
        if t == '..':
            return ('c', f._replace(phase='dotdot'), ()) if f.it1E else None
        return None
    if ph == 'dotdot':
        if t == ']':
            return ('d', Res('bracket', False, False, True))
        if t == '^':
            return ('c', f._replace(phase='caret_hi'), ())
        return ('r', f._replace(phase='hi'), (expr_item(),))
    if ph == 'caret_hi':
        return ('r', f._replace(phase='hi'), (expr_item(),))
    if ph == 'afterhi':
        if t == ']':
            return ('d', Res('bracket', False, False, True)) if f.it1E else None
        return None
    return None


def r_bracket(f, res):
    ph = f.phase
    if ph == 'item1':
        return f._replace(phase='after1', it1T=res.a, it1E=res.b)
    if ph == 'itemN':
        return f._replace(phase='afterN') if (res.a or res.b) else None
    if ph == 'hi0':
        return f._replace(phase='afterhi0', it1E=res.b)
    if ph == 'citem':
        return f._replace(phase='afterc', it1E=res.b)
    if ph == 'hi':
        return f._replace(phase='afterhi', it1E=res.b)
    return None


def h_paren(f, t):
    ph = f.phase
    if ph == 'first':
        if t == ')':
            if f.fn or f.lp:
                return ('d', Res('paren', f.fn, False, False, f.lp))
            return None
        return ('r', f._replace(phase='item1'), (new_item(f.fn or f.grp, f.pe or f.lp),))
    if ph == 'after1':
        if t == ')':
            fn_ok = f.fn and f.itT
            grp_ok = f.grp and f.itT
            pe_ok = f.pe and f.itE
            lp_ok = f.lp and f.bare
            if not (fn_ok or grp_ok or pe_ok or lp_ok):
                return None
            return ('d', Res('paren', fn_ok, grp_ok, pe_ok, lp_ok))
        if t == ':':
            if (f.fn or f.lp) and f.bare:
                return ('c', f._replace(phase='ptype', grp=False, pe=False), (type_item(),))
            return None
        if t == ',':
            fn = f.fn and f.itT
            lp = f.lp and f.bare
            if not (fn or lp):
                return None
            return ('c', f._replace(phase='comma', fn=fn, lp=lp, grp=False, pe=False, n=1), ())
        return None
    if ph == 'aftertype':
        if t == ')':
            return ('d', Res('paren', f.fn, False, False, f.lp))
        if t == ',':
            return ('c', f._replace(phase='comma'), ())
        return None
    if ph == 'comma':
        if t == ')':
            return None
        if not f.fn and t != 'Name':
            return None          # only lambda parameters fit now, and each is one Name
        return ('r', f._replace(phase='itemN'), (new_item(f.fn, f.lp),))
    if ph == 'afterN':
        if t == ')':
            fn = f.fn and f.itT
            lp = f.lp and f.bare
            if not (fn or lp):
                return None
            return ('d', Res('paren', fn, False, False, lp))
        if t == ':':
            if (f.fn or f.lp) and f.bare:
                return ('c', f._replace(phase='ptype'), (type_item(),))
            return None
        if t == ',':
            fn = f.fn and f.itT
            lp = f.lp and f.bare
            if not (fn or lp):
                return None
            return ('c', f._replace(phase='comma', fn=fn, lp=lp), ())
        return None
    return None


def r_paren(f, res):
    ph = f.phase
    if ph == 'item1':
        return f._replace(phase='after1', itT=res.a, itE=res.b, bare=res.c)
    if ph == 'itemN':
        return f._replace(phase='afterN', itT=res.a, itE=res.b, bare=res.c)
    if ph == 'ptype':
        return f._replace(phase='aftertype') if res.a else None
    return None


def h_args(f, t):
    ph = f.phase
    if ph == 'first':
        if t == ')':
            return ('d', Res('args'))
        return ('r', f._replace(phase='item'), (expr_item(lam=True),))
    if ph == 'after':
        if t == ')':
            return ('d', Res('args')) if (f.itE and f.mode != 'named') else None
        if t == ',':
            if f.itE and f.mode != 'named':
                return ('c', f._replace(phase='comma', mode='pos'), ())
            return None
        if t == '=':
            if f.bare and f.mode != 'pos':
                return ('c', f._replace(phase='named', mode='named'), (expr_item(lam=True),))
            return None
        return None
    if ph == 'afternamed':
        if t == ')':
            return ('d', Res('args'))
        if t == ',':
            return ('c', f._replace(phase='comma'), ())
        return None
    if ph == 'comma':
        if t == ')':
            return None
        return ('r', f._replace(phase='item'), (expr_item(lam=True),))
    return None


def r_args(f, res):
    if f.phase == 'item':
        return f._replace(phase='after', itE=res.b, bare=res.c)
    if f.phase == 'named':
        return f._replace(phase='afternamed') if res.b else None
    return None


def h_brace(f, t):
    m, ph = f.mode, f.phase
    if m == 'fields':
        if ph in ('first', 'comma'):
            if t == '}':
                return ('d', Res('brace', False, True))
            if t == 'Name':
                return ('c', f._replace(phase='key'), ())
            return None
        if ph == 'key':
            return ('c', f._replace(phase='val'), (expr_item(),)) if t == ':' else None
        if ph == 'afterval':
            if t == ',':
                return ('c', f._replace(phase='comma'), ())
            if t == '}':
                return ('d', Res('brace', False, True))
            return None
        return None
    if m == 'json':
        if ph == 'first':
            if t == '}':
                return ('d', Res('brace', False, True))
            if t == 'JsonString':
                return ('c', f._replace(phase='key'), ())
            return None
        if ph == 'comma':
            if t == 'JsonString':
                return ('c', f._replace(phase='key'), ())
            return None
        if ph == 'key':
            return ('c', f._replace(phase='val'), (JsonVal('first'),)) if t == ':' else None
        if ph == 'afterval':
            if t == ',':
                return ('c', f._replace(phase='comma'), ())
            if t == '}':
                return ('d', Res('brace', False, True))
            return None
        return None
    # map literal and/or map type
    if ph == 'first':
        if t == '}':
            return ('d', Res('brace', False, True)) if f.ml else None
        return ('r', f._replace(phase='key'), (new_item(f.mt, f.ml),))
    if ph == 'afterkey':
        if t == ':':
            return ('c', f._replace(phase='val'), (new_item(f.kT, f.kE),))
        return None
    if ph == 'afterval':
        if t == '}':
            return ('d', Res('brace', f.mt, f.ml))
        if t == ',':
            return ('c', f._replace(phase='comma', mt=False), ()) if f.ml else None
        return None
    if ph == 'comma':
        if t == '}':
            return ('d', Res('brace', False, True))
        return ('r', f._replace(phase='key2'), (expr_item(),))
    if ph == 'afterkey2':
        if t == ':':
            return ('c', f._replace(phase='val2'), (expr_item(),))
        return None
    if ph == 'afterval2':
        if t == '}':
            return ('d', Res('brace', False, True))
        if t == ',':
            return ('c', f._replace(phase='comma'), ())
        return None
    return None


def r_brace(f, res):
    m, ph = f.mode, f.phase
    if m in ('fields', 'json'):
        return f._replace(phase='afterval') if ph == 'val' else None
    if ph == 'key':
        kT = f.mt and res.a
        kE = f.ml and res.b
        if not (kT or kE):
            return None
        return f._replace(phase='afterkey', kT=kT, kE=kE)
    if ph == 'val':
        mt = f.kT and res.a
        ml = f.kE and res.b
        if not (mt or ml):
            return None
        return f._replace(phase='afterval', mt=mt, ml=ml)
    if ph == 'key2':
        return f._replace(phase='afterkey2') if res.b else None
    if ph == 'val2':
        return f._replace(phase='afterval2') if res.b else None
    return None


def h_jsonval(f, t):
    ph = f.phase
    if ph == 'first':
        if t == '{':
            return ('c', f._replace(phase='obj'), (Brace(mode='json'),))
        if t == '[':
            return ('c', f._replace(phase='arr'), (JsonArr('first'),))
        if t in ('JsonString', 'JsonNumber', 'true', 'false', 'null'):
            return ('d', Res('json'))
        if t == '-':
            return ('c', f._replace(phase='neg'), ())
        if t == 'Name':
            return ('c', f._replace(phase='name'), ())
        if t == 'to':
            return ('c', f._replace(phase='to'), ())
        if t == '(':
            return ('c', f._replace(phase='expr'), (expr_item(),))
        return None
    if ph == 'neg':
        return ('d', Res('json')) if t == 'JsonNumber' else None
    if ph == 'name':
        if t == '::':
            return ('c', f._replace(phase='qual'), ())
        if t == '.':
            return ('c', f._replace(phase='dot'), ())
        return ('x', Res('json'))
    if ph == 'qual':
        return ('c', f._replace(phase='path'), ()) if t == 'Name' else None
    if ph == 'path':
        if t == '.':
            return ('c', f._replace(phase='dot'), ())
        return ('x', Res('json'))
    if ph == 'dot':
        return ('c', f._replace(phase='path'), ()) if t == 'Name' else None
    if ph == 'to':
        return ('c', f._replace(phase='dot'), ()) if t == '.' else None
    if ph == 'closeparen':
        return ('d', Res('json')) if t == ')' else None
    return None


def r_jsonval(f, res):
    if f.phase in ('obj', 'arr'):
        return f._replace(phase='done')
    if f.phase == 'expr':
        return f._replace(phase='closeparen') if res.b else None
    return None


def h_jsonval_done(f, t):
    return ('x', Res('json'))


def h_jsonarr(f, t):
    ph = f.phase
    if ph == 'first':
        if t == ']':
            return ('d', Res('jsonarr'))
        return ('r', f._replace(phase='val'), (JsonVal('first'),))
    if ph == 'after':
        if t == ',':
            return ('c', f._replace(phase='comma'), ())
        if t == ']':
            return ('d', Res('jsonarr'))
        return None
    if ph == 'comma':
        if t == ']':
            return None
        return ('r', f._replace(phase='val'), (JsonVal('first'),))
    return None


def r_jsonarr(f, res):
    return f._replace(phase='after') if f.phase == 'val' else None


def h_jcap(f, t):
    ph = f.phase
    if ph == 'first':
        if t == '..':
            return ('c', f._replace(phase='size', E=False), (expr_item(),)) if f.T else None
        if t == ']':
            return ('d', Res('jcap', False, True)) if f.E else None
        if f.E:
            return ('r', f._replace(phase='val', T=False), (JsonVal('first'),))
        return None
    if ph == 'aftersize':
        return ('d', Res('jcap', True, False)) if t == ']' else None
    if ph == 'after':
        if t == ',':
            return ('c', f._replace(phase='comma'), ())
        if t == ']':
            return ('d', Res('jcap', False, True))
        return None
    if ph == 'comma':
        if t == ']':
            return None
        return ('r', f._replace(phase='val'), (JsonVal('first'),))
    return None


def r_jcap(f, res):
    if f.phase == 'size':
        return f._replace(phase='aftersize') if res.b else None
    if f.phase == 'val':
        return f._replace(phase='after')
    return None


def h_array(f, t):
    ph = f.phase
    if ph == 'first':
        if t == ']':
            return ('d', Res('array'))
        return ('r', f._replace(phase='item'), (expr_item(),))
    if ph == 'after':
        if t == ',':
            return ('c', f._replace(phase='comma'), ())
        if t == ']':
            return ('d', Res('array'))
        return None
    if ph == 'comma':
        if t == ']':
            return ('d', Res('array'))
        return ('r', f._replace(phase='item'), (expr_item(),))
    return None


def r_array(f, res):
    return f._replace(phase='after') if (f.phase == 'item' and res.b) else None


def h_if(f, t):
    if f.phase == 'thenwait':
        return ('c', f._replace(phase='then'), (expr_item(),)) if t == 'then' else None
    if f.phase == 'elsewait':
        return ('c', f._replace(phase='else'), (expr_item(),)) if t == 'else' else None
    if f.phase == 'done':
        return ('x', Res('if'))
    return None


def r_if(f, res):
    if not res.b:
        return None
    return {'cond': f._replace(phase='thenwait'), 'then': f._replace(phase='elsewait'),
            'else': f._replace(phase='done')}.get(f.phase)


def h_for(f, t):
    ph = f.phase
    if ph == 'name1':
        return ('c', f._replace(phase='after1'), ()) if t == 'Name' else None
    if ph in ('after1', 'aftertype1'):
        if t == ':' and ph == 'after1':
            return ('c', f._replace(phase='type1'), (type_item(),))
        if t == ',':
            return ('c', f._replace(phase='name2'), ())
        if t == 'in':
            return ('c', f._replace(phase='iter'), (expr_item(),))
        return None
    if ph == 'name2':
        return ('c', f._replace(phase='after2'), ()) if t == 'Name' else None
    if ph in ('after2', 'aftertype2'):
        if t == ':' and ph == 'after2':
            return ('c', f._replace(phase='type2'), (type_item(),))
        if t == 'in':
            return ('c', f._replace(phase='iter'), (expr_item(),))
        return None
    if ph == 'afteriter':
        if t == '..':
            return ('c', f._replace(phase='iter2'), (expr_item(),))
        if t == 'into':
            return ('c', f._replace(phase='accname'), ())
        return None
    if ph == 'afteriter2':
        return ('c', f._replace(phase='accname'), ()) if t == 'into' else None
    if ph == 'accname':
        return ('c', f._replace(phase='afteracc'), ()) if t == 'Name' else None
    if ph in ('afteracc', 'afteracctype'):
        if t == ':' and ph == 'afteracc':
            return ('c', f._replace(phase='acctype'), (type_item(),))
        if t == 'from':
            return ('c', f._replace(phase='from'), (expr_item(),))
        return None
    if ph == 'afterfrom':
        return ('c', f._replace(phase='body'), (expr_item(),)) if t == ':' else None
    if ph == 'done':
        return ('x', Res('for'))
    return None


def r_for(f, res):
    ph = f.phase
    if ph in ('type1', 'type2', 'acctype'):
        if not res.a:
            return None
        return f._replace(phase={'type1': 'aftertype1', 'type2': 'aftertype2', 'acctype': 'afteracctype'}[ph])
    if not res.b:
        return None
    return f._replace(phase={'iter': 'afteriter', 'iter2': 'afteriter2', 'from': 'afterfrom', 'body': 'done'}.get(ph, 'bad')) \
        if ph in ('iter', 'iter2', 'from', 'body') else None


def h_root4(f, t):
    if f.phase == 'open':
        return ('c', f._replace(phase='inner'), (Bracket(targs=True, dim=True),)) if t == '[' else None
    if f.phase == 'done':
        return ('x', Res('root4', f.targs, f.dim))
    return None


def r_root4(f, res):
    return f._replace(phase='done', targs=res.a, dim=res.b) if f.phase == 'inner' else None


HANDLE = {Item: h_item, Bracket: h_bracket, Paren: h_paren, Args: h_args, Brace: h_brace,
          JsonVal: h_jsonval, JsonArr: h_jsonarr, JsonArrOrCap: h_jcap, Array: h_array,
          If: h_if, For: h_for, Root4: h_root4}
RESUME = {Item: r_item, Bracket: r_bracket, Paren: r_paren, Args: r_args, Brace: r_brace,
          JsonVal: r_jsonval, JsonArr: r_jsonarr, JsonArrOrCap: r_jcap, Array: r_array,
          If: r_if, For: r_for, Root4: r_root4}


def _handle(f, t):
    if type(f) is JsonVal and f.phase == 'done':
        return h_jsonval_done(f, t)
    return HANDLE[type(f)](f, t)


def init(position):
    if position in ('free', 'typearg'):
        return (new_item(True, True),)
    if position == 'is':
        return (new_item(True, True, tsub='alt', esub='fb', tstart='alt', ifok=False, notok=False),)
    if position == 'bracket':
        return (Root4(),)
    raise ValueError(position)


DEAD = None


def step(stack, t):
    """Feed one token.  Returns the new stack, or None when the sequence is a
    syntax fault in every reading.  For END, returns ('answer', T, E)."""
    while True:
        top = stack[-1]
        act = _handle(top, t)
        if act is None:
            return None
        kind = act[0]
        if kind == 'c' or kind == 'r':
            stack = stack[:-1] + (act[1],) + act[2]
            if kind == 'c':
                return stack
            continue
        res = act[1]
        if res is None:
            return None
        if len(stack) == 1:
            if t == END and kind == 'x':
                if res.kind == 'root4':
                    return ('answer', res.a, res.b)
                return ('answer', res.a, res.b)
            return None
        parent = RESUME[type(stack[-2])](stack[-2], res)
        if parent is None:
            return None
        stack = stack[:-2] + (parent,)
        if kind == 'd':
            return stack


def answer(stack):
    """Classify the sequence read so far (as if it ended here)."""
    if stack is None:
        return 'neither'
    r = step(stack, END)
    if r is None or r[0] != 'answer':
        return 'neither'
    _, T, E = r
    if T and E:
        return 'both'
    if T:
        return 'type-only'
    if E:
        return 'expression-only'
    return 'neither'


def classify(position, toks):
    st = init(position)
    for t in toks:
        st = step(st, t)
        if st is None:
            return 'neither'
    return answer(st)


if __name__ == '__main__':
    import os
    import sys
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from grammar import tokenize
    for src in sys.argv[1:] or ('Band[4]', 'i32[3]', 'x + 1', 'json[2]', 'json[..2]', '(a)', 'a | b'):
        toks = tokenize(src)
        print('%-30s %-40s free=%s is=%s' % (src, ' '.join(toks), classify('free', toks), classify('is', toks)))
