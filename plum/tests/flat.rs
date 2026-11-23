//! Flat engine contracts. Guest addresses deliberately have a nonzero base.
use plum::flat::FlatUME;
use plum::gpr::PartialRegister;
use plum::prelude::*;

#[path = "support/registers.rs"]
mod registers;

#[path = "support/instructions.rs"]
mod instructions;

#[cfg(feature = "sse")]
#[path = "../examples/support/contracts/simd.rs"]
mod numeric_shared;
#[path = "../examples/support/contracts/flat.rs"]
mod shared;

#[cfg(feature = "sse")]
#[test]
fn f06_numeric_fixed_expectations_match_the_wasm_contract() {
    assert_eq!(numeric_shared::flat(), 2);
}

const BASE: u32 = 0x1000;

fn machine(code: &[u8]) -> FlatUME {
    with_memory(code, 512)
}
fn with_memory(code: &[u8], size: usize) -> FlatUME {
    let mut memory = vec![0; size];
    memory[..code.len()].copy_from_slice(code);
    memory[code.len()..code.len() + 2].copy_from_slice(&[0xcd, 0x40]);
    plum::ume::flat(
        memory,
        Offset32(BASE),
        Offset32(BASE),
        Offset32(BASE + size as u32),
    )
    .unwrap()
}
fn done(cpu: &mut FlatUME) {
    assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
}

#[test]
fn f01_dispatch_connects_carry_partial_registers_branch_and_rmw() {
    let mut cpu = machine(&[
        0xb8, 0xff, 0xff, 0xff, 0xff, 0x83, 0xc0, 1, 0x83, 0xd3, 0, 0x75, 1, 0x41, 0xb4, 7,
    ]);
    done(&mut cpu);
    assert_eq!(
        (cpu.state().eax().read(), cpu.state().ebx().read()),
        (0x700, 1)
    );
    assert_eq!(cpu.state().ecx().read(), 0);
    let mut cpu = machine(&[0x0f, 0xc1, 0]);
    cpu.state().eax().write(BASE + 128);
    cpu.write_memory32(BASE + 128, 7).unwrap();
    done(&mut cpu);
    assert_eq!(cpu.state().eax().read(), 7);
    assert_eq!(cpu.read_memory32(BASE + 128).unwrap(), BASE + 135);
}

#[test]
fn f01_representative_integer_forms_produce_fixed_results() {
    // Code at BASE, data at BASE + 0x800, stack at the end of memory.
    let data = BASE + 0x800;
    let mut cpu = with_memory(instructions::CODE, 0x1000);
    cpu.state().ebx().write(data + instructions::EBX_OFFSET);
    done(&mut cpu);
    instructions::check("flat", |slot| cpu.read_memory32(data + 4 * slot).unwrap());
}

#[test]
fn f02_boundary_fault_preserves_state_and_can_retry() {
    let mut cpu = machine(&[0x83, 0, 1]);
    cpu.state().eax().write(BASE + 510);
    let before = cpu.memory().to_vec();
    cpu.state_mut().flags_mut().set_bits(Flags::CF | Flags::ZF);
    assert!(
        matches!(cpu.execute(), Err(Exception::SegmentationViolation(Offset32(address))) if address==BASE+510)
    );
    assert_eq!(cpu.state().eip().read(), BASE);
    assert_eq!(cpu.state().eax().read(), BASE + 510);
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x41);
    assert_eq!(cpu.memory(), before);
    cpu.state().eax().write(BASE + 128);
    done(&mut cpu);
    assert_eq!(cpu.read_memory32(BASE + 128).unwrap(), 1);

    // MOVSX EAX,word [EAX+ECX*2+1] crosses the end of memory.
    let mut cpu = machine(&[0x0f, 0xbf, 0x44, 0x48, 1]);
    cpu.state().eax().write(BASE + 508);
    cpu.state().ecx().write(1);
    assert!(
        matches!(cpu.execute(), Err(Exception::SegmentationViolation(Offset32(address))) if address == BASE + 511)
    );
    assert_eq!(cpu.state().eax().read(), BASE + 508);
    assert_eq!(cpu.state().eip().read(), BASE);
    cpu.brk(BASE + 1024).unwrap();
    cpu.write_memory16(BASE + 511, 0x8001).unwrap();
    done(&mut cpu);
    assert_eq!(cpu.state().eax().read(), 0xffff8001);
}

