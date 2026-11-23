//! Contracts for the opt-in Harvard trace rewrite.
#![cfg(feature = "flag-elision")]

use plum::gpr::PartialRegister;
use plum::harvard::HarvardUME;
use plum::harvard::tracer::TraceDecoder;
use plum::harvard::uop::Uop;
use plum::prelude::*;

fn machine(bytes: &[u8]) -> HarvardUME {
    plum::ume::harvard(bytes.into(), vec![0; 64].into(), Offset32(0), Offset32(64))
}

fn stop(cpu: &mut HarvardUME) {
    assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
}

#[test]
fn consecutive_adds_keep_register_values_and_the_last_flags() {
    let mut cpu = machine(&[
        0x90, 0x01, 0xc8, 0x01, 0xc8, 0x01, 0xc8, 0x9c, 0x5b, 0xcd, 0x40,
    ]);
    cpu.state().eax().write(u32::MAX - 2);
    cpu.state().ecx().write(1);
    stop(&mut cpu);
    assert_eq!(cpu.state().eax().read(), 0);
    assert_eq!(cpu.state().ebx().read() & 0x8d5, 0x55);
    assert_eq!(
        cpu.tracer()
            .uop_cache()
            .iter()
            .filter(|op| matches!(op, Uop::ValueAddR(..)))
            .count(),
        2
    );
}

#[test]
fn carry_and_fault_boundaries_are_observable() {
    for middle in [&[0x41][..], &[0xc1, 0xe1, 0][..]] {
        // INC ECX or SHL ECX,0
        let mut code = vec![0x90, 0x83, 0xc0, 1];
        code.extend_from_slice(middle);
        code.extend_from_slice(&[0x83, 0xd3, 0, 0xcd, 0x40]); // ADC EBX,0
        let mut cpu = machine(&code);
        cpu.state().eax().write(u32::MAX);
        stop(&mut cpu);
        assert_eq!(cpu.state().ebx().read(), 1);
    }
    let mut cpu = machine(&[0x90, 0x83, 0xc0, 1, 0x3b, 0x0a, 0xcd, 0x40]);
    cpu.state().eax().write(u32::MAX);
    cpu.state().edx().write(64);
    assert!(matches!(
        cpu.execute(),
        Err(Exception::SegmentationViolation(Offset32(64)))
    ));
    assert_eq!(cpu.state().eip().read(), 4);
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x55);
    cpu.state().edx().write(0);
    stop(&mut cpu);
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x44);
}

#[test]
fn decode_boundaries_leave_the_last_add_intact() {
    for tail in [&[][..], &[0x81, 0xf8, 1][..]] {
        let mut code = vec![0x90, 0x83, 0xc0, 1];
        code.extend_from_slice(tail);
        let mut tracer = TraceDecoder::new(code.into(), Offset32(0));
        tracer.fetch_and_decode();
        assert!(matches!(tracer.uop_cache().last(), Some(Uop::AddI(..))));
    }
    let mut tracer = TraceDecoder::new(vec![0x83, 0xc0, 1, 0x39, 0xc0, 0xcc].into(), Offset32(0));
    tracer.fetch_and_decode();
    assert!(matches!(tracer.uop_cache()[0], Uop::AddI(..)));
}

#[test]
fn alternate_entry_at_cmp_preserves_the_add_result() {
    // Resume after INT, jump directly to the CMP already present in the cache.
    let mut cpu = machine(&[0x90, 0x83, 0xc0, 1, 0x39, 0xc0, 0xcd, 0x40, 0xeb, 0xfa]);
    stop(&mut cpu);
    cpu.state().eax().write(7);
    cpu.resume_next();
    stop(&mut cpu);
    assert_eq!(cpu.state().eax().read(), 7);
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x44);
}

