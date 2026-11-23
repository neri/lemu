//! Shared native / Wasm semantic probe and load-extension / dispatch guards.
#[allow(dead_code)]
#[path = "load_extend_encoding.rs"]
mod cases;
#[path = "../contracts/harvard.rs"]
mod contracts;
#[allow(dead_code)]
#[path = "cpu_dispatch_bench.rs"]
mod cpu_guard;
#[allow(dead_code)]
#[path = "return_dispatch_bench.rs"]
mod return_guard;
#[allow(dead_code)]
#[path = "string_bench.rs"]
mod string_guard;
use plum::gpr::PartialRegister;
use plum::prelude::*;

#[unsafe(no_mangle)]
pub extern "C" fn run_load_extend_probe() -> u32 {
    contracts::load_extend()
}
#[unsafe(no_mangle)]
pub extern "C" fn load_extend_uop_size() -> u32 {
    size_of::<plum::harvard::uop::Uop>() as u32
}
#[unsafe(no_mangle)]
pub extern "C" fn load_extend_minor_size() -> u32 {
    size_of::<plum::harvard::uop::UopMinor>() as u32
}

/// 0..4: four extensions and mixed; 5: ALU; 6: memory ADD; 7: fib;
/// 8..11: RET variants; 12: STOS; 13: SCAS. Returns a guest checksum.
#[unsafe(no_mangle)]
pub extern "C" fn bench_load_extend(case: u32, iterations: u32) -> u32 {
    assert!(case < 14 && iterations > 0);
    match case {
        7 => return cpu_guard::bench_cpu_dispatch(iterations),
        8..=11 => return return_guard::bench_return_dispatch(case - 8, iterations),
        12 => return string_guard::bench_string(1, 4, 65536, 0, 0, 0, iterations),
        13 => return string_guard::bench_string(4, 1, 65536, 0, 0, 0, iterations),
        _ => {}
    }
    // ECX=10000; EDX=0; EBX=129; EAX=1. All memory loads have a
    // plain base and an unaligned address; their results feed a guest sum.
    let mut code = vec![
        0xb9, 0x10, 0x27, 0, 0, 0x31, 0xd2, 0xbb, 129, 0, 0, 0, 0xb8, 1, 0, 0, 0,
    ];
    let start = code.len();
    if case < 5 {
        for kind in 0..4 {
            if case == 4 || case == kind {
                code.extend(cases::instruction(kind as u8, 0, 3));
                code.extend([0x01, 0xc2]); // ADD EDX,EAX consumes each loaded value.
            }
        }
    } else if case == 5 {
        code.extend([0x83, 0xc0, 3, 0xc1, 0xc0, 7, 0x35, 0x21, 0x43, 0x65, 0x87]);
    } else {
        code.extend([0x01, 0x03, 0x03, 0x13]); // ADD [EBX],EAX; ADD EDX,[EBX]
    }
    code.extend([0x49, 0x75]);
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
        checksum = checksum.wrapping_add(cpu.state().edx().read() ^ cpu.state().eax().read());
        cpu.resume_next();
    }
    checksum
}