#[test]
fn f02_stack_fault_and_growth_preserve_existing_memory() {
    let mut cpu = machine(&[0x50]);
    cpu.state().esp().write(BASE + 2);
    assert!(matches!(cpu.execute(), Err(Exception::StackFault)));
    assert_eq!(cpu.state().esp().read(), BASE + 2);
    let before = cpu.memory().to_vec();
    assert_eq!(cpu.brk(BASE + 1024), Ok(BASE + 1024));
    assert_eq!(&cpu.memory()[..512], before);
    assert_eq!(&cpu.memory()[512..], &[0; 512]);
    assert_eq!(cpu.brk(BASE + 128), Err(BASE + 1024));
}

#[test]
fn f02_pop_memory_uses_incremented_esp_but_commits_it_after_the_store() {
    let mut cpu = machine(&[0x8f, 0x44, 0x24, 4]);
    cpu.state().esp().write(BASE + 504);
    cpu.write_memory32(BASE + 504, 0x12345678).unwrap();
    cpu.state_mut().flags_mut().set_bits(Flags::CF | Flags::ZF);
    let before = cpu.memory().to_vec();
    assert!(
        matches!(cpu.execute(), Err(Exception::SegmentationViolation(Offset32(at))) if at == BASE + 512)
    );
    assert_eq!(cpu.state().esp().read(), BASE + 504);
    assert_eq!(cpu.state().eip().read(), BASE);
    assert_eq!(cpu.memory(), before);
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x41);
    cpu.state().esp().write(BASE + 500);
    cpu.write_memory32(BASE + 500, 0xabcdef01).unwrap();
    done(&mut cpu);
    assert_eq!(cpu.state().esp().read(), BASE + 504);
    assert_eq!(cpu.read_memory32(BASE + 508).unwrap(), 0xabcdef01);
    assert_eq!(cpu.read_memory32(BASE + 504).unwrap(), 0x12345678);
}

#[test]
fn f02_pop_checks_stack_source_before_destination_and_retries_each_fault() {
    let mut cpu = machine(&[0x8f, 0]);
    cpu.state().esp().write(BASE + 510);
    cpu.state().eax().write(BASE + 510);
    let before = cpu.memory().to_vec();
    assert!(matches!(cpu.execute(), Err(Exception::StackFault)));
    assert_eq!(cpu.state().esp().read(), BASE + 510);
    assert_eq!(cpu.memory(), before);
    cpu.state().esp().write(BASE + 128);
    cpu.write_memory32(BASE + 128, 0x12345678).unwrap();
    assert!(
        matches!(cpu.execute(), Err(Exception::SegmentationViolation(Offset32(at))) if at == BASE + 510)
    );
    assert_eq!(cpu.state().esp().read(), BASE + 128);
    assert_eq!(cpu.read_memory32(BASE + 128).unwrap(), 0x12345678);
    cpu.state().eax().write(BASE + 64);
    done(&mut cpu);
    assert_eq!(cpu.state().esp().read(), BASE + 132);
    assert_eq!(cpu.read_memory32(BASE + 64).unwrap(), 0x12345678);
}

