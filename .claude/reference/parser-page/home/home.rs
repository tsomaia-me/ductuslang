//! The peer's trial for the parser's page, question 1: a construct that the stack splits keeps
//! one home when it is one type, the entry it leaves on the stack, with one `impl` block.
//! Not part of the standard and no design: the worked example's cut, widened by level 1
//! (`if`), a group, a call's arguments, an index or slice, and an array literal.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokKind {
    Name, Number,
    Or, And, Not,
    Lt, Le, Gt, Ge,
    Fallback,
    Plus, Minus, Without,
    Star, Slash, Div, Percent,
    Pow,
    Open, Close,
    OpenSq, CloseSq,
    Comma, Assign, DotDot,
    If, Then, Else,
    End,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Level(u8);

const LOOSEST: Level = Level(1);

#[derive(Clone, Copy)]
enum Assoc { Left, Right { operand: Level }, None }

#[derive(Clone, Copy)]
struct Prefix { level: Level }

#[derive(Clone, Copy)]
struct Infix { level: Level, assoc: Assoc }

/// What a token is where an operand is expected.
enum Starts { Leaf, Group, Array, If, Prefix(Prefix), Nothing }

/// What a token is where an operator is expected.
enum Continues { Infix(Infix), Call, Index, Stops }

const fn prefix(level: u8) -> Starts {
    Starts::Prefix(Prefix { level: Level(level) })
}

const fn infix(level: u8, assoc: Assoc) -> Continues {
    Continues::Infix(Infix { level: Level(level), assoc })
}

const fn right(operand: u8) -> Assoc {
    Assoc::Right { operand: Level(operand) }
}

impl TokKind {
    const fn starts(self) -> Starts {
        match self {
            Self::Name | Self::Number => Starts::Leaf,
            Self::Open => Starts::Group,
            Self::OpenSq => Starts::Array,
            Self::If => Starts::If,
            Self::Not => prefix(4),
            Self::Minus => prefix(9),
            Self::Or | Self::And | Self::Lt | Self::Le | Self::Gt | Self::Ge | Self::Fallback
            | Self::Plus | Self::Without | Self::Star | Self::Slash | Self::Div
            | Self::Percent | Self::Pow | Self::Close | Self::CloseSq | Self::Comma
            | Self::Assign | Self::DotDot | Self::Then | Self::Else | Self::End => Starts::Nothing,
        }
    }

