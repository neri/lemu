//! Native and Node/Wasm kernel versus the former checked-element approach.
use plum::gpr::PartialRegister;
use plum::harvard::HarvardUME;
use plum::prelude::*;

fn read(cpu: &HarvardUME, cs: bool, addr: u32, width: u32) -> u32 {
    if cs {
        let bytes = &cpu.code()[addr as usize..addr as usize + width as usize];
        let mut v = [0; 4];
        v[..width as usize].copy_from_slice(bytes);
        u32::from_le_bytes(v)
    } else {
        match width {
            1 => cpu.read_memory8(addr).unwrap() as u32,
            2 => cpu.read_memory16(addr).unwrap() as u32,
            _ => cpu.read_memory32(addr).unwrap(),
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn bench_string(
    kind: u32,
    width: u32,
    count: u32,
    reverse: u32,
    scenario: u32,
    reference: u32,
    iterations: u32,
) -> u32 {
    let size = count as usize * width as usize;
    let mut code = Vec::new();
    if scenario == 1 {
        code.push(0x2e);
    }
    code.push(if kind == 4 { 0xf2 } else { 0xf3 });
    if width == 2 {
        code.push(0x66);
    }
    code.push([0xa4, 0xaa, 0xac, 0xa6, 0xae][kind as usize] + u8::from(width != 1));
    code.extend([0xcd, 0x40, 0xeb]);
    code.push((-(code.len() as i32 + 1)) as u8);
    code.resize(size * 3 + 32, 0x5a);
    let mut cpu = plum::ume::harvard(
        code.into_boxed_slice(),
        vec![0x5a; size * 3 + 32].into_boxed_slice(),
        Offset32(0),
        Offset32(0),
    );
    let base_src = 16;
    let base_dest = match scenario {
        2 => 17,
        3 => 15,
        _ => size as u32 + 16,
    };
    let offset = if reverse != 0 { (count - 1) * width } else { 0 };
    let src = base_src + offset;
    let dest = base_dest + offset;
    if scenario == 4 && kind >= 3 {
        cpu.write_memory32(dest, 0).unwrap();
    }
    let accumulator = if kind == 4 { 0 } else { 0x5a5a_5a5a };
    let delta = if reverse != 0 {
        width.wrapping_neg()
    } else {
        width
    };
    let mut checksum = 0u32;
    for _ in 0..iterations {
        if reference == 0 {
            cpu.state().esi().write(src);
            cpu.state().edi().write(dest);
            cpu.state().ecx().write(count);
            cpu.state().eax().write(accumulator);
            cpu.state_mut()
                .flags_mut()
                .set_static(Flags::DF, reverse != 0);
            assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
            checksum = checksum
                .wrapping_add(cpu.state().edi().read())
                .wrapping_add(cpu.state().eax().read());
            cpu.resume_next();
        } else {
            let mut si = src;
            let mut di = dest;
            let mut eax = accumulator;
            let mut last = None;
            for _ in 0..count {
                let a = if kind == 0 || kind == 2 || kind == 3 {
                    read(&cpu, scenario == 1, si, width)
                } else {
                    accumulator & (u32::MAX >> ((4 - width) * 8))
                };
                match kind {
                    0 | 1 => match width {
                        1 => cpu.write_memory8(di, a as u8).unwrap(),
                        2 => cpu.write_memory16(di, a as u16).unwrap(),
                        _ => cpu.write_memory32(di, a).unwrap(),
                    },
                    2 => {
                        let mask = u32::MAX >> ((4 - width) * 8);
                        eax = (eax & !mask) | a;
                    }
                    _ => {
                        last = Some((a, read(&cpu, false, di, width)));
                    }
                }
                if kind == 0 || kind == 2 || kind == 3 {
                    si = si.wrapping_add(delta);
                }
                if kind != 2 {
                    di = di.wrapping_add(delta);
                }
                if let Some((a, b)) = last {
                    if (a == b) == (kind == 4) {
                        break;
                    }
                }
            }
            if let Some((a, b)) = last {
                match width {
                    1 => {
                        cpu.alu().sub8(a as u8, b as u8);
                    }
                    2 => {
                        cpu.alu().sub16(a as u16, b as u16);
                    }
                    _ => {
                        cpu.alu().sub32(a, b);
                    }
                }
            }
            checksum = checksum.wrapping_add(di).wrapping_add(eax);
        }
    }
    // Make writes observable to the caller and optimizer.
    checksum.wrapping_add(cpu.data()[base_dest as usize] as u32)
}
