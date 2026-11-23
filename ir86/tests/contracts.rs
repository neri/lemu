//! Small, fixed decoder contracts. IDs and intentional gaps: docs/TESTING.md.
use ir86::prelude::registers::*;
use ir86::prelude::*;

// A trailing INT3 proves that decoding consumes exactly one instruction.
fn check(use32: bool, bytes: &[u8], expected: IrOp) {
    let mut stream = bytes.to_vec();
    stream.push(0xcc);
    let mut fetch = SimpleFetcher::new(&stream);
    let mut decoder = Decoder::new(use32);
    assert_eq!(
        decoder.decode(&mut fetch),
        Ok(expected),
        "{bytes:02x?}, use32={use32}"
    );
    assert_eq!(fetch.pos(), bytes.len(), "{bytes:02x?}");
    assert_eq!(decoder.decode(&mut fetch), Ok(IrOp::INT3));
    assert_eq!(fetch.pos(), stream.len());
}

#[test]
fn ir01_operand_width_and_register_fields() {
    for (use32, bytes, expected) in [
        (false, &[0xb9, 0x34, 0x12][..], IrOp::MOV_Rw_Iw(CX, 0x1234)),
        (
            true,
            &[0xb9, 0x78, 0x56, 0x34, 0x12],
            IrOp::MOV_Rd_Id(ECX, 0x12345678),
        ),
        (
            true,
            &[0x66, 0x66, 0xb9, 0x34, 0x12],
            IrOp::MOV_Rw_Iw(CX, 0x1234),
        ),
        (
            false,
            &[0x66, 0xb9, 0x78, 0x56, 0x34, 0x12],
            IrOp::MOV_Rd_Id(ECX, 0x12345678),
        ),
        (true, &[0x8a, 0xe3], IrOp::MOV_Rb_Rb(AH, BL)),
        (true, &[0x8b, 0xcb], IrOp::MOV_Rd_Rd(ECX, EBX)),
    ] {
        check(use32, bytes, expected);
    }
}

#[test]
fn ir02_signed_immediate() {
    check(
        false,
        &[0x6b, 0xcb, 0x80],
        IrOp::IMUL_Rw_Rw_Iw(CX, BX, 0xff80),
    );
    check(
        true,
        &[0x6b, 0xcb, 0x80],
        IrOp::IMUL_Rd_Rd_Id(ECX, EBX, 0xffff_ff80),
    );
    check(
        true,
        &[0x6b, 0xcb, 0x7f],
        IrOp::IMUL_Rd_Rd_Id(ECX, EBX, 0x7f),
    );
}

#[test]
fn ir03_address16_and_independent_operand_override() {
    let bp_si = MemOpr::A16(MemOpr16 {
        segment: SS,
        base_index: Some(BaseIndex16::BpSi),
        disp: Offset16(0xfffe),
    });
    check(false, &[0x8b, 0x42, 0xfe], IrOp::MOV_Rw_Mw(AX, bp_si));
    check(true, &[0x67, 0x8b, 0x42, 0xfe], IrOp::MOV_Rd_Md(EAX, bp_si));
    check(
        false,
        &[0x8b, 0x06, 0x34, 0x12],
        IrOp::MOV_Rw_Mw(
            AX,
            MemOpr::A16(MemOpr16::from_sel_off(DS, Offset16(0x1234))),
        ),
    );
    check(
        false,
        &[0x66, 0x8b, 0x00],
        IrOp::MOV_Rd_Md(
            EAX,
            MemOpr::A16(MemOpr16 {
                segment: DS,
                base_index: Some(BaseIndex16::BxSi),
                disp: Offset16(0),
            }),
        ),
    );
}

#[test]
fn ir04_address32_sib_and_segment() {
    // Expected components are literal x86 addresses, not a second SIB encoder.
    for (use32, bytes, segment, base, index, scale, disp) in [
        (
            true,
            &[0x8b, 0x44, 0x8d, 0xf0][..],
            SS,
            Some(EBP),
            Some(ECX),
            Some(Scale::DWord),
            0xfffffff0,
        ),
        (
            true,
            &[0x8b, 0x04, 0xcd, 0x78, 0x56, 0x34, 0x12],
            DS,
            None,
            Some(ECX),
            Some(Scale::QWord),
            0x12345678,
        ),
        (true, &[0x8b, 0x04, 0x24], SS, Some(ESP), None, None, 0),
        (
            true,
            &[0x8b, 0x05, 0x78, 0x56, 0x34, 0x12],
            DS,
            None,
            None,
            None,
            0x12345678,
        ),
        (
            true,
            &[0x64, 0x8b, 0x45, 0x00],
            FS,
            Some(EBP),
            None,
            None,
            0,
        ),
        (false, &[0x67, 0x8b, 0x00], DS, Some(EAX), None, None, 0),
    ] {
        let mut fetch = SimpleFetcher::new(bytes);
        let ir = Decoder::new(use32).decode(&mut fetch).unwrap();
        let mem = match ir {
            IrOp::MOV_Rd_Md(EAX, MemOpr::A32(mem)) if use32 => mem,
            IrOp::MOV_Rw_Mw(AX, MemOpr::A32(mem)) if !use32 => mem,
            _ => panic!("{bytes:02x?}: {ir:?}"),
        };
        assert_eq!(
            (
                mem.segment,
                mem.base_index.base(),
                mem.base_index.index(),
                mem.base_index.scale(),
                mem.disp.0
            ),
            (segment, base, index, scale, disp),
            "{bytes:02x?}"
        );
        check(use32, bytes, ir);
    }
}

