//! Harvard engine contracts: results, commit order, and host restart boundaries.
use plum::gpr::PartialRegister;
use plum::harvard::HarvardUME;
use plum::prelude::*;

#[path = "../examples/support/contracts/harvard.rs"]
mod shared;

#[path = "support/registers.rs"]
mod registers;

#[path = "support/instructions.rs"]
mod instructions;

#[cfg(feature = "sse")]
#[path = "../examples/support/contracts/simd.rs"]
mod numeric_shared;

#[cfg(feature = "sse")]
#[test]
fn h06_fixed_numeric_results_are_shared_with_the_wasm_runner() {
    assert_eq!(numeric_shared::harvard(), 2);
}

#[test]
fn h01_native_and_wasm_share_rmw_and_narrow_load_contracts() {
    assert_eq!(shared::rmw(), 1);
    assert_eq!(shared::load_extend(), 1);
    assert_eq!(shared::narrow_load(), 1);
}

fn machine(bytes: &[u8]) -> HarvardUME {
    let mut code = bytes.to_vec();
    code.extend([0xcd, 0x40]);
    plum::ume::harvard(
        code.into_boxed_slice(),
        vec![0; 256].into_boxed_slice(),
        Offset32(0),
        Offset32(256),
    )
}

fn done(cpu: &mut HarvardUME) {
    assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
}

#[test]
fn h01_lowering_connects_carry_partial_register_and_branch() {
    // ADD wraps, ADC consumes CF, JNZ skips INC ECX, MOV AH preserves AL.
    let mut cpu = machine(&[
        0xb8, 0xff, 0xff, 0xff, 0xff, 0x83, 0xc0, 1, 0x83, 0xd3, 0, 0x75, 1, 0x41, 0xb4, 7,
    ]);
    done(&mut cpu);
    assert_eq!(cpu.state().eax().read(), 0x700);
    assert_eq!(cpu.state().ebx().read(), 1);
    assert_eq!(cpu.state().ecx().read(), 0);
    assert_eq!(cpu.state().eip().read(), 16);
}

#[test]
fn h01_representative_integer_forms_produce_fixed_results() {
    // Data at offset 0 of the separate data space; the stack sits above it.
    let mut cpu = plum::ume::harvard(
        instructions::CODE.into(),
        vec![0; 0x800].into_boxed_slice(),
        Offset32(0),
        Offset32(0x800),
    );
    cpu.state().ebx().write(instructions::EBX_OFFSET);
    done(&mut cpu);
    instructions::check("harvard", |slot| cpu.read_memory32(4 * slot).unwrap());
}

#[test]
fn h01_calls_return_to_the_guest_stack_target() {
    // CALL 8; INT 40h; NOP; INC EBX; RET.
    let mut cpu = machine(&[0xe8, 3, 0, 0, 0, 0xcd, 0x40, 0x90, 0x43, 0xc3]);
    done(&mut cpu);
    assert_eq!(cpu.state().eip().read(), 5);
    assert_eq!(cpu.state().ebx().read(), 1);
    assert_eq!(cpu.state().esp().read(), 256);
    assert_eq!(cpu.read_memory32(252).unwrap(), 5);
}

#[test]
fn h02_code_and_data_are_distinct_at_the_same_offset() {
    let mut code = vec![
        0x2e, 0xa1, 64, 0, 0, 0, 0xa3, 32, 0, 0, 0, 0xa1, 64, 0, 0, 0, 0xcd, 0x40,
    ];
    code.resize(64, 0x90);
    code.extend(0x12345678u32.to_le_bytes());
    let mut cpu = machine(&code);
    cpu.write_memory32(64, 0xabcdef01).unwrap();
    done(&mut cpu);
    assert_eq!(cpu.read_memory32(32).unwrap(), 0x12345678);
    assert_eq!(cpu.state().eax().read(), 0xabcdef01);
    assert_eq!(&cpu.code()[64..68], &0x12345678u32.to_le_bytes());
}

