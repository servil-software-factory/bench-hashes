/// A dirty tree's fingerprint, as its provenance records it: `dirty-` and
/// the BLAKE3 hash of its changes written as 64 lowercase hex characters.
/// build.rs writes it, and tests check it, through this one function
/// (`include!`d by both).
pub fn fingerprint(hash: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity("dirty-".len() + hash.len() * 2);
    out.push_str("dirty-");
    for &byte in hash {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
