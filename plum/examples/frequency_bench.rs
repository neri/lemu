//! Fixed compiler-generated mixed workloads. Build without uop-profile for timing.
#[macro_use]
#[path = "support/guest_bytes.rs"]
mod guest_bytes;
use plum::gpr::PartialRegister;
use plum::harvard::HarvardUME;
use plum::prelude::*;

const GUESTS: [&[u8]; 17] = [
    guest_bytes!("../../fixtures/frequency-v1/guest/integer-O2.bin.hex"),
    guest_bytes!("../../fixtures/frequency-v1/guest/integer-Os.bin.hex"),
    guest_bytes!("../../fixtures/frequency-v1/guest/byte-word-O2.bin.hex"),
    guest_bytes!("../../fixtures/frequency-v1/guest/byte-word-Os.bin.hex"),
    guest_bytes!("../../fixtures/frequency-v1/guest/updates-O2.bin.hex"),
    guest_bytes!("../../fixtures/frequency-v1/guest/updates-Os.bin.hex"),
    guest_bytes!("../../fixtures/frequency-v1/guest/calls-O2.bin.hex"),
    guest_bytes!("../../fixtures/frequency-v1/guest/calls-Os.bin.hex"),
    guest_bytes!("../../fixtures/frequency-v1/guest/copy-compare-O2.bin.hex"),
    guest_bytes!("../../fixtures/frequency-v1/guest/copy-compare-Os.bin.hex"),
    guest_bytes!("../../fixtures/frequency-v1/guest/updates-immediate-O2.bin.hex"),
    guest_bytes!("../../fixtures/frequency-v1/guest/updates-immediate-Os.bin.hex"),
    guest_bytes!("../../fixtures/frequency-v1/guest/updates-register-O2.bin.hex"),
    guest_bytes!("../../fixtures/frequency-v1/guest/updates-register-Os.bin.hex"),
    // ADD EAX,1; CMP EAX,0; LOOP back. INT yields once per outer iteration.
    &[
        0xb9, 0x00, 0x04, 0, 0, 0x31, 0xc0, 0x83, 0xc0, 1, 0x83, 0xf8, 0, 0xe2, 0xf8, 0xcd, 0x40,
        0xeb, 0xed,
    ],
    // ADC reads the preceding ADD's CF. No flag write can be elided.
    &[
        0xb9, 0x00, 0x04, 0, 0, 0xb8, 0, 0, 0, 0, 0x83, 0xc0, 1, 0x83, 0xd0, 0, 0xe2, 0xf8, 0xcd,
        0x40, 0xeb, 0xea,
    ],
    // Ten dword operand forms per iteration. CMP overwrites the final XOR flags.
    &[
        0xb9, 0, 4, 0, 0, 0x31, 0xc0, // ECX=1024, EAX=0
        0x01, 0xc8, 0x83, 0xc0, 3, // ADD EAX,ECX / 3
        0x29, 0xc8, 0x83, 0xe8, 7, // SUB EAX,ECX / 7
        0x21, 0xc8, 0x83, 0xe0, 63, // AND EAX,ECX / 63
        0x09, 0xc8, 0x83, 0xc8, 1, // OR EAX,ECX / 1
        0x31, 0xc8, 0x83, 0xf0, 5, // XOR EAX,ECX / 5
        0x39, 0xc0, 0xe2, 0xe3, // CMP EAX,EAX; LOOP to ADD
        0xcd, 0x40, 0xeb, 0xd8, // INT; restart at MOV ECX
    ],
];
// Single-threaded harness; exports are called sequentially, never reentrantly.
static mut CPU: *mut HarvardUME = core::ptr::null_mut();
static mut EXPECTED: u32 = 0;
fn step(cpu: &mut HarvardUME) -> u32 {
    let result = cpu.execute();
    assert!(matches!(result, Err(Exception::Swi(0x40))), "{result:?}");
    let result = cpu.state().eax().read();
    cpu.resume_next();
    result
}
fn machine(case: u32) -> HarvardUME {
    let data = (0..65536u32)
        .map(|i| ((i * 37 + (i >> 3) * 11) ^ (i >> 5)) as u8)
        .collect::<Vec<_>>();
    plum::ume::harvard(
        [&vec![0; 0x100000][..], GUESTS[case as usize]]
            .concat()
            .into_boxed_slice(),
        data.into_boxed_slice(),
        Offset32(0x100000),
        Offset32(65536),
    )
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn setup_series(case: u32) {
    let next = Box::new(machine(case));
    unsafe {
        if !CPU.is_null() {
            drop(Box::from_raw(CPU));
        }
        CPU = Box::into_raw(next);
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn guest_series(runs: u32) -> u32 {
    unsafe {
        assert!(!CPU.is_null());
        let cpu = &mut *CPU;
        let mut sum = 0u32;
        for i in 0..runs {
            if i != 0 {
                cpu.resume_next();
            }
            assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
            sum = sum.wrapping_add(cpu.state().eax().read());
        }
        sum
    }
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn resume_series() {
    unsafe {
        assert!(!CPU.is_null());
        (&mut *CPU).resume_next();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn prepare_frequency(case: u32) -> u32 {
    let mut cpu = Box::new(machine(case));
    let expected = step(&mut cpu);
    unsafe {
        if !CPU.is_null() {
            drop(Box::from_raw(CPU));
        }
        CPU = Box::into_raw(cpu);
        EXPECTED = expected;
    }
    expected
}
#[unsafe(no_mangle)]
pub extern "C" fn run_frequency(iterations: u32) -> u32 {
    unsafe {
        assert!(!CPU.is_null());
        let mut sum = 0u32;
        for _ in 0..iterations {
            let value = step(&mut *CPU);
            let expected = EXPECTED;
            assert_eq!(value, expected);
            sum = sum.wrapping_add(value);
        }
        sum
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn frequency_uop_size() -> u32 {
    assert!(
        !cfg!(feature = "uop-profile"),
        "profiled artifact cannot be timed"
    );
    core::mem::size_of::<plum::harvard::uop::Uop>() as u32
}
fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let case = args[0].parse().unwrap();
        if args.get(1).is_some_and(|s| s == "series") {
            for n in (0..=10).map(|i| 1 << i).chain([0]) {
                let mut cpu = machine(case);
                if n == 0 {
                    for _ in 0..4 {
                        step(&mut cpu);
                    }
                }
                let runs = if n == 0 { 128 } else { n };
                let start = std::time::Instant::now();
                let mut sum = 0u32;
                for i in 0..runs {
                    if i != 0 {
                        cpu.resume_next();
                    }
                    let result = cpu.execute();
                    assert!(matches!(result, Err(Exception::Swi(0x40))));
                    sum = sum.wrapping_add(cpu.state().eax().read());
                }
                let ns = start.elapsed().as_nanos();
                let hash = cpu.memory().0.iter().fold(2166136261u32, |h, b| {
                    (h ^ u32::from(*b)).wrapping_mul(16777619)
                });
                println!("{n},{ns},{sum},{hash}");
            }
            return;
        }
        let cold_start = std::time::Instant::now();
        let expected = prepare_frequency(case);
        let cold_ms = cold_start.elapsed().as_secs_f64() * 1000.;
        if args.get(1).is_some_and(|s| s == "cold") {
            println!("{expected} {cold_ms:.6}");
            return;
        }
        #[cfg(feature = "uop-profile")]
        {
            println!("# checksum={expected}");
            print!("{}", unsafe { (&*CPU).execution_profile_tsv() });
            return;
        }
        #[cfg(not(feature = "uop-profile"))]
        {
            let iterations: u32 = args[1].parse().unwrap();
            for _ in 0..3 {
                run_frequency(iterations);
            }
            let start = std::time::Instant::now();
            let checksum = run_frequency(iterations);
            let elapsed = start.elapsed().as_secs_f64() * 1000.;
            assert_eq!(checksum, expected.wrapping_mul(iterations));
            println!("{checksum} {elapsed:.6}");
        }
    }
}
