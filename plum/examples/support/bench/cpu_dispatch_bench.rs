//! Register ALU / branch / call-return guard for RMW dispatcher changes.
//! This contains no string instructions and measures the common Uop dispatcher.
use plum::gpr::PartialRegister;
use plum::prelude::*;

const FIB: &[u8] = &[
    0x55, 0x89, 0xe5, 0x53, 0x8b, 0x55, 0x08, 0x89, 0xd0, 0x83, 0xfa, 0x01, 0x7e, 0x19, 0x31, 0xdb,
    0xb9, 0x01, 0, 0, 0, 0x83, 0xfa, 0x01, 0x7e, 0x0b, 0x4a, 0x8d, 0x04, 0x19, 0x4a, 0x89, 0xcb,
    0x89, 0xc1, 0x75, 0xf6, 0x89, 0xc8, 0x5b, 0x5d, 0xc3,
];

#[unsafe(no_mangle)]
pub extern "C" fn bench_cpu_dispatch(iterations: u32) -> u32 {
    // push 10000; call _fib; add esp,4; int 0x40; jmp to the beginning.
    let mut code = vec![
        0x68, 0x10, 0x27, 0, 0, 0xe8, 8, 0, 0, 0, 0x83, 0xc4, 4, 0xcd, 0x40, 0xeb, 0xef, 0x90,
    ];
    code.extend_from_slice(FIB);
    let mut cpu = plum::ume::harvard(
        code.into_boxed_slice(),
        vec![0; 256].into_boxed_slice(),
        Offset32(0),
        Offset32(256),
    );
    let mut checksum = 0u32;
    for _ in 0..iterations {
        assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
        checksum = checksum.wrapping_add(cpu.state().eax().read());
        cpu.resume_next();
    }
    checksum
}
