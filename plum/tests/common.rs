//! Public common-component contracts. No engine is constructed here.
use plum::alu::Alu;
use plum::cpu::ProcessorState;
use plum::gpr::{GeneralPurposeRegister, PartialRegister};
use plum::prelude::*;

fn arithmetic_flags(state: &mut ProcessorState) -> u32 {
    state.flags_mut().resolve().bits() & 0x8d5
}

#[test]
fn c01_partial_registers_preserve_other_bits() {
    let reg = GeneralPurposeRegister::new(0x12345678);
    reg.h().write(0xab);
    assert_eq!(reg.e().read(), 0x1234ab78);
    reg.l().write(0xcd);
    assert_eq!(reg.x().read(), 0xabcd);
    reg.x().write(0xef01);
    assert_eq!(reg.e().read(), 0x1234ef01);
    let cloned = reg.clone();
    reg.e().write(0);
    assert_eq!(cloned.e().read(), 0x1234ef01);
}

#[test]
fn c01_state_reset_and_clone_are_independent() {
    let mut state = ProcessorState::latest();
    state.eax().write(0x1234);
    let cloned = state.clone();
    state.init();
    assert_eq!(state.eax().read(), 0);
    assert_eq!(state.eip().read(), 0xfff0);
    assert_eq!(cloned.eax().read(), 0x1234);
}

#[test]
fn c02_arithmetic_resolves_defined_flags() {
    let mut state = ProcessorState::latest();
    assert_eq!(Alu::new(&mut state).add8(0xff, 1), 0);
    assert_eq!(arithmetic_flags(&mut state), 0x55); // CF/PF/AF/ZF
    assert_eq!(Alu::new(&mut state).add16(0x7fff, 1), 0x8000);
    assert_eq!(arithmetic_flags(&mut state), 0x894); // PF/AF/SF/OF
    assert_eq!(Alu::new(&mut state).sub32(0, 1), 0xffff_ffff);
    assert_eq!(arithmetic_flags(&mut state), 0x95); // CF/PF/AF/SF
}

#[test]
fn c02_lazy_carry_survives_partial_updates() {
    let mut state = ProcessorState::latest();
    Alu::new(&mut state).add8(0xff, 1);
    assert_eq!(Alu::new(&mut state).inc8(0x7f), 0x80);
    assert_eq!(arithmetic_flags(&mut state), 0x891);
    assert_eq!(Alu::new(&mut state).adc8(0, 0), 1);
    assert_eq!(arithmetic_flags(&mut state), 0);
    Alu::new(&mut state).sub8(0, 1);
    assert_eq!(Alu::new(&mut state).sbb16(0, 0), 0xffff);
    assert_eq!(arithmetic_flags(&mut state), 0x95);
    assert_eq!(Alu::new(&mut state).dec32(1), 0);
    assert_eq!(arithmetic_flags(&mut state), 0x45);
}

#[test]
fn c02_shift_counts_and_rotate_preserve_unaffected_flags() {
    let mut state = ProcessorState::latest();
    state.flags_mut().set_bits(Flags::CF | Flags::ZF);
    assert_eq!(Alu::new(&mut state).shl32(0x1234, 32), 0x1234);
    assert_eq!(arithmetic_flags(&mut state), 0x41);
    assert_eq!(Alu::new(&mut state).shl8(0x81, 1), 2);
    assert_eq!(arithmetic_flags(&mut state) & 0x8c5, 0x801);
    state.flags_mut().set_bits(Flags::ZF);
    assert_eq!(Alu::new(&mut state).rol8(0x81, 1), 3);
    assert_eq!(arithmetic_flags(&mut state) & 0x841, 0x841);
    assert_eq!(Alu::new(&mut state).sar16(0x8000, 15), 0xffff);
    assert_eq!(arithmetic_flags(&mut state) & 0xc5, 0x84);
}

