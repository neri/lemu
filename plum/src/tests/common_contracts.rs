//! Private common-component contracts, independent of all engine dispatchers.
use crate::alu::division::divide;
use crate::string_ops::{self as strings, CompareRepeat};

#[cfg(feature = "x87")]
#[test]
fn common_c05_extended_divide_and_sqrt_keep_the_rounding_remainder() {
    use crate::fpu::{FpuValue, PRECISION, Raw80, arithmetic};

    for (cw, third, sqrt_two) in [
        (0x077f, 0xaaaa_aaaa_aaaa_aaaa, 0xb504_f333_f9de_6484),
        (0x0b7f, 0xaaaa_aaaa_aaaa_aaab, 0xb504_f333_f9de_6485),
    ] {
        let (q, flags) = arithmetic::binary(FpuValue::F64(1.0), FpuValue::F64(3.0), 6, cw);
        assert_eq!(
            q.raw(),
            Raw80 {
                significand: third,
                sign_exponent: 0x3ffd
            }
        );
        assert_eq!(flags, PRECISION);
        let (r, flags) = arithmetic::sqrt(FpuValue::F64(2.0), cw);
        assert_eq!(
            r.raw(),
            Raw80 {
                significand: sqrt_two,
                sign_exponent: 0x3fff
            }
        );
        assert_eq!(flags, PRECISION);
        for (value, flags) in [
            arithmetic::sqrt(FpuValue::F64(4.0), cw),
            arithmetic::binary(FpuValue::F64(3.0), FpuValue::F64(1.5), 6, cw),
        ] {
            assert_eq!(value.raw(), FpuValue::F64(2.0).raw());
            assert_eq!(flags, 0);
        }
    }

    // For 1/47 and sqrt(6), the three extra bits are zero. Only the
    // remaining fraction tells round-up to increment and set PRECISION.
    let round_up = 0x0b7f;
    let (quotient, flags) =
        arithmetic::binary(FpuValue::F64(1.0), FpuValue::F64(47.0), 6, round_up);
    assert_eq!(
        quotient.raw(),
        Raw80 {
            significand: 0xae4c_415c_9882_b932,
            sign_exponent: 0x3ff9,
        }
    );
    assert_eq!(flags, PRECISION);
    let (root, flags) = arithmetic::sqrt(FpuValue::F64(6.0), round_up);
    assert_eq!(
        root.raw(),
        Raw80 {
            significand: 0x9cc4_70a0_4909_73e9,
            sign_exponent: 0x4000,
        }
    );
    assert_eq!(flags, PRECISION);
}

#[test]
fn common_c03_checked_division_has_signed_remainders_and_limits() {
    for (dividend, divisor, bits, signed, expected) in [
        (100, 7, 8, false, Some((14, 2))),
        (0xff9c, 7, 8, true, Some(((-14i32) as u32, (-2i32) as u32))),
        (0xff9c, 0xf9, 8, true, Some((14, (-2i32) as u32))),
        (0xffff, 1, 16, false, Some((65535, 0))),
        (0x1_0000_0000, 2, 32, false, Some((0x80000000, 0))),
        (256, 1, 8, false, None),
        (128, 1, 8, true, None),
        (0x8000, 0xff, 8, true, None),
        (i64::MIN as u64, u32::MAX, 32, true, None),
        (1, 0, 32, false, None),
    ] {
        assert_eq!(
            divide(dividend, divisor, bits, signed),
            expected,
            "dividend={dividend:x} divisor={divisor:x} bits={bits} signed={signed}"
        );
    }
}

