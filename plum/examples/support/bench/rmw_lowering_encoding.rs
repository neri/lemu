//! Guest instruction construction for timing workloads.
pub const CASES: u32 = 119;
pub fn width(case: u32) -> usize {
    match case {
        0..42 => [1, 2, 4][((case / 2) % 3) as usize],
        42..54 => [1, 2, 4][((case - 42) % 3) as usize],
        54..96 => [1, 2, 4][(((case - 54) / 2) % 3) as usize],
        96..104 => [2, 4][(((case - 96) / 2) % 2) as usize],
        104..116 => [2, 4][(((case - 104) / 2) % 2) as usize],
        _ => [1, 2, 4][((case - 116) % 3) as usize],
    }
}
/// tail is the ModRM r/m operand (and its optional SIB/disp), with reg bits zero.
pub fn instruction(case: u32, tail: &[u8], source: u8, immediate: u32) -> Vec<u8> {
    let n = width(case);
    let mut bytes = Vec::new();
    if n == 2 {
        bytes.push(0x66);
    }
    let mut operand = tail.to_vec();
    let field;
    match case {
        0..42 => {
            let op = case / 6;
            if case % 2 == 0 {
                bytes.push((op as u8) * 8 + u8::from(n != 1));
                field = source;
            } else {
                bytes.push(if n == 1 { 0x80 } else { 0x81 });
                field = op as u8;
            }
        }
        42..54 => {
            let op = (case - 42) / 3;
            bytes.push(if op < 2 {
                if n == 1 { 0xfe } else { 0xff }
            } else if n == 1 {
                0xf6
            } else {
                0xf7
            });
            field = [0, 1, 3, 2][op as usize]; // INC DEC NEG NOT
        }
        54..96 => {
            let op = (case - 54) / 6;
            bytes.push(if case % 2 == 0 {
                if n == 1 { 0xd2 } else { 0xd3 }
            } else if n == 1 {
                0xc0
            } else {
                0xc1
            });
            field = [4, 5, 7, 0, 1, 2, 3][op as usize];
        }
        96..104 => {
            let op = (case - 96) / 4;
            bytes.extend([0x0f, [0xa5, 0xad][op as usize] - u8::from(case % 2 == 1)]);
            field = source;
        }
        104..116 => {
            let op = (case - 104) / 4;
            bytes.extend([
                0x0f,
                if case % 2 == 0 {
                    [0xab, 0xb3, 0xbb][op as usize]
                } else {
                    0xba
                },
            ]);
            field = if case % 2 == 0 { source } else { 5 + op as u8 };
        }
        _ => {
            bytes.push(if n == 1 { 0x86 } else { 0x87 });
            field = source;
        }
    }
    operand[0] |= field << 3;
    bytes.extend(operand);
    if case < 42 && case % 2 == 1 {
        bytes.extend(&immediate.to_le_bytes()[..n]);
    } else if (54..116).contains(&case) && case % 2 == 1 {
        bytes.push(immediate as u8);
    }
    bytes
}