#[test]
fn f02_xadd_fault_preserves_source_and_flags_at_each_operand_width() {
    // Nonzero bytes after narrow operands expose stores wider than BL/BX.
    for (width, bytes, source, exchanged, initial, expected) in [
        (
            1,
            &[0x0f, 0xc0, 0x18][..],
            0xaaaa0007u32,
            0xaaaa000b,
            0xc3b2a10bu32,
            0xc3b2a112u32,
        ),
        (
            2,
            &[0x66, 0x0f, 0xc1, 0x18],
            0xaaaa0007,
            0xaaaa000b,
            0xc3b2000b,
            0xc3b20012,
        ),
        (4, &[0x0f, 0xc1, 0x18], 7, 11, 11, 18),
    ] {
        let mut cpu = machine(bytes);
        let address = BASE + 513 - width;
        cpu.state().eax().write(address);
        cpu.state().ebx().write(source);
        cpu.state_mut().flags_mut().set_bits(Flags::CF | Flags::ZF);
        let before = cpu.memory().to_vec();
        assert!(
            matches!(cpu.execute(), Err(Exception::SegmentationViolation(Offset32(at))) if at == address),
            "width={width}"
        );
        assert_eq!(cpu.state().ebx().read(), source, "width={width}");
        assert_eq!(cpu.state().eip().read(), BASE);
        assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x41);
        assert_eq!(cpu.memory(), before);
        cpu.state().eax().write(BASE + 128);
        cpu.write_bytes(BASE + 127, &[0x5a]).unwrap();
        cpu.write_bytes(BASE + 132, &[0xa5]).unwrap();
        cpu.write_memory32(BASE + 128, initial).unwrap();
        done(&mut cpu);
        assert_eq!(cpu.state().ebx().read(), exchanged, "width={width}");
        assert_eq!(
            cpu.read_memory32(BASE + 128).unwrap(),
            expected,
            "width={width}"
        );
        assert_eq!(
            (cpu.memory()[127], cpu.memory()[132]),
            (0x5a, 0xa5),
            "width={width}"
        );
        assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x14);
    }
}

#[test]
fn f03_host_code_update_replaces_cached_instruction() {
    assert_eq!(shared::code_update(), 1);
}

#[test]
fn f03_guest_self_modification_invalidates_without_replaying_stores() {
    let mut code = vec![0xff, 5]; // INC [BASE+128]
    code.extend((BASE + 128).to_le_bytes());
    code.extend([0xc6, 5]); // MOV byte [next MOV immediate],2
    code.extend((BASE + 14).to_le_bytes());
    code.extend([2, 0xb8, 1, 0, 0, 0]);
    let mut cpu = machine(&code);
    done(&mut cpu);
    assert_eq!(cpu.state().eax().read(), 2);
    assert_eq!(cpu.read_memory32(BASE + 128).unwrap(), 1);
}

#[test]
fn f04_call_return_and_cached_ret_follow_stack_and_host_resume() {
    let mut cpu = machine(&[0xe8, 3, 0, 0, 0, 0xcd, 0x40, 0x90, 0x43, 0xc3]);
    done(&mut cpu);
    assert_eq!(
        (
            cpu.state().ebx().read(),
            cpu.state().esp().read(),
            cpu.state().eip().read()
        ),
        (1, BASE + 512, BASE + 5)
    );
    // Stack writes use a separate page so the cached RET is retained.
    let mut cpu = with_memory(
        &[0xc3, 0x43, 0xcd, 0x40, 0x43, 0xcd, 0x40, 0xeb, 0xf7],
        8192,
    );
    cpu.state().esp().write(BASE + 4096);
    cpu.write_memory32(BASE + 4096, BASE + 1).unwrap();
    done(&mut cpu);
    assert_eq!(cpu.state().ebx().read(), 1);
    cpu.resume_next();
    done(&mut cpu);
    assert_eq!(cpu.state().ebx().read(), 2);
    cpu.state().esp().write(BASE + 4096);
    cpu.write_memory32(BASE + 4096, BASE + 4).unwrap();
    cpu.resume_next();
    done(&mut cpu);
    assert_eq!(cpu.state().ebx().read(), 3);
}

#[test]
fn f05_rep_fault_keeps_progress_and_can_continue_with_repaired_source() {
    let mut cpu = machine(&[0xf3, 0xa4]);
    cpu.write_bytes(BASE + 510, &[1, 2]).unwrap();
    cpu.state().esi().write(BASE + 510);
    cpu.state().edi().write(BASE + 128);
    cpu.state().ecx().write(4);
    assert!(matches!(
        cpu.execute(),
        Err(Exception::SegmentationViolation(_))
    ));
    assert_eq!(
        (
            cpu.state().esi().read(),
            cpu.state().edi().read(),
            cpu.state().ecx().read()
        ),
        (BASE + 512, BASE + 130, 2)
    );
    assert_eq!(cpu.state().eip().read(), BASE);
    cpu.write_bytes(BASE + 256, &[9, 10]).unwrap();
    cpu.state().esi().write(BASE + 256);
    done(&mut cpu);
    assert_eq!(&cpu.memory()[128..132], &[1, 2, 9, 10]);
}