#[test]
fn h03_rmw_fault_preserves_memory_registers_and_flags() {
    let mut cpu = machine(&[0x83, 0, 1]); // ADD dword [EAX],1 crosses the end
    cpu.state().eax().write(254);
    cpu.data().fill(0x5a);
    cpu.state_mut().flags_mut().set_bits(Flags::CF | Flags::ZF);
    let before = cpu.data().to_vec();
    assert!(matches!(
        cpu.execute(),
        Err(Exception::SegmentationViolation(Offset32(254)))
    ));
    assert_eq!(cpu.state().eip().read(), 0);
    assert_eq!(cpu.state().eax().read(), 254);
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x41);
    assert_eq!(cpu.data(), before);
}

#[test]
fn h03_division_fault_can_retry_without_committing_registers() {
    let mut cpu = machine(&[0xf7, 0xf1]); // DIV ECX
    cpu.state().eax().write(10);
    assert!(matches!(cpu.execute(), Err(Exception::DivisionError)));
    assert_eq!(
        (
            cpu.state().eax().read(),
            cpu.state().edx().read(),
            cpu.state().eip().read()
        ),
        (10, 0, 0)
    );
    cpu.state().ecx().write(3);
    done(&mut cpu);
    assert_eq!((cpu.state().eax().read(), cpu.state().edx().read()), (3, 1));
}

#[test]
fn h03_stack_fault_does_not_move_esp() {
    let mut cpu = machine(&[0x50]);
    cpu.state().esp().write(2);
    assert!(matches!(cpu.execute(), Err(Exception::StackFault)));
    assert_eq!(cpu.state().esp().read(), 2);
    assert_eq!(cpu.state().eip().read(), 0);
}

#[test]
fn h03_pop_memory_uses_incremented_esp_but_commits_it_after_the_store() {
    let mut cpu = machine(&[0x8f, 0x44, 0x24, 4]); // POP [ESP+4]
    cpu.state().esp().write(248);
    cpu.write_memory32(248, 0x12345678).unwrap();
    cpu.state_mut().flags_mut().set_bits(Flags::CF | Flags::ZF);
    let before = cpu.data().to_vec();
    // Effective address is (248+4)+4=256, not 252.
    assert!(matches!(
        cpu.execute(),
        Err(Exception::SegmentationViolation(Offset32(256)))
    ));
    assert_eq!(cpu.state().esp().read(), 248);
    assert_eq!(cpu.state().eip().read(), 0);
    assert_eq!(cpu.data(), before);
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x41);
    cpu.state().esp().write(244);
    cpu.write_memory32(244, 0xabcdef01).unwrap();
    done(&mut cpu);
    assert_eq!(cpu.state().esp().read(), 248);
    assert_eq!(cpu.read_memory32(252).unwrap(), 0xabcdef01);
    assert_eq!(cpu.read_memory32(248).unwrap(), 0x12345678);
}

#[test]
fn h03_pop_checks_stack_source_before_destination_and_retries_each_fault() {
    let mut cpu = machine(&[0x8f, 0]); // POP [EAX]
    cpu.state().esp().write(254);
    cpu.state().eax().write(254);
    let before = cpu.data().to_vec();
    assert!(matches!(cpu.execute(), Err(Exception::StackFault)));
    assert_eq!(cpu.state().esp().read(), 254);
    assert_eq!(cpu.data(), before);
    cpu.state().esp().write(128);
    cpu.write_memory32(128, 0x12345678).unwrap();
    assert!(matches!(
        cpu.execute(),
        Err(Exception::SegmentationViolation(Offset32(254)))
    ));
    assert_eq!(cpu.state().esp().read(), 128);
    assert_eq!(cpu.read_memory32(128).unwrap(), 0x12345678);
    cpu.state().eax().write(64);
    done(&mut cpu);
    assert_eq!(cpu.state().esp().read(), 132);
    assert_eq!(cpu.read_memory32(64).unwrap(), 0x12345678);
}

