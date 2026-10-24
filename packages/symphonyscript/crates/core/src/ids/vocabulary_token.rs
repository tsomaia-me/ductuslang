use core::sync::atomic::{AtomicU32, Ordering};

static NEXT: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct VocabularyToken(u32);

impl VocabularyToken {
    pub(crate) fn fresh() -> Self {
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}