const CONDITIONS: [(CC, CC); 8] = [
    (CC::O, CC::NO),
    (CC::C, CC::NC),
    (CC::Z, CC::NZ),
    (CC::BE, CC::NBE),
    (CC::S, CC::NS),
    (CC::P, CC::NP),
    (CC::L, CC::NL),
    (CC::LE, CC::NLE),
];

#[test]
fn c02_rotates_match_single_bit_steps_for_all_masked_counts() {
    macro_rules! check_width {
        ($ty:ty, $rol:ident, $ror:ident, $rcl:ident, $rcr:ident) => {
            for (through_carry, right) in
                [(false, false), (false, true), (true, false), (true, true)]
            {
                for count in 0..=63u8 {
                    for initial_cf in [false, true] {
                        let mut state = ProcessorState::latest();
                        // Leave SF, PF, AF and OF pending across the rotate.
                        Alu::new(&mut state).add8(0x7f, 1);
                        state.flags_mut().set_dynamic(Flags::CF, initial_cf);
                        let input = 0x82b3_1a95u32 as $ty;
                        let mut expected = input;
                        let mut cf = initial_cf;
                        let masked = u32::from(count & 31);
                        let steps = if through_carry {
                            masked % (<$ty>::BITS + 1)
                        } else {
                            masked
                        };
                        for _ in 0..steps {
                            let outgoing = if right {
                                expected & 1 != 0
                            } else {
                                expected >> (<$ty>::BITS - 1) != 0
                            };
                            let incoming = if through_carry { cf } else { outgoing } as $ty;
                            expected = if right {
                                (expected >> 1) | (incoming << (<$ty>::BITS - 1))
                            } else {
                                (expected << 1) | incoming
                            };
                            cf = outgoing;
                        }
                        let mut alu = Alu::new(&mut state);
                        let actual = match (through_carry, right) {
                            (false, false) => alu.$rol(input, count),
                            (false, true) => alu.$ror(input, count),
                            (true, false) => alu.$rcl(input, count),
                            (true, true) => alu.$rcr(input, count),
                        };
                        let of = masked != 1
                            || if right {
                                (expected ^ (expected << 1)) >> (<$ty>::BITS - 1) != 0
                            } else {
                                (expected >> (<$ty>::BITS - 1) != 0) != cf
                            };
                        assert_eq!(actual, expected);
                        assert_eq!(
                            arithmetic_flags(&mut state),
                            0x90 | u32::from(cf) | (u32::from(of) << 11)
                        );
                    }
                }
            }
        };
    }
    check_width!(u8, rol8, ror8, rcl8, rcr8);
    check_width!(u16, rol16, ror16, rcl16, rcr16);
    check_width!(u32, rol32, ror32, rcl32, rcr32);
}