#[test]
fn narrow_arithmetic_and_transparent_register_moves_preserve_upper_bits() {
    // ADD EAX,1; MOV EDX,EAX; SUB CL,1; ADD CH,1; INT.
    let mut cpu = machine(&[
        0x90, 0x83, 0xc0, 1, 0x89, 0xc2, 0x80, 0xe9, 1, 0x80, 0xc5, 1, 0xcd, 0x40,
    ]);
    cpu.state().eax().write(0x7fff_ffff);
    cpu.state().ecx().write(0x1234_7f00);
    stop(&mut cpu);
    assert_eq!(cpu.state().edx().read(), 0x8000_0000);
    assert_eq!(cpu.state().ecx().read(), 0x1234_80ff);
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x890);
    assert!(
        cpu.tracer()
            .uop_cache()
            .iter()
            .any(|op| matches!(op, Uop::ValueAddI(..)))
    );
}

#[test]
fn dword_value_forms_keep_results_and_execute_the_following_compare() {
    // Start each guest with a lazy carry from ADD EBX,1. The tested instruction
    // writes EAX, then CMP EAX,EAX and ADD EDI,1 both overwrite the flags.
    for (bytes, expected) in [
        (&[0x01, 0xc8][..], 0x80000004),
        (&[0x83, 0xc0, 3], 0x80000004), // ADD
        (&[0x29, 0xc8], 0x7ffffffe),
        (&[0x83, 0xe8, 3], 0x7ffffffe), // SUB
        (&[0x11, 0xc8], 0x80000005),
        (&[0x83, 0xd0, 3], 0x80000005), // ADC
        (&[0x19, 0xc8], 0x7ffffffd),
        (&[0x83, 0xd8, 3], 0x7ffffffd), // SBB
        (&[0x21, 0xc8], 1),
        (&[0x83, 0xe0, 3], 1), // AND
        (&[0x09, 0xc8], 0x80000003),
        (&[0x83, 0xc8, 3], 0x80000003), // OR
        (&[0x31, 0xc8], 0x80000002),
        (&[0x83, 0xf0, 3], 0x80000002), // XOR
        (&[0x40], 0x80000002),
        (&[0x48], 0x80000000),
        (&[0xf7, 0xd8], 0x7fffffff), // INC/DEC/NEG
        (&[0xd3, 0xe0], 8),
        (&[0xc1, 0xe0, 35], 8), // SHL
        (&[0xd3, 0xe8], 0x10000000),
        (&[0xc1, 0xe8, 3], 0x10000000), // SHR
        (&[0xd3, 0xf8], 0xf0000000),
        (&[0xc1, 0xf8, 3], 0xf0000000), // SAR
        (&[0x0f, 0xaf, 0xc1], 0x80000003),
        (&[0x6b, 0xc1, 0xfd], 0xfffffff7), // IMUL
    ] {
        let mut code = vec![0x90, 0x83, 0xc3, 1];
        code.extend_from_slice(bytes);
        let cmp_eip = code.len() as u32;
        code.extend([0x39, 0xc0, 0x83, 0xc7, 1, 0xcd, 0x40]);
        let mut cpu = machine(&code);
        cpu.state().eax().write(0x80000001);
        cpu.state().ecx().write(3);
        cpu.state().ebx().write(u32::MAX);
        stop(&mut cpu);
        assert_eq!(cpu.state().eax().read(), expected, "{bytes:02x?}");
        let tracer = cpu.tracer();
        let op = |addr| tracer.uop_cache()[tracer.address_map()[&Offset32(addr)].0 as usize];
        assert!(
            matches!(
                op(4),
                Uop::ValueAddR(..)
                    | Uop::ValueAddI(..)
                    | Uop::ValueSubI(..)
                    | Uop::ValueSubR(..)
                    | Uop::ValueAndR(..)
                    | Uop::ValueOrR(..)
                    | Uop::ValueOrI(..)
                    | Uop::ValueXorI(..)
                    | Uop::ValueAndI(..)
                    | Uop::ValueXorR(..)
                    | Uop::LeaBD(..)
                    | Uop::ValueReg(..)
                    | Uop::ValueImm(..)
                    | Uop::ValueNeg(..)
                    | Uop::ValueMulR(..)
                    | Uop::ValueMulRI(..)
                    | Uop::ValueShift(..)
            ),
            "{bytes:02x?}"
        );
        assert!(matches!(op(cmp_eip), Uop::CmpR(..)));
    }
}

