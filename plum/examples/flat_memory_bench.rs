//! Comparable split/flat workloads. Disable uop-profile for timings.
use plum::flat::FlatUME;
use plum::gpr::PartialRegister;
use plum::harvard::HarvardUME;
use plum::prelude::*;

enum BenchCpu {
    Split(HarvardUME),
    Flat(FlatUME),
}

trait Cpu {
    fn step(&mut self) -> u32;
    fn byte(&self, address: usize) -> u8;
    fn patch(&mut self, value: u32);
}

impl Cpu for HarvardUME {
    fn step(&mut self) -> u32 {
        assert!(matches!(self.execute(), Err(Exception::Swi(0x40))));
        let value = self.state().eax().read();
        self.resume_next();
        value
    }
    fn byte(&self, address: usize) -> u8 {
        self.data()[address]
    }
    fn patch(&mut self, _: u32) {
        panic!("split code is immutable")
    }
}

impl Cpu for FlatUME {
    fn step(&mut self) -> u32 {
        assert!(matches!(self.execute(), Err(Exception::Swi(0x40))));
        let value = self.state().eax().read();
        self.resume_next();
        value
    }
    fn byte(&self, address: usize) -> u8 {
        self.memory()[address]
    }
    fn patch(&mut self, value: u32) {
        self.write_memory32(1, value).unwrap();
    }
}

static mut CPU: *mut BenchCpu = core::ptr::null_mut();

#[unsafe(no_mangle)]
pub extern "C" fn prepare_flat_bench(case: u32, flat: u32) {
    assert!(!cfg!(feature = "uop-profile"));
    let mut code = match case {
        0 => vec![
            0xb9, 0x20, 0x4e, 0, 0, 0x31, 0xc0, 0x83, 0xc0, 3, 0x49, 0x75, 0xfa,
        ],
        1 => vec![
            0xbf, 0, 0x10, 0, 0, 0xc7, 7, 0, 0, 0, 0, 0xb9, 0x20, 0x4e, 0, 0, 0x83, 7, 1, 0x49,
            0x75, 0xfa, 0x8b, 7,
        ],
        2 => vec![
            0xb9, 0, 4, 0, 0, 0xbf, 0, 0x10, 0, 0, 0xbe, 0, 0x20, 0, 0, 0xf3, 0xa4,
        ],
        3 => vec![0xb8, 1, 0, 0, 0], // host patches the cached immediate between stops
        _ => panic!("unknown case"),
    };
    code.extend([0xcd, 0x40, 0xe9]);
    let displacement = -(code.len() as i32 + 4);
    code.extend(displacement.to_le_bytes());
    let mut memory = vec![0; 16384];
    memory[8192..9216].fill(0x5a);
    let cpu = if flat != 0 {
        memory[..code.len()].copy_from_slice(&code);
        BenchCpu::Flat(
            plum::ume::flat(
                memory.into_boxed_slice(),
                Offset32(0),
                Offset32(0),
                Offset32(16384),
            )
            .unwrap(),
        )
    } else {
        assert!(case != 3, "code replacement requires flat mode");
        BenchCpu::Split(plum::ume::harvard(
            code.into_boxed_slice(),
            memory.into_boxed_slice(),
            Offset32(0),
            Offset32(16384),
        ))
    };
    unsafe {
        if !CPU.is_null() {
            drop(Box::from_raw(CPU));
        }
        CPU = Box::into_raw(Box::new(cpu));
    }
}

fn run<C: Cpu>(cpu: &mut C, case: u32, iterations: u32) -> u32 {
    let mut checksum = 0u32;
    for i in 0..iterations {
        if case == 3 {
            cpu.patch(i + 1);
        }
        let value = cpu.step();
        match case {
            0 => assert_eq!(value, 60000),
            1 => assert_eq!(value, 20000),
            2 => assert_eq!(cpu.byte(4096), 0x5a),
            3 => assert_eq!(value, i + 1),
            _ => unreachable!(),
        }
        checksum = checksum.wrapping_add(value);
    }
    checksum
}

#[unsafe(no_mangle)]
pub extern "C" fn run_flat_bench(case: u32, iterations: u32) -> u32 {
    unsafe {
        assert!(!CPU.is_null());
        // Select once per batch; each guest loop is monomorphized for its type.
        match &mut *CPU {
            BenchCpu::Split(cpu) => run(cpu, case, iterations),
            BenchCpu::Flat(cpu) => run(cpu, case, iterations),
        }
    }
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<_> = std::env::args()
            .skip(1)
            .map(|v| v.parse::<u32>().unwrap())
            .collect();
        let case = args.first().copied().unwrap_or(0);
        let flat = args.get(1).copied().unwrap_or(1);
        let iterations = args.get(2).copied().unwrap_or(64);
        prepare_flat_bench(case, flat);
        run_flat_bench(case, 2);
        for _ in 0..7 {
            let start = std::time::Instant::now();
            let checksum = run_flat_bench(case, iterations);
            println!("{:.6} {checksum}", start.elapsed().as_secs_f64() * 1000.);
        }
    }
}
