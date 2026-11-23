//! Identical x87 guests for the split/flat engines, native and Wasm.
use plum::flat::FlatUME;
use plum::harvard::HarvardUME;
use plum::prelude::*;

// The harness is single-threaded; exports must be called sequentially.
static mut SPLIT: *mut HarvardUME = core::ptr::null_mut();
static mut FLAT: *mut FlatUME = core::ptr::null_mut();
static mut MODE: u32 = 0;
static mut EXPECTED: u32 = 0;
fn memory(code: &mut Vec<u8>, opcode: u8, reg: u8, addr: u32) {
    code.extend([opcode, reg << 3 | 5]);
    code.extend(addr.to_le_bytes());
}
fn guest(case: u32) -> Vec<u8> {
    let mut code = vec![0xdb, 0xe3, 0xd9, 0xe8, 0xd9, 0xe8, 0xb9]; // FNINIT; FLD1 twice
    code.extend(2000u32.to_le_bytes());
    let start = code.len();
    match case {
        0 => code.extend([0xd8, 0xc1]), // FADD ST0,ST1
        1 => {
            memory(&mut code, 0xdd, 0, 512);
            memory(&mut code, 0xdd, 3, 528);
        }
        2 => {
            memory(&mut code, 0xdf, 5, 512);
            memory(&mut code, 0xdf, 7, 528);
        }
        3 => memory(&mut code, 0xd9, 6, 640), // FNSTENV
        _ => panic!("unknown x87 workload"),
    }
    code.extend([0x49, 0x75]);
    code.push((start as isize - code.len() as isize - 1) as i8 as u8);
    memory(&mut code, 0xdd, 3, 544); // Deterministic result and status checksum.
    code.extend([0xcd, 0x40, 0xe9]);
    code.extend((-(code.len() as i32 + 4)).to_le_bytes());
    code
}
fn step_split(cpu: &mut HarvardUME) -> u32 {
    assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
    let checksum = cpu.read_memory32(548).unwrap() ^ u32::from(cpu.state().fpu().status_word());
    cpu.resume_next();
    checksum
}
fn step_flat(cpu: &mut FlatUME) -> u32 {
    assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
    let checksum = cpu.read_memory32(548).unwrap() ^ u32::from(cpu.state().fpu().status_word());
    cpu.resume_next();
    checksum
}
#[unsafe(no_mangle)]
pub extern "C" fn prepare_fpu(case: u32, mode: u32) -> u32 {
    assert!(
        !cfg!(feature = "uop-profile"),
        "profiled artifacts cannot be timed"
    );
    let code = guest(case);
    let mut data = vec![0; 8192];
    data[512..520].copy_from_slice(&1.25f64.to_le_bytes());
    let expected;
    unsafe {
        match mode {
            0 => {
                if !SPLIT.is_null() {
                    drop(Box::from_raw(SPLIT));
                }
                let mut cpu = Box::new(plum::ume::harvard(
                    [vec![0; 4096], code].concat().into_boxed_slice(),
                    data.into_boxed_slice(),
                    Offset32(4096),
                    Offset32(8192),
                ));
                expected = step_split(&mut cpu);
                SPLIT = Box::into_raw(cpu);
            }
            1 => {
                if !FLAT.is_null() {
                    drop(Box::from_raw(FLAT));
                }
                data[4096..4096 + code.len()].copy_from_slice(&code);
                let mut cpu = Box::new(
                    plum::ume::flat(
                        data.into_boxed_slice(),
                        Offset32(0),
                        Offset32(4096),
                        Offset32(8192),
                    )
                    .unwrap(),
                );
                expected = step_flat(&mut cpu);
                FLAT = Box::into_raw(cpu);
            }
            _ => panic!("memory mode not compiled"),
        }
        MODE = mode;
        EXPECTED = expected;
    }
    expected
}
#[unsafe(no_mangle)]
pub extern "C" fn run_fpu(iterations: u32) -> u32 {
    let mut sum = 0u32;
    unsafe {
        // Select the engine outside the timed guest loop.
        match MODE {
            0 => {
                assert!(!SPLIT.is_null());
                for _ in 0..iterations {
                    let value = step_split(&mut *SPLIT);
                    let expected = EXPECTED;
                    assert_eq!(value, expected);
                    sum = sum.wrapping_add(value);
                }
            }
            1 => {
                assert!(!FLAT.is_null());
                for _ in 0..iterations {
                    let value = step_flat(&mut *FLAT);
                    let expected = EXPECTED;
                    assert_eq!(value, expected);
                    sum = sum.wrapping_add(value);
                }
            }
            _ => unreachable!(),
        }
    }
    sum
}
fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let case = args[0].parse().unwrap();
        let iterations = args[1].parse().unwrap();
        let mode = args.get(2).map_or(0, |s| s.parse().unwrap());
        let expected = prepare_fpu(case, mode);
        for _ in 0..3 {
            run_fpu(iterations);
        }
        let start = std::time::Instant::now();
        let sum = run_fpu(iterations);
        assert_eq!(sum, expected.wrapping_mul(iterations));
        println!("{sum} {:.6}", start.elapsed().as_secs_f64() * 1000.);
    }
}