#[test]
fn h03_xadd_fault_preserves_source_and_flags_at_each_operand_width() {
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
        let mut cpu = machine(bytes); // XADD [EAX],BL/BX/EBX
        let address = 257 - width;
        cpu.state().eax().write(address);
        cpu.state().ebx().write(source);
        cpu.state_mut().flags_mut().set_bits(Flags::CF | Flags::ZF);
        let before = cpu.data().to_vec();
        assert!(
            matches!(cpu.execute(), Err(Exception::SegmentationViolation(Offset32(at))) if at == address),
            "width={width}"
        );
        assert_eq!(cpu.state().ebx().read(), source, "width={width}");
        assert_eq!(cpu.state().eip().read(), 0);
        assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x41);
        assert_eq!(cpu.data(), before);
        cpu.state().eax().write(128);
        cpu.data()[127] = 0x5a;
        cpu.data()[132] = 0xa5;
        cpu.write_memory32(128, initial).unwrap();
        done(&mut cpu);
        assert_eq!(cpu.state().ebx().read(), exchanged, "width={width}");
        assert_eq!(cpu.read_memory32(128).unwrap(), expected, "width={width}");
        assert_eq!(
            (cpu.data()[127], cpu.data()[132]),
            (0x5a, 0xa5),
            "width={width}"
        );
        assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x14);
    }
}

#[test]
fn h04_host_stop_resumes_once_and_ret_uses_the_new_stack_value() {
    // Revisit the same cached RET with a different return address.
    let mut cpu = machine(&[0xc3, 0x43, 0xcd, 0x40, 0x43, 0xcd, 0x40, 0xeb, 0xf7]);
    cpu.state().esp().write(128);
    cpu.write_memory32(128, 1).unwrap();
    done(&mut cpu);
    assert_eq!((cpu.state().ebx().read(), cpu.state().eip().read()), (1, 2));
    cpu.resume_next();
    done(&mut cpu);
    assert_eq!((cpu.state().ebx().read(), cpu.state().eip().read()), (2, 5));
    cpu.state().esp().write(128);
    cpu.write_memory32(128, 4).unwrap();
    cpu.resume_next();
    done(&mut cpu);
    assert_eq!((cpu.state().ebx().read(), cpu.state().eip().read()), (3, 5));
}

#[test]
fn h05_rep_commits_completed_elements_and_retries_remaining_elements() {
    let mut cpu = machine(&[0xf3, 0xa4]);
    cpu.state().esi().write(254);
    cpu.state().edi().write(64);
    cpu.state().ecx().write(4);
    cpu.data()[254..].copy_from_slice(&[1, 2]);
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
        (256, 66, 2)
    );
    assert_eq!(cpu.state().eip().read(), 0);
    cpu.data()[128..130].copy_from_slice(&[9, 10]);
    cpu.state().esi().write(128);
    done(&mut cpu);
    assert_eq!(&cpu.data()[64..68], &[1, 2, 9, 10]);
    assert_eq!(cpu.state().ecx().read(), 0);
}

#[test]
fn h05_reverse_rep_movs_preserves_element_order() {
    for (width, code) in [
        (1, &[0xfd, 0xf3, 0xa4][..]),
        (2, &[0xfd, 0x66, 0xf3, 0xa5]),
        (4, &[0xfd, 0xf3, 0xa5]),
    ] {
        let count = 8;
        let src = 96;
        for dest in [
            src - count * width,
            src - 1,
            src,
            src + 1,
            src + count * width,
        ] {
            let mut cpu = machine(code);
            let mut expected: Vec<u8> = (0..=255).collect();
            cpu.data().copy_from_slice(&expected);
            for i in (0..count).rev() {
                let start = src + i * width;
                expected.copy_within(start..start + width, dest + i * width);
            }
            let last = (count - 1) * width;
            cpu.state().esi().write((src + last) as u32);
            cpu.state().edi().write((dest + last) as u32);
            cpu.state().ecx().write(count as u32);
            done(&mut cpu);
            assert_eq!(
                cpu.data(),
                expected.as_slice(),
                "width={width}, dest={dest}"
            );
        }
    }
}

