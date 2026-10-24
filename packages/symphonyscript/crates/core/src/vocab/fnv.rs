pub(crate) struct Fnv(u64);

impl Fnv {
    pub(crate) fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    pub(crate) fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    pub(crate) fn byte(&mut self, byte: u8) {
        self.bytes(&[byte]);
    }

    pub(crate) fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    pub(crate) fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    pub(crate) fn len(&mut self, len: usize) {
        self.u32(len as u32);
    }

    pub(crate) fn str(&mut self, text: &str) {
        self.len(text.len());
        self.bytes(text.as_bytes());
    }

    pub(crate) fn finish(&self) -> u64 {
        self.0
    }
}
