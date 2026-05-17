#[derive(Debug)]
pub enum KernelError {
    MemoryLimitExceeded,
    CapacityExhausted,
    InsufficientCapacity,
    SchemaMismatch,
}