#[test]
fn h05_rep_zero_does_not_access_memory_and_df_reverses_progress() {
    let mut cpu = machine(&[0xf3, 0xa4]);
    cpu.state().esi().write(u32::MAX);
    cpu.state().edi().write(u32::MAX);
    done(&mut cpu);
    assert_eq!(
        (
            cpu.state().esi().read(),
            cpu.state().edi().read(),
            cpu.state().ecx().read()
        ),
        (u32::MAX, u32::MAX, 0)
    );
    let mut cpu = machine(&[0xfd, 0xf3, 0xa4, 0xfc]);
    cpu.data()[128..131].copy_from_slice(&[1, 2, 3]);
    cpu.state().esi().write(130);
    cpu.state().edi().write(66);
    cpu.state().ecx().write(3);
    done(&mut cpu);
    assert_eq!(&cpu.data()[64..67], &[1, 2, 3]);
    assert_eq!(
        (cpu.state().esi().read(), cpu.state().edi().read()),
        (127, 63)
    );
}

#[test]
fn h05_reverse_repe_cmps_stops_at_the_first_unequal_element() {
    for (width, code) in [
        (1, &[0xfd, 0xf3, 0xa6][..]),
        (2, &[0xfd, 0x66, 0xf3, 0xa7]),
        (4, &[0xfd, 0xf3, 0xa7]),
    ] {
        let count = 96 / width;
        for mismatch in [
            Some(0),
            Some(64 / width - 1),
            Some(64 / width),
            Some(count - 1),
            None,
        ] {
            let mut cpu = machine(code);
            let src = 16 + 96 - width;
            let dest = 144 + 96 - width;
            if let Some(i) = mismatch {
                // The high byte differs; the final flags use the whole element.
                cpu.data()[dest - i * width + width - 1] = 0x80;
                if i + 1 < count {
                    cpu.data()[144] = 1; // A later mismatch must not take precedence.
                }
            }
            cpu.state().esi().write(src as u32);
            cpu.state().edi().write(dest as u32);
            cpu.state().ecx().write(count as u32);
            done(&mut cpu);
            let consumed = mismatch.map_or(count, |i| i + 1);
            assert_eq!(cpu.state().ecx().read(), (count - consumed) as u32);
            assert_eq!(cpu.state().esi().read(), (src - consumed * width) as u32);
            assert_eq!(cpu.state().edi().read(), (dest - consumed * width) as u32);
            let flags = cpu.state_mut().flags_mut().resolve().bits();
            let mask = Flags::CF | Flags::ZF | Flags::SF | Flags::OF;
            let expected = if mismatch.is_some() {
                Flags::CF | Flags::SF | Flags::OF
            } else {
                Flags::ZF
            };
            assert_eq!(flags & mask.bits(), expected.bits());
        }
    }
}

#[test]
fn h05_repe_cmps_fault_preserves_entry_flags_then_retries_the_remainder() {
    let mut cpu = machine(&[0xf3, 0xa6]);
    cpu.data()[254..].copy_from_slice(&[7, 7]);
    cpu.data()[64..68].copy_from_slice(&[7, 7, 9, 9]);
    cpu.state().esi().write(254);
    cpu.state().edi().write(64);
    cpu.state().ecx().write(4);
    cpu.state_mut()
        .flags_mut()
        .set_bits(Flags::CF | Flags::SF | Flags::OF);
    let before = cpu.data().to_vec();
    assert!(matches!(
        cpu.execute(),
        Err(Exception::SegmentationViolation(Offset32(256)))
    ));
    assert_eq!(
        (
            cpu.state().esi().read(),
            cpu.state().edi().read(),
            cpu.state().ecx().read()
        ),
        (256, 66, 2)
    );
    assert_eq!(cpu.state().eip().read(), 0);
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x881);
    assert_eq!(cpu.data(), before);
    cpu.data()[128] = 8;
    cpu.state().esi().write(128);
    done(&mut cpu);
    assert_eq!(
        (
            cpu.state().esi().read(),
            cpu.state().edi().read(),
            cpu.state().ecx().read()
        ),
        (129, 67, 1)
    );
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x95); // 8-9
}

