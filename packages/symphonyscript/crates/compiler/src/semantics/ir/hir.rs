use super::block::Block;
use super::body_item::BodyItem;
use super::expr::Expr;
use super::fn_decl::FnDecl;
use super::ids::{BlockId, ExprId, FnId, InputId, LetId, PatternId, PlacementId, WireId};
use super::input::Input;
use super::let_decl::LetDecl;
use super::pattern_decl::PatternDecl;
use super::placement::Placement;
use super::prelude_end::PreludeEnd;
use super::top::Top;
use super::unknown_names::UnknownNames;
use super::wire::Wire;
use crate::semantics::Internal;

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
    pub skipped_top: UnknownNames,
    pub prelude: PreludeEnd,
}

impl Hir {
    pub fn expr(&self, id: ExprId) -> Result<&Expr, Internal> {
        self.exprs
            .get(id.index())
            .ok_or(Internal::missed("expression"))
    }

    pub fn block(&self, id: BlockId) -> Result<&Block, Internal> {
        self.blocks.get(id.index()).ok_or(Internal::missed("block"))
    }

    pub fn placement(&self, id: PlacementId) -> Result<&Placement, Internal> {
        self.placements
            .get(id.index())
            .ok_or(Internal::missed("placement"))
    }

    pub fn wire(&self, id: WireId) -> Result<&Wire, Internal> {
        self.wires.get(id.index()).ok_or(Internal::missed("wire"))
    }

    pub fn input(&self, id: InputId) -> Result<&Input, Internal> {
        self.inputs.get(id.index()).ok_or(Internal::missed("input"))
    }

    pub fn let_decl(&self, id: LetId) -> Result<&LetDecl, Internal> {
        self.lets.get(id.index()).ok_or(Internal::missed("let"))
    }

    pub fn fn_decl(&self, id: FnId) -> Result<&FnDecl, Internal> {
        self.fns.get(id.index()).ok_or(Internal::missed("function"))
    }

    pub fn pattern(&self, id: PatternId) -> Result<&PatternDecl, Internal> {
        self.patterns
            .get(id.index())
            .ok_or(Internal::missed("pattern"))
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

    pub fn body_unknown(&self, root: BlockId) -> Result<UnknownNames, Internal> {
        let mut unknown = UnknownNames::default();
        let mut blocks = vec![root];
        while let Some(block) = blocks.pop() {
            let block = self.block(block)?;
            unknown.join(&block.unknown);
            for item in &block.items {
                match item {
                    BodyItem::When {
                        arms, otherwise, ..
                    } => {
                        blocks.extend(arms.iter().map(|arm| arm.body));
                        blocks.extend(otherwise.iter().copied());
                    }
                    BodyItem::Placement(_)
                    | BodyItem::Wire(_)
                    | BodyItem::Bundle { .. }
                    | BodyItem::For { .. } => {}
                }
            }
        }
        Ok(unknown)
    }

    pub fn nested_placements(&self, root: BlockId) -> Result<Vec<PlacementId>, Internal> {
        let mut found = Vec::new();
        let mut blocks = vec![root];
        while let Some(block) = blocks.pop() {
            for item in &self.block(block)?.items {
                match item {
                    BodyItem::Placement(id) => {
                        found.push(*id);
                        blocks.extend(self.placement(*id)?.body);
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
        Ok(found)
    }
}
