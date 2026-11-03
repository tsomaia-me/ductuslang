use std::hash::Hasher;
use std::sync::Arc;

use super::sym::Sym;
use super::symbol_table::SymbolTable;
use crate::semantics::analysis::fast_hash::FastHasher;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Symbols {
    frozen: Option<Arc<SymbolTable>>,
    own: SymbolTable,
}

impl Symbols {
    pub fn freeze(&mut self) {
        let first = self.own.end();
        let own = std::mem::replace(&mut self.own, SymbolTable::starting_at(first));
        self.frozen = Some(Arc::new(own));
    }

    pub fn layer(&self) -> Self {
        match &self.frozen {
            Some(frozen) => Self {
                frozen: Some(Arc::clone(frozen)),
                own: SymbolTable::starting_at(frozen.end()),
            },
            None => Self::default(),
        }
    }

    pub fn intern(&mut self, text: &str) -> Sym {
        let mut hasher = FastHasher::default();
        hasher.write(text.as_bytes());
        let hash = hasher.finish();
        if let Some(sym) = self
            .frozen
            .as_ref()
            .and_then(|frozen| frozen.find(text, hash))
        {
            return Sym(sym);
        }
        if let Some(sym) = self.own.find(text, hash) {
            return Sym(sym);
        }
        Sym(self.own.insert(text, hash))
    }

    pub fn text(&self, sym: Sym) -> &str {
        if sym.0 < self.own.first() {
            return self
                .frozen
                .as_ref()
                .and_then(|frozen| frozen.text(sym.0))
                .unwrap_or_default();
        }
        self.own.text(sym.0).unwrap_or_default()
    }
}