#[test]
fn common_c04_spans_validate_whole_elements_and_direction() {
    assert_eq!(strings::span(8, 7, 1, false, 3), 0);
    assert_eq!(strings::span(8, 2, 1, false, 9), 3);
    assert_eq!(strings::span(8, 2, 1, true, 9), 2);
    assert_eq!(strings::span(8, 2, 1, false, 0), 0);
    assert_eq!(strings::range::<2, true>(4, 3), 0..6);
    let mut forward = [1, 2, 3, 4, 5];
    strings::copy::<1, false>(&mut forward, 0..4, 1..5, 4);
    assert_eq!(forward, [1, 1, 1, 1, 1]);
    let mut reverse = [1, 2, 3, 4, 5];
    strings::copy::<1, true>(&mut reverse, 1..5, 0..4, 4);
    assert_eq!(reverse, [5, 5, 5, 5, 5]);
    let mut words = [1, 2, 3, 4, 5];
    strings::copy::<2, false>(&mut words, 0..4, 1..5, 2);
    assert_eq!(words, [1, 1, 2, 2, 4]); // each full word is read before storing
}

#[test]
fn common_c04_fill_and_compare_report_the_last_complete_element() {
    let mut bytes = [0; 6];
    strings::fill::<2>(&mut bytes, 0x1234);
    assert_eq!(bytes, [0x34, 0x12, 0x34, 0x12, 0x34, 0x12]);
    assert_eq!(
        strings::compare::<1, false>(Some(&[1, 2, 3]), &[1, 9, 3], 0, CompareRepeat::WhileZero),
        (2, 2, 9)
    );
    assert_eq!(
        strings::compare::<1, true>(None, &[1, 7, 3], 7, CompareRepeat::WhileNotZero),
        (2, 7, 7)
    );
    assert_eq!(
        strings::compare::<2, false>(
            Some(&[1, 0, 2, 0]),
            &[1, 0, 3, 0],
            0,
            CompareRepeat::WhileZero
        ),
        (2, 2, 3)
    );
    // Forward REPE CMPS skips equal 64-byte blocks before the element scan.
    let source = [0x11; 132];
    for (difference, expected_byte, expected_dword) in [
        (Some(5), (6, 0x11, 0x22), (2, 0x11111111, 0x11112211)),
        (Some(70), (71, 0x11, 0x22), (18, 0x11111111, 0x11221111)),
        (None, (132, 0x11, 0x11), (33, 0x11111111, 0x11111111)),
    ] {
        let mut dest = source;
        if let Some(at) = difference {
            dest[at] = 0x22;
        }
        assert_eq!(
            strings::compare::<1, false>(Some(&source), &dest, 0, CompareRepeat::WhileZero),
            expected_byte,
            "difference={difference:?}"
        );
        assert_eq!(
            strings::compare::<4, false>(Some(&source), &dest, 0, CompareRepeat::WhileZero),
            expected_dword,
            "difference={difference:?}"
        );
    }
}

#[cfg(feature = "x87")]
mod x87 {
    use crate::cpu::ProcessorState;
    use crate::fpu::uop::{F32, F80, I16};
    use crate::fpu::{self, FpuState, FpuValue, Raw80, codec, ops, register_ops};

    #[test]
    fn common_c05_stack_tags_and_underflow() {
        let mut fpu = FpuState::default();
        assert_eq!(fpu.tag_word(), 0xffff);
        fpu.push(FpuValue::F64(2.0));
        fpu.push(FpuValue::F64(0.0));
        assert_eq!(fpu.status_word() & 0x3800, 6 << 11);
        assert_eq!(fpu.tag_word(), 0x1fff);
        assert_eq!(fpu.st(0).unwrap().raw(), Raw80::from_i64(0));
        fpu.pop();
        assert_eq!(fpu.st(0).unwrap().raw(), Raw80::from_i64(2));
        fpu.pop();
        assert_eq!(fpu.read(0).raw(), fpu::INDEFINITE);
        assert_eq!(fpu.status_word() & 0x41, 0x41);
    }

