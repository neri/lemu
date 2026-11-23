//! Native / Node-Wasm benchmark for the same RMW operand matrix as the tests.
//! Build Wasm with opt-level=s, LTO, codegen-units=1 and panic=abort.
#[allow(dead_code)]
#[path = "support/bench/rmw_lowering_encoding.rs"]
mod cases;
#[path = "support/contracts/harvard.rs"]
mod contracts;
use plum::gpr::PartialRegister;
use plum::harvard::uop::{Uop, UopMinor};
use plum::prelude::*;

#[unsafe(no_mangle)]
pub extern "C" fn run_rmw_probe() -> u32 {
    contracts::rmw()
}
#[unsafe(no_mangle)]
pub extern "C" fn uop_size() -> u32 {
    size_of::<Uop>() as u32
}
#[unsafe(no_mangle)]
pub extern "C" fn minor_size() -> u32 {
    size_of::<UopMinor>() as u32
}

/// scenario: aligned base, unaligned base, streaming, base+disp, SIB, disp-only.
#[unsafe(no_mangle)]
pub extern "C" fn bench_rmw(case: u32, scenario: u32, iterations: u32) -> u32 {
    assert!(case < cases::CASES && scenario < 6 && iterations > 0);
    // Source EBX is restored each loop because XCHG returns the old memory value.
    let mut code = vec![0xba, 0x88, 0x13, 0, 0, 0xb8]; // EDX=5000, EAX=address
    code.extend((128u32 + u32::from(scenario == 1)).to_le_bytes());
    code.extend([0xb9, 3, 0, 0, 0, 0xbe, 1, 0, 0, 0]); // ECX=count, ESI=index
    let start = code.len();
    code.extend([0xbb, 3, 0, 0, 0]);
    let tail = match scenario {
        3 => vec![0x40, 4],
        4 => vec![0x44, 0x70, 4],
        5 => vec![5, 128, 0, 0, 0],
        _ => vec![0],
    };
    code.extend(cases::instruction(case, &tail, 3, 3));
    if scenario == 2 {
        code.extend([0x8d, 0x40, 4, 0x25, 0xfc, 0x3f, 0, 0]);
    }
    code.extend([0x4a, 0x75]);
    let branch = code.len();
    code.push((start as i32 - branch as i32 - 1) as u8);
    code.extend([0xcd, 0x40, 0xeb]);
    let branch = code.len();
    code.push((-(branch as i32) - 1) as u8);
    let mut cpu = plum::ume::harvard(
        code.into_boxed_slice(),
        vec![0x5a; 16384].into_boxed_slice(),
        Offset32(0),
        Offset32(16384),
    );
    for _ in 0..iterations {
        assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
        assert_eq!(cpu.state().edx().read(), 0);
        cpu.resume_next();
    }
    cpu.data().chunks_exact(4).fold(0u32, |s, b| {
        s.wrapping_add(u32::from_le_bytes(b.try_into().unwrap()))
    }) ^ cpu.state_mut().flags_mut().resolve().bits()
}
fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<_> = std::env::args()
            .skip(1)
            .map(|s| s.parse::<u32>().unwrap())
            .collect();
        let selected = args.first().copied();
        for case in 0..cases::CASES {
            if selected.is_some_and(|c| c != case) {
                continue;
            }
            for scenario in 0..6 {
                let expected = bench_rmw(case, scenario, 8);
                for _ in 0..3 {
                    assert_eq!(bench_rmw(case, scenario, 8), expected);
                }
                let mut samples = Vec::new();
                for _ in 0..9 {
                    let start = std::time::Instant::now();
                    assert_eq!(bench_rmw(case, scenario, 8), expected);
                    samples.push(start.elapsed().as_secs_f64() * 1000.);
                }
                samples.sort_by(f64::total_cmp);
                println!("{case},{scenario},{expected},{:.6},{samples:?}", samples[4]);
            }
        }
    }
}