#[test]
fn ir05_relative_targets_use_next_eip_and_effective_width() {
    for (use32, base, bytes, expected) in [
        (
            true,
            0x1000,
            &[0xeb, 0xfc][..],
            IrOp::JMP_Jd(Offset32(0x0ffe)),
        ),
        (false, 0xfffe, &[0xeb, 0x01], IrOp::JMP_Jw(Offset16(1))),
        (
            true,
            0xffff_fffe,
            &[0x74, 0x01],
            IrOp::JCC_Jd(CC::Z, Offset32(1)),
        ),
        (
            true,
            0x10000,
            &[0x66, 0xe9, 0x01, 0x00],
            IrOp::JMP_Jw(Offset16(5)),
        ),
        (
            false,
            0xfffe,
            &[0x66, 0xe8, 0, 0, 0, 0],
            IrOp::CALL_Jd(Offset32(0x10004)),
        ),
    ] {
        let mut fetch = OffsetFetcher::new(bytes, Offset32(base));
        assert_eq!(
            Decoder::new(use32).decode(&mut fetch),
            Ok(expected),
            "{bytes:02x?}"
        );
        assert_eq!(fetch.pos(), bytes.len());
    }
}

#[test]
fn ir06_prefixes_do_not_leak_to_the_next_instruction() {
    let mut decoder = Decoder::use32();
    let mut fetch = SimpleFetcher::new(&[0x66, 0xb8, 1, 0, 0xb8, 2, 0, 0, 0]);
    assert_eq!(decoder.decode(&mut fetch), Ok(IrOp::MOV_Rw_Iw(AX, 1)));
    assert_eq!(decoder.decode(&mut fetch), Ok(IrOp::MOV_Rd_Id(EAX, 2)));
    assert_eq!(fetch.pos(), 9);
    decoder.set_use32(false);
    let mut fetch = SimpleFetcher::new(&[0xb8, 3, 0]);
    assert_eq!(decoder.decode(&mut fetch), Ok(IrOp::MOV_Rw_Iw(AX, 3)));
    assert_eq!(fetch.pos(), 3);
}

#[test]
fn ir07_repeat_lock_and_string_attributes() {
    let source = StringOpr {
        segment: FS,
        is_addr32: false,
    };
    check(
        true,
        &[0xf0, 0xf3, 0x64, 0x67, 0x66, 0xa5],
        IrOp::REP_MOVSW(source),
    );
    check(
        true,
        &[0xf2, 0xa6],
        IrOp::REPNZ_CMPSB(StringOpr {
            segment: DS,
            is_addr32: true,
        }),
    );
    check(
        true,
        &[0xf3, 0xa6],
        IrOp::REPZ_CMPSB(StringOpr {
            segment: DS,
            is_addr32: true,
        }),
    );
}

#[test]
fn ir08_system_registers_and_interrupt_identity() {
    check(
        true,
        &[0x0f, 0x20, 0xc0],
        IrOp::MOV_Rd_Cr(EAX, CrIndex::CR0),
    );
    check(
        true,
        &[0x0f, 0x21, 0xc0],
        IrOp::MOV_Rd_Dr(EAX, DrIndex::DR0),
    );
    check(true, &[0xcc], IrOp::INT3);
    check(true, &[0xcd, 3], IrOp::INT_Ib(3));
}