    #[test]
    fn common_c05_arithmetic_compare_and_codec_have_fixed_expectations() {
        let (sum, flags) = ops::arithmetic(FpuValue::F64(1.5), FpuValue::F64(2.5), 0, 0x37f);
        assert_eq!((sum.raw(), flags), (Raw80::from_i64(4), 0));
        let (_, flags) = ops::arithmetic(FpuValue::F64(1.0), FpuValue::F64(0.0), 6, 0x37f);
        assert_eq!(flags, fpu::ZERO_DIVIDE);
        let raw = Raw80 {
            significand: 0x8000000000000001,
            sign_exponent: 0x3fff,
        };
        let (bytes, flags) = codec::encode(raw, F80, 0);
        assert_eq!(&bytes, &[1, 0, 0, 0, 0, 0, 0, 0x80, 0xff, 0x3f]);
        let (decoded, read_flags) = codec::decode(&bytes, F80).unwrap();
        assert_eq!((decoded.raw(), flags, read_flags), (raw, 0, 0));
        let (one, _) = codec::decode(&[0, 0, 0x80, 0x3f], F32).unwrap();
        assert_eq!(one.raw(), Raw80::from_i64(1));
        // 2.5: nearest-even=2, upward=3; both conversions are inexact.
        for (rounding, expected) in [(0, 2u16), (2, 3)] {
            let (bytes, flags) = codec::encode(Raw80::from_f64(2.5), I16, rounding);
            assert_eq!(&bytes[..2], &expected.to_le_bytes());
            assert_eq!(flags, fpu::PRECISION);
        }
        let mut state = ProcessorState::latest();
        state.fpu.push(FpuValue::F64(1.0));
        register_ops::compare(&mut state, FpuValue::F64(2.0), false, false);
        assert_eq!(state.fpu.status_word() & 0x4500, 0x100);
    }

    #[test]
    fn common_c05_compare_and_conditional_move_consume_lazy_integer_flags() {
        use crate::alu::Alu;
        use crate::flags::Flags;
        use crate::fpu::uop::{FpuMinorUop, FpuOp};
        for (a, b, conditions) in [
            (0xff, 1, [true, true, true, true]),
            (1, 2, [false, false, false, true]),
            (1, 0, [false, false, false, false]),
        ] {
            for condition in 0..4 {
                for invert in [false, true] {
                    let mut state = ProcessorState::latest();
                    state.fpu.push(FpuValue::F64(2.0));
                    state.fpu.push(FpuValue::F64(1.0));
                    Alu::new(&mut state).add8(a, b);
                    let arg = 1 | ((condition as u8) << 3) | if invert { 0x20 } else { 0 };
                    register_ops::execute(
                        &mut state,
                        FpuMinorUop::new(FpuOp::ConditionalMove, arg),
                    )
                    .unwrap();
                    let expected = if conditions[condition] ^ invert { 2 } else { 1 };
                    assert_eq!(state.fpu.st(0).unwrap().raw(), Raw80::from_i64(expected));
                }
            }
        }
        let mut state = ProcessorState::latest();
        state.flags_mut().set_static(Flags::DF, true);
        Alu::new(&mut state).add32(0x7fff_ffff, 1);
        state.fpu.push(FpuValue::F64(1.0));
        register_ops::compare(&mut state, FpuValue::F64(2.0), true, true);
        let flags = state.flags_mut().resolve();
        assert_eq!(flags.bits() & 0x8d5, Flags::CF.bits());
        assert!(flags.contains(Flags::DF));
    }

    #[test]
    fn common_c05_environment_has_fixed_layout_and_restores_stack() {
        let mut fpu = FpuState::default();
        fpu.push(FpuValue::F64(1.0));
        fpu.instruction_pointer = 0x12345678;
        fpu.instruction_selector = 8;
        fpu.data_pointer = 0x87654321;
        fpu.data_selector = 16;
        let (bytes, len) = fpu.environment(true, true, 0x123);
        assert_eq!(len, 108);
        assert_eq!(&bytes[..4], &[0x7f, 3, 0xff, 0xff]);
        assert_eq!(&bytes[12..20], &[0x78, 0x56, 0x34, 0x12, 8, 0, 0x23, 1]);
        let mut restored = FpuState::default();
        assert!(restored.restore_environment(&bytes[..len], true, true));
        assert_eq!(restored.st(0).unwrap().raw(), Raw80::from_i64(1));
        assert_eq!(restored.instruction_pointer(), 0x12345678);
        assert_eq!(restored.data_pointer(), 0x87654321);
        assert_eq!(restored.tag_word(), fpu.tag_word());
    }

