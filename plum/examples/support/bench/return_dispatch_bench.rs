//! Frequent RET workloads: one callsite, four sites, depth-eight recursion,
//! and sixteen sites. Used for native and Node/Wasm cache comparisons.
use plum::gpr::PartialRegister;
use plum::prelude::*;

#[unsafe(no_mangle)]
pub extern "C" fn bench_return_dispatch(kind: u32, iterations: u32) -> u32 {
    assert!(kind < 4);
    let kind = kind + 14;
    let mut code = vec![0xb9, 0x10, 0x27, 0, 0, 0x31, 0xc0];
    let start = code.len();
    let mut fixups = Vec::new();
    let sites = match kind {
        15 => 4,
        17 => 16,
        _ => 1,
    };
    for _ in 0..sites {
        if kind == 16 {
            code.extend([0xba, 8, 0, 0, 0]);
        }
        code.push(0xe8);
        fixups.push(code.len());
        code.extend([0; 4]);
    }
    code.extend([0x49, 0x75]);
    let disp = code.len();
    code.push((start as i32 - disp as i32 - 1) as u8);
    code.extend([0xcd, 0x40, 0xeb]);
    let disp = code.len();
    code.push((-(disp as i32) - 1) as u8);
    let leaf = code.len();
    if kind == 16 {
        code.extend([0x4a, 0x74, 5, 0xe8]);
        let disp = code.len();
        code.extend((leaf as i32 - disp as i32 - 4).to_le_bytes());
    }
    code.extend([0x40, 0xc3]);
    for disp in fixups {
        code[disp..disp + 4].copy_from_slice(&(leaf as i32 - disp as i32 - 4).to_le_bytes());
    }
    let mut cpu = plum::ume::harvard(
        code.into_boxed_slice(),
        vec![0; 4096].into_boxed_slice(),
        Offset32(0),
        Offset32(4096),
    );
    let mut result = 0u32;
    for _ in 0..iterations {
        assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
        result = result.wrapping_add(cpu.state().eax().read());
        cpu.resume_next();
    }
    result
}
