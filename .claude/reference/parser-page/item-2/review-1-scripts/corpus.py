"""Rebuild, by my own method, the count of body lines with text after a colon in the 60 programs,
and how many of those tails fit the children reading, the destination reading, or both.

Method: my own tokenizer with byte offsets (so adjacency is known); a line classifier that tracks
the enclosing block's opener (match arms, guard arms, type fields and function bodies are not body
lines); the tail after the head's first depth-0 colon is mapped to abstract token kinds and tested
with an Earley recogniser against grammars written from SPEC 7.1, 7.5 to 7.9, 8.1, 8.2, 9.1 and 5.1.
Run: python3 -I corpus.py <files...>
"""
import re, sys, collections

# ---------------- Earley ----------------
def earley(grammar, start, toks):
    n = len(toks)
    chart = [set() for _ in range(n + 1)]
    for i in range(n + 1):
        agenda = list(chart[i])
        if i == 0:
            for ai in range(len(grammar[start])):
                it = (start, ai, 0, 0); chart[0].add(it); agenda.append(it)
        while agenda:
            nt, ai, dot, origin = agenda.pop()
            alt = grammar[nt][ai]
            if dot < len(alt):
                sym = alt[dot]
                if sym in grammar:
                    for ai2 in range(len(grammar[sym])):
                        it = (sym, ai2, 0, i)
                        if it not in chart[i]:
                            chart[i].add(it); agenda.append(it)
                    for (nt2, ai2, dot2, o2) in list(chart[i]):
                        if nt2 == sym and o2 == i and dot2 == len(grammar[nt2][ai2]):
                            adv = (nt, ai, dot + 1, origin)
                            if adv not in chart[i]:
                                chart[i].add(adv); agenda.append(adv)
                elif i < n and toks[i] == sym:
                    adv = (nt, ai, dot + 1, origin)
                    chart[i + 1].add(adv)
            else:
                for (nt2, ai2, dot2, o2) in list(chart[origin]):
                    alt2 = grammar[nt2][ai2]
                    if dot2 < len(alt2) and alt2[dot2] == nt:
                        adv = (nt2, ai2, dot2 + 1, o2)
                        if adv not in chart[i]:
                            chart[i].add(adv); agenda.append(adv)
    return any(nt == start and dot == len(grammar[nt][ai]) and o == 0 for (nt, ai, dot, o) in chart[n])

# ---------------- tokenizer ----------------
KEYWORDS = set("""let state stream channel fn pattern type default const cell out children if then else match when
for in into from as on without is not and or div to true false none u8 i32 i64 f32 f64 bool str json node wire
require use publish""".split())
SCALARS = {'u8', 'i32', 'i64', 'f32', 'f64', 'bool'}
TOKRE = re.compile(r'''(?P<ws>[ \t]+)|(?P<comment>//[^\n]*)|(?P<str>"(?:\\.|[^"\\\n])*")|(?P<num>\d[\w.]*)
  |(?P<name>[^\W\d][\w\#]*|\#[\w\#]*)|(?P<op>::|\.\.|\*\*|\?\?|<=|>=|=>|->|[^\s\w])''', re.X)

def tokenize(line):
    out = []; pos = 0; spaced = True
    while pos < len(line):
        m = TOKRE.match(line, pos)
        if not m: break
        pos = m.end()
        if m.group('ws'): spaced = True; continue
        if m.group('comment'): break
        kind = m.lastgroup; text = m.group(kind)
        out.append((kind, text, spaced)); spaced = False
    return out

