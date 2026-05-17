/// "SYCT" in ASCII - identifies valid ControlPlane in memory.
pub const CONTROLLER_MAGIC: u32 = 0x53594354;

/// "SYSC" in ASCII - identifies valid SymphonyScript Kernel in memory.
pub const KERNEL_MAGIC: i32 = 0x53595343;

/// Kernel binary protocol version. Checked on `bind()` to reject version mismatches.
pub const KERNEL_VERSION: i32 = 0x01;

/// Fixed structural slot width for graph node (i32 count, including 1 reserved)
pub const NODE_STRIDE: usize = 8;

/// Fixed structural slot width for graph synapse (i32 count, including 1 reserved)
pub const SYNAPSE_STRIDE: usize = 8;
