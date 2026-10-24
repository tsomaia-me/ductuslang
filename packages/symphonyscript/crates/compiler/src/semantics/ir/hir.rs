use super::block::Block;
use super::body_item::BodyItem;
use super::expr::Expr;
use super::expr_kind::ExprKind;
use super::fn_decl::FnDecl;
use super::ident::Ident;
use super::ids::{BlockId, ExprId, FnId, InputId, LetId, PatternId, PlacementId, WireId};
use super::input::Input;
use super::let_decl::LetDecl;
use super::loc::Loc;
use super::pattern_decl::PatternDecl;
use super::placement::Placement;
use super::prelude_end::PreludeEnd;
use super::top::Top;
use super::wire::Wire;

#[derive(Clone, Debug, Default)]
pub(crate) struct Hir {
    pub files: Vec<String>,
    pub exprs: Vec<Expr>,
    pub blocks: Vec<Block>,
    pub placements: Vec<Placement>,
    pub wires: Vec<Wire>,
    pub inputs: Vec<Input>,
    pub lets: Vec<LetDecl>,
    pub fns: Vec<FnDecl>,
    pub patterns: Vec<PatternDecl>,
    pub top: Vec<Top>,
    pub prelude: PreludeEnd,
}

const NOWHERE: Loc = Loc {
    file: 0,
    start: 0,
    end: 0,
};

static MISSING_EXPR: Expr = Expr {
    kind: ExprKind::Error,
    loc: NOWHERE,
};

static MISSING_BLOCK: Block = Block { items: Vec::new() };

static MISSING_PLACEMENT: Placement = Placement {
    kind: None,
    flags: Vec::new(),
    name: None,
    has_as: false,
    default: None,
    gate: None,
    attrs: Vec::new(),
    body: None,
    inline: false,
    loc: NOWHERE,
};

static MISSING_WIRE: Wire = Wire {
    kind: Ident {
        text: String::new(),
        loc: NOWHERE,
    },
    flags: Vec::new(),
    name: None,
    default: None,
    gate: None,
    attrs: Vec::new(),
    destination: ExprId(u32::MAX),
    loc: NOWHERE,
};

static MISSING_INPUT: Input = Input {
    name: None,
    ty: None,
    value: ExprId(u32::MAX),
    loc: NOWHERE,
};

static MISSING_LET: LetDecl = LetDecl {
    name: None,
    ty: None,
    value: ExprId(u32::MAX),
    loc: NOWHERE,
};

static MISSING_FN: FnDecl = FnDecl {
    name: None,
    params: Vec::new(),
    lets: Vec::new(),
    value: ExprId(u32::MAX),
    loc: NOWHERE,
};

static MISSING_PATTERN: PatternDecl = PatternDecl {
    name: None,
    params: Vec::new(),
    lets: Vec::new(),
    body: BlockId(u32::MAX),
    loc: NOWHERE,
};

impl Hir {
    pub fn expr(&self, id: ExprId) -> &Expr {
        self.exprs.get(id.index()).unwrap_or(&MISSING_EXPR)
    }

    pub fn block(&self, id: BlockId) -> &Block {
        self.blocks.get(id.index()).unwrap_or(&MISSING_BLOCK)
    }

    pub fn placement(&self, id: PlacementId) -> &Placement {
        self.placements
            .get(id.index())
            .unwrap_or(&MISSING_PLACEMENT)
    }

    pub fn wire(&self, id: WireId) -> &Wire {
        self.wires.get(id.index()).unwrap_or(&MISSING_WIRE)
    }

    pub fn input(&self, id: InputId) -> &Input {
        self.inputs.get(id.index()).unwrap_or(&MISSING_INPUT)
    }

    pub fn let_decl(&self, id: LetId) -> &LetDecl {
        self.lets.get(id.index()).unwrap_or(&MISSING_LET)
    }

    pub fn fn_decl(&self, id: FnId) -> &FnDecl {
        self.fns.get(id.index()).unwrap_or(&MISSING_FN)
    }

    pub fn pattern(&self, id: PatternId) -> &PatternDecl {
        self.patterns.get(id.index()).unwrap_or(&MISSING_PATTERN)
    }

    pub fn seal_prelude(&mut self) {
        self.prelude = PreludeEnd {
            top: self.top.len(),
            files: self.files.len(),
            fns: self.fns.len(),
            patterns: self.patterns.len(),
            lets: self.lets.len(),
        };
    }

    pub fn is_prelude_top(&self, position: usize) -> bool {
        position < self.prelude.top
    }

    pub fn is_prelude_fn(&self, id: FnId) -> bool {
        id.index() < self.prelude.fns
    }

    pub fn is_prelude_pattern(&self, id: PatternId) -> bool {
        id.index() < self.prelude.patterns
    }

    pub fn is_prelude_let(&self, id: LetId) -> bool {
        id.index() < self.prelude.lets
    }

    pub fn nested_placements(&self, root: BlockId) -> Vec<PlacementId> {
        let mut found = Vec::new();
        let mut blocks = vec![root];
        while let Some(block) = blocks.pop() {
            for item in &self.block(block).items {
                match item {
                    BodyItem::Placement(id) => {
                        found.push(*id);
                        blocks.extend(self.placement(*id).body);
                    }
                    BodyItem::Wire(_) => {}
                    BodyItem::Bundle { members, each, .. } => {
                        found.extend(members.iter().copied());
                        if let Some(each) = each {
                            found.extend(each.members.iter().copied());
                        }
                    }
                    BodyItem::For { body, .. } => blocks.push(*body),
                    BodyItem::When {
                        arms, otherwise, ..
                    } => {
                        blocks.extend(arms.iter().map(|arm| arm.body));
                        blocks.extend(otherwise.iter().copied());
                    }
                }
            }
        }
        found
    }
}
