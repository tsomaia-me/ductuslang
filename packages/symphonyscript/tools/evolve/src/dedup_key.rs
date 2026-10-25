use crate::program::Program;
use crate::signal::Signal;

const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

pub fn dedup_key(signal: Signal, codes: &[String], shrunk: &Program) -> String {
    let mut sorted = codes.to_vec();
    sorted.sort();
    sorted.dedup();
    let mut hash = FNV_OFFSET;
    for source in shrunk.ordered_sources() {
        for byte in source
            .path
            .bytes()
            .chain([0])
            .chain(source.text.bytes())
            .chain([0])
        {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    format!("{signal}|{}|{hash:016x}", sorted.join(","))
}
