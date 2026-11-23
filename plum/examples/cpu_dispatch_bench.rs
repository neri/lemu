//! Native CLI / Wasm exports for the shared workload.
#[allow(dead_code)]
#[path = "support/bench/cpu_dispatch_bench.rs"]
mod workload;
use workload::*;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let iterations = std::env::args()
            .nth(1)
            .map(|n| n.parse::<u32>().unwrap())
            .unwrap_or(256);
        let expected = bench_cpu_dispatch(1).wrapping_mul(iterations);
        bench_cpu_dispatch(16);
        for _ in 0..7 {
            let start = std::time::Instant::now();
            assert_eq!(bench_cpu_dispatch(iterations), expected);
            println!("{:.3}", start.elapsed().as_secs_f64() * 1000.);
        }
    }
}
