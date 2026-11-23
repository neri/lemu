//! SSE workloads (examples/support/simd/bench.s) on HarvardUME and FlatUME.
//! Prints the median of 5 timed runs per engine and workload, in ns per
//! guest SIMD instruction, and a checksum of the results.
//!
//! cargo run -p plum --release --features sse --example simd_bench [loops]
#[macro_use]
#[path = "support/guest_bytes.rs"]
mod guest_bytes;
use std::time::Instant;

use plum::cpu::{ProcessorState, RtReg};
use plum::gpr::PartialRegister;
use plum::prelude::*;

const CODE: usize = 0x1000;
const LEN: usize = 0x20000;
const GUEST: &[u8] = guest_bytes!("support/simd/bench.bin.hex");
const NAMES: [&str; 3] = ["int", "fp", "mmx"];
/// SIMD instructions per inner iteration and inner iterations per loop.
const SIMD: [(u64, u64); 3] = [(12, 256), (10, 256), (10, 512)];

fn data(loops: u32) -> Vec<u8> {
    let mut m: Vec<u8> = (0..LEN).map(|i| (i * 37 ^ (i >> 3)) as u8).collect();
    m[0x3000..0x3004].copy_from_slice(&loops.to_le_bytes());
    // Floating-point inputs: small positive floats at 0x10000.., constants.
    for i in 0..0x1000 {
        let f = 1.0f32 + (i % 97) as f32 / 8.0;
        m[0x10000 + i * 4..0x10004 + i * 4].copy_from_slice(&f.to_le_bytes());
    }
    for (at, f) in [(0x3040, 1.5f32)] {
        for lane in 0..4 {
            m[at + lane * 4..at + lane * 4 + 4].copy_from_slice(&f.to_le_bytes());
        }
    }
    for lane in 0..4 {
        m[0x3050 + lane * 4..0x3054 + lane * 4].copy_from_slice(&0x7fff_ffffu32.to_le_bytes());
    }
    m
}

fn digest(state: &ProcessorState, memory: &[u8]) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    let mut add = |bytes: &[u8]| {
        for &b in bytes {
            hash = (hash ^ u32::from(b)).wrapping_mul(0x0100_0193);
        }
    };
    for i in 0..8 {
        add(state.simd().xmm(i));
    }
    add(&state.simd().mxcsr().to_le_bytes());
    add(&memory[0x14000..0x18000]);
    hash
}

fn run(engine: usize, workload: u32, loops: u32) -> (f64, u32) {
    let mut code = GUEST.to_vec();
    code.extend([0xcd, 0x40]);
    let start;
    let checksum;
    if engine == 0 {
        let mut cpu = plum::ume::harvard(
            [vec![0; CODE], code].concat().into_boxed_slice(),
            data(loops).into_boxed_slice(),
            Offset32(CODE as u32),
            Offset32(LEN as u32),
        );
        cpu.state_mut().reg(RtReg::EAX).e().write(workload);
        start = Instant::now();
        assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
        let elapsed = start.elapsed();
        checksum = digest(cpu.state(), cpu.data());
        return (elapsed.as_secs_f64(), checksum);
    }
    let mut memory = data(loops);
    memory[CODE..CODE + code.len()].copy_from_slice(&code);
    let mut cpu = plum::ume::flat(
        memory.into_boxed_slice(),
        Offset32(0),
        Offset32(CODE as u32),
        Offset32(LEN as u32),
    )
    .unwrap();
    cpu.state_mut().reg(RtReg::EAX).e().write(workload);
    start = Instant::now();
    assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
    let elapsed = start.elapsed();
    checksum = digest(cpu.state(), cpu.memory());
    (elapsed.as_secs_f64(), checksum)
}

fn main() {
    let loops: u32 = std::env::args().nth(1).map_or(200, |s| s.parse().unwrap());
    println!("workload\tengine\tns_per_simd\tchecksum");
    for workload in 0..3 {
        for (engine, name) in ["harvard", "flat"].iter().enumerate() {
            let mut times = Vec::new();
            let mut checksum = 0;
            for _ in 0..5 {
                let (t, c) = run(engine, workload, loops);
                times.push(t);
                checksum = c;
            }
            times.sort_by(f64::total_cmp);
            let (per, inner) = SIMD[workload as usize];
            let count = (per * inner * u64::from(loops)) as f64;
            println!(
                "{}\t{name}\t{:.2}\t{checksum:08x}",
                NAMES[workload as usize],
                times[2] / count * 1e9
            );
        }
    }
}
