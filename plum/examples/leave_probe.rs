//! LEAVE's fault and retry boundary on both user-mode engines, native and Wasm.
#[path = "support/contracts/flat.rs"]
mod flat;
#[path = "support/contracts/harvard.rs"]
mod harvard;

#[unsafe(no_mangle)]
pub extern "C" fn leave_checks() -> u32 {
    harvard::leave_fault_boundary() + flat::leave_fault_boundary()
}

fn main() {
    println!("{} LEAVE checks passed", leave_checks());
}