#[test]
fn c02_every_condition_follows_each_lazy_result_family() {
    type Op = fn(&mut Alu);
    // (case, CF before, operation, expected O C Z BE S P L LE); all five flags are defined.
    #[rustfmt::skip]
    let cases: [(&str, bool, Op, [u8; 8]); 14] = [
        ("SUB32 1-2", false, |a| _ = a.sub32(1, 2), [0, 1, 0, 1, 1, 1, 1, 1]),
        ("SUB8 7F-81", false, |a| _ = a.sub8(0x7f, 0x81), [1, 1, 0, 1, 1, 0, 0, 0]),
        ("SUB16 5-5", false, |a| _ = a.sub16(5, 5), [0, 0, 1, 1, 0, 1, 0, 1]),
        ("ADD16 7FFF+1", false, |a| _ = a.add16(0x7fff, 1), [1, 0, 0, 0, 1, 1, 0, 0]),
        ("ADC32 FFFFFFFF+0+CF", true, |a| _ = a.adc32(u32::MAX, 0), [0, 1, 1, 1, 0, 1, 0, 1]),
        ("SBB8 80-0-CF", true, |a| _ = a.sbb8(0x80, 0), [1, 0, 0, 0, 0, 0, 1, 1]),
        ("AND8 81&80", false, |a| _ = a.and8(0x81, 0x80), [0, 0, 0, 0, 1, 0, 1, 1]),
        ("OR32 40|1", false, |a| _ = a.or32(0x40, 1), [0, 0, 0, 0, 0, 1, 0, 0]),
        ("XOR16 FFFF^7FFF", false, |a| _ = a.xor16(0xffff, 0x7fff), [0, 0, 0, 0, 1, 1, 1, 1]),
        ("INC8 7F keeps CF", true, |a| _ = a.inc8(0x7f), [1, 1, 0, 1, 1, 0, 0, 0]),
        ("DEC32 80000000", false, |a| _ = a.dec32(0x8000_0000), [1, 0, 0, 0, 0, 1, 1, 1]),
        ("SHR8 81>>1", false, |a| _ = a.shr8(0x81, 1), [1, 1, 0, 1, 0, 0, 1, 1]),
        ("SAR32 80000001>>1", false, |a| _ = a.sar32(0x8000_0001, 1), [0, 1, 0, 1, 1, 1, 1, 1]),
        ("SHL16 4000<<1", false, |a| _ = a.shl16(0x4000, 1), [1, 0, 0, 0, 1, 1, 0, 0]),
    ];
    for (name, carry, op, expected) in cases {
        let mut state = ProcessorState::latest();
        if carry {
            state.flags_mut().set_bits(Flags::CF);
        }
        op(&mut Alu::new(&mut state));
        let unresolved = state.clone();
        for ((cc, negated), want) in CONDITIONS.into_iter().zip(expected) {
            // Exercise a branch immediately after the ALU operation, before
            // any earlier condition in this table has resolved SF or OF.
            let mut fresh = unresolved.clone();
            assert_eq!(fresh.eval_cc(cc), want == 1, "{name}: fresh {cc:?}");
            assert_eq!(state.eval_cc(cc), want == 1, "{name}: {cc:?}");
            assert_eq!(state.eval_cc(negated), want == 0, "{name}: {negated:?}");
        }
        // Evaluating conditions must not disturb the materialized image.
        let [o, c, z, _, s, p, _, _] = expected.map(u32::from);
        let image = o << 11 | s << 7 | z << 6 | p << 2 | c;
        assert_eq!(state.flags_mut().resolve().bits() & 0x8c5, image, "{name}");
    }
    for flag in [Flags::SF, Flags::OF] {
        let mut state = ProcessorState::latest();
        Alu::new(&mut state).sub32(1, 0); // SF=OF=0 before the override.
        state.flags_mut().set_dynamic(flag, true);
        assert!(state.eval_cc(CC::L));
        assert!(!state.eval_cc(CC::NLE));
    }
}

#[test]
fn c02_multiply_defines_only_carry_and_overflow_conditions() {
    let mut state = ProcessorState::latest();
    let carry_overflow = |state: &mut ProcessorState| {
        let pair = (state.eval_cc(CC::C), state.eval_cc(CC::O));
        assert_eq!(
            state.flags_mut().resolve().bits() & 0x801,
            pair.0 as u32 | (pair.1 as u32) << 11
        );
        pair
    };
    assert_eq!(Alu::new(&mut state).mul8(0x10, 0x10), 0x100);
    assert_eq!(carry_overflow(&mut state), (true, true));
    assert_eq!(Alu::new(&mut state).mul16(2, 3), 6);
    assert_eq!(carry_overflow(&mut state), (false, false));
    assert_eq!(Alu::new(&mut state).mul32(0x10000, 0x10000), 0x1_0000_0000);
    assert_eq!(carry_overflow(&mut state), (true, true));
    assert_eq!(Alu::new(&mut state).imul8(16, 8), -128);
    assert_eq!(carry_overflow(&mut state), (true, true));
    assert_eq!(Alu::new(&mut state).imul8(-2, 3), -6);
    assert_eq!(carry_overflow(&mut state), (false, false));
    assert_eq!(Alu::new(&mut state).imul16(300, 300), 0x5f90);
    assert_eq!(carry_overflow(&mut state), (true, true));
    assert_eq!(Alu::new(&mut state).imul32(-65536, 65536), 0);
    assert_eq!(carry_overflow(&mut state), (true, true));
}

