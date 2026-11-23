//! Warm cached CALL graph: separate data, mixed-page data, and code updates.
#[allow(dead_code)]
#[path = "support/bench/flat_stress_workload.rs"]
mod cases;

static mut WORKLOAD: *mut cases::Stress = core::ptr::null_mut();

#[unsafe(no_mangle)]
pub extern "C" fn prepare_stress(functions: u32) -> u32 {
    assert!(
        !cfg!(feature = "uop-profile"),
        "disable counters for timing"
    );
    let workload = cases::Stress::new(functions);
    let count = workload.cpu.tracer().uop_cache().len() as u32;
    unsafe {
        if !WORKLOAD.is_null() {
            drop(Box::from_raw(WORKLOAD));
        }
        WORKLOAD = Box::into_raw(Box::new(workload));
    }
    count
}

#[unsafe(no_mangle)]
pub extern "C" fn run_stress(mode: u32, iterations: u32) -> u32 {
    unsafe {
        assert!(!WORKLOAD.is_null());
        let workload = &mut *WORKLOAD;
        let mut checksum = 0u32;
        for i in 0..iterations {
            checksum = checksum.wrapping_add(workload.step(mode, 1 + (i & 1)));
        }
        checksum
    }
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<u32> = std::env::args()
            .skip(1)
            .map(|v| v.parse().unwrap())
            .collect();
        let [functions, mode, iterations] = args[..] else {
            panic!("Usage: flat_memory_stress functions mode iterations");
        };
        let count = prepare_stress(functions);
        for _ in 0..7 {
            let start = std::time::Instant::now();
            let checksum = run_stress(mode, iterations);
            println!(
                "{} {count} {checksum}",
                start.elapsed().as_secs_f64() * 1000.
            );
        }
    }
}
