//! Guest instruction construction for timing workloads.
pub fn instruction(kind: u8, rd: u8, base: u8) -> Vec<u8> {
    let mut bytes = vec![0x0f, [0xb6, 0xb7, 0xbe, 0xbf][kind as usize]];
    bytes.push((rd << 3) | base | if base == 5 { 0x40 } else { 0 });
    if base == 4 {
        bytes.push(0x24);
    } // ESP encoding is normalized to a plain decoded base.
    if base == 5 {
        bytes.push(0);
    } // [EBP+0] is a plain decoded base.
    bytes
}
