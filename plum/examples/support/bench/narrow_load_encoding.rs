//! Guest instruction construction for timing workloads.
/// base=8 means absolute address; otherwise select disp8 or disp32 explicitly.
pub fn instruction(word: bool, rd: u8, base: u8, disp: u32) -> Vec<u8> {
    let mut code = if word { vec![0x66] } else { vec![] };
    code.push(if word { 0x8b } else { 0x8a });
    if base == 8 {
        code.push((rd << 3) | 5);
        code.extend(disp.to_le_bytes());
    } else {
        let short = (disp as i32) == (disp as i8 as i32);
        let mode = if disp == 0 && base != 5 {
            0
        } else if short {
            0x40
        } else {
            0x80
        };
        code.push(mode | (rd << 3) | base);
        if base == 4 {
            code.push(0x24);
        }
        if mode == 0x40 {
            code.push(disp as u8);
        }
        if mode == 0x80 {
            code.extend(disp.to_le_bytes());
        }
    }
    code
}