#[test]
fn f05_rep_code_write_updates_following_instruction_and_zero_is_noop() {
    let mut cpu = machine(&[0xf3, 0xa4, 0xb8, 1, 0, 0, 0]);
    cpu.write_bytes(BASE + 128, &[2]).unwrap();
    cpu.state().esi().write(BASE + 128);
    cpu.state().edi().write(BASE + 3);
    cpu.state().ecx().write(1);
    done(&mut cpu);
    assert_eq!(cpu.state().eax().read(), 2);
    let mut cpu = machine(&[0xf3, 0xa4]);
    cpu.state().esi().write(u32::MAX);
    cpu.state().edi().write(u32::MAX);
    let before = cpu.memory().to_vec();
    done(&mut cpu);
    assert_eq!(cpu.memory(), before);
    assert_eq!(
        (
            cpu.state().esi().read(),
            cpu.state().edi().read(),
            cpu.state().ecx().read()
        ),
        (u32::MAX, u32::MAX, 0)
    );
}

#[test]
fn f05_repe_cmps_fault_preserves_entry_flags_then_retries_the_remainder() {
    let mut cpu = machine(&[0xf3, 0xa6]);
    cpu.write_bytes(BASE + 510, &[7, 7]).unwrap();
    cpu.write_bytes(BASE + 128, &[7, 7, 9, 9]).unwrap();
    cpu.state().esi().write(BASE + 510);
    cpu.state().edi().write(BASE + 128);
    cpu.state().ecx().write(4);
    cpu.state_mut()
        .flags_mut()
        .set_bits(Flags::CF | Flags::SF | Flags::OF);
    let before = cpu.memory().to_vec();
    assert!(
        matches!(cpu.execute(), Err(Exception::SegmentationViolation(Offset32(at))) if at == BASE + 512)
    );
    assert_eq!(
        (
            cpu.state().esi().read(),
            cpu.state().edi().read(),
            cpu.state().ecx().read()
        ),
        (BASE + 512, BASE + 130, 2)
    );
    assert_eq!(cpu.state().eip().read(), BASE);
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x881);
    assert_eq!(cpu.memory(), before);
    cpu.write_bytes(BASE + 256, &[8]).unwrap();
    cpu.state().esi().write(BASE + 256);
    done(&mut cpu);
    assert_eq!(
        (
            cpu.state().esi().read(),
            cpu.state().edi().read(),
            cpu.state().ecx().read()
        ),
        (BASE + 257, BASE + 131, 1)
    );
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x95);
}

#[test]
fn f05_repne_scas_reverse_stops_at_match_and_zero_preserves_flags() {
    let mut cpu = machine(&[0xf2, 0xae]);
    cpu.write_bytes(BASE + 128, &[1, 7, 3]).unwrap();
    cpu.state().al().write(7);
    cpu.state().edi().write(BASE + 130);
    cpu.state().ecx().write(3);
    cpu.state_mut().flags_mut().set_bits(Flags::DF);
    done(&mut cpu);
    assert_eq!(
        (cpu.state().edi().read(), cpu.state().ecx().read()),
        (BASE + 128, 1)
    );
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0xcd5, 0x444);
    let mut cpu = machine(&[0xf3, 0xa6]);
    cpu.state().esi().write(u32::MAX);
    cpu.state().edi().write(u32::MAX);
    cpu.state_mut().flags_mut().set_bits(Flags::CF | Flags::OF);
    done(&mut cpu);
    assert_eq!(
        (cpu.state().esi().read(), cpu.state().edi().read()),
        (u32::MAX, u32::MAX)
    );
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x801);
}