#[test]
fn c02_remaining_shifts_rotates_and_double_shifts_have_fixed_results() {
    let mut state = ProcessorState::latest();
    assert_eq!(Alu::new(&mut state).shr32(0x8000_0001, 4), 0x0800_0000);
    assert_eq!(arithmetic_flags(&mut state) & 0xc5, 0x04);
    assert_eq!(Alu::new(&mut state).shr16(0x8001, 1), 0x4000);
    assert_eq!(arithmetic_flags(&mut state) & 0x8c5, 0x805); // OF = original MSB
    assert_eq!(Alu::new(&mut state).sar8(0x81, 1), 0xc0);
    assert_eq!(arithmetic_flags(&mut state) & 0x8c5, 0x85);

    // Rotates change only CF (and OF for a count of one).
    state.flags_mut().set_bits(Flags::ZF);
    assert_eq!(Alu::new(&mut state).ror8(1, 1), 0x80);
    assert_eq!(arithmetic_flags(&mut state), 0x841);
    assert_eq!(Alu::new(&mut state).ror32(1, 4), 0x1000_0000);
    assert_eq!(arithmetic_flags(&mut state) & 0x41, 0x40);
    // RCL/RCR rotate through CF modulo width + 1 after masking the count to 5 bits.
    state.flags_mut().set_bits(Flags::CF);
    assert_eq!(Alu::new(&mut state).rcl8(0x55, 9), 0x55);
    assert_eq!(Alu::new(&mut state).rcr16(0x1234, 17), 0x1234);
    assert_eq!(arithmetic_flags(&mut state), 0x01);
    state.flags_mut().set_bits(Flags::ZERO);
    assert_eq!(Alu::new(&mut state).rcl8(0x80, 1), 0);
    assert_eq!(arithmetic_flags(&mut state), 0x801);
    state.flags_mut().set_bits(Flags::ZERO);
    assert_eq!(Alu::new(&mut state).rcr8(1, 10), 0);
    assert_eq!(arithmetic_flags(&mut state) & 1, 1);
    state.flags_mut().set_bits(Flags::CF);
    assert_eq!(Alu::new(&mut state).rcr16(1, 2), 0xc000);
    assert_eq!(arithmetic_flags(&mut state) & 1, 0);
    state.flags_mut().set_bits(Flags::CF);
    assert_eq!(Alu::new(&mut state).rcl32(0x4000_0000, 33), 0x8000_0001);
    assert_eq!(arithmetic_flags(&mut state), 0x800);

    assert_eq!(Alu::new(&mut state).shld16(0x1234, 0xabcd, 4), 0x234a);
    assert_eq!(arithmetic_flags(&mut state) & 0xc5, 0x01);
    assert_eq!(Alu::new(&mut state).shld32(0x4000_0000, 0, 1), 0x8000_0000);
    assert_eq!(arithmetic_flags(&mut state) & 0x8c5, 0x884);
    assert_eq!(
        Alu::new(&mut state).shrd32(0x1234_5678, 0x9abc_def0, 8),
        0xf012_3456
    );
    assert_eq!(arithmetic_flags(&mut state) & 0xc5, 0x84);
    assert_eq!(Alu::new(&mut state).shrd16(1, 0x8000, 1), 0);
    assert_eq!(arithmetic_flags(&mut state) & 0x8c5, 0x45);

    // Count > 1 leaves OF at its previous value in this emulator; AF also
    // remains available from the preceding ADD after the other flags change.
    Alu::new(&mut state).add8(0x7f, 1);
    assert_eq!(Alu::new(&mut state).shld32(1, 0, 2), 4);
    assert_eq!(arithmetic_flags(&mut state), 0x810);
}
