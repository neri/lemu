//! Fixed scalar rounding and sNaN contracts for each engine and Wasm.
#![allow(dead_code)]
use plum::prelude::*;

const CODE: &[u8] = &[
    0xf3, 0x0f, 0x10, 5, 128, 0, 0, 0, 0xf3, 0x0f, 0x58, 5, 132, 0, 0, 0, 0xcd, 0x40,
];
// 1+2^-24 rounds to even; signaling NaN preserves payload and sets IE.
const CASES: [(u32, u32, u32, u32); 2] = [
    (0x3f800000, 0x33800000, 0x3f800000, 0x20),
    (0x7f800001, 0x3f800000, 0x7fc00001, 1),
];

pub fn harvard() -> u32 {
    for (lhs, rhs, expected, flags) in CASES {
        let mut cpu = plum::ume::harvard(
            CODE.to_vec().into_boxed_slice(),
            vec![0; 256].into_boxed_slice(),
            Offset32(0),
            Offset32(256),
        );
        cpu.data()[128..132].copy_from_slice(&lhs.to_le_bytes());
        cpu.data()[132..136].copy_from_slice(&rhs.to_le_bytes());
        assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
        assert_eq!(&cpu.state().simd().xmm(0)[..4], &expected.to_le_bytes());
        assert_eq!(cpu.state().simd().mxcsr() & 0x3f, flags);
    }
    2
}

pub fn flat() -> u32 {
    for (lhs, rhs, expected, flags) in CASES {
        let mut memory = vec![0; 256];
        memory[..CODE.len()].copy_from_slice(CODE);
        memory[128..132].copy_from_slice(&lhs.to_le_bytes());
        memory[132..136].copy_from_slice(&rhs.to_le_bytes());
        let mut cpu = plum::ume::flat(memory, Offset32(0), Offset32(0), Offset32(256)).unwrap();
        assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
        assert_eq!(&cpu.state().simd().xmm(0)[..4], &expected.to_le_bytes());
        assert_eq!(cpu.state().simd().mxcsr() & 0x3f, flags);
    }
    2
}