#[test]
fn f06_x87_store_invalidates_code_or_is_rejected_without_feature() {
    let mut code = vec![0xd9, 0xe8, 0xd9, 0x1d]; // FLD1; FSTP [next MOV immediate]
    code.extend((BASE + 9).to_le_bytes());
    code.extend([0xb8, 0, 0, 0, 0]);
    let mut cpu = machine(&code);
    #[cfg(feature = "x87")]
    {
        done(&mut cpu);
        assert_eq!(cpu.state().eax().read(), 0x3f800000);
        assert!(cpu.state().fpu().st(0).is_none());
        assert_eq!(cpu.state().fpu().instruction_pointer(), BASE + 2);
        assert_eq!(cpu.state().fpu().data_pointer(), BASE + 9);
    }
    #[cfg(not(feature = "x87"))]
    assert!(matches!(cpu.execute(), Err(Exception::Unimplemented2(_))));
}

#[test]
fn f06_simd_memory_adapter_enforces_alignment_and_feature() {
    let mut cpu = machine(&[0xf3, 0x0f, 0x6f, 5, 128, 0x10, 0, 0]);
    #[cfg(feature = "sse")]
    {
        cpu.write_bytes(BASE + 128, &[0x5a; 16]).unwrap();
        done(&mut cpu);
        assert_eq!(cpu.state().simd().xmm(0), &[0x5a; 16]);
        let mut cpu = machine(&[0x66, 0x0f, 0x6f, 5, 129, 0x10, 0, 0]);
        assert!(matches!(cpu.execute(), Err(Exception::GeneralProtection)));
        assert_eq!(cpu.state().simd().xmm(0), &[0; 16]);
    }
    #[cfg(not(feature = "sse"))]
    assert!(matches!(cpu.execute(), Err(Exception::InvalidOpcode)));
}

#[test]
fn f02_pushad_popad_preserve_layout_and_preflight_the_whole_frame() {
    let initial = [1, 2, 3, 4, BASE + 128, 6, 7, 8];
    let mut cpu = machine(&[0x60, 0xcd, 0x40, 0x61]);
    registers::write(cpu.state(), initial);
    cpu.state_mut().flags_mut().set_bits(Flags::CF | Flags::OF);
    done(&mut cpu);
    assert_eq!(cpu.state().esp().read(), BASE + 96);
    let expected = [8u32, 7, 6, BASE + 128, 4, 3, 2, 1];
    for (slot, value) in expected.into_iter().enumerate() {
        assert_eq!(
            cpu.read_memory32(BASE + 96 + slot as u32 * 4).unwrap(),
            value,
            "slot={slot}"
        );
    }
    registers::write(cpu.state(), [9, 9, 9, 9, BASE + 96, 9, 9, 9]);
    cpu.write_memory32(BASE + 108, 0xdeadbeef).unwrap(); // POPAD discards saved ESP
    cpu.resume_next();
    done(&mut cpu);
    assert_eq!(registers::read(cpu.state()), initial);
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x801);
    for (opcode, sp) in [(0x60, BASE + 16), (0x61, BASE + 512 - 16)] {
        let mut cpu = machine(&[opcode]);

        registers::write(cpu.state(), [1, 2, 3, 4, sp, 6, 7, 8]);
        let old_regs = registers::read(cpu.state());
        let before = cpu.memory().to_vec();
        assert!(
            matches!(cpu.execute(), Err(Exception::StackFault)),
            "opcode={opcode:x}"
        );
        assert_eq!(registers::read(cpu.state()), old_regs, "opcode={opcode:x}");
        assert_eq!(cpu.memory(), before);
        assert_eq!(cpu.state().eip().read(), BASE);
    }
}

#[test]
fn f02_enter_zero_builds_a_frame_and_leave_restores_it() {
    let mut cpu = machine(&[0xc8, 16, 0, 0, 0xcd, 0x40, 0xc9]);
    cpu.state().esp().write(BASE + 128);
    cpu.state().ebp().write(0xabcdef01);
    cpu.state_mut().flags_mut().set_bits(Flags::CF | Flags::OF);
    done(&mut cpu);
    assert_eq!(
        (cpu.state().esp().read(), cpu.state().ebp().read()),
        (BASE + 108, BASE + 124)
    );
    assert_eq!(cpu.read_memory32(BASE + 124).unwrap(), 0xabcdef01);
    cpu.resume_next();
    done(&mut cpu);
    assert_eq!(
        (cpu.state().esp().read(), cpu.state().ebp().read()),
        (BASE + 128, 0xabcdef01)
    );
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x801);
    // The saved BP store crosses the allocation boundary; do not establish a frame.
    let mut cpu = machine(&[0xc8, 16, 0, 0]);
    cpu.state().esp().write(BASE + 512 + 2);
    cpu.state().ebp().write(0x12345678);
    let before = cpu.memory().to_vec();
    assert!(matches!(cpu.execute(), Err(Exception::StackFault)));
    assert_eq!(
        (cpu.state().esp().read(), cpu.state().ebp().read()),
        (BASE + 512 + 2, 0x12345678)
    );
    assert_eq!(cpu.memory(), before);
    assert_eq!(cpu.state().eip().read(), BASE);
}

