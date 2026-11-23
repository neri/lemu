//! Fixed compiler-generated mixed workloads. Build without uop-profile for timing.
#[macro_use]
#[path = "support/guest_bytes.rs"]
mod guest_bytes;
use plum::flat::FlatUME;
use plum::gpr::PartialRegister;
use plum::prelude::*;

const GUESTS: [&[u8]; 14] = [
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
];
// Single-threaded harness; exports are called sequentially, never reentrantly.
static mut CPU: *mut FlatUME = core::ptr::null_mut();
static mut EXPECTED: u32 = 0;
fn step(cpu: &mut FlatUME) -> u32 {
    let result = cpu.execute();
    assert!(matches!(result, Err(Exception::Swi(0x40))), "{result:?}");
    let result = cpu.state().eax().read();
    cpu.resume_next();
    result
}
#[unsafe(no_mangle)]
pub extern "C" fn prepare_frequency(case: u32) -> u32 {
    prepare_frequency_mode(case, 1)
}

/// Same compiler-generated guests and checksums in split and flat modes.
#[unsafe(no_mangle)]
pub extern "C" fn prepare_frequency_mode(case: u32, flat: u32) -> u32 {
    let data = (0..65536u32)
        .map(|i| ((i * 37 + (i >> 3) * 11) ^ (i >> 5)) as u8)
        .collect::<Vec<_>>();
    assert_eq!(flat, 1, "use frequency_bench for the legacy engine");
    let guest = GUESTS[case as usize];
    let mut memory = vec![0; 0x100000 + guest.len()];
    memory[..data.len()].copy_from_slice(&data);
    memory[0x100000..].copy_from_slice(guest);
    let mut cpu = Box::new(
        plum::ume::flat(
            memory.into_boxed_slice(),
            Offset32(0),
            Offset32(0x100000),
            Offset32(65536),
        )
        .unwrap(),
    );
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
    core::mem::size_of::<plum::flat::uop::Uop>() as u32
}
fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let case = args[0].parse().unwrap();
        let cold_start = std::time::Instant::now();
        let mode = args.get(2).map_or(1, |s| s.parse().unwrap());
        let expected = prepare_frequency_mode(case, mode);
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