def abstract(tok, prev):
    kind, text, spaced = tok
    adj = '' if spaced else '~'
    if kind == 'str': k = 'STR'
    elif kind == 'num': k = 'NUM'
    elif kind == 'name':
        if text in SCALARS: k = 'I32'
        elif text in ('true', 'false'): k = 'TRUE'
        elif text == 'none': k = 'NONE'
        elif text == 'null': k = 'NONE_J'
        elif text == 'str': k = 'STRT'
        elif text == 'json': k = 'JSON'
        elif text in ('and',): k = 'AND'
        elif text in ('or',): k = 'OR'
        elif text == 'not': k = 'NOT'
        elif text == 'is': k = 'IS'
        elif text in ('div',): k = 'STAR'
        elif text == 'without': k = 'PLUS'
        elif text in KEYWORDS: k = text.upper()
        else: k = 'N'
    else:
        k = {'::': 'COLON2', '..': 'DOTDOT', '**': 'POW', '??': 'QQ', '<=': 'LT', '>=': 'LT', '<': 'LT', '>': 'LT', '=>': 'ARROW2', '->': 'ARROW',
             '+': 'PLUS', '-': 'MINUS', '*': 'STAR', '/': 'SLASH', '%': 'STAR', '(': 'LP', ')': 'RP', '[': 'LB', ']': 'RB', '{': 'LBR', '}': 'RBR',
             ',': 'COMMA', ':': 'COLON', '^': 'CARET', '.': 'DOT', '=': 'EQ', '|': 'BAR', '?': 'QMARK', '!': 'BANG',
             "'": 'FLAG', '~': 'FLAG', '@': 'FLAG', '$': 'FLAG'}.get(text, 'X')
    return adj + k

def abstract_line(toks):
    out = []
    i = 0
    while i < len(toks):
        # fold Name :: Name into Q
        if toks[i][0] == 'name' and toks[i][1] not in KEYWORDS and i + 2 < len(toks) and toks[i+1][1] == '::' and toks[i+2][0] == 'name':
            out.append(('' if toks[i][2] else '~') + 'Q'); i += 3; continue
        out.append(abstract(toks[i], toks[i-1] if i else None)); i += 1
    return out

# ---------------- grammars over real abstract tokens ----------------
def anyf(t): return [[t], ['~' + t]]   # spaced or adjacent

G = {}
# expressions: adjacency is irrelevant, so every terminal takes both forms through A_* helpers
EXPR_TERMS = ['N','Q','NUM','STR','TRUE','TO','NONE','MINUS','PLUS','STAR','SLASH','LP','RP','LB','RB','LBR','RBR','COMMA','COLON','DOTDOT','CARET','DOT',
              'IS','NOT','IF','THEN','ELSE','I32','STRT','JSON','QMARK','ARROW','BAR','OR','AND','LT','QQ','POW','EQ','FOR','IN','INTO','FROM','MATCH','NONE_J','WHEN']
for t in EXPR_TERMS:
    G['a' + t] = anyf(t)