#[test]
fn f02_leave_fault_preserves_registers_and_retries_at_the_frame_boundary() {
    assert_eq!(shared::leave_fault_boundary(), 1);
    let mut cpu = machine(&[0xc9]);
    registers::write(cpu.state(), [1, 2, 3, 4, BASE + 128, BASE + 512 - 2, 7, 8]);
    cpu.write_memory32(BASE + 512 - 4, 0x12345678).unwrap();
    cpu.state_mut().flags_mut().set_bits(Flags::CF | Flags::OF);
    let before_regs = registers::read(cpu.state());
    let before_memory = cpu.memory().to_vec();
    for _ in 0..2 {
        assert!(matches!(cpu.execute(), Err(Exception::StackFault)));
        assert_eq!(registers::read(cpu.state()), before_regs);
        assert_eq!(cpu.memory(), before_memory);
        assert_eq!(cpu.state().eip().read(), BASE);
        assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x801);
    }
    cpu.state().ebp().write(BASE + 512 - 4);
    // Retry the same LEAVE, without resume_next(), using the repaired frame.
    done(&mut cpu);
    assert_eq!(
        registers::read(cpu.state()),
        [1, 2, 3, 4, BASE + 512, 0x12345678, 7, 8]
    );
    assert_eq!(cpu.memory(), before_memory);
    assert_eq!(cpu.state().eip().read(), BASE + 1);
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x801);
}

#[cfg(feature = "uop-profile")]
#[path = "support/profile.rs"]
mod profile_output;

#[cfg(feature = "uop-profile")]
#[path = "flat/profile.rs"]
mod profile_contracts;

#[cfg(feature = "flag-elision")]
#[test]
fn elision_stops_before_a_faulting_segment_override() {
    use plum::flat::uop::Uop;
    // The GS-prefixed CMP is unsupported by FlatUME, even with register operands.
    let mut cpu = machine(&[0x83, 0xc0, 1, 0x39, 0xc0, 0x83, 0xc3, 1, 0x65, 0x39, 0xc0]);
    cpu.state().ebx().write(u32::MAX);
    assert!(matches!(cpu.execute(), Err(Exception::Unimplemented2(..))));
    assert!(matches!(cpu.tracer().uop_cache()[0], Uop::ValueAddI(..)));
    assert_eq!(cpu.state().eax().read(), 1);
    assert_eq!(cpu.state().ebx().read(), 0);
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x55);
}

#[test]
fn memory_cmp_and_test_retry_after_a_load_fault() {
    for (opcode, expected) in [(0x3b, 0), (0x39, 0x81), (0x85, 0x40)] {
        let mut cpu = machine(&[opcode, 0x03]); // EAX and [EBX]
        cpu.state().eax().write(8);
        cpu.state().ebx().write(BASE + 510);
        cpu.state_mut().flags_mut().set_bits(Flags::CF | Flags::ZF);
        assert!(
            matches!(cpu.execute(), Err(Exception::SegmentationViolation(addr)) if addr.0 == BASE + 510)
        );
        assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0xc1, 0x41);
        assert_eq!(cpu.state().eip().read(), BASE);
        cpu.state().ebx().write(BASE + 128);
        cpu.write_memory32(BASE + 128, 7).unwrap();
        done(&mut cpu);
        assert_eq!(
            cpu.state_mut().flags_mut().resolve().bits() & 0xc1,
            expected
        );
        assert_eq!(cpu.state().eax().read(), 8);
        assert_eq!(cpu.read_memory32(BASE + 128).unwrap(), 7);
    }
}
