use crate::errors::ring_buffer_error::RingBufferError;

#[derive(Debug)]
pub enum SlotAllocatorError {
    InvalidSlot,
    DoubleFree,
    RingBuffer(RingBufferError),
}

impl From<RingBufferError> for SlotAllocatorError {
    fn from(value: RingBufferError) -> Self {
        SlotAllocatorError::RingBuffer(value)
    }
}
