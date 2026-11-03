use super::NONE;
use super::flat_ir::FlatIr;
use super::item_rec::ItemRec;

impl FlatIr {
    pub fn named(&self, name: u32) -> bool {
        self.name(name).is_some_and(|entry| !entry.faulty)
    }

    pub fn dropped(&self, item: u32) -> bool {
        match self.items.get(item as usize) {
            None => true,
            Some(ItemRec::Placement(placement)) => !self.named(placement.kind),
            Some(ItemRec::Wire(wire)) => !self.named(wire.kind),
            Some(ItemRec::Paren(inner)) => *inner == NONE || self.dropped(*inner),
            Some(ItemRec::Line(members)) => (members.0..members.1).any(|at| {
                self.members
                    .get(at as usize)
                    .is_none_or(|member| self.dropped(*member))
            }),
            Some(ItemRec::Bundle(bundle)) => {
                bundle.start_touched
                    || self.members_fail(bundle.members)
                    || bundle
                        .each
                        .is_some_and(|each| self.members_fail(each.members))
            }
            Some(ItemRec::For(_) | ItemRec::When(_)) => false,
        }
    }

    pub fn members_fail(&self, members: (u32, u32)) -> bool {
        (members.0..members.1).any(|at| {
            let member = self.members.get(at as usize).copied().unwrap_or(NONE);
            match self.items.get(member as usize) {
                Some(ItemRec::Bundle(_)) | None => true,
                Some(ItemRec::Paren(inner)) => *inner == NONE || self.dropped(*inner),
                Some(_) => self.dropped(member),
            }
        })
    }
}
