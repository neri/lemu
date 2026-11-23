//! Native CLI / Wasm exports for the shared workload.
#[allow(dead_code)]
#[path = "support/bench/load_extend_bench.rs"]
mod workload;
use workload::*;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args.first().is_some_and(|s| s == "probe") {
            println!(
                "{} {} {}",
                run_load_extend_probe(),
                load_extend_uop_size(),
                load_extend_minor_size()
            );
            return;
        }
        let case = args[0].parse().unwrap();
        let iterations = args[1].parse().unwrap();
        let expected = bench_load_extend(case, iterations);
        for _ in 0..3 {
            assert_eq!(bench_load_extend(case, iterations), expected);
        }
        let start = std::time::Instant::now();
        let actual = bench_load_extend(case, iterations);
        let elapsed = start.elapsed().as_secs_f64() * 1000.;
        assert_eq!(actual, expected);
        println!("{actual} {elapsed:.6}");
    }
}