#[test]
fn shift_by_cl_preserves_live_flags_when_count_masks_to_zero() {
    // ADD EAX,1; SHR EDX,CL; ADC EBX,0; CMP EBX,1.
    let mut cpu = machine(&[
        0x90, 0x83, 0xc0, 1, 0xd3, 0xea, 0x83, 0xd3, 0, 0x83, 0xfb, 1, 0xcd, 0x40,
    ]);
    cpu.state().eax().write(u32::MAX);
    cpu.state().ecx().write(32);
    cpu.state().edx().write(2);
    stop(&mut cpu);
    assert_eq!(cpu.state().ebx().read(), 1);
    assert_eq!(cpu.state().edx().read(), 2);
    assert!(cpu.state_mut().flags_mut().resolve().contains(Flags::ZF));
}

#[test]
fn dword_add_sub_and_or_xor_lower_to_dedicated_register_and_immediate_uops() {
    let mut cpu = machine(&[
        0x90, 0x01, 0xc8, 0x83, 0xc0, 3, 0x29, 0xc8, 0x83, 0xe8, 7, 0x21, 0xc8, 0x83, 0xe0, 63,
        0x09, 0xc8, 0x83, 0xc8, 1, 0x31, 0xc8, 0x83, 0xf0, 5, 0x39, 0xc0, 0xcd, 0x40,
    ]);
    stop(&mut cpu);
    use Uop::*;
    assert!(matches!(
        cpu.tracer().uop_cache(),
        [
            Nop,
            ValueAddR(..),
            ValueAddI(..),
            ValueSubR(..),
            ValueSubI(..),
            ValueAndR(..),
            ValueAndI(..),
            ValueOrR(..),
            ValueOrI(..),
            ValueXorR(..),
            ValueXorI(..),
            CmpR(..),
            ..
        ]
    ));
}

#[test]
fn register_extensions_allow_elision_before_a_compare() {
    // MOVZX/MOVSX EDX or DX, CL or CX. A 16-bit destination retains its high word.
    for (extension, expected) in [
        (&[0x0f, 0xb6, 0xd1][..], 0x80),
        (&[0x0f, 0xbe, 0xd1], 0xffff_ff80),
        (&[0x0f, 0xb7, 0xd1], 0x8180),
        (&[0x0f, 0xbf, 0xd1], 0xffff_8180),
        (&[0x66, 0x0f, 0xb6, 0xd1], 0x1234_0080),
        (&[0x66, 0x0f, 0xbe, 0xd1], 0x1234_ff80),
    ] {
        let mut code = vec![0x90, 0x83, 0xc0, 1]; // NOP; ADD EAX,1
        code.extend_from_slice(extension);
        code.extend_from_slice(&[0x39, 0xc0, 0xcd, 0x40]); // CMP EAX,EAX; INT 40h
        let mut cpu = machine(&code);
        cpu.state().ecx().write(0x8180);
        cpu.state().edx().write(0x1234_dead);
        stop(&mut cpu);
        assert!(matches!(cpu.tracer().uop_cache()[1], Uop::ValueAddI(..)));
        assert_eq!(cpu.state().edx().read(), expected);
    }
}

#[test]
fn memory_extension_fault_exposes_preceding_flags() {
    for opcode in [0xb6, 0xbe, 0xb7, 0xbf] {
        let mut cpu = machine(&[
            0x90, 0x83, 0xc0, 1, 0x0f, opcode, 0x0a, 0x39, 0xc0, 0xcd, 0x40,
        ]);
        cpu.state().eax().write(u32::MAX);
        cpu.state().edx().write(64);
        assert!(matches!(
            cpu.execute(),
            Err(Exception::SegmentationViolation(Offset32(64)))
        ));
        assert_eq!(cpu.state().eip().read(), 4);
        assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x55);
    }
}