#[test]
fn ir09_simd_prefix_selects_opcode_and_keeps_operands() {
    for (bytes, op, dst, src) in [
        (
            &[0x0f, 0x6f, 0xca][..],
            SimdOp::MOVQ_Pq_Qq,
            SimdReg::Mm(MmIndex::MM1),
            SimdReg::Mm(MmIndex::MM2),
        ),
        (
            &[0x66, 0x0f, 0x6f, 0xca],
            SimdOp::MOVDQA_Vo_Wo,
            SimdReg::Xmm(XmmIndex::XMM1),
            SimdReg::Xmm(XmmIndex::XMM2),
        ),
        (
            &[0xf3, 0x0f, 0x6f, 0xca],
            SimdOp::MOVDQU_Vo_Wo,
            SimdReg::Xmm(XmmIndex::XMM1),
            SimdReg::Xmm(XmmIndex::XMM2),
        ),
    ] {
        check(true, bytes, IrOp::SIMD_R(op, dst, src, 0));
    }
    check(
        true,
        &[0x66, 0x0f, 0x70, 0x48, 0x10, 0x1b],
        IrOp::SIMD_M(
            SimdOp::PSHUFD_Vo_Wo_Ib,
            SimdReg::Xmm(XmmIndex::XMM1),
            MemOpr::A32(MemOpr32 {
                segment: DS,
                base_index: BaseIndex32::Base(EAX),
                disp: Offset32(0x10),
            }),
            0x1b,
        ),
    );
}

#[test]
fn ir10_invalid_simd_consumes_its_address_and_immediate() {
    check(
        true,
        &[0xf2, 0x0f, 0x60, 0x84, 0x88, 0x78, 0x56, 0x34, 0x12],
        IrOp::UD,
    );
    check(true, &[0x0f, 0x71, 0xc0, 5], IrOp::UD);
}

#[test]
fn ir11_x87_metadata_is_separate_from_address_size() {
    for (bytes, op32) in [
        (&[0x67, 0xd9, 0x30][..], true),
        (&[0x66, 0x67, 0xd9, 0x30], false),
    ] {
        let mut fetch = SimpleFetcher::new(bytes);
        let ir = Decoder::use32().decode(&mut fetch).unwrap();
        let IrOp::ESC_Mb(meta, 6, mem) = ir else {
            panic!("{ir:?}")
        };
        assert_eq!(
            (meta.opcode, meta.modrm, meta.is_opsize32, meta.fop()),
            (0xd9, 0x30, op32, 0x130)
        );
        assert_eq!(
            mem,
            MemOpr::A16(MemOpr16 {
                segment: DS,
                base_index: Some(BaseIndex16::BxSi),
                disp: Offset16(0)
            })
        );
        check(true, bytes, ir);
    }
}

#[test]
fn ir12_truncated_inputs_propagate_fetch_errors() {
    // Every strict prefix is incomplete: prefix, opcode, ModR/M, SIB, disp32, imm8.
    let bytes = [0x66, 0x0f, 0x70, 0x8c, 0x88, 0x78, 0x56, 0x34, 0x12, 0x1b];
    for end in 0..bytes.len() {
        let mut fetch = SimpleFetcher::new(&bytes[..end]);
        assert_eq!(Decoder::use32().decode(&mut fetch), Err(()), "end={end}");
        // Wide fetchers may fail atomically; partial consumption is not a decoder promise.
        assert!(fetch.pos() <= end);
    }
    struct Failing;
    impl Fetch for Failing {
        type E = &'static str;
        fn fetch_u8(&mut self) -> Result<u8, Self::E> {
            Err("fetch failed")
        }
        fn current_eip(&self) -> Offset32 {
            Offset32(0)
        }
    }
    assert_eq!(Decoder::use32().decode(&mut Failing), Err("fetch failed"));
}

#[test]
fn ir13_fetch_values_bounds_and_offset_wrapping() {
    let mut fetch = SimpleFetcher::new(&[0x34, 0x12, 0xfe, 0xff, 0xff, 0xff]);
    assert_eq!(fetch.fetch_u16(), Ok(0x1234));
    assert_eq!(fetch.fetch_i32(), Ok(-2));
    assert_eq!(fetch.pos(), 6);
    assert_eq!(fetch.fetch_u8(), Err(()));
    fetch.set_pos(usize::MAX);
    assert_eq!(fetch.fetch_u16(), Err(()));
    assert_eq!(fetch.fetch_u32(), Err(()));
    let mut fetch = OffsetFetcher::new(&[0x34, 0x12, 0x80], Offset32(0xffff_fffe));
    assert_eq!(fetch.fetch_u16(), Ok(0x1234));
    assert_eq!(fetch.current_eip(), Offset32(0));
    assert_eq!(fetch.fetch_i8(), Ok(-128));
    fetch.set_pos(1);
    assert_eq!(fetch.current_eip(), Offset32(0xffff_ffff));
    assert_eq!(fetch.fetch_u32(), Err(()));
}
