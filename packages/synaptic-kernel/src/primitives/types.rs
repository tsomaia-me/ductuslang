use std::sync::atomic::AtomicI32;
use std::sync::Arc;

pub type AtomicBuffer = Arc<[AtomicI32]>;