    #[test]
    fn common_c05_integer_conversion_checks_rounding_bounds_and_raw_precision() {
        use crate::fpu::uop::{I64, TRUNCATE};
        // RC: nearest-even, down, up, truncate. Flags are precision only.
        for (rc, positive, negative) in [(0, 2i16, -2i16), (1, 2, -3), (2, 3, -2), (3, 2, -2)] {
            for (input, expected) in [(2.5, positive), (-2.5, negative)] {
                let (bytes, flags) = codec::encode(Raw80::from_f64(input), I16, rc);
                assert_eq!(
                    &bytes[..2],
                    &expected.to_le_bytes(),
                    "input={input} rc={rc}"
                );
                assert_eq!(flags, fpu::PRECISION);
            }
        }
        for (input, expected, flags) in [
            (32767, 0x7fffu16, 0),
            (-32768, 0x8000, 0),
            (32768, 0x8000, fpu::INVALID),
            (-32769, 0x8000, fpu::INVALID),
        ] {
            let (bytes, actual_flags) = codec::encode(Raw80::from_i64(input), I16, 0);
            assert_eq!(
                (&bytes[..2], actual_flags),
                (&expected.to_le_bytes()[..], flags),
                "input={input}"
            );
        }
        let (bytes, flags) = codec::encode(Raw80::from_f64(2.5), I16 | TRUNCATE, 2);
        assert_eq!((&bytes[..2], flags), (&[2, 0][..], fpu::PRECISION));
        // This integer cannot be transported exactly through f64.
        let exact = 9_007_199_254_740_993i64;
        let (decoded, flags) = codec::decode(&exact.to_le_bytes(), I64).unwrap();
        assert_eq!(flags, 0);
        let (bytes, flags) = codec::encode(decoded.raw(), I64, 0);
        assert_eq!((&bytes[..8], flags), (&exact.to_le_bytes()[..], 0));
    }
}

#[cfg(feature = "sse")]
mod simd {
    use ir86::prelude::SimdOp;

    use crate::cpu::ProcessorState;
    use crate::gpr::PartialRegister;
    use crate::simd::{self, Rm, SimdFault};

    fn step(state: &mut ProcessorState, op: SimdOp, imm: u8) {
        simd::execute(state, op, 0, Rm::Reg(1), imm, false).unwrap();
    }

    #[test]
    fn common_c06_widening_is_exact_in_all_rounding_modes() {
        let mut state = ProcessorState::latest();
        for rc in 0..4 {
            let mxcsr = 0x1f81 | (rc << 13); // preserve sticky IE without raising PE
            for (op, inputs, outputs) in [
                (
                    SimdOp::CVTDQ2PD_Vo_Wq,
                    [i32::MIN as u32, i32::MAX as u32],
                    [-2147483648.0f64, 2147483647.0],
                ),
                (
                    SimdOp::CVTPS2PD_Vo_Wq,
                    [0x3f80_0001, 0x8000_0000],
                    [1.0 + 1.0 / 8388608.0, -0.0],
                ),
            ] {
                state.simd.mxcsr = mxcsr;
                for (chunk, input) in state.simd.xmm[1].chunks_exact_mut(4).zip(inputs) {
                    chunk.copy_from_slice(&input.to_le_bytes());
                }
                step(&mut state, op, 0);
                for (chunk, output) in state.simd.xmm[0].chunks_exact(8).zip(outputs) {
                    assert_eq!(chunk, output.to_le_bytes());
                }
                assert_eq!(state.simd.mxcsr, mxcsr);
            }
        }
    }

