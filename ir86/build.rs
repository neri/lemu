use std::collections::BTreeMap;
use std::fs::{File, read_to_string};
use std::io::*;

fn main() {
    {
        let csv_path = "./src/ir.csv";
        let mut lines = Vec::new();
        for line in read_to_string(csv_path).unwrap().lines() {
            if !line.is_empty() && !line.starts_with("#") {
                lines.push(line.to_string());
            }
        }

        let mut os = File::create("./src/_generated/ir.rs").unwrap();

        make_ir(&mut os, lines.as_slice());

        println!("cargo:rerun-if-changed={}", csv_path);
    }
    {
        let csv_path = "./src/simd.csv";
        let mut lines = Vec::new();
        for line in read_to_string(csv_path).unwrap().lines() {
            if !line.is_empty() && !line.starts_with("#") {
                lines.push(line.to_string());
            }
        }

        let mut os = File::create("./src/_generated/simd.rs").unwrap();

        make_simd(&mut os, lines.as_slice());

        println!("cargo:rerun-if-changed={}", csv_path);
    }
}

fn make_ir(os: &mut File, lines: &[String]) {
    let mut ir_orders = Vec::new();
    let mut ir_sources = BTreeMap::new();
    let mut mnemonics = BTreeMap::new();

    for (line_num, line) in lines.iter().enumerate().skip(1) {
        let cols = line.split(',').map(|s| s.trim()).collect::<Vec<_>>();

        let mnemonic = cols.get(0).unwrap_or(&"");
        if mnemonic.is_empty() || mnemonic.starts_with("#") {
            // Skip empty or commented mnemonic
            continue;
        }
        let mode = *cols.get(1).unwrap_or(&"");
        let sub_category = *cols.get(2).unwrap_or(&"");

        let mode = Mode::from_str(mode);
        let signature = format!("{}_{:#?}", mnemonic, mode);
        let sub_category = SubCategory::parse(sub_category);

        let ir_source = IrSource {
            signature: signature.clone(),
            mnemonic: mnemonic.to_uppercase().to_string(),
            ir_mnemonic: mnemonic.to_uppercase().replace('.', "_"),
            mode,
            sub_category,
        };

        if ir_sources.contains_key(&signature) {
            panic!(
                "Duplicate mnemonic '{}' at line {}",
                ir_source.signature,
                line_num + 1
            );
        }
        if !mnemonics.contains_key(&ir_source.mnemonic) {
            mnemonics.insert(ir_source.mnemonic.clone(), ());
        }
        ir_orders.push(signature.clone());
        ir_sources.insert(signature.clone(), ir_source);
    }
    let mut mnemonics = mnemonics
        .into_iter()
        .map(|(mnemonic, _)| mnemonic)
        .collect::<Vec<_>>();
    mnemonics.sort();

    let mut ir2s = Vec::new();
    for ir in ir_orders {
        let ir = ir_sources.get(&ir).unwrap();

        match ir.mode {
            Mode::Implied => {
                ir2s.push(Ir2 {
                    comment: format!("`{}`", ir.mnemonic),
                    variant_name: format!("{}", ir.ir_mnemonic),
                    args: "".to_string(),
                    n_args: 0,
                    mnemonic: ir.mnemonic.clone(),
                });
            }
            Mode::ModRM_R2 => {
                for (mode1, mode2) in [
                    (SubMode::Rb, SubMode::Rb),
                    (SubMode::Rw, SubMode::Rw),
                    (SubMode::Rd, SubMode::Rd),
                    (SubMode::Rb, SubMode::Mb),
                    (SubMode::Rw, SubMode::Mw),
                    (SubMode::Rd, SubMode::Md),
                    (SubMode::Mb, SubMode::Rb),
                    (SubMode::Mw, SubMode::Rw),
                    (SubMode::Md, SubMode::Rd),
                    (SubMode::Rb, SubMode::Ib),
                    (SubMode::Rw, SubMode::Iw),
                    (SubMode::Rd, SubMode::Id),
                    (SubMode::Mb, SubMode::Ib),
                    (SubMode::Mw, SubMode::Iw),
                    (SubMode::Md, SubMode::Id),
                ] {
                    if !ir
                        .sub_category
                        .test(mode1.sub_category_flags(), mode2.sub_category_flags())
                    {
                        continue;
                    }
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{} {}, {}`",
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                        ),
                        args: SubMode::signatures(&[mode1, mode2]),
                        n_args: 2,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::ModRM_R1 => {
                for submode in [
                    SubMode::Rb,
                    SubMode::Rw,
                    SubMode::Rd,
                    SubMode::Mb,
                    SubMode::Mw,
                    SubMode::Md,
                    SubMode::Mp,
                ] {
                    if (ir.sub_category.flags & submode.sub_category_flags())
                        != submode.sub_category_flags()
                    {
                        continue;
                    }
                    if ir.sub_category.contains(SubCategory::SPLIT_OPERAND_SIZE) {
                        for bits in [16, 32] {
                            ir2s.push(Ir2 {
                                comment: format!(
                                    "`o{}: {} {}`",
                                    bits,
                                    ir.mnemonic,
                                    submode.comment()
                                ),
                                variant_name: format!(
                                    "{}_{}O{}",
                                    ir.ir_mnemonic,
                                    submode.symbol(),
                                    bits
                                ),
                                args: format!("({})", submode.type_signature()),
                                n_args: 1,
                                mnemonic: ir.mnemonic.clone(),
                            });
                        }
                        continue;
                    }
                    ir2s.push(Ir2 {
                        comment: format!("`{} {}`", ir.mnemonic, submode.comment()),
                        variant_name: format!("{}_{}", ir.ir_mnemonic, submode.symbol()),
                        args: format!("({})", submode.type_signature()),
                        n_args: 1,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::ModRM_Sr => {
                for (mode1, mode2) in [
                    (SubMode::Rw, SubMode::Sr),
                    (SubMode::Rd, SubMode::Sr),
                    (SubMode::Mw, SubMode::Sr),
                    (SubMode::Md, SubMode::Sr),
                    (SubMode::Sr, SubMode::Rw),
                    (SubMode::Sr, SubMode::Rd),
                    (SubMode::Sr, SubMode::Mw),
                    (SubMode::Sr, SubMode::Md),
                ] {
                    if !ir
                        .sub_category
                        .test(mode1.sub_category_flags(), mode2.sub_category_flags())
                    {
                        continue;
                    }
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{} {}, {}`",
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                        ),
                        args: SubMode::signatures(&[mode1, mode2]),
                        n_args: 2,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Shift => {
                for (mode1, mode2) in [
                    (SubMode::Rb, SubMode::Ib),
                    (SubMode::Rw, SubMode::Ib),
                    (SubMode::Rd, SubMode::Ib),
                    (SubMode::Mb, SubMode::Ib),
                    (SubMode::Mw, SubMode::Ib),
                    (SubMode::Md, SubMode::Ib),
                    (SubMode::Rb, SubMode::Cl),
                    (SubMode::Rw, SubMode::Cl),
                    (SubMode::Rd, SubMode::Cl),
                    (SubMode::Mb, SubMode::Cl),
                    (SubMode::Mw, SubMode::Cl),
                    (SubMode::Md, SubMode::Cl),
                ] {
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{} {}, {}`",
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                        ),
                        args: SubMode::signatures(&[mode1, mode2]),
                        n_args: if matches!(mode2, SubMode::Cl) { 1 } else { 2 },
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::MovSx => {
                for (mode1, mode2) in [
                    (SubMode::Rw, SubMode::Rb),
                    (SubMode::Rd, SubMode::Rb),
                    (SubMode::Rd, SubMode::Rw),
                    (SubMode::Rw, SubMode::Mb),
                    (SubMode::Rd, SubMode::Mb),
                    (SubMode::Rd, SubMode::Mw),
                ] {
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{} {}, {}`",
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                        ),
                        args: SubMode::signatures(&[mode1, mode2]),
                        n_args: 2,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::SelectorQuery => {
                for (dst, src) in [
                    (SubMode::Rw, SubMode::Rw),
                    (SubMode::Rw, SubMode::Mw),
                    (SubMode::Rd, SubMode::Rw),
                    (SubMode::Rd, SubMode::Mw),
                ] {
                    ir2s.push(Ir2 {
                        comment: format!("`{} {}, {}`", ir.mnemonic, dst.comment(), src.comment()),
                        variant_name: format!(
                            "{}_{}_{}",
                            ir.ir_mnemonic,
                            dst.symbol(),
                            src.symbol()
                        ),
                        args: SubMode::signatures(&[dst, src]),
                        n_args: 2,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Imul3 => {
                for (mode1, mode2, mode3) in [
                    (SubMode::Rw, SubMode::Rw, SubMode::Iw),
                    (SubMode::Rd, SubMode::Rd, SubMode::Id),
                    (SubMode::Rw, SubMode::Mw, SubMode::Iw),
                    (SubMode::Rd, SubMode::Md, SubMode::Id),
                ] {
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{} {}, {}, {}`",
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                            mode3.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                            mode3.symbol(),
                        ),
                        args: SubMode::signatures(&[mode1, mode2, mode3]),
                        n_args: 3,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Jcc | Mode::Loop | Mode::Jump => {
                for (suffix, ty) in [("Jw", "Offset16"), ("Jd", "Offset32")] {
                    let (args, n_args) = match ir.mode {
                        Mode::Jcc => (format!("(CC, {})", ty), 2),
                        Mode::Loop if !matches!(ir.mnemonic.as_str(), "JCXZ" | "JECXZ") => {
                            (format!("(bool, {})", ty), 2)
                        }
                        _ => (format!("({})", ty), 1),
                    };
                    ir2s.push(Ir2 {
                        comment: if matches!(ir.mode, Mode::Loop)
                            && !matches!(ir.mnemonic.as_str(), "JCXZ" | "JECXZ")
                        {
                            format!(
                                "`{} addr32, {}`; addr32: true = ECX, false = CX",
                                ir.mnemonic, ty
                            )
                        } else {
                            format!("`{} {}`", ir.mnemonic, ty)
                        },
                        variant_name: format!("{}_{}", ir.ir_mnemonic, suffix),
                        args,
                        n_args,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
                if !matches!(ir.mode, Mode::Jump) {
                    continue;
                }
                for submode in [SubMode::Rw, SubMode::Rd, SubMode::Mw, SubMode::Md] {
                    ir2s.push(Ir2 {
                        comment: format!("`{} {}`", ir.mnemonic, submode.comment()),
                        variant_name: format!("{}_{}", ir.ir_mnemonic, submode.symbol()),
                        args: format!("({})", submode.type_signature()),
                        n_args: 1,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::JumpF => {
                for (bits, ty) in [(16, "Offset16"), (32, "Offset32")] {
                    ir2s.push(Ir2 {
                        comment: format!("`o{}: {} a16:{}`", bits, ir.mnemonic, bits),
                        variant_name: format!("{}_ApO{}", ir.ir_mnemonic, bits),
                        args: format!("(SegmentSelector, {})", ty),
                        n_args: 2,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
                let submode = SubMode::Mp;
                for bits in [16, 32] {
                    ir2s.push(Ir2 {
                        comment: format!("`o{}: {} {}`", bits, ir.mnemonic, submode.comment()),
                        variant_name: format!("{}_{}O{}", ir.ir_mnemonic, submode.symbol(), bits),
                        args: format!("({})", submode.type_signature()),
                        n_args: 1,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Ret => {
                for bits in [16, 32] {
                    ir2s.push(Ir2 {
                        comment: format!("`d{}: {}`", bits, ir.mnemonic),
                        variant_name: format!("{}_D{}", ir.ir_mnemonic, bits),
                        args: "(u16)".to_string(),
                        n_args: 1,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Ib => {
                ir2s.push(Ir2 {
                    comment: format!("`{} imm8`", ir.mnemonic),
                    variant_name: format!("{}_Ib", ir.ir_mnemonic),
                    args: "(u8)".to_string(),
                    n_args: 1,
                    mnemonic: ir.mnemonic.clone(),
                });
            }
            Mode::Iv => {
                ir2s.push(Ir2 {
                    comment: format!("`{} imm16`", ir.mnemonic),
                    variant_name: format!("{}_Iw", ir.ir_mnemonic),
                    args: "(u16)".to_string(),
                    n_args: 1,
                    mnemonic: ir.mnemonic.clone(),
                });
                ir2s.push(Ir2 {
                    comment: format!("`{} imm32`", ir.mnemonic),
                    variant_name: format!("{}_Id", ir.ir_mnemonic),
                    args: "(u32)".to_string(),
                    n_args: 1,
                    mnemonic: ir.mnemonic.clone(),
                });
            }
            Mode::Sr => {
                for bits in [16, 32] {
                    ir2s.push(Ir2 {
                        comment: format!("`o{}: {} sr`", bits, ir.mnemonic),
                        variant_name: format!("{}_SrO{}", ir.ir_mnemonic, bits),
                        args: "(SrIndex)".to_string(),
                        n_args: 1,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Out => {
                for (symbol, args, match_args, comment) in [
                    ("Ib_Al", "(u8)", 1, "imm8, al"),
                    ("Dx_Al", "", 0, "dx, al"),
                    ("Ib_Aw", "(u8)", 1, "imm8, ax"),
                    ("Dx_Aw", "", 0, "dx, ax"),
                    ("Ib_Ad", "(u8)", 1, "imm8, eax"),
                    ("Dx_Ad", "", 0, "dx, eax"),
                ] {
                    ir2s.push(Ir2 {
                        comment: format!("`{} {}`", ir.mnemonic, comment),
                        variant_name: format!("{}_{}", ir.ir_mnemonic, symbol),
                        args: args.to_string(),
                        n_args: match_args,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::In => {
                for (symbol, args, match_args, comment) in [
                    ("Al_Ib", "(u8)", 1, "al, imm8"),
                    ("Al_Dx", "", 0, "al, dx"),
                    ("Aw_Ib", "(u8)", 1, "ax, imm8"),
                    ("Aw_Dx", "", 0, "ax, dx"),
                    ("Ad_Ib", "(u8)", 1, "eax, imm8"),
                    ("Ad_Dx", "", 0, "eax, dx"),
                ] {
                    ir2s.push(Ir2 {
                        comment: format!("`{} {}`", ir.mnemonic, comment),
                        variant_name: format!("{}_{}", ir.ir_mnemonic, symbol),
                        args: args.to_string(),
                        n_args: match_args,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::String => {
                for suffix in ["b", "w", "d"] {
                    let comment_args = if ir.sub_category.contains(SubCategory::STRING_ES) {
                        "(StringOprEs)"
                    } else {
                        "(StringOpr)"
                    };
                    let variant_args = if ir.sub_category.contains(SubCategory::STRING_ES) {
                        "(StringOprEs)"
                    } else {
                        "(StringOpr)"
                    };
                    ir2s.push(Ir2 {
                        comment: format!("`{}{} {}`", ir.mnemonic, suffix, comment_args),
                        variant_name: format!("{}{}", ir.ir_mnemonic, suffix.to_uppercase()),
                        args: variant_args.to_string(),
                        n_args: 1,
                        mnemonic: ir.mnemonic.clone(),
                    });
                    if ir.sub_category.contains(SubCategory::ALLOWS_REPZ) {
                        ir2s.push(Ir2 {
                            comment: format!("`REPZ {}{} {}`", ir.mnemonic, suffix, comment_args),
                            variant_name: format!(
                                "REPZ_{}{}",
                                ir.ir_mnemonic,
                                suffix.to_uppercase()
                            ),
                            args: variant_args.to_string(),
                            n_args: 1,
                            mnemonic: ir.mnemonic.clone(),
                        });
                        ir2s.push(Ir2 {
                            comment: format!("`REPNZ {}{} {}`", ir.mnemonic, suffix, comment_args),
                            variant_name: format!(
                                "REPNZ_{}{}",
                                ir.ir_mnemonic,
                                suffix.to_uppercase()
                            ),
                            args: variant_args.to_string(),
                            n_args: 1,
                            mnemonic: ir.mnemonic.clone(),
                        });
                    } else {
                        ir2s.push(Ir2 {
                            comment: format!("`REP {}{} {}`", ir.mnemonic, suffix, comment_args),
                            variant_name: format!(
                                "REP_{}{}",
                                ir.ir_mnemonic,
                                suffix.to_uppercase()
                            ),
                            args: variant_args.to_string(),
                            n_args: 1,
                            mnemonic: ir.mnemonic.clone(),
                        });
                    }
                }
            }
            Mode::Enter => {
                ir2s.push(Ir2 {
                    comment: format!("`{} imm16, imm8`", ir.mnemonic),
                    variant_name: format!("{}_Iw_Ib", ir.ir_mnemonic),
                    args: "(u16, u8)".to_string(),
                    n_args: 2,
                    mnemonic: ir.mnemonic.clone(),
                });
            }
            Mode::Cr => {
                for (mode1, mode2) in [
                    (SubMode::Rd, SubMode::Cr),
                    (SubMode::Cr, SubMode::Rd),
                    (SubMode::Rd, SubMode::Dr),
                    (SubMode::Dr, SubMode::Rd),
                ] {
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{} {}, {}`",
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                        ),
                        args: SubMode::signatures(&[mode1, mode2]),
                        n_args: 2,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Bt => {
                for (mode1, mode2) in [
                    (SubMode::Rw, SubMode::Rw),
                    (SubMode::Rd, SubMode::Rd),
                    (SubMode::Mw, SubMode::Rw),
                    (SubMode::Md, SubMode::Rd),
                    (SubMode::Rw, SubMode::Ib),
                    (SubMode::Rd, SubMode::Ib),
                    (SubMode::Mw, SubMode::Ib),
                    (SubMode::Md, SubMode::Ib),
                ] {
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{} {}, {}`",
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                        ),
                        args: SubMode::signatures(&[mode1, mode2]),
                        n_args: 2,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::SetCc => {
                for submode in [SubMode::Rb, SubMode::Mb] {
                    ir2s.push(Ir2 {
                        comment: format!("`{} cc, {}`", ir.mnemonic, submode.comment(),),
                        variant_name: format!("{}_{}", ir.ir_mnemonic, submode.symbol()),
                        args: format!("(CC, {})", submode.type_signature()),
                        n_args: 2,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Shld => {
                for (mode1, mode2, mode3) in [
                    (SubMode::Rw, SubMode::Rw, SubMode::Ib),
                    (SubMode::Rd, SubMode::Rd, SubMode::Ib),
                    (SubMode::Mw, SubMode::Rw, SubMode::Ib),
                    (SubMode::Md, SubMode::Rd, SubMode::Ib),
                    (SubMode::Rw, SubMode::Rw, SubMode::Cl),
                    (SubMode::Rd, SubMode::Rd, SubMode::Cl),
                    (SubMode::Mw, SubMode::Rw, SubMode::Cl),
                    (SubMode::Md, SubMode::Rd, SubMode::Cl),
                ] {
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{} {}, {}, {}`",
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                            mode3.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                            mode3.symbol()
                        ),
                        args: SubMode::signatures(&[mode1, mode2, mode3]),
                        n_args: if matches!(mode3, SubMode::Cl) { 2 } else { 3 },
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Cmov => {
                for (mode1, mode2) in [
                    (SubMode::Rw, SubMode::Rw),
                    (SubMode::Rd, SubMode::Rd),
                    (SubMode::Rw, SubMode::Mw),
                    (SubMode::Rd, SubMode::Md),
                ] {
                    ir2s.push(Ir2 {
                        comment: format!(
                            "`{} cc, {}, {}`",
                            ir.mnemonic,
                            mode1.comment(),
                            mode2.comment(),
                        ),
                        variant_name: format!(
                            "{}_{}_{}",
                            ir.ir_mnemonic,
                            mode1.symbol(),
                            mode2.symbol(),
                        ),
                        args: format!(
                            "(CC, {}, {})",
                            mode1.type_signature(),
                            mode2.type_signature()
                        ),
                        n_args: 3,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Simd => {
                for (suffix, comment, args, n_args) in [
                    ("", "op", "(SimdOp)", 0),
                    (
                        "_R",
                        "op reg, r/m-reg, imm8",
                        "(SimdOp, SimdReg, SimdReg, u8)",
                        3,
                    ),
                    (
                        "_M",
                        "op reg, [mem], imm8",
                        "(SimdOp, SimdReg, MemOpr, u8)",
                        3,
                    ),
                    (
                        "_S",
                        "op reg, r/m-reg, implicit [seg:(e)di]",
                        "(SimdOp, SimdReg, SimdReg, StringOpr)",
                        3,
                    ),
                ] {
                    ir2s.push(Ir2 {
                        comment: format!("`simd {}`", comment),
                        variant_name: format!("{}{}", ir.ir_mnemonic, suffix),
                        args: args.to_string(),
                        n_args: n_args + 1,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
            Mode::Esc => {
                for submode in [SubMode::Rb, SubMode::Mb] {
                    ir2s.push(Ir2 {
                        comment: format!("`esc op3, reg3, {}`", submode.comment(),),
                        variant_name: format!("{}_{}", ir.ir_mnemonic, submode.symbol()),
                        args: format!("(EscOpcode, u8, {})", submode.type_signature()),
                        n_args: 3,
                        mnemonic: ir.mnemonic.clone(),
                    });
                }
            }
        }
    }
    ir2s.sort_by(|a, b| a.variant_name.cmp(&b.variant_name));

    write!(
        os,
        "// This file is automatically @generated at build time. DO NOT EDIT DIRECTLY.

use crate::prelude::*;

/// Intermediate Representation for x86 opcodes
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrOp {{
"
    )
    .unwrap();

    for ir2 in &ir2s {
        writeln!(os, "    /// {}", ir2.comment).unwrap();
        writeln!(os, "    {}{},", ir2.variant_name, ir2.args).unwrap();
    }

    writeln!(
        os,
        "}}

impl IrOp {{
    pub fn mnemonic(&self) -> Mnemonic {{
        match self {{"
    )
    .unwrap();

    for ir2 in &ir2s {
        writeln!(
            os,
            "            Self::{}{} => Mnemonic::{},",
            ir2.variant_name,
            ir2.match_args_anonymous(),
            ir2.mnemonic
        )
        .unwrap();
    }

    writeln!(
        os,
        "        }}
    }}

    pub fn name(&self) -> &str {{
        match self {{"
    )
    .unwrap();

    for ir2 in &ir2s {
        writeln!(
            os,
            "            Self::{}{} => {:?},",
            ir2.variant_name,
            ir2.match_args_anonymous(),
            ir2.variant_name
        )
        .unwrap();
    }

    writeln!(
        os,
        "        }}
    }}

    pub fn n_args(&self) -> usize {{
        match self {{"
    )
    .unwrap();

    for ir2 in &ir2s {
        writeln!(
            os,
            "            Self::{}{} => {},",
            ir2.variant_name,
            ir2.match_args_anonymous(),
            ir2.n_args,
        )
        .unwrap();
    }

    writeln!(
        os,
        "        }}
    }}

    /// Returns `true` for an MMX, SSE or SSE2 form (`SIMD`, `SIMD_R`, `SIMD_M` or `SIMD_S`).
    #[inline]
    pub const fn is_simd(&self) -> bool {{
        matches!(
            self,
            Self::SIMD(..) | Self::SIMD_R(..) | Self::SIMD_M(..) | Self::SIMD_S(..)
        )
    }}
}}

/// Well Known Mnemonic for x86 instructions
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mnemonic {{"
    )
    .unwrap();

    for mnemonic in mnemonics {
        writeln!(os, "    {},", mnemonic.to_uppercase()).unwrap();
    }

    writeln!(os, "}}").unwrap();
}

struct IrSource {
    signature: String,
    mnemonic: String,
    ir_mnemonic: String,
    mode: Mode,
    sub_category: SubCategory,
}

struct Ir2 {
    comment: String,
    variant_name: String,
    args: String,
    n_args: usize,
    mnemonic: String,
}

impl Ir2 {
    fn match_args_anonymous(&self) -> String {
        let n_args = self.n_args;
        if n_args == 0 {
            return String::new();
        }
        let args = (0..n_args).map(|_| "_").collect::<Vec<_>>().join(", ");
        format!("({})", args)
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Mode {
    /// Implied
    Implied,
    /// BT/BTS/BTR/BTC instructions
    Bt,
    /// CMOVcc instructions
    Cmov,
    /// MOV to/from Control Register
    Cr,
    /// ENTER instruction
    Enter,
    /// opcode flows immediate byte, used for INT instruction and some instructions
    Ib,
    /// IMUL with 3 operands
    Imul3,
    /// IN instruction
    In,
    /// opcode flows immediate word
    Iv,
    /// Jcc instruction
    Jcc,
    /// JMP instruction
    Jump,
    /// JMP FAR instruction
    JumpF,
    /// Loop instruction (loop, loopz, loopnz, jcxz, jecxz)
    Loop,
    /// ModR/M with single operand
    ModRM_R1,
    /// ModR/M generic format (two operands)
    ModRM_R2,
    /// ModR/M with Segment Register
    ModRM_Sr,
    /// MOVSX/MOVZX instruction
    MovSx,
    /// LAR/LSL: selector input is always 16-bit; result is 16/32-bit.
    SelectorQuery,
    /// OUT instruction
    Out,
    /// RET instruction
    Ret,
    /// SETcc instruction
    SetCc,
    /// Shift and rotate instructions (ROL, ROR, RCL, RCR, SHL/SAL, SHR, SAR)
    Shift,
    /// SHLD/SHRD instruction
    Shld,
    /// Segment Register
    Sr,
    /// String instructions (MOVS, CMPS, STOS, LODS, SCAS)
    String,
    /// ESC instruction
    Esc,
    /// MMX/SSE/SSE2 forms described by `SimdOp` (src/simd.csv)
    Simd,
}

impl Mode {
    fn from_str(s: &str) -> Self {
        match s {
            "" | "implied" => Self::Implied,
            "bt" => Self::Bt,
            "cmov" => Self::Cmov,
            "cr" => Self::Cr,
            "enter" => Self::Enter,
            "esc" => Self::Esc,
            "ib" => Self::Ib,
            "imul3" => Self::Imul3,
            "in" => Self::In,
            "iv" => Self::Iv,
            "jcc" => Self::Jcc,
            "jump" => Self::Jump,
            "jumpf" => Self::JumpF,
            "loop" => Self::Loop,
            "modrm_r1" => Self::ModRM_R1,
            "modrm_r2" => Self::ModRM_R2,
            "modrm_sr" => Self::ModRM_Sr,
            "movsx" => Self::MovSx,
            "selector_query" => Self::SelectorQuery,
            "out" => Self::Out,
            "ret" => Self::Ret,
            "setcc" => Self::SetCc,
            "shift" => Self::Shift,
            "shld" => Self::Shld,
            "simd" => Self::Simd,
            "sr" => Self::Sr,
            "string" => Self::String,
            _ => panic!("Unknown mode '{}'", s),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SubMode {
    Rb,
    Rw,
    Rd,
    Ib,
    Iw,
    Id,
    Mb,
    Mw,
    Md,
    Mp,
    Cl,
    Sr,
    Cr,
    Dr,
}

impl SubMode {
    fn comment(&self) -> String {
        match self {
            Self::Rb => "reg8",
            Self::Rw => "reg16",
            Self::Rd => "reg32",
            Self::Cl => "cl",
            Self::Ib => "imm8",
            Self::Iw => "imm16",
            Self::Id => "imm32",
            Self::Mb => "[mem8]",
            Self::Mw => "[mem16]",
            Self::Md => "[mem32]",
            Self::Mp => "[mem]",
            Self::Sr => "sr",
            Self::Cr => "cr",
            Self::Dr => "dr",
        }
        .to_string()
    }

    fn symbol(&self) -> String {
        match self {
            Self::Rb => "Rb",
            Self::Rw => "Rw",
            Self::Rd => "Rd",
            Self::Ib => "Ib",
            Self::Iw => "Iw",
            Self::Id => "Id",
            Self::Cl => "Cl",
            Self::Sr => "Sr",
            Self::Cr => "Cr",
            Self::Dr => "Dr",
            Self::Mb => "Mb",
            Self::Mw => "Mw",
            Self::Md => "Md",
            Self::Mp => "Mp",
        }
        .to_string()
    }

    fn type_signature(&self) -> String {
        match self {
            Self::Rb => "GprIndex8",
            Self::Rw => "GprIndex16",
            Self::Rd => "GprIndex32",
            Self::Ib => "u8",
            Self::Iw => "u16",
            Self::Id => "u32",
            Self::Cl => "",
            Self::Sr => "SrIndex",
            Self::Cr => "CrIndex",
            Self::Dr => "DrIndex",
            Self::Mb | Self::Mw | Self::Md | Self::Mp => "MemOpr",
        }
        .to_string()
    }

    fn sub_category_flags(&self) -> u32 {
        match self {
            Self::Ib => SubCategory::ALLOWS_IMMEDIATE | SubCategory::ALLOWS_BYTE,
            Self::Iw => SubCategory::ALLOWS_IMMEDIATE | SubCategory::ALLOWS_WORD,
            Self::Id => SubCategory::ALLOWS_IMMEDIATE | SubCategory::ALLOWS_DWORD,
            Self::Rb => SubCategory::ALLOWS_REGISTER | SubCategory::ALLOWS_BYTE,
            Self::Rw => SubCategory::ALLOWS_REGISTER | SubCategory::ALLOWS_WORD,
            Self::Rd => SubCategory::ALLOWS_REGISTER | SubCategory::ALLOWS_DWORD,
            Self::Mb => SubCategory::ALLOWS_MEMORY | SubCategory::ALLOWS_BYTE,
            Self::Mw => SubCategory::ALLOWS_MEMORY | SubCategory::ALLOWS_WORD,
            Self::Md => SubCategory::ALLOWS_MEMORY | SubCategory::ALLOWS_DWORD,
            Self::Mp => SubCategory::ALLOWS_MEMORY | SubCategory::ALLOWS_FWORD,
            Self::Cl => 0,
            Self::Sr => SubCategory::ALLOWS_SEGMENT,
            Self::Cr => SubCategory::ALLOWS_DWORD,
            Self::Dr => SubCategory::ALLOWS_DWORD,
        }
    }

    fn signatures(types: &[SubMode]) -> String {
        let mut signatures = Vec::new();
        for submode in types {
            let type_signature = submode.type_signature();
            if !type_signature.is_empty() {
                signatures.push(type_signature);
            }
        }
        if signatures.is_empty() {
            "".to_string()
        } else {
            format!("({})", signatures.join(", "))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubCategory {
    flags: u32,
}

impl SubCategory {
    pub const ALLOWS_BYTE: u32 = 0b0000_0000_0000_0001;
    pub const ALLOWS_WORD: u32 = 0b0000_0000_0000_0010;
    pub const ALLOWS_DWORD: u32 = 0b0000_0000_0000_0100;
    pub const ALLOWS_FWORD: u32 = 0b0000_0000_0000_1000;

    pub const ALLOWS_REGISTER: u32 = 0b0000_0000_0001_0000;
    pub const ALLOWS_MEMORY: u32 = 0b0000_0000_0010_0000;
    pub const ALLOWS_IMMEDIATE: u32 = 0b0000_0000_0100_0000;
    pub const ALLOWS_SEGMENT: u32 = 0b0000_0000_1000_0000;

    pub const ALLOWS_LOAD: u32 = 0b0000_0001_0000_0000;
    pub const ALLOWS_STORE: u32 = 0b0000_0010_0000_0000;

    pub const ALLOWS_REPZ: u32 = 0b0001_0000_0000_0000;
    pub const STRING_ES: u32 = 0b0010_0000_0000_0000;
    /// Emit separate o16/o32 variants when the operand size changes the memory format.
    pub const SPLIT_OPERAND_SIZE: u32 = 0b0100_0000_0000_0000;

    pub const ALLOWS_SIZE: u32 =
        Self::ALLOWS_BYTE | Self::ALLOWS_WORD | Self::ALLOWS_DWORD | Self::ALLOWS_FWORD;
    pub const ALLOWS_SIZE_DEFAULT: u32 = Self::ALLOWS_BYTE | Self::ALLOWS_WORD | Self::ALLOWS_DWORD;

    pub const ALLOWS_RMI: u32 =
        Self::ALLOWS_REGISTER | Self::ALLOWS_MEMORY | Self::ALLOWS_IMMEDIATE;

    pub const ALLOWS_LOAD_STORE: u32 = Self::ALLOWS_LOAD | Self::ALLOWS_STORE;

    pub fn parse(s: &str) -> Self {
        let mut flags = 0;

        for ch in s.chars() {
            match ch {
                'b' => flags |= Self::ALLOWS_BYTE,
                'w' => flags |= Self::ALLOWS_WORD,
                'd' => flags |= Self::ALLOWS_DWORD,
                'p' => flags |= Self::ALLOWS_FWORD,
                'r' => flags |= Self::ALLOWS_REGISTER,
                'm' => flags |= Self::ALLOWS_MEMORY,
                'i' => flags |= Self::ALLOWS_IMMEDIATE,
                'l' => flags |= Self::ALLOWS_LOAD,
                's' => flags |= Self::ALLOWS_STORE,
                'z' => flags |= Self::ALLOWS_REPZ,
                'e' => flags |= Self::STRING_ES,
                'o' => flags |= Self::SPLIT_OPERAND_SIZE,
                _ => panic!("Unknown subcategory character '{}'", ch),
            }
        }

        if flags & Self::ALLOWS_SIZE == 0 {
            flags |= Self::ALLOWS_SIZE_DEFAULT;
        }
        if flags & Self::ALLOWS_RMI == 0 {
            flags |= Self::ALLOWS_RMI;
        }
        if flags & Self::ALLOWS_LOAD_STORE == 0 {
            flags |= Self::ALLOWS_LOAD_STORE;
        }
        Self { flags }
    }

    #[inline]
    pub fn contains(&self, flags: u32) -> bool {
        (self.flags & flags) == flags
    }

    pub fn test(&self, flags1: u32, flags2: u32) -> bool {
        let mut actual = flags1 | flags2;
        if (flags1 & Self::ALLOWS_MEMORY) != 0 {
            actual |= Self::ALLOWS_STORE;
        }
        if (flags2 & Self::ALLOWS_MEMORY) != 0 {
            actual |= Self::ALLOWS_LOAD;
        }

        if (self.flags & actual & Self::ALLOWS_SIZE) == 0 {
            return false;
        }
        if (actual & Self::ALLOWS_MEMORY) != 0
            && (self.flags & actual & Self::ALLOWS_LOAD_STORE) == 0
        {
            return false;
        }

        if !self.contains(Self::ALLOWS_REGISTER) && self.contains(Self::ALLOWS_MEMORY) {
            let expected = Self::ALLOWS_REGISTER | Self::ALLOWS_MEMORY;
            (actual & expected) == expected
        } else {
            let lhs = self.flags & Self::ALLOWS_RMI;
            let rhs = actual & Self::ALLOWS_RMI;
            (lhs & rhs) == rhs
        }
    }
}

/// One row of src/simd.csv.
struct SimdRow {
    name: String,
    mnemonic: String,
    prefix: &'static str,
    opcode: u8,
    ext: Option<u8>,
    /// `RegKind` of the ModR/M.reg operand.
    reg: &'static str,
    /// `RegKind` of the ModR/M.r/m operand in its register form.
    rm: &'static str,
    has_reg_form: bool,
    has_mem_form: bool,
    has_modrm: bool,
    mem_size: u16,
    has_imm8: bool,
    rm_is_dest: bool,
    class: &'static str,
    isa: &'static str,
    unaligned: bool,
    implicit_di: bool,
}

fn simd_width(s: &str, name: &str) -> u16 {
    match s {
        "b" => 1,
        "w" => 2,
        "d" => 4,
        "q" => 8,
        "o" => 16,
        "512" => 512,
        _ => panic!("Unknown operand width '{}' in {}", s, name),
    }
}

fn parse_simd_row(line: &str) -> SimdRow {
    let cols = line.split(',').map(|s| s.trim()).collect::<Vec<_>>();
    assert_eq!(cols.len(), 8, "simd.csv: expected 8 columns: {}", line);
    let name = cols[0].to_string();
    let mut parts = name.split('_');
    let mnemonic = parts.next().unwrap().to_string();

    let mut reg = "None";
    let mut rm = "None";
    let mut has_reg_form = false;
    let mut has_mem_form = false;
    let mut mem_size = 0;
    let mut has_imm8 = false;
    let mut rm_is_dest = None;
    for (index, operand) in parts.enumerate() {
        let (kind, width) = operand.split_at(1);
        let is_rm = match kind {
            "V" | "P" | "G" => {
                assert_eq!(reg, "None", "{}: two ModR/M.reg operands", name);
                reg = match kind {
                    "V" => "Xmm",
                    "P" => "Mm",
                    _ => "Gpr",
                };
                simd_width(width, &name);
                false
            }
            "W" | "U" | "Q" | "N" | "E" | "M" => {
                assert_eq!(rm, "None", "{}: two ModR/M.r/m operands", name);
                assert!(!has_mem_form && !has_reg_form, "{}: two r/m operands", name);
                rm = match kind {
                    "W" | "U" => "Xmm",
                    "Q" | "N" => "Mm",
                    "E" => "Gpr",
                    _ => "None",
                };
                has_reg_form = kind != "M";
                has_mem_form = !matches!(kind, "U" | "N");
                if has_mem_form {
                    mem_size = simd_width(width, &name);
                } else {
                    simd_width(width, &name);
                }
                true
            }
            "I" => {
                assert_eq!(width, "b", "{}: only imm8 is supported", name);
                has_imm8 = true;
                continue;
            }
            _ => panic!("Unknown operand kind '{}' in {}", kind, name),
        };
        if rm_is_dest.is_none() {
            rm_is_dest = Some(is_rm && index == 0);
        }
    }

    let prefix = match cols[1] {
        "*" => "_",
        "-" => "None",
        "66" => "P66",
        "F3" => "F3",
        "F2" => "F2",
        p => panic!("{}: unknown prefix '{}'", name, p),
    };
    let opcode = u8::from_str_radix(cols[2], 16).unwrap();
    let ext = match cols[3] {
        "-" => None,
        e => Some(e.parse::<u8>().unwrap()),
    };
    if ext.is_some() {
        assert_eq!(reg, "None", "{}: ModR/M.reg is an opcode extension", name);
    }
    match cols[4] {
        "-" => {}
        "r" | "m" => {
            assert!(
                !has_reg_form && !has_mem_form,
                "{}: mod column is only for forms without an r/m operand",
                name
            );
            assert!(
                ext.is_some(),
                "{}: mod column needs an opcode extension",
                name
            );
            has_reg_form = cols[4] == "r";
            has_mem_form = cols[4] == "m";
        }
        m => panic!("{}: unknown mod '{}'", name, m),
    }
    let has_modrm = has_reg_form || has_mem_form;
    if !has_modrm {
        assert!(
            ext.is_none() && reg == "None",
            "{}: needs a ModR/M form",
            name
        );
    }
    let class = match cols[5] {
        "xmm" => "Xmm",
        "mmxmm" => "MmXmm",
        "mm" => "Mm",
        "fx" => "FxState",
        "none" => "NoCheck",
        c => panic!("{}: unknown class '{}'", name, c),
    };
    let isa = match cols[6] {
        "mmx" => "Mmx",
        "sse" => "Sse",
        "sse2" => "Sse2",
        "fxsr" => "Fxsr",
        "clfsh" => "Clfsh",
        i => panic!("{}: unknown isa '{}'", name, i),
    };
    let mut unaligned = false;
    let mut implicit_di = false;
    let mut store = false;
    match cols[7] {
        "-" => {}
        "u" => unaligned = true,
        "di" => implicit_di = true,
        "st" => store = true,
        f => panic!("{}: unknown flags '{}'", name, f),
    }
    // With a ModR/M.reg operand, operand order says which side is written.
    // A lone r/m operand is a source unless marked `st`; a lone register-only
    // r/m operand (U/N with an opcode extension) is read and written.
    let rm_is_dest = if reg != "None" {
        assert!(!store, "{}: st is only for a lone r/m operand", name);
        rm_is_dest.unwrap_or(false)
    } else {
        store || (rm != "None" && has_reg_form && !has_mem_form)
    };
    if unaligned {
        assert_eq!(mem_size, 16, "{}: u is only for m128", name);
    }
    if implicit_di {
        assert!(
            has_reg_form && !has_mem_form,
            "{}: di needs a register-only r/m",
            name
        );
    }

    SimdRow {
        name,
        mnemonic,
        prefix,
        opcode,
        ext,
        reg,
        rm,
        has_reg_form,
        has_mem_form,
        has_modrm,
        mem_size,
        has_imm8,
        rm_is_dest,
        class,
        isa,
        unaligned,
        implicit_di,
    }
}

fn make_simd(os: &mut File, lines: &[String]) {
    let rows = lines
        .iter()
        .skip(1)
        .map(|line| parse_simd_row(line))
        .collect::<Vec<_>>();

    // Every (opcode, prefix, ext, form) selects at most one row.
    let mut slots = BTreeMap::new();
    let mut names = BTreeMap::new();
    for row in &rows {
        if names.insert(row.name.clone(), ()).is_some() {
            panic!("simd.csv: duplicate name {}", row.name);
        }
        let exts = match row.ext {
            Some(ext) => vec![ext],
            None => (0..8).collect(),
        };
        let forms = [
            (false, row.has_reg_form || !row.has_modrm),
            (true, row.has_mem_form),
        ];
        let prefixes = match row.prefix {
            "_" => vec!["None", "P66", "F3", "F2"],
            p => vec![p],
        };
        for prefix in prefixes {
            for &ext in &exts {
                for (is_mem, present) in forms {
                    if !present {
                        continue;
                    }
                    let key = (row.opcode, prefix, ext, is_mem);
                    if let Some(other) = slots.insert(key, row.name.clone()) {
                        panic!("simd.csv: {} overlaps {}", row.name, other);
                    }
                }
            }
        }
    }

    // ModR/M and imm8 presence is a property of the opcode, whatever the prefix.
    let mut shapes: BTreeMap<u8, (bool, bool, String)> = BTreeMap::new();
    for row in &rows {
        let shape = (row.has_modrm, row.has_imm8);
        match shapes.get(&row.opcode) {
            Some((m, i, other)) if (*m, *i) != shape => {
                panic!(
                    "simd.csv: {} and {} disagree on ModR/M or imm8",
                    row.name, other
                )
            }
            Some(_) => {}
            None => {
                shapes.insert(row.opcode, (shape.0, shape.1, row.name.clone()));
            }
        }
    }

    write!(
        os,
        "// This file is automatically @generated at build time. DO NOT EDIT DIRECTLY.

use crate::_prelude_::*;
use super::{{SimdClass, SimdIsa, SimdPrefix, SimdRegKind, SimdShape}};

/// An MMX, SSE or SSE2 form (src/simd.csv).
///
/// Operands follow Intel order: V/P/G are ModR/M.reg (xmm, mm, r32); W/Q/E are
/// ModR/M.r/m as a register (xmm, mm, r32) or memory; U/N are r/m registers
/// only; M is memory only; I is imm8. The width suffix is b/w/d/q/o (1-16
/// bytes) or 512.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SimdOp {{
"
    )
    .unwrap();
    for row in &rows {
        let prefix = match row.prefix {
            "None" => "".to_string(),
            "_" => "(any prefix) ".to_string(),
            "P66" => "66 ".to_string(),
            p => format!("{} ", p),
        };
        let ext = row.ext.map(|e| format!(" /{}", e)).unwrap_or_default();
        writeln!(os, "    /// `{}0F {:02X}{}`", prefix, row.opcode, ext).unwrap();
        writeln!(os, "    {},", row.name).unwrap();
    }
    writeln!(os, "}}\n\nimpl SimdOp {{").unwrap();
    writeln!(os, "    /// Every form, indexed by its discriminant.").unwrap();
    writeln!(os, "    pub const ALL: [SimdOp; {}] = [", rows.len()).unwrap();
    for row in &rows {
        writeln!(os, "        SimdOp::{},", row.name).unwrap();
    }
    writeln!(
        os,
        "    ];

    /// The form whose discriminant (`op as u16`) is `value`.
    #[inline]
    pub const fn from_u16(value: u16) -> Option<Self> {{
        if (value as usize) < Self::ALL.len() {{
            Some(Self::ALL[value as usize])
        }} else {{
            None
        }}
    }}
"
    )
    .unwrap();

    let mut method = |doc: &str, sig: &str, value: &dyn Fn(&SimdRow) -> String| {
        writeln!(os, "    /// {}", doc).unwrap();
        writeln!(os, "    pub const fn {} {{", sig).unwrap();
        writeln!(os, "        match self {{").unwrap();
        for row in &rows {
            writeln!(os, "            Self::{} => {},", row.name, value(row)).unwrap();
        }
        writeln!(os, "        }}\n    }}\n").unwrap();
    };
    method(
        "Variant name, such as `\"MOVDQA_Vo_Wo\"`.",
        "name(self) -> &'static str",
        &|r| format!("{:?}", r.name),
    );
    method(
        "Mnemonic, such as `\"MOVDQA\"`.",
        "mnemonic(self) -> &'static str",
        &|r| format!("{:?}", r.mnemonic),
    );
    method(
        "Exception class: which of CR0.EM, CR0.TS, CR4.OSFXSR and pending x87 exceptions apply.",
        "class(self) -> SimdClass",
        &|r| format!("SimdClass::{}", r.class),
    );
    method(
        "CPUID feature that defines this form.",
        "isa(self) -> SimdIsa",
        &|r| format!("SimdIsa::{}", r.isa),
    );
    method(
        "Register class of the ModR/M.reg operand (`None` for an opcode extension).",
        "reg_kind(self) -> SimdRegKind",
        &|r| format!("SimdRegKind::{}", r.reg),
    );
    method(
        "Register class of the ModR/M.r/m operand in its register form.",
        "rm_kind(self) -> SimdRegKind",
        &|r| format!("SimdRegKind::{}", r.rm),
    );
    method(
        "Memory access width in bytes for the memory form, or 0 when there is none.",
        "mem_size(self) -> u16",
        &|r| r.mem_size.to_string(),
    );
    method(
        "Whether the memory form must be 16-byte aligned (#GP(0) otherwise).",
        "needs_align16(self) -> bool",
        &|r| (r.mem_size >= 16 && !r.unaligned).to_string(),
    );
    method(
        "Whether an imm8 follows the ModR/M operands.",
        "has_imm8(self) -> bool",
        &|r| r.has_imm8.to_string(),
    );
    method(
        "Whether the r/m operand is the destination (a store for the memory form).",
        "rm_is_dest(self) -> bool",
        &|r| r.rm_is_dest.to_string(),
    );
    method(
        "Whether the form stores through the implicit `seg:[(E)DI]` (`IrOp::SIMD_S`).",
        "implicit_di(self) -> bool",
        &|r| r.implicit_di.to_string(),
    );
    writeln!(os, "}}\n").unwrap();

    writeln!(
        os,
        "/// ModR/M and imm8 presence of `0F opcode` in the SIMD ranges, whatever the prefix.
pub(crate) const fn simd_shape(opcode: u8) -> SimdShape {{
    match opcode {{"
    )
    .unwrap();
    for (opcode, (has_modrm, has_imm8, _)) in &shapes {
        writeln!(
            os,
            "        0x{:02x} => SimdShape {{ has_modrm: {}, has_imm8: {} }},",
            opcode, has_modrm, has_imm8
        )
        .unwrap();
    }
    writeln!(
        os,
        "        _ => SimdShape {{ has_modrm: true, has_imm8: false }},
    }}
}}

/// Selects the form of `0F opcode` for the mandatory prefix, the ModR/M.reg
/// field and the ModR/M form, or `None` for an undefined encoding.
pub(crate) const fn simd_lookup(
    opcode: u8,
    prefix: SimdPrefix,
    ext: Index3,
    is_mem: bool,
) -> Option<SimdOp> {{
    match (opcode, prefix, ext, is_mem) {{"
    )
    .unwrap();
    for row in &rows {
        let ext = match row.ext {
            Some(e) => format!("Index3::I{:03b}", e),
            None => "_".to_string(),
        };
        let form = match (row.has_modrm, row.has_reg_form, row.has_mem_form) {
            (false, _, _) => "_",
            (true, true, true) => "_",
            (true, true, false) => "false",
            (true, false, true) => "true",
            _ => unreachable!(),
        };
        let prefix = match row.prefix {
            "_" => "_".to_string(),
            p => format!("SimdPrefix::{}", p),
        };
        writeln!(
            os,
            "        (0x{:02x}, {}, {}, {}) => Some(SimdOp::{}),",
            row.opcode, prefix, ext, form, row.name
        )
        .unwrap();
    }
    writeln!(os, "        _ => None,\n    }}\n}}").unwrap();
}