#[test]
fn h05_repne_scasb_finds_first_match_across_word_boundaries() {
    let start = 65;
    let count = 27usize;
    for reverse in [false, true] {
        let address = |i| start + if reverse { count - 1 - i } else { i };
        for first in 0..=count {
            let mut cpu = machine(&[0xf2, 0xae]);
            cpu.data().fill(0x81);
            if first < count {
                cpu.data()[address(first)] = 0x80;
                cpu.data()[address(count - 1)] = 0x80;
            }
            cpu.state().al().write(0x80);
            cpu.state().edi().write(address(0) as u32);
            cpu.state().ecx().write(count as u32);
            cpu.state_mut().flags_mut().set_static(Flags::DF, reverse);
            done(&mut cpu);
            let consumed = (first + 1).min(count);
            let end = if reverse {
                address(0) - consumed
            } else {
                start + consumed
            };
            assert_eq!(cpu.state().edi().read(), end as u32);
            assert_eq!(cpu.state().ecx().read(), (count - consumed) as u32);
            let flags = cpu.state_mut().flags_mut().resolve().bits();
            assert_eq!(flags & 0x8d5, if first < count { 0x44 } else { 0x95 });
        }
    }
}

#[test]
fn h05_repne_scas_reverse_stops_at_match_and_zero_preserves_flags() {
    let mut cpu = machine(&[0xf2, 0xae]);
    cpu.data()[64..67].copy_from_slice(&[1, 7, 3]);
    cpu.state().al().write(7);
    cpu.state().edi().write(66);
    cpu.state().ecx().write(3);
    cpu.state_mut().flags_mut().set_bits(Flags::DF);
    done(&mut cpu);
    assert_eq!(
        (cpu.state().edi().read(), cpu.state().ecx().read()),
        (64, 1)
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
fn h06_x87_dispatch_obeys_feature_and_writes_memory() {
    let mut cpu = machine(&[0xd9, 0xe8, 0xd9, 0xe8, 0xde, 0xc1, 0xd9, 0x1d, 128, 0, 0, 0]);
    #[cfg(feature = "x87")]
    {
        done(&mut cpu);
        assert_eq!(cpu.read_memory32(128).unwrap(), 0x40000000); // 1+1=2.0f32
        assert!(cpu.state().fpu().st(0).is_none());
        assert_eq!(cpu.state().fpu().instruction_pointer(), 6);
        assert_eq!(cpu.state().fpu().data_pointer(), 128);
    }
    #[cfg(not(feature = "x87"))]
    assert!(matches!(cpu.execute(), Err(Exception::Unimplemented(_))));
}

#[test]
fn h06_simd_dispatch_and_alignment_fault_do_not_overwrite_the_destination() {
    let mut cpu = machine(&[0xf3, 0x0f, 0x6f, 5, 128, 0, 0, 0]);
    #[cfg(feature = "sse")]
    {
        cpu.data()[128..144].fill(0x5a);
        done(&mut cpu);
        assert_eq!(cpu.state().simd().xmm(0), &[0x5a; 16]);
        let mut cpu = machine(&[0x66, 0x0f, 0x6f, 5, 129, 0, 0, 0]);
        assert!(matches!(cpu.execute(), Err(Exception::GeneralProtection)));
        assert_eq!(cpu.state().simd().xmm(0), &[0; 16]);
        assert_eq!(cpu.state().eip().read(), 0);
    }
    #[cfg(not(feature = "sse"))]
    assert!(matches!(cpu.execute(), Err(Exception::InvalidOpcode)));
}

#[test]
fn h03_pushad_popad_preserve_layout_and_preflight_the_whole_frame() {
    for sp in 128..=131 {
        let base = sp - 32;
        let initial = [0x12345678, 2, 3, 4, sp, 6, 7, 8];
        let mut cpu = machine(&[0x60, 0xcd, 0x40, 0x61]);
        registers::write(cpu.state(), initial);
        cpu.state_mut().flags_mut().set_bits(Flags::CF | Flags::OF);
        done(&mut cpu);
        assert_eq!(cpu.state().esp().read(), base);
        let expected = [8u32, 7, 6, sp, 4, 3, 2, 0x12345678];
        for (slot, value) in expected.into_iter().enumerate() {
            assert_eq!(
                cpu.read_memory32(base + slot as u32 * 4).unwrap(),
                value,
                "slot={slot}"
            );
        }
        registers::write(cpu.state(), [9, 9, 9, 9, base, 9, 9, 9]);
        cpu.write_memory32(base + 12, 0xdeadbeef).unwrap(); // POPAD discards saved ESP
        cpu.resume_next();
        done(&mut cpu);
        assert_eq!(registers::read(cpu.state()), initial);
        assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x801);
    }
    for (opcode, sp) in [(0x60, 0 + 16), (0x61, 0 + 256 - 16)] {
        let mut cpu = machine(&[opcode]);
        cpu.data().fill(0x5a);
        registers::write(cpu.state(), [1, 2, 3, 4, sp, 6, 7, 8]);
        let old_regs = registers::read(cpu.state());
        let before = cpu.data().to_vec();
        assert!(
            matches!(cpu.execute(), Err(Exception::StackFault)),
            "opcode={opcode:x}"
        );
        assert_eq!(registers::read(cpu.state()), old_regs, "opcode={opcode:x}");
        assert_eq!(cpu.data(), before);
        assert_eq!(cpu.state().eip().read(), 0);
    }
}

#[test]
fn h03_enter_zero_builds_a_frame_and_leave_restores_it() {
    let mut cpu = machine(&[0xc8, 16, 0, 0, 0xcd, 0x40, 0xc9]);
    cpu.state().esp().write(0 + 128);
    cpu.state().ebp().write(0xabcdef01);
    cpu.state_mut().flags_mut().set_bits(Flags::CF | Flags::OF);
    done(&mut cpu);
    assert_eq!(
        (cpu.state().esp().read(), cpu.state().ebp().read()),
        (0 + 108, 0 + 124)
    );
    assert_eq!(cpu.read_memory32(0 + 124).unwrap(), 0xabcdef01);
    cpu.resume_next();
    done(&mut cpu);
    assert_eq!(
        (cpu.state().esp().read(), cpu.state().ebp().read()),
        (0 + 128, 0xabcdef01)
    );
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x801);
    // The saved BP store crosses the allocation boundary; do not establish a frame.
    let mut cpu = machine(&[0xc8, 16, 0, 0]);
    cpu.state().esp().write(0 + 256 + 2);
    cpu.state().ebp().write(0x12345678);
    let before = cpu.data().to_vec();
    assert!(matches!(cpu.execute(), Err(Exception::StackFault)));
    assert_eq!(
        (cpu.state().esp().read(), cpu.state().ebp().read()),
        (0 + 256 + 2, 0x12345678)
    );
    assert_eq!(cpu.data(), before);
    assert_eq!(cpu.state().eip().read(), 0);
}

