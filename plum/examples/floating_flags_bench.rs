//! Compiler-generated audio, graphics, statistics and finance kernels.
#[macro_use]
#[path = "support/guest_bytes.rs"]
mod guest_bytes;
use plum::cpu::ProcessorState;
use plum::flat::FlatUME;
use plum::gpr::PartialRegister;
use plum::harvard::HarvardUME;
use plum::prelude::*;

const GUESTS: [&[u8]; 16] = [
    guest_bytes!("support/bench/floating-flags/guest/x87-audio-O2.bin.hex"),
    guest_bytes!("support/bench/floating-flags/guest/x87-audio-Os.bin.hex"),
    guest_bytes!("support/bench/floating-flags/guest/x87-transform-O2.bin.hex"),
    guest_bytes!("support/bench/floating-flags/guest/x87-transform-Os.bin.hex"),
    guest_bytes!("support/bench/floating-flags/guest/x87-statistics-O2.bin.hex"),
    guest_bytes!("support/bench/floating-flags/guest/x87-statistics-Os.bin.hex"),
    guest_bytes!("support/bench/floating-flags/guest/x87-finance-O2.bin.hex"),
    guest_bytes!("support/bench/floating-flags/guest/x87-finance-Os.bin.hex"),
    guest_bytes!("support/bench/floating-flags/guest/sse2-audio-O2.bin.hex"),
    guest_bytes!("support/bench/floating-flags/guest/sse2-audio-Os.bin.hex"),
    guest_bytes!("support/bench/floating-flags/guest/sse2-transform-O2.bin.hex"),
    guest_bytes!("support/bench/floating-flags/guest/sse2-transform-Os.bin.hex"),
    guest_bytes!("support/bench/floating-flags/guest/sse2-statistics-O2.bin.hex"),
    guest_bytes!("support/bench/floating-flags/guest/sse2-statistics-Os.bin.hex"),
    guest_bytes!("support/bench/floating-flags/guest/sse2-finance-O2.bin.hex"),
    guest_bytes!("support/bench/floating-flags/guest/sse2-finance-Os.bin.hex"),
];
enum Cpu {
    Flat(FlatUME),
    Harvard(HarvardUME),
}
impl Cpu {
    fn step(&mut self) -> u32 {
        let (status, state) = match self {
            Self::Flat(cpu) => {
                let status = cpu.execute();
                (status, cpu.state())
            }
            Self::Harvard(cpu) => {
                let status = cpu.execute();
                (status, cpu.state())
            }
        };
        assert!(matches!(status, Err(Exception::Swi(0x40))), "{status:?}");
        let digest = digest(state);
        match self {
            Self::Flat(cpu) => cpu.resume_next(),
            Self::Harvard(cpu) => cpu.resume_next(),
        }
        digest
    }
    #[cfg(feature = "uop-profile")]
    fn profile(&self) -> String {
        match self {
            Self::Flat(cpu) => cpu.execution_profile_tsv(),
            Self::Harvard(cpu) => cpu.execution_profile_tsv(),
        }
    }
}
fn digest(state: &ProcessorState) -> u32 {
    state.eax().read()
        ^ u32::from(state.fpu().status_word()).rotate_left(7)
        ^ state.simd().mxcsr().rotate_left(17)
}
// This harness is single-threaded; its exported functions are not reentrant.
static mut CPU: *mut Cpu = core::ptr::null_mut();
static mut EXPECTED: u32 = 0;
static mut OUTPUT_SIZE: usize = 0;
#[unsafe(no_mangle)]
pub extern "C" fn prepare_frequency_mode(case: u32, flat: u32) -> u32 {
    let guest = GUESTS[case as usize];
    let mut memory = vec![0; 0x100000 + guest.len() + 64];
    memory[0x100000..0x100000 + guest.len()].copy_from_slice(guest);
    let mut cpu = Box::new(if flat == 1 {
        Cpu::Flat(
            plum::ume::flat(memory, Offset32(0), Offset32(0x100000), Offset32(0x10000)).unwrap(),
        )
    } else {
        Cpu::Harvard(plum::ume::harvard(
            memory.into_boxed_slice(),
            vec![0; 0x120000].into_boxed_slice(),
            Offset32(0x100000),
            Offset32(0x10000),
        ))
    });
    // Harvard code and data are distinct; constants occupy the same guest addresses.
    if let Cpu::Harvard(cpu) = &mut *cpu {
        cpu.write_memory(0x100000, guest).unwrap();
    }
    let expected = cpu.step();
    unsafe {
        if !CPU.is_null() {
            drop(Box::from_raw(CPU));
        }
        CPU = Box::into_raw(cpu);
        EXPECTED = expected;
        OUTPUT_SIZE = match case % 8 / 2 {
            0 => 16384,
            1 => 24576,
            _ => 8192,
        };
    }
    expected
}
#[unsafe(no_mangle)]
pub extern "C" fn run_frequency(iterations: u32) -> u32 {
    unsafe {
        assert!(!CPU.is_null());
        let mut sum = 0u32;
        for _ in 0..iterations {
            let value = (*CPU).step();
            let expected = EXPECTED;
            assert_eq!(value, expected);
            sum = sum.wrapping_add(value);
        }
        sum
    }
}
/// Checks every output bit on the host, outside the execution timer.
#[unsafe(no_mangle)]
pub extern "C" fn output_digest() -> u32 {
    unsafe {
        let bytes = match &*CPU {
            Cpu::Flat(cpu) => cpu.read_memory(0x20000, OUTPUT_SIZE).unwrap(),
            Cpu::Harvard(cpu) => cpu.read_memory(0x20000, OUTPUT_SIZE).unwrap(),
        };
        bytes.iter().fold(0x811c9dc5u32, |h, b| {
            (h ^ u32::from(*b)).wrapping_mul(16777619)
        })
    }
}
fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let case = args[0].parse().unwrap();
        let mode = args.get(2).map_or(1, |s| s.parse().unwrap());
        let expected = prepare_frequency_mode(case, mode);
        #[cfg(feature = "uop-profile")]
        {
            println!("# before");
            print!("{}", unsafe { (*CPU).profile() });
            run_frequency(1);
            println!("# after");
            print!("{}", unsafe { (*CPU).profile() });
        }
        #[cfg(not(feature = "uop-profile"))]
        {
            let iterations: u32 = args[1].parse().unwrap();
            for _ in 0..3 {
                run_frequency(iterations);
            }
            let start = std::time::Instant::now();
            let checksum = run_frequency(iterations);
            let ms = start.elapsed().as_secs_f64() * 1000.;
            assert_eq!(checksum, expected.wrapping_mul(iterations));
            let output = output_digest();
            println!("{checksum} {output} {ms:.6}");
        }
        let _ = expected;
    }
}