    #[test]
    fn common_c06_sqrt_rounds_from_the_exact_remainder() {
        let mut state = ProcessorState::latest();
        // A negative normal in a packed instruction still takes the invalid
        // path. Positive normals and signed zeros retain their lane results.
        for (lane, value) in state.simd.xmm[1]
            .chunks_exact_mut(4)
            .zip([-1.0f32, -0.0, 4.0, 0.0])
        {
            lane.copy_from_slice(&value.to_le_bytes());
        }
        step(&mut state, SimdOp::SQRTPS_Vo_Wo, 0);
        for (lane, bits) in
            state.simd.xmm[0]
                .chunks_exact(4)
                .zip([0xffc0_0000u32, 0x8000_0000, 0x4000_0000, 0])
        {
            assert_eq!(lane, bits.to_le_bytes());
        }
        assert_eq!(state.simd.mxcsr, 0x1f81);
        for (rc, single, double) in [
            (1, 0x3fb5_04f3u32, 0x3ff6_a09e_667f_3bccu64),
            (2, 0x3fb5_04f4, 0x3ff6_a09e_667f_3bcd),
            (3, 0x3fb5_04f3, 0x3ff6_a09e_667f_3bcc),
        ] {
            let mxcsr = 0x1f80 | (rc << 13);
            for (input, s, d, flags) in [
                (2, single, double, 0x20),
                (4, 2f32.to_bits(), 2f64.to_bits(), 0),
            ] {
                state.simd.mxcsr = mxcsr;
                state.simd.xmm[1][..4].copy_from_slice(&(input as f32).to_le_bytes());
                step(&mut state, SimdOp::SQRTSS_Vd_Wd, 0);
                assert_eq!(&state.simd.xmm[0][..4], &s.to_le_bytes());
                assert_eq!(state.simd.mxcsr, mxcsr | flags);
                state.simd.mxcsr = mxcsr;
                state.simd.xmm[1][..8].copy_from_slice(&(input as f64).to_le_bytes());
                step(&mut state, SimdOp::SQRTSD_Vq_Wq, 0);
                assert_eq!(&state.simd.xmm[0][..8], &d.to_le_bytes());
                assert_eq!(state.simd.mxcsr, mxcsr | flags);
            }
        }
    }

    #[test]
    fn common_c06_compare_replaces_lazy_flags_and_preserves_control_flags() {
        use crate::alu::Alu;
        use crate::flags::Flags;
        let mut state = ProcessorState::latest();
        state.flags_mut().set_static(Flags::DF, true);
        // All arithmetic flags are replaced, including still-pending OF/AF/SF.
        Alu::new(&mut state).add32(0x7fff_ffff, 1);
        state.simd.xmm[0][..4].copy_from_slice(&1.0f32.to_le_bytes());
        state.simd.xmm[1][..4].copy_from_slice(&2.0f32.to_le_bytes());
        step(&mut state, SimdOp::UCOMISS_Vd_Wd, 0);
        let flags = state.flags_mut().resolve();
        assert_eq!(flags.bits() & 0x8d5, Flags::CF.bits());
        assert!(flags.contains(Flags::DF));
        // A following partial update must retain the comparison's CF.
        Alu::new(&mut state).inc32(0);
        assert!(state.flags_mut().resolve_cf());
    }

