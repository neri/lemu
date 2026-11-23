//! Small tracked-code-update contract on native and Wasm.
#[path = "support/contracts/flat.rs"]
mod contracts;

#[unsafe(no_mangle)]
pub extern "C" fn flat_memory_checks() -> u32 {
    contracts::code_update()
}

fn main() {
    println!("{} flat memory checks passed", flat_memory_checks());
}
