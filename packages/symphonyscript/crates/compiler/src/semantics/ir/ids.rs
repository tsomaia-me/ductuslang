macro_rules! index_type {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
        pub(crate) struct $name(pub u32);

        impl $name {
            pub fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}

pub(crate) use index_type;

index_type!(ExprId);
index_type!(BlockId);
index_type!(PlacementId);
index_type!(WireId);
index_type!(InputId);
index_type!(LetId);
index_type!(FnId);
index_type!(PatternId);