    const fn continues(self) -> Continues {
        match self {
            Self::Or => infix(2, Assoc::Left),
            Self::And => infix(3, Assoc::Left),
            Self::Lt | Self::Le | Self::Gt | Self::Ge => infix(5, Assoc::None),
            Self::Fallback => infix(6, right(6)),
            Self::Plus | Self::Minus | Self::Without => infix(7, Assoc::Left),
            Self::Star | Self::Slash | Self::Div | Self::Percent => infix(8, Assoc::Left),
            Self::Pow => infix(10, right(9)),
            Self::Open => Continues::Call,
            Self::OpenSq => Continues::Index,
            Self::Name | Self::Number | Self::Not | Self::Close | Self::CloseSq | Self::Comma
            | Self::Assign | Self::DotDot | Self::If | Self::Then | Self::Else | Self::End => {
                Continues::Stops
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TokId(usize);

pub struct Toks { kinds: Vec<TokKind> }

impl Toks {
    pub fn end(mut kinds: Vec<TokKind>) -> Toks {
        kinds.push(TokKind::End);
        Toks { kinds }
    }

    pub fn first(&self) -> TokId {
        TokId(0)
    }

    pub fn read(&self, at: TokId) -> (TokKind, TokId) {
        let after = if at.0 + 1 < self.kinds.len() { TokId(at.0 + 1) } else { at };
        (self.kinds[at.0], after)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NodeId(usize);

/// A run of rows in the tree's list column: the parts of one node, in source order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Parts { first: usize, len: usize }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Node {
    Leaf(TokId),
    Prefix { op: TokId, operand: NodeId },
    Infix { op: TokId, left: NodeId, right: NodeId },
    Group { inside: NodeId },
    If { condition: NodeId, then: NodeId, otherwise: NodeId },
    Call { callee: NodeId, args: Parts },
    Named { name: TokId, value: NodeId },
    Index { target: NodeId, index: NodeId },
    Slice { target: NodeId, lower: Option<NodeId>, upper: Option<NodeId> },
    Array { items: Parts },
    Fault(Fault),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fault { pub code: FaultCode, pub at: TokId }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FaultCode {
    OperandExpected,
    CloserExpected,
    ComparisonChain,
    UnparenthesizedIf,
    ThenExpected,
    ElseExpected,
    MixedArguments,
    ArgumentName,
}

pub struct Tree { nodes: Vec<Node>, lists: Vec<NodeId> }

impl Tree {
    fn push(&mut self, node: Node) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        id
    }

    fn list(&mut self, parts: &[NodeId]) -> Parts {
        let first = self.lists.len();
        self.lists.extend_from_slice(parts);
        Parts { first, len: parts.len() }
    }

    pub fn node(&self, id: NodeId) -> Node {
        self.nodes[id.0]
    }

    pub fn parts(&self, parts: Parts) -> &[NodeId] {
        &self.lists[parts.first..parts.first + parts.len]
    }
}

#[derive(Clone, Copy)]
enum Op {
    Prefix { at: TokId, row: Prefix },
    Infix { at: TokId, row: Infix, left: NodeId },
}

enum Meets { Waits, Completes, Refused(FaultCode) }

impl Op {
    fn operand(self) -> Level {
        match self {
            Op::Prefix { row, .. } => row.level,
            Op::Infix { row, .. } => match row.assoc {
                Assoc::Left | Assoc::None => Level(row.level.0 + 1),
                Assoc::Right { operand } => operand,
            },
        }
    }

    /// An open construct bounds what stands inside it, whichever construct it is: no
    /// operator reads which.
    fn floor(top: Option<&Pending>) -> Level {
        match top {
            None | Some(Pending::Open(_)) => LOOSEST,
            Some(Pending::Op(op)) => op.operand(),
        }
    }

    fn inside(level: Level, floor: Level) -> bool {
        level >= floor
    }

    fn meets(self, next: Infix) -> Meets {
        if Op::inside(next.level, self.operand()) {
            return Meets::Waits;
        }
        match self {
            Op::Prefix { .. } => Meets::Completes,
            Op::Infix { row, .. } => match row.assoc {
                Assoc::Left | Assoc::Right { .. } => Meets::Completes,
                Assoc::None if next.level == row.level => {
                    Meets::Refused(FaultCode::ComparisonChain)
                }
                Assoc::None => Meets::Completes,
            },
        }
    }

    fn over(self, operand: NodeId) -> Node {
        match self {
            Op::Prefix { at, .. } => Node::Prefix { op: at, operand },
            Op::Infix { at, left, .. } => Node::Infix { op: at, left, right: operand },
        }
    }
}

#[derive(Clone, Copy)]
enum Pending {
    Op(Op),
    Open(Open),
}

/// The constructs whose inside nests. One variant a construct, and never one a place to
/// resume at: where a construct stands in its own production is its own type's business.
#[derive(Clone, Copy)]
enum Open {
    Group(Group),
    If(IfExpr),
    Args(Args),
    Index(Index),
    Array(Array),
}

/// A construct is entered again when a part of its inside is complete, or when a token
/// arrives where a part was expected and starts none.
#[derive(Clone, Copy)]
struct Arrives {
    /// The complete part, or none when the token starts no part where one was expected.
    part: Option<NodeId>,
    at: TokId,
    kind: TokKind,
    /// The token after `at`.
    after: TokId,
}

impl Open {
    /// The chooser of open constructs: each arm is one call.
    fn resume(self, toks: &Toks, tree: &mut Tree, scratch: &mut Scratch, arrives: Arrives) -> Step {
        match self {
            Open::Group(group) => group.resume(tree, arrives),
            Open::If(if_expr) => if_expr.resume(tree, arrives),
            Open::Args(args) => args.resume(toks, tree, scratch, arrives),
            Open::Index(index) => index.resume(tree, arrives),
            Open::Array(array) => array.resume(tree, scratch, arrives),
        }
    }
}

pub struct Scratch {
    stack: Vec<Pending>,
    /// The complete parts of the open lists, innermost last.
    parts: Vec<NodeId>,
}

pub struct Expr {
    pub root: NodeId,
    pub next: TokId,
}

#[derive(Clone, Copy)]
enum Expect { Operand, Operator(NodeId) }

enum Step {
    /// An entry for the stack; the token is taken. An operand is expected next.
    Push(Pending),
    /// A complete operand; the token is taken. An operator is expected next.
    Operand(NodeId),
    /// A complete operand that ends before this token. The token is offered again.
    Before(NodeId),
    /// The expression's root. It ends before this token.
    Done(NodeId),
    Fault(Fault),
}

fn fault(code: FaultCode, at: TokId) -> Step {
    Step::Fault(Fault { code, at })
}

pub fn expression(toks: &Toks, from: TokId, tree: &mut Tree, scratch: &mut Scratch) -> Expr {
    let mut at = from;
    let mut expect = Expect::Operand;
    loop {
        let (kind, after) = toks.read(at);
        let step = match expect {
            Expect::Operand => match kind.starts() {
                Starts::Leaf => leaf(tree, at),
                Starts::Group => Group::open(),
                Starts::Array => Array::open(scratch),
                Starts::If => IfExpr::open(scratch, at),
                Starts::Prefix(row) => prefix_operator(scratch, at, row),
                Starts::Nothing => no_part(toks, tree, scratch, at, kind, after),
            },
            Expect::Operator(operand) => match kind.continues() {
                Continues::Infix(row) => infix_operator(tree, scratch, operand, at, row),
                Continues::Call => Args::open(scratch, operand, after),
                Continues::Index => Index::open(operand),
                Continues::Stops => stop(toks, tree, scratch, operand, at, kind, after),
            },
        };
        match step {
            Step::Push(entry) => {
                scratch.stack.push(entry);
                expect = Expect::Operand;
                at = after;
            }
            Step::Operand(node) => {
                expect = Expect::Operator(node);
                at = after;
            }
            Step::Before(node) => expect = Expect::Operator(node),
            Step::Done(root) => return Expr { root, next: at },
            Step::Fault(fault) => {
                scratch.stack.clear();
                scratch.parts.clear();
                return Expr { root: tree.push(Node::Fault(fault)), next: at };
            }
        }
    }
}

fn leaf(tree: &mut Tree, at: TokId) -> Step {
    Step::Operand(tree.push(Node::Leaf(at)))
}

fn prefix_operator(scratch: &Scratch, at: TokId, row: Prefix) -> Step {
    if Op::inside(row.level, Op::floor(scratch.stack.last())) {
        Step::Push(Pending::Op(Op::Prefix { at, row }))
    } else {
        fault(FaultCode::OperandExpected, at)
    }
}

fn infix_operator(
    tree: &mut Tree,
    scratch: &mut Scratch,
    operand: NodeId,
    at: TokId,
    row: Infix,
) -> Step {
    match complete(tree, scratch, operand, row) {
        Ok(left) => Step::Push(Pending::Op(Op::Infix { at, row, left })),
        Err(code) => fault(code, at),
    }
}

/// A token that starts no operand arrives where one is expected. It is the innermost open
/// construct's to take, when that construct is what expects the operand.
fn no_part(
    toks: &Toks,
    tree: &mut Tree,
    scratch: &mut Scratch,
    at: TokId,
    kind: TokKind,
    after: TokId,
) -> Step {
    match scratch.stack.last() {
        Some(&Pending::Open(open)) => {
            scratch.stack.pop();
            open.resume(toks, tree, scratch, Arrives { part: None, at, kind, after })
        }
        Some(Pending::Op(_)) | None => fault(FaultCode::OperandExpected, at),
    }
}

/// A token that continues no operand arrives after one. The pending operators complete, and
/// the operand is a part of the innermost open construct, or the expression's root.
fn stop(
    toks: &Toks,
    tree: &mut Tree,
    scratch: &mut Scratch,
    mut operand: NodeId,
    at: TokId,
    kind: TokKind,
    after: TokId,
) -> Step {
    loop {
        match scratch.stack.pop() {
            Some(Pending::Op(top)) => operand = tree.push(top.over(operand)),
            Some(Pending::Open(open)) => {
                let arrives = Arrives { part: Some(operand), at, kind, after };
                return open.resume(toks, tree, scratch, arrives);
            }
            None => return Step::Done(operand),
        }
    }
}

fn complete(
    tree: &mut Tree,
    scratch: &mut Scratch,
    mut operand: NodeId,
    next: Infix,
) -> Result<NodeId, FaultCode> {
    loop {
        let top = match scratch.stack.last() {
            Some(&Pending::Op(top)) => top,
            Some(Pending::Open(_)) | None => return Ok(operand),
        };
        match top.meets(next) {
            Meets::Waits => return Ok(operand),
            Meets::Refused(code) => return Err(code),
            Meets::Completes => {}
        }
        scratch.stack.pop();
        operand = tree.push(top.over(operand));
    }
}

// SPEC §5.1 Primary "'(' Expr ')'"
#[derive(Clone, Copy)]
struct Group;

impl Group {
    fn open() -> Step {
        Step::Push(Pending::Open(Open::Group(Group)))
    }

    fn resume(self, tree: &mut Tree, arrives: Arrives) -> Step {
        let Some(inside) = arrives.part else {
            return fault(FaultCode::OperandExpected, arrives.at);
        };
        if arrives.kind == TokKind::Close {
            Step::Operand(tree.push(Node::Group { inside }))
        } else {
            fault(FaultCode::CloserExpected, arrives.at)
        }
    }
}

// SPEC §5.1 IfExpr
#[derive(Clone, Copy)]
enum IfExpr {
    Condition,
    Then { condition: NodeId },
    Else { condition: NodeId, then: NodeId },
}

impl IfExpr {
    // SPEC §5.1 "An `if` that is an operand of another operator is parenthesized"
    fn open(scratch: &Scratch, at: TokId) -> Step {
        if Op::inside(LOOSEST, Op::floor(scratch.stack.last())) {
            Step::Push(Pending::Open(Open::If(IfExpr::Condition)))
        } else {
            fault(FaultCode::UnparenthesizedIf, at)
        }
    }

    fn resume(self, tree: &mut Tree, arrives: Arrives) -> Step {
        let Some(part) = arrives.part else {
            return fault(FaultCode::OperandExpected, arrives.at);
        };
        match self {
            IfExpr::Condition => {
                if arrives.kind == TokKind::Then {
                    Step::Push(Pending::Open(Open::If(IfExpr::Then { condition: part })))
                } else {
                    fault(FaultCode::ThenExpected, arrives.at)
                }
            }
            IfExpr::Then { condition } => {
                if arrives.kind == TokKind::Else {
                    Step::Push(Pending::Open(Open::If(IfExpr::Else { condition, then: part })))
                } else {
                    fault(FaultCode::ElseExpected, arrives.at)
                }
            }
            IfExpr::Else { condition, then } => {
                Step::Before(tree.push(Node::If { condition, then, otherwise: part }))
            }
        }
    }
}

// SPEC §5.1 Postfix "'(' Args? ')'"
#[derive(Clone, Copy)]
struct Args {
    callee: NodeId,
    /// Where this list's parts start in `Scratch::parts`.
    first: usize,
    /// The first token of the argument being read.
    item: TokId,
    /// The name of the argument being read, when it has one.
    name: Option<TokId>,
    /// Whether the arguments before this one were named. None before the first.
    named: Option<bool>,
}

impl Args {
    fn open(scratch: &Scratch, callee: NodeId, item: TokId) -> Step {
        let args = Args { callee, first: scratch.parts.len(), item, name: None, named: None };
        Step::Push(Pending::Open(Open::Args(args)))
    }

    fn resume(mut self, toks: &Toks, tree: &mut Tree, scratch: &mut Scratch, arrives: Arrives) -> Step {
        let Arrives { part, at, kind, after } = arrives;
        let Some(part) = part else {
            // `f()`
            let empty = scratch.parts.len() == self.first && self.name.is_none();
            return if empty && kind == TokKind::Close {
                self.close(tree, scratch)
            } else {
                fault(FaultCode::OperandExpected, at)
            };
        };
        // SPEC §5.1 NamedArg
        if kind == TokKind::Assign {
            let is_name = toks.read(self.item).0 == TokKind::Name;
            if self.name.is_some() || !is_name || tree.node(part) != Node::Leaf(self.item) {
                return fault(FaultCode::ArgumentName, at);
            }
            self.name = Some(self.item);
            return Step::Push(Pending::Open(Open::Args(self)));
        }
        if kind != TokKind::Comma && kind != TokKind::Close {
            return fault(FaultCode::CloserExpected, at);
        }
        // SPEC §5.7 "mixing them in one call is an error"
        let named = self.name.is_some();
        if self.named.is_some_and(|before| before != named) {
            return fault(FaultCode::MixedArguments, at);
        }
        let arg = match self.name {
            Some(name) => tree.push(Node::Named { name, value: part }),
            None => part,
        };
        scratch.parts.push(arg);
        if kind == TokKind::Close {
            return self.close(tree, scratch);
        }
        self.item = after;
        self.name = None;
        self.named = Some(named);
        Step::Push(Pending::Open(Open::Args(self)))
    }

    fn close(self, tree: &mut Tree, scratch: &mut Scratch) -> Step {
        let args = tree.list(&scratch.parts[self.first..]);
        scratch.parts.truncate(self.first);
        Step::Operand(tree.push(Node::Call { callee: self.callee, args }))
    }
}

// SPEC §5.5 Index
#[derive(Clone, Copy)]
enum Index {
    /// After `[`: a bound or `..` comes.
    First { target: NodeId },
    /// After `..`: a bound or `]` comes.
    Upper { target: NodeId, lower: Option<NodeId> },
}

impl Index {
    fn open(target: NodeId) -> Step {
        Step::Push(Pending::Open(Open::Index(Index::First { target })))
    }

    fn resume(self, tree: &mut Tree, arrives: Arrives) -> Step {
        let Arrives { part, at, kind, .. } = arrives;
        match self {
            Index::First { target } => {
                if kind == TokKind::DotDot {
                    return Step::Push(Pending::Open(Open::Index(Index::Upper { target, lower: part })));
                }
                let Some(index) = part else {
                    return fault(FaultCode::OperandExpected, at);
                };
                if kind == TokKind::CloseSq {
                    Step::Operand(tree.push(Node::Index { target, index }))
                } else {
                    fault(FaultCode::CloserExpected, at)
                }
            }
            Index::Upper { target, lower } => {
                if kind == TokKind::CloseSq {
                    Step::Operand(tree.push(Node::Slice { target, lower, upper: part }))
                } else if part.is_some() {
                    fault(FaultCode::CloserExpected, at)
                } else {
                    fault(FaultCode::OperandExpected, at)
                }
            }
        }
    }
}

// SPEC §5.1 ArrayLit
#[derive(Clone, Copy)]
struct Array {
    /// Where this list's parts start in `Scratch::parts`.
    first: usize,
}

impl Array {
    fn open(scratch: &Scratch) -> Step {
        Step::Push(Pending::Open(Open::Array(Array { first: scratch.parts.len() })))
    }

    fn resume(self, tree: &mut Tree, scratch: &mut Scratch, arrives: Arrives) -> Step {
        let Arrives { part, at, kind, .. } = arrives;
        let Some(item) = part else {
            // `[]`, and the comma that may end the list.
            return if kind == TokKind::CloseSq {
                self.close(tree, scratch)
            } else {
                fault(FaultCode::OperandExpected, at)
            };
        };
        if kind != TokKind::Comma && kind != TokKind::CloseSq {
            return fault(FaultCode::CloserExpected, at);
        }
        scratch.parts.push(item);
        if kind == TokKind::CloseSq {
            self.close(tree, scratch)
        } else {
            Step::Push(Pending::Open(Open::Array(self)))
        }
    }

    fn close(self, tree: &mut Tree, scratch: &mut Scratch) -> Step {
        let items = tree.list(&scratch.parts[self.first..]);
        scratch.parts.truncate(self.first);
        Step::Operand(tree.push(Node::Array { items }))
    }
}
