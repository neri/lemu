//! Native / Node / browser EA+narrow-load comparison using one guest workload.
#[allow(dead_code)]
#[path = "narrow_load_encoding.rs"]
mod cases;
#[path = "../contracts/harvard.rs"]
mod contracts;
#[allow(dead_code)]
#[path = "load_extend_bench.rs"]
pub mod guards;
use plum::gpr::PartialRegister;
use plum::prelude::*;

#[unsafe(no_mangle)]
pub extern "C" fn run_narrow_load_probe() -> u32 {
    contracts::narrow_load()
}

/// 0..7 targets; 8..15 load/register guards; 16..24 existing common-dispatch guards.
#[unsafe(no_mangle)]
pub extern "C" fn bench_narrow_load(case: u32, iterations: u32) -> u32 {
    assert!(case < 25 && iterations > 0);
    if case >= 16 {
        return guards::bench_load_extend(case - 11, iterations);
    }
    let mut code = vec![
        0xb9, 0x10, 0x27, 0, 0, 0x31, 0xd2, 0xbb, 125, 0, 0, 0, 0xbe, 2, 0, 0, 0, 0xb8, 0, 0xcc,
        0xbb, 0xaa,
    ];
    let start = code.len();
    match case {
        0 => code.extend(cases::instruction(false, 0, 3, 4)),
        1 => code.extend(cases::instruction(true, 0, 3, 4)),
        2 => code.extend(cases::instruction(false, 0, 8, 129)),
        3 => code.extend(cases::instruction(true, 0, 8, 129)),
        4 => code.extend([0x3a, 0x43, 4, 0x0f, 0x92, 0xc0]), // CMP AL,[EBX+4]; SETB AL
        5 => code.extend([0x66, 0x03, 0x43, 4]),             // ADD AX,[EBX+4]
        6 => code.extend([0x0f, 0xb6, 0x43, 4]),             // MOVZX EAX,byte [EBX+4]
        7 => code.extend([0x0f, 0xbf, 0x43, 4]),             // MOVSX EAX,word [EBX+4]
        8 => code.extend(cases::instruction(false, 0, 3, 0)),
        9 => code.extend(cases::instruction(true, 0, 3, 0)),
        10 => code.extend([0x8a, 0x44, 0x73, 0]), // [EBX+ESI*2], EA=129
        11 => code.extend([0x66, 0x8b, 0x44, 0x73, 0]),
        12 => code.extend([0x8b, 0x43, 4]),
        13 => code.extend([0x8b, 0x44, 0x73, 0]),
        14 => code.extend([0x0f, 0xb6, 0xc3]), // MOVZX EAX,BL
        15 => code.extend([0x0f, 0xbf, 0xc3]), // MOVSX EAX,BX
        _ => unreachable!(),
    }
    code.extend([0x01, 0xc2, 0x49, 0x75]); // Accumulate every result; DEC ECX; JNZ
    let disp = code.len();
    code.push((start as i32 - disp as i32 - 1) as u8);
    code.extend([0xcd, 0x40, 0xeb]);
    let disp = code.len();
    code.push((-(disp as i32) - 1) as u8);
    let mut cpu = plum::ume::harvard(
        code.into_boxed_slice(),
        vec![0xa5; 4096].into_boxed_slice(),
        Offset32(0),
        Offset32(4096),
    );
    let mut checksum = 0u32;
    for _ in 0..iterations {
        assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
        checksum = checksum.wrapping_add(cpu.state().edx().read());
        cpu.resume_next();
    }
    checksum
}
