//! Native CLI / Wasm exports for the shared workload.
#[allow(dead_code)]
#[path = "support/bench/return_dispatch_bench.rs"]
mod workload;
use workload::*;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let iterations = std::env::args()
            .nth(1)
            .map(|n| n.parse::<u32>().unwrap())
            .unwrap_or(64);
        for kind in 0..4 {
            let expected = [10_000, 40_000, 80_000, 160_000][kind as usize] * iterations;
            for _ in 0..3 {
                assert_eq!(bench_return_dispatch(kind, iterations), expected);
            }
            let mut samples = Vec::new();
            for _ in 0..9 {
                let start = std::time::Instant::now();
                assert_eq!(bench_return_dispatch(kind, iterations), expected);
                samples.push(start.elapsed().as_secs_f64() * 1000.);
            }
            samples.sort_by(f64::total_cmp);
            println!("kind={kind} median_ms={:.3}", samples[4]);
        }
    }
}
