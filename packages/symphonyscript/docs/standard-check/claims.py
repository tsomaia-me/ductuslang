#!/usr/bin/env python3
"""The check of what STANDARD.md's "What to see in it" says of the good form's text.
Reads the block exactly as the document prints it, from the file run.sh hands it. Exits 1 on the first
claim that does not hold."""
import re, sys

src = open(sys.argv[1], encoding="utf-8").read().split("\n")


def block(header):
    """The lines of the item whose first line starts with `header` after its indentation."""
    start = next(i for i, l in enumerate(src) if l.strip().startswith(header))
    indent = src[start][: len(src[start]) - len(src[start].lstrip())]
    end = next(i for i in range(start, len(src)) if src[i] == indent + "}" or src[i] == indent + "};")
    return start, end


def inside(i, span):
    return span[0] <= i <= span[1]


def fail(what):
    print("claim fails:", what)
    sys.exit(1)


code = [(i, l) for i, l in enumerate(src) if not l.strip().startswith("//")]
kinds = block("impl TokKind {")
op = block("impl Op {")
toks = block("impl Toks {")
stop = block("fn stop(")
group = block("impl Group {")
nest = block("impl Nest {")
expression = block("pub fn expression(")
push = block("fn push(")
read = block("pub fn read(")
node = block("pub fn node(")
picks = block("let step = match expect {")
applies = block("match step {")

# The table is data.
if any(re.search(r"\bif\b", l) for i, l in code if inside(i, kinds)):
    fail("an arm of `starts` or `continues` holds logic")
table = ["OrExpr", "AndExpr", "NotExpr", 'CmpExpr "CmpOp Fallback"', "Fallback", "AddExpr", "MulExpr", "UnaryExpr", "PowExpr"]
for production in table:
    at = [i for i, l in enumerate(src) if l.strip() == "// SPEC §5.1 " + production]
    if len(at) != 1 or not inside(at[0], kinds):
        fail("the row of %s carries no mark of its own in `impl TokKind`" % production)
compare = re.compile(r">=|<=|==|!=| < | > ")
for i, l in code:
    if re.search(r"level", l, re.I) and compare.search(l) and not inside(i, op):
        fail("a comparison of two levels outside `impl Op`, line %d" % (i + 1))

# The questions about a kind.
for i, l in code:
    if re.search(r"\b_ =>", l):
        fail("a catch-all arm, line %d" % (i + 1))
    if "matches!(" in l or re.search(r"\bif let\b", l) or re.search(r"\blet\b.*\belse\b", l):
        fail("`matches!`, `if let` or `let … else`, line %d" % (i + 1))
    if re.search(r"\buse\b.*::\*", l):
        fail("a glob import, line %d" % (i + 1))
    if re.search(r"TokKind::\w+\s*(=>|\|)", l) or re.search(r"\|\s*TokKind::", l):
        fail("a `match` names a token kind outside `impl TokKind`, line %d" % (i + 1))
for name in ("const fn starts(self)", "const fn continues(self)"):
    if not any(name in l for i, l in code if inside(i, kinds)):
        fail("`impl TokKind` lacks %s" % name)
if sum(1 for i, l in code if inside(i, kinds) and re.search(r"\bfn\b", l)) != 2:
    fail("`impl TokKind` holds another question than the two")

# `==` on a kind stands once.
equal = [i for i, l in code if "== TokKind::" in l]
if len(equal) != 1 or not inside(equal[0], group) or any("!= TokKind::" in l for i, l in code):
    fail("`==` or `!=` on a kind stands elsewhere than once in `impl Group`")

# `expression` is a chooser.
handlers = set()
for i, l in code:
    if inside(i, picks) and "=>" in l and not l.rstrip().endswith("{"):
        calls = re.findall(r"(?<!\w)([a-z_][a-z0-9_]*)\(", l.split("=>", 1)[1])
        if len(calls) > 1:
            fail("an arm of the chooser makes more than one call, line %d" % (i + 1))
        handlers.update(calls)
if handlers != {"leaf", "open", "prefix_operator", "infix_operator", "stop", "no_operand"}:
    fail("the chooser's handlers are %s" % sorted(handlers))
if any(re.search(r"TokKind|Starts|Continues", l) for i, l in code if inside(i, applies)):
    fail("the chooser's second `match` names a token or a construct")

# A fault is a value.
if any(re.search(r":\s*bool\b|Vec<Fault>|Vec<String>", l) for i, l in code):
    fail("a flag or a list of faults")

# Impossible states.
sites = [i for i, l in code if re.search(r"(?<!fn )\binternal\(", l)]
if len(sites) != 1 or not inside(sites[0], group):
    fail("`internal` is not called once, in `impl Group`")