G.update({
 'Dest':     [['Expr'], ['aMATCH', 'Expr', 'aCOLON']],
 'Expr':     [['IfExpr'], ['OrExpr']],
 'IfExpr':   [['aIF', 'Expr', 'aTHEN', 'Expr', 'aELSE', 'Expr']],
 'ForExpr':  [['aFOR', 'aN', 'aIN', 'Iter', 'aINTO', 'aN', 'aFROM', 'Expr', 'aCOLON', 'Expr'], ['aFOR', 'aN', 'aCOMMA', 'aN', 'aIN', 'Iter', 'aINTO', 'aN', 'aFROM', 'Expr', 'aCOLON', 'Expr']],
 'Iter':     [['Expr'], ['Expr', 'aDOTDOT', 'Expr']],
 'OrExpr':   [['AndExpr'], ['OrExpr', 'aOR', 'AndExpr']],
 'AndExpr':  [['NotExpr'], ['AndExpr', 'aAND', 'NotExpr']],
 'NotExpr':  [['aNOT', 'NotExpr'], ['CmpExpr']],
 'CmpExpr':  [['Fallback'], ['Fallback', 'aLT', 'Fallback'], ['Fallback', 'aIS', 'Fallback'], ['Fallback', 'aIS', 'aNOT', 'Fallback'], ['Fallback', 'aIS', 'AltType'], ['Fallback', 'aIS', 'aNOT', 'AltType']],
 'Fallback': [['AddExpr'], ['AddExpr', 'aQQ', 'Fallback']],
 'AddExpr':  [['MulExpr'], ['AddExpr', 'aPLUS', 'MulExpr'], ['AddExpr', 'aMINUS', 'MulExpr']],
 'MulExpr':  [['UnaryExpr'], ['MulExpr', 'aSTAR', 'UnaryExpr'], ['MulExpr', 'aSLASH', 'UnaryExpr']],
 'UnaryExpr':[['aMINUS', 'UnaryExpr'], ['PowExpr']],
 'PowExpr':  [['Postfix'], ['Postfix', 'aPOW', 'UnaryExpr']],
 'Postfix':  [['Primary'], ['Postfix', 'aDOT', 'aN'], ['Postfix', 'aLB', 'Index', 'aRB'], ['Postfix', 'aLP', 'aRP'], ['Postfix', 'aLP', 'Args', 'aRP']],
 'Primary':  [['aNUM'], ['aTRUE'], ['aSTR'], ['RecordLit'], ['MapLit'], ['JsonLit'], ['aNONE'], ['aN'], ['aQ'], ['aTO'], ['Conversion'], ['ArrayLit'], ['aLP', 'Expr', 'aRP'], ['aLP', 'ForExpr', 'aRP']],
 'ArrayLit': [['aLB', 'aRB'], ['aLB', 'ExprList', 'aRB'], ['aLB', 'ExprList', 'aCOMMA', 'aRB']],
 'ExprList': [['Expr'], ['ExprList', 'aCOMMA', 'Expr']],
 'Args':     [['ArgList'], ['NamedList']],
 'ArgList':  [['Arg'], ['ArgList', 'aCOMMA', 'Arg']],
 'Arg':      [['Expr'], ['Lambda']],
 'NamedList':[['NamedArg'], ['NamedList', 'aCOMMA', 'NamedArg']],
 'NamedArg': [['aN', 'aEQ', 'Arg']],
 'Lambda':   [['aN', 'aCOLON', 'Expr'], ['aLP', 'aRP', 'aCOLON', 'Expr'], ['aLP', 'LParams', 'aRP', 'aCOLON', 'Expr'],
              ['aLP', 'aRP', 'aARROW', 'Type', 'aCOLON', 'Expr'], ['aLP', 'LParams', 'aRP', 'aARROW', 'Type', 'aCOLON', 'Expr']],
 'LParams':  [['LParam'], ['LParams', 'aCOMMA', 'LParam']],
 'LParam':   [['aN'], ['aN', 'aCOLON', 'Type']],
 'Index':    [['Bound'], ['aDOTDOT'], ['Bound', 'aDOTDOT'], ['aDOTDOT', 'Bound'], ['Bound', 'aDOTDOT', 'Bound']],
 'Bound':    [['Expr'], ['aCARET', 'Expr']],
 'Conversion':[['aI32', 'aLP', 'Expr', 'aRP']],
 'RecordLit':[['aN', 'aLBR', 'Fields', 'aRBR'], ['aQ', 'aLBR', 'Fields', 'aRBR'], ['aN', 'TypeArgs', 'aLBR', 'Fields', 'aRBR'], ['aQ', 'TypeArgs', 'aLBR', 'Fields', 'aRBR']],
 'Fields':   [[], ['FieldList'], ['FieldList', 'aCOMMA']],
 'FieldList':[['aN', 'aCOLON', 'Expr'], ['FieldList', 'aCOMMA', 'aN', 'aCOLON', 'Expr']],
 'MapLit':   [['aLBR', 'aRBR'], ['aLBR', 'EntryList', 'aRBR'], ['aLBR', 'EntryList', 'aCOMMA', 'aRBR']],
 'EntryList':[['Entry'], ['EntryList', 'aCOMMA', 'Entry']],
 'Entry':    [['Expr', 'aCOLON', 'Expr']],
 'JsonLit':  [['aJSON', 'JsonValue']],
 'JsonValue':[['aLBR', 'aRBR'], ['aLBR', 'JEntries', 'aRBR'], ['aLB', 'aRB'], ['aLB', 'JList', 'aRB'], ['aSTR'], ['aNUM'], ['aMINUS', 'aNUM'], ['aTRUE'], ['aNONE_J'], ['JPath'], ['aTO', 'aDOT', 'aN'], ['aLP', 'Expr', 'aRP']],
 'JEntries': [['aSTR', 'aCOLON', 'JsonValue'], ['JEntries', 'aCOMMA', 'aSTR', 'aCOLON', 'JsonValue']],
 'JList':    [['JsonValue'], ['JList', 'aCOMMA', 'JsonValue']],
 'JPath':    [['aN'], ['aQ'], ['JPath', 'aDOT', 'aN']],
 'Type':     [['FnType'], ['Sum']],
 'Sum':      [['Base'], ['Sum', 'aBAR', 'Base']],
 'FnType':   [['aLP', 'aRP', 'aARROW', 'Type'], ['aLP', 'FnParams', 'aRP', 'aARROW', 'Type']],
 'FnParams': [['FnParam'], ['FnParams', 'aCOMMA', 'FnParam']],
 'FnParam':  [['Type'], ['aN', 'aCOLON', 'Type']],
 'Base':     [['Base0'], ['Base0', 'Dims'], ['Base0', 'aQMARK'], ['Base0', 'Dims', 'aQMARK']],
 'Base0':    [['aI32'], ['StrType'], ['MapType'], ['JsonType'], ['aN'], ['aQ'], ['aN', 'TypeArgs'], ['aQ', 'TypeArgs'], ['aNONE'], ['aLP', 'Type', 'aRP']],
 'Dims':     [['Dim'], ['Dims', 'Dim']],
 'Dim':      [['aLB', 'Expr', 'aRB'], ['aLB', 'aDOTDOT', 'Expr', 'aRB']],
 'StrType':  [['aSTRT'], ['aSTRT', 'aLB', 'aDOTDOT', 'Expr', 'aRB']],
 'MapType':  [['aLBR', 'Type', 'aCOLON', 'Type', 'aRBR', 'aLB', 'aDOTDOT', 'Expr', 'aRB']],
 'JsonType': [['aJSON'], ['aJSON', 'aLB', 'aDOTDOT', 'Expr', 'aRB']],
 'TypeArgs': [['aLB', 'TypeArgList', 'aRB']],
 'TypeArgList':[['TypeArg'], ['TypeArgList', 'aCOMMA', 'TypeArg']],
 'TypeArg':  [['Type'], ['Expr']],
 'AltType':  [['aNONE'], ['AltBase'], ['AltBase', 'Dims']],
 'AltBase':  [['aI32'], ['StrType'], ['MapType'], ['JsonType'], ['aN'], ['aQ'], ['aN', 'TypeArgs'], ['aQ', 'TypeArgs']],
 # ---- the children reading. A member after the first starts spaced (the ruling); a flag is adjacent (7.4).
 'InlineChildren': [['First'], ['First', 'Rest']],
 'First':    [['MemberAny'], ['BundleAny']],
 'Rest':     [['MemberSp'], ['BundleSp'], ['Rest', 'MemberSp'], ['Rest', 'BundleSp']],
 'MemberAny':[['KindAny', 'MTail'], ['aLP', 'Placement', 'aRP']],
 'MemberSp': [['KindSp', 'MTail'], ['LP', 'Placement', 'aRP']],
 'KindAny':  [['aN'], ['aQ']],
 'KindSp':   [['N'], ['Q']],
 'MTail':    [[], ['Flags'], ['AsName'], ['Flags', 'AsName'], ['DefaultArg'], ['Flags', 'DefaultArg'], ['AsName', 'DefaultArg'], ['Flags', 'AsName', 'DefaultArg']],
 'Flags':    [['Flag'], ['Flags', 'Flag']],
 'Flag':     [['~FLAG'], ['~STAR'], ['~PLUS'], ['~BANG'], ['~QMARK']],
 'AsName':   [['aAS', 'aN']],
 'DefaultArg':[['aSLASH', 'Atom'], ['aSLASH', 'aLP', 'Expr', 'aRP']],
 'Atom':     [['aNUM'], ['aMINUS', 'aNUM'], ['aTRUE'], ['aSTR'], ['Path'], ['aTO', 'DotNames']],
 'Path':     [['aN'], ['aQ'], ['Path', 'aDOT', 'aN']],
 'DotNames': [['aDOT', 'aN'], ['DotNames', 'aDOT', 'aN']],
 'Placement':[['KindAny', 'MTail', 'PTail']],
 'PTail':    [[], ['Gate'], ['Attrs'], ['Gate', 'Attrs']],
 'Gate':     [['aWHEN', 'Expr']],
 'Attrs':    [['aBAR', 'Entries']],
 'Entries':  [['AttrEntry'], ['Entries', 'AttrEntry']],
 'AttrEntry':[['aN', 'aEQ', 'Expr'], ['aN', 'aARROW2', 'Path'], ['aN'], ['aBANG', 'aN']],
 'BundleAny':[['aLB', 'BInside', 'aRB']],
 'BundleSp': [['LB', 'BInside', 'aRB']],
 'BInside':  [['BMembers'], ['BMembers', 'BundleFor'], ['BundleFor']],
 'BMembers': [['MemberAny'], ['BMembers', 'MemberSp']],
 'BundleFor':[['aFOR', 'aN', 'aIN', 'Iter', 'aCOLON', 'BMembers'], ['aFOR', 'aN', 'aCOMMA', 'aN', 'aIN', 'Iter', 'aCOLON', 'BMembers']],
})
G['aARROW2'] = anyf('ARROW2'); G['aBANG'] = anyf('BANG'); G['aAS'] = anyf('AS'); G['aMATCH'] = anyf('MATCH')