#[test]
fn h03_leave_fault_preserves_registers_and_retries_at_the_frame_boundary() {
    assert_eq!(shared::leave_fault_boundary(), 1);
    let mut cpu = machine(&[0xc9]);
    registers::write(cpu.state(), [1, 2, 3, 4, 0 + 128, 0 + 256 - 2, 7, 8]);
    cpu.write_memory32(0 + 256 - 4, 0x12345678).unwrap();
    cpu.state_mut().flags_mut().set_bits(Flags::CF | Flags::OF);
    let before_regs = registers::read(cpu.state());
    let before_memory = cpu.data().to_vec();
    for _ in 0..2 {
        assert!(matches!(cpu.execute(), Err(Exception::StackFault)));
        assert_eq!(registers::read(cpu.state()), before_regs);
        assert_eq!(cpu.data(), before_memory);
        assert_eq!(cpu.state().eip().read(), 0);
        assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x801);
    }
    cpu.state().ebp().write(0 + 256 - 4);
    // Retry the same LEAVE, without resume_next(), using the repaired frame.
    done(&mut cpu);
    assert_eq!(
        registers::read(cpu.state()),
        [1, 2, 3, 4, 0 + 256, 0x12345678, 7, 8]
    );
    assert_eq!(cpu.data(), before_memory);
    assert_eq!(cpu.state().eip().read(), 0 + 1);
    assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x801);
}