    #[test]
    fn common_c06_sticky_precision_does_not_hide_later_exceptions_or_clear() {
        let mut state = ProcessorState::latest();
        state.simd.xmm[0][..4].copy_from_slice(&1.0f32.to_le_bytes());
        state.simd.xmm[1][..4].copy_from_slice(&(2.0f32.powi(-24)).to_le_bytes());
        step(&mut state, SimdOp::ADDSS_Vd_Wd, 0);
        assert_eq!(state.simd.mxcsr() & 0x3f, 0x20);
        // PE already set: exact arithmetic preserves it, while invalid still accumulates.
        state.simd.xmm[1][..4].copy_from_slice(&1.0f32.to_le_bytes());
        step(&mut state, SimdOp::ADDSS_Vd_Wd, 0);
        state.simd.xmm[0][..4].copy_from_slice(&0.0f32.to_le_bytes());
        state.simd.xmm[1][..4].copy_from_slice(&0.0f32.to_le_bytes());
        step(&mut state, SimdOp::DIVSS_Vd_Wd, 0);
        assert_eq!(state.simd.mxcsr() & 0x3f, 0x21);
        assert!(state.simd.set_mxcsr(0x1f80));
        state.simd.xmm[0][..4].copy_from_slice(&1.0f32.to_le_bytes());
        state.simd.xmm[1][..4].copy_from_slice(&(2.0f32.powi(-24)).to_le_bytes());
        step(&mut state, SimdOp::ADDSS_Vd_Wd, 0);
        assert_eq!(state.simd.mxcsr() & 0x3f, 0x20);
        let image = simd::fxsave(&state, 0);
        assert_eq!(
            u32::from_le_bytes(image[24..28].try_into().unwrap()),
            0x1fa0
        );
    }

    #[test]
    fn common_c06_precision_history_survives_overwrites_copy_save_and_reset() {
        let mut state = ProcessorState::latest();
        for i in 0..12 {
            state.simd.xmm[0][..4].copy_from_slice(&1.0f32.to_le_bytes());
            let rhs = if i == 0 { 2.0f32.powi(-24) } else { 1.0 };
            state.simd.xmm[1][..4].copy_from_slice(&rhs.to_le_bytes());
            step(&mut state, SimdOp::ADDSS_Vd_Wd, 0);
        }
        let copied = state.clone();
        assert_eq!(copied.simd.mxcsr() & 0x3f, 0x20);
        let image = simd::fxsave(&state, 0);
        assert_eq!(
            u32::from_le_bytes(image[24..28].try_into().unwrap()),
            0x1fa0
        );
        assert!(state.simd.set_mxcsr(0x1f80));
        assert_eq!(state.simd.mxcsr() & 0x3f, 0);
        simd::fxrstor(&mut state, &image, false).unwrap();
        assert_eq!(state.simd.mxcsr() & 0x3f, 0x20);
        let stored = simd::store(&mut state, SimdOp::STMXCSR_Md, 0);
        assert_eq!(&stored[..4], &0x1fa0u32.to_le_bytes());
    }

    #[test]
    fn common_c06_lanes_saturate_and_shuffle_without_crossing_boundaries() {
        let mut state = ProcessorState::latest();
        state.simd.xmm[0] = [250; 16];
        state.simd.xmm[1] = [10; 16];
        step(&mut state, SimdOp::PADDUSB_Vo_Wo, 0);
        assert_eq!(state.simd.xmm[0], [255; 16]);
        state.simd.xmm[1] = [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0];
        step(&mut state, SimdOp::PSHUFD_Vo_Wo_Ib, 0x1b);
        assert_eq!(
            state.simd.xmm[0],
            [4, 0, 0, 0, 3, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0]
        );
    }

