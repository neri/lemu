//! Native CLI / Wasm exports for the shared workload.
#[allow(dead_code)]
#[path = "support/bench/string_bench.rs"]
mod workload;
use workload::*;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("kind,width,count,reverse,scenario,kernel_ms,reference_ms");
        for kind in 0..5 {
            for width in [1, 2, 4] {
                for count in [1, 32, 65536] {
                    for reverse in [0, 1] {
                        let scenarios: &[u32] = match kind {
                            0 => &[0, 1, 2, 3],
                            2 => &[0, 1],
                            3 => &[0, 1, 4],
                            4 => &[0, 4],
                            _ => &[0],
                        };
                        for &scenario in scenarios {
                            let iterations = (262144u32 / count).clamp(4, 2048);
                            let start = std::time::Instant::now();
                            let fast =
                                bench_string(kind, width, count, reverse, scenario, 0, iterations);
                            let kernel = start.elapsed().as_secs_f64() * 1000.;
                            let start = std::time::Instant::now();
                            let slow =
                                bench_string(kind, width, count, reverse, scenario, 1, iterations);
                            let reference = start.elapsed().as_secs_f64() * 1000.;
                            assert_eq!(fast, slow, "{kind}/{width}/{count}/{reverse}/{scenario}");
                            println!(
                                "{kind},{width},{count},{reverse},{scenario},{kernel:.6},{reference:.6}"
                            );
                        }
                    }
                }
            }
        }
    }
}