def fits(start, toks): return earley(G, start, toks)

# ---------------- line classification ----------------
LINE_OPENERS = {'match': 'match', 'type': 'type', 'fn': 'fn'}
def indent_of(raw):
    return len(raw) - len(raw.lstrip(' '))

def main():
    counts = collections.Counter(); examples = collections.defaultdict(list)
    files = 0
    for path in sys.argv[1:]:
        files += 1
        text = open(path, encoding='utf-8').read()
        stack = []           # (indent, opener kind)
        depth = 0            # unclosed brackets carried across lines
        for raw in text.split('\n'):
            if depth > 0:
                for t in tokenize(raw):
                    if t[1] in '([{': depth += 1
                    elif t[1] in ')]}': depth -= 1
                continue
            toks = tokenize(raw)
            if not toks: continue
            ind = indent_of(raw)
            while stack and stack[-1][0] >= ind: stack.pop()
            enclosing = stack[-1][1] if stack else 'top'
            # the first depth-0 colon
            d = 0; colon = None
            for i, t in enumerate(toks):
                if t[1] in '([{': d += 1
                elif t[1] in ')]}': d -= 1
                elif t[1] == ':' and d == 0 and colon is None: colon = i
            depth = max(d, 0)
            first = toks[0][1]
            ends_colon = toks[-1][1] == ':'
            # what block does this line open
            opener = None
            if ends_colon:
                if any(t[1] == 'match' for t in toks): opener = 'match'
                elif first == 'when' and colon == 1: opener = 'guard'
                elif first in ('type', 'fn'): opener = first
                else: opener = 'body'
            is_body_line = (ind > 0 and enclosing in ('body', 'guard_arm') and toks[0][0] == 'name' and first not in KEYWORDS
                            and not (len(toks) > 1 and toks[1][1] in ('=', '=>')))
            if enclosing == 'guard' and ends_colon:
                opener = 'guard_arm'   # a guard arm's block holds body items
            if opener: stack.append((ind, opener))
            if not is_body_line or colon is None: continue
            tail = toks[colon+1:]
            if not tail: continue
            counts['lines'] += 1
            abs_toks = abstract_line(tail)
            ch = fits('InlineChildren', abs_toks)
            de = fits('Dest', abs_toks)
            cls = 'both' if ch and de else 'children only' if ch else 'destination only' if de else 'neither'
            counts[cls] += 1
            if cls == 'both':
                counts['both: single bare name' if len(abs_toks) == 1 and abs_toks[0].lstrip('~') in ('N', 'Q') else 'both: longer'] += 1
            if len(examples[cls]) < 12 or cls in ('neither', 'children only') or (cls == 'both' and len(abs_toks) > 1):
                examples[cls].append(f'{path.split("/")[-1]}: {raw.strip()}   [{" ".join(abs_toks)}]')
    print('files', files)
    for k, v in sorted(counts.items()): print(f'{k:28s} {v}')
    for k, v in examples.items():
        print('---', k)
        for e in v[:40]: print('   ', e)

main()