    #[test]
    fn common_c06_integer_conversion_distinguishes_rounding_truncation_and_invalid() {
        for (rc, positive, negative) in [(0, 2i32, -2i32), (1, 2, -3), (2, 3, -2), (3, 2, -2)] {
            for (bits, expected, truncated) in
                [(0x40200000u32, positive, 2i32), (0xc0200000, negative, -2)]
            {
                for (op, expected) in [
                    (SimdOp::CVTSS2SI_Gd_Wd, expected),
                    (SimdOp::CVTTSS2SI_Gd_Wd, truncated),
                ] {
                    let mut state = ProcessorState::latest();
                    state.simd.mxcsr = 0x1f80 | (rc << 13);
                    state.simd.xmm[1][..4].copy_from_slice(&bits.to_le_bytes());
                    step(&mut state, op, 0);
                    assert_eq!(
                        state.eax().read(),
                        expected as u32,
                        "op={op:?} bits={bits:x} rc={rc}"
                    );
                    assert_eq!(state.simd.mxcsr, 0x1fa0 | (rc << 13));
                }
            }
        }
        for (bits, expected, flags) in [
            (0x4effffffu32, 0x7fffff80u32, 0), // largest f32 below +2^31
            (0xcf000000, 0x80000000, 0),       // -2^31 is valid
            (0x4f000000, 0x80000000, 1),       // +2^31 is invalid
            (0x7fc00000, 0x80000000, 1),       // quiet NaN is invalid for integer conversion
        ] {
            let mut state = ProcessorState::latest();
            state.simd.xmm[1][..4].copy_from_slice(&bits.to_le_bytes());
            step(&mut state, SimdOp::CVTTSS2SI_Gd_Wd, 0);
            assert_eq!(
                (state.eax().read(), state.simd.mxcsr & 0x3f),
                (expected, flags),
                "bits={bits:x}"
            );
        }
    }

    #[test]
    fn common_c06_float_rounding_nan_and_denormal_modes() {
        // 1 + 2^-24 is exactly halfway between two f32 values.
        for (mxcsr, lhs, rhs, result, flags) in [
            (0x1f80, 0x3f800000u32, 0x33800000u32, 0x3f800000u32, 0x20),
            (0x5f80, 0x3f800000, 0x33800000, 0x3f800001, 0x20),
            (0x1fc0, 1, 0, 0, 0), // DAZ: input subnormal becomes zero
            (0x9f80, 0x00800000, 0x3f000000, 0, 0x30), // FTZ: min-normal * .5
            (0x1f80, 0x7f800001, 0x3f800000, 0x7fc00001, 1), // sNaN quieted
        ] {
            let mut state = ProcessorState::latest();
            state.simd.mxcsr = mxcsr;
            state.simd.xmm[0][..4].copy_from_slice(&lhs.to_le_bytes());
            state.simd.xmm[0][4..].fill(0xa5);
            state.simd.xmm[1][..4].copy_from_slice(&rhs.to_le_bytes());
            let op = if mxcsr == 0x9f80 {
                SimdOp::MULSS_Vd_Wd
            } else {
                SimdOp::ADDSS_Vd_Wd
            };
            step(&mut state, op, 0);
            assert_eq!(
                &state.simd.xmm[0][..4],
                &result.to_le_bytes(),
                "mxcsr={mxcsr:x}"
            );
            assert_eq!(&state.simd.xmm[0][4..], &[0xa5; 12]);
            assert_eq!(state.simd.mxcsr & 0x3f, flags, "mxcsr={mxcsr:x}");
        }
    }

    #[test]
    fn common_c06_fx_state_has_fixed_offsets_and_rejects_invalid_mxcsr_atomically() {
        let mut state = ProcessorState::latest();
        state.fpu.push(crate::fpu::FpuValue::F64(1.0));
        state.simd.xmm[3] = [0x5a; 16];
        let image = simd::fxsave(&state, 0x123);
        assert_eq!(&image[..2], &[0x7f, 3]);
        assert_eq!(image[4], 0x80);
        assert_eq!(&image[24..28], &[0x80, 0x1f, 0, 0]);
        assert_eq!(&image[208..224], &[0x5a; 16]);
        let mut restored = ProcessorState::latest();
        simd::fxrstor(&mut restored, &image, false).unwrap();
        assert_eq!(restored.simd.xmm[3], [0x5a; 16]);
        assert_eq!(
            restored.fpu.st(0).unwrap().raw(),
            crate::fpu::Raw80::from_i64(1)
        );
        let before = simd::fxsave(&restored, 0x123);
        // FCW and XMM3 also differ, so a partial restore before the check is visible.
        let mut invalid = image;
        invalid[0] = 0x7e;
        invalid[208..224].fill(0xa5);
        invalid[26] = 1;
        assert_eq!(
            simd::fxrstor(&mut restored, &invalid, false),
            Err(SimdFault::GeneralProtection)
        );
        assert_eq!(simd::fxsave(&restored, 0x123), before);
        simd::execute(
            &mut restored,
            SimdOp::MOVQ_Pq_Qq,
            0,
            Rm::Mem(&[7; 8]),
            0,
            false,
        )
        .unwrap();
        assert_eq!(restored.fpu.tag_word(), 0);
        assert_eq!(
            restored.fpu.registers[0].raw().significand,
            0x0707070707070707
        );
    }
}