#[cfg(feature = "uop-profile")]
#[path = "support/profile.rs"]
mod profile_output;

#[cfg(feature = "uop-profile")]
#[path = "harvard/profile.rs"]
mod profile_contracts;

#[test]
fn register_alu_loads_use_the_address_before_writing_the_destination() {
    use plum::harvard::uop::Uop;

    // EAX is both destination and base. Make the SIB address wrap back to 64.
    for (opcode, expected, flags) in [
        (0x03, 65, 0x04), // ADD
        (0x2b, 63, 0x14), // SUB
        (0x23, 0, 0x44),  // AND
        (0x0b, 65, 0x04), // OR
        (0x33, 65, 0x04), // XOR
        (0x3b, 64, 0x14), // CMP EAX,[mem]
        (0x39, 64, 0x81), // CMP [mem],EAX
        (0x85, 64, 0x44), // TEST [mem],EAX
    ] {
        for suffix in [&[0x00][..], &[0x44, 0x88, 4][..]] {
            let mut code = vec![opcode];
            code.extend_from_slice(suffix);
            let mut cpu = machine(&code);
            cpu.state().eax().write(64);
            cpu.state().ecx().write(u32::MAX);
            cpu.write_memory32(64, 1).unwrap();
            done(&mut cpu);
            assert_eq!(cpu.state().eax().read(), expected);
            assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, flags);
            assert_eq!(cpu.read_memory32(64).unwrap(), 1);
            let ops = cpu.tracer().uop_cache();
            let index = usize::from(suffix.len() > 1);
            if index == 1 {
                assert!(matches!(ops[0], Uop::LeaSIB(..)));
            }
            assert!(matches!(
                ops[index],
                Uop::AddLoad(..)
                    | Uop::SubLoad(..)
                    | Uop::AndLoad(..)
                    | Uop::OrLoad(..)
                    | Uop::XorLoad(..)
                    | Uop::CmpRegMem(..)
                    | Uop::CmpMemReg(..)
                    | Uop::TestMemReg(..)
            ));
        }
    }
}

#[test]
fn register_alu_load_checks_its_source_before_updating_flags() {
    // A dword at offset 4 crosses the end of code but fits in data.
    for opcode in [0x2b, 0x3b, 0x39, 0x85] {
        let mut cpu = machine(&[0x2e, opcode, 0x00]);
        cpu.state().eax().write(4);
        cpu.state_mut().flags_mut().set_bits(Flags::CF | Flags::ZF);
        assert!(matches!(
            cpu.execute(),
            Err(Exception::SegmentationViolation(Offset32(4)))
        ));
        assert_eq!(cpu.state().eax().read(), 4);
        assert_eq!(cpu.state().eip().read(), 0);
        assert_eq!(cpu.state_mut().flags_mut().resolve().bits() & 0x8d5, 0x41);
    }
}
