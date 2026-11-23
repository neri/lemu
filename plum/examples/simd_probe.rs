//! Fixed numeric contracts on native and Wasm, independently of engine agreement.
#[path = "support/contracts/simd.rs"]
mod contracts;

#[unsafe(no_mangle)]
pub extern "C" fn simd_checksum() -> u32 {
    contracts::harvard() + contracts::flat()
}
fn main() {
    println!("{}", simd_checksum());
}
