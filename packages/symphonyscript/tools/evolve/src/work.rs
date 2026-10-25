use allocation_counter::AllocationInfo;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Work {
    pub allocations: u64,
    pub peak_bytes: u64,
}

impl From<AllocationInfo> for Work {
    fn from(counted: AllocationInfo) -> Work {
        Work {
            allocations: counted.count_total,
            peak_bytes: counted.bytes_max,
        }
    }
}
