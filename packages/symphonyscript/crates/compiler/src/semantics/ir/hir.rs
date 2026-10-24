use super::block::Block;
use super::body_item::BodyItem;
use super::expr::Expr;
use super::fn_decl::FnDecl;
use super::ids::{BlockId, ExprId, FnId, InputId, LetId, PatternId, PlacementId, WireId};
use super::input::Input;
use super::let_decl::LetDecl;
use super::pattern_decl::PatternDecl;
use super::placement::Placement;
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

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PreludeEnd {
    pub top: usize,
    pub files: usize,
    pub fns: usize,
    pub patterns: usize,
    pub lets: usize,
}

impl Hir {
    pub fn expr(&self, id: ExprId) -> &Expr {
        &self.exprs[id.index()]
    }

    pub fn block(&self, id: BlockId) -> &Block {
        &self.blocks[id.index()]
    }

    pub fn placement(&self, id: PlacementId) -> &Placement {
        &self.placements[id.index()]
    }

    pub fn wire(&self, id: WireId) -> &Wire {
        &self.wires[id.index()]
    }

    pub fn input(&self, id: InputId) -> &Input {
        &self.inputs[id.index()]
    }

    pub fn let_decl(&self, id: LetId) -> &LetDecl {
        &self.lets[id.index()]
    }

    pub fn fn_decl(&self, id: FnId) -> &FnDecl {
        &self.fns[id.index()]
    }

    pub fn pattern(&self, id: PatternId) -> &PatternDecl {
        &self.patterns[id.index()]
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