opening = " ".join(l[3:].strip() for l in src if l.startswith("//!"))
if "`internal` call in `Group::resume`" not in opening:
    fail("the module's invariants do not list the site in `Group::resume`")
if any(re.search(r"Pending::Op|\bOp::", l) for i, l in code if inside(i, stop)):
    fail("`stop` has an arm for an operator")
if any("FaultCode" in l for i, l in code if inside(i, op) or re.search(r"\benum (Meets|Left)\b", l)):
    fail("an operator's refusal can carry a fault code")

# Each fault code is built in one place.
declared = block("pub enum FaultCode {")
for name in ("OperandExpected", "CloserExpected", "ComparisonChain", "Internal"):
    built = [i for i, l in code if "FaultCode::" + name in l and not inside(i, declared)]
    if len(built) != 1:
        fail("the fault code %s is built in %d places" % (name, len(built)))

# The group is one type with one block.
for name in ("fn open(", "fn floor(", "fn resume(", "fn close("):
    if not any(l.strip().startswith(name) for i, l in code if inside(i, group)):
        fail("`impl Group` lacks %s" % name)
named = sorted(l.strip() for i, l in code if re.search(r"\bGroup\b", l) and not inside(i, group))
expected = sorted([
    "struct Group;",
    "enum Starts { Leaf, Group, Prefix(Prefix), Nothing }",
    "Self::Open => Starts::Group,",
    "enum Nest { Group(Group) }",
    "Nest::Group(group) => group.floor(),",
    "Nest::Group(group) => group.resume(inside, at, why),",
    "Starts::Group => Group::open(after),",
])
if named != expected:
    fail("the group is named outside its block in other lines than the seven: %s" % named)
if any("Nest::" in l for i, l in code if not inside(i, nest) and not inside(i, group) and "Pending::Nest" not in l and "Under::Nest" not in l):
    fail("code outside `impl Nest` and `impl Group` names which construct is open")

# No input draws a panic: who indexes, and who makes an id.
indexes = [i for i, l in code if re.search(r"\[[^\]]*\]", l) and re.search(r"\w\[", l) and "#[" not in l]
if len(indexes) != 2 or not inside(indexes[0], read) or not inside(indexes[1], node):
    fail("indexing stands elsewhere than in `Toks::read` and `Tree::node`: lines %s" % [i + 1 for i in indexes])
for i, l in code:
    if re.search(r"\bTokId\(", l) and "struct TokId" not in l and not inside(i, toks):
        fail("a `TokId` is made outside `impl Toks`, line %d" % (i + 1))
    if re.search(r"\bNodeId\(", l) and "struct NodeId" not in l and not inside(i, push):
        if re.search(r"\(NodeId\)|Operator\(NodeId\)|Operand\(NodeId\)|Done\(NodeId\)", l):
            continue
        fail("a `NodeId` is made outside `Tree::push`, line %d" % (i + 1))

# Marks.
marks = [(i, l.strip()) for i, l in enumerate(src) if re.match(r"\s*// (SPEC §|PAGE \")", l)]
for i, mark in marks:
    if src[i + 1].strip().startswith("//"):
        fail("a mark does not stand on the line above its home: %s" % mark)
texts = [m for _, m in marks]
if len(set(texts)) != len(texts):
    fail("a mark stands twice")
if sum(1 for m in texts if m.startswith("// PAGE")) != 3:
    fail("the invariants marked at their homes are not three")
for mark, where in (
    ('// SPEC §5.1 "From loosest to tightest"', "impl Op {"),
    ('// SPEC §5.1 "Comparisons do not chain"', "Assoc::None if"),
    ('// SPEC §5.1 Primary "Name"', "Self::Name =>"),
    ('// SPEC §5.1 Primary "NumberLit"', "Self::Number =>"),
    ("// SPEC §5.1 Primary \"'(' Expr ')'\"", "impl Group {"),
    ("// SPEC §5.1 Primary", "fn no_operand("),
):
    at = [i for i, m in marks if m == mark]
    if len(at) != 1 or not src[at[0] + 1].strip().startswith(where):
        fail("the mark %s does not stand above %s" % (mark, where))

# The loops state their total.
for name in ("fn complete(tree", "fn close(tree"):
    span = block(name)
    body = src[span[0] : span[1] + 1]
    if not any(l.strip().startswith("// Total:") for l in body) or not any("scratch.turn()" in l for l in body):
        fail("%s states no total or counts no turn" % name)

print("claims: the text of the good form is as the document says")