#[cfg(feature = "uop-profile")]
#[test]
fn common_c07_profile_counts_internal_dispatches_without_creating_rows() {
    let mut counters = crate::profile_support::Counters::default();
    counters.record(Some((16, true)));
    counters.record(Some((16, false)));
    counters.record(None);
    let uops = [1u8, 2];
    assert_eq!(
        counters.tsv([(16, "ADD", uops.as_slice()), (32, "NOP", &[])].into_iter()),
        "# dispatches=3 faults=0 retries=0 string_elements=0 rep_elements=0\neip\tentries\tdispatches\tir\tuops\n16\t1\t2\tADD\t[1, 2]\n"
    );
}

#[test]
fn common_c02_decimal_adjustments_use_the_original_carry_and_af() {
    use crate::alu::Alu;
    use crate::cpu::ProcessorState;
    use crate::gpr::PartialRegister;
    use crate::prelude::*;

    // (case, adjustment, AX, input flags, AX after, defined flags mask, flags after)
    type Adjust = fn(&mut Alu) -> Option<()>;
    #[rustfmt::skip]
    let cases: [(&str, Adjust, u16, Flags, u16, u32, u32); 12] = [
        ("DAA 79+35", |a| Some(a.decimal_adjust(false)), 0xae, Flags::ZERO, 0x14, 0xd5, 0x15),
        ("DAA CF only", |a| Some(a.decimal_adjust(false)), 0, Flags::CF, 0x60, 0xd5, 0x05),
        ("DAS 35-47", |a| Some(a.decimal_adjust(true)), 0xee, Flags::CF | Flags::AF, 0x88, 0xd5, 0x95),
        ("DAS low borrow", |a| Some(a.decimal_adjust(true)), 0, Flags::AF, 0xfa, 0xd5, 0x95),
        ("AAA 9+8", |a| Some(a.ascii_adjust(false)), 0x11, Flags::AF, 0x107, 0x11, 0x11),
        ("AAA none", |a| Some(a.ascii_adjust(false)), 0x105, Flags::ZERO, 0x105, 0x11, 0),
        ("AAS 102-5", |a| Some(a.ascii_adjust(true)), 0x1fd, Flags::AF, 0x7, 0x11, 0x11),
        ("AAM 10", |a| a.aam(10), 79, Flags::ZERO, 0x709, 0xc4, 0x04),
        ("AAM 0 is #DE", |a| a.aam(0), 0x1234, Flags::CF, 0x1234, 0xd5, 0x01),
        ("AAD 10", |a| Some(a.aad(10)), 0x709, Flags::ZERO, 0x4f, 0xc4, 0),
        ("AAD 16", |a| Some(a.aad(16)), 0x203, Flags::ZERO, 0x23, 0xc4, 0),
        ("AAD wraps", |a| Some(a.aad(16)), 0x880, Flags::ZERO, 0, 0xc4, 0x44),
    ];
    for (name, adjust, ax, input, ax_after, mask, flags) in cases {
        let mut state = ProcessorState::latest();
        state.ax().write(ax);
        state.flags_mut().set_bits(input);
        let result = adjust(&mut Alu::new(&mut state));
        assert_eq!(result.is_none(), name.ends_with("#DE"), "{name}");
        assert_eq!(state.ax().read(), ax_after, "{name}");
        assert_eq!(state.flags_mut().resolve().bits() & mask, flags, "{name}");
    }
}
