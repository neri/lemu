//! Small native/Wasm contracts shared with the engine tests and benchmark probes.
#![allow(dead_code)]
use plum::gpr::PartialRegister;
use plum::harvard::HarvardUME;
use plum::prelude::*;

fn machine(code: &[u8]) -> HarvardUME {
    let code = [code, &[0xcd, 0x40]].concat();
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

pub fn rmw() -> u32 {
    // XADD [EAX],EAX: address must survive the register write-back.
    let mut cpu = machine(&[0x0f, 0xc1, 0]);
    cpu.state().eax().write(64);
    cpu.write_memory32(64, 7).unwrap();
    done(&mut cpu);
    assert_eq!(cpu.state().eax().read(), 7);
    assert_eq!(cpu.read_memory32(64).unwrap(), 71);
    assert_eq!(cpu.read_memory32(68).unwrap(), 0);
    1
}

pub fn load_extend() -> u32 {
    let mut cpu = machine(&[0x0f, 0xbe, 0x0d, 255, 0, 0, 0]);
    cpu.data()[255] = 0x80;
    done(&mut cpu);
    assert_eq!(cpu.state().ecx().read(), 0xffffff80);

    // The destination is also the base register in [EAX+ECX*2+1].
    let mut cpu = machine(&[0x0f, 0xbf, 0x44, 0x48, 1]);
    cpu.state().eax().write(250);
    cpu.state().ecx().write(1);
    cpu.data()[253..255].copy_from_slice(&0x8001u16.to_le_bytes());
    done(&mut cpu);
    assert_eq!(cpu.state().eax().read(), 0xffff8001);
    1
}

pub fn narrow_load() -> u32 {
    let mut cpu = machine(&[0x8a, 0x25, 255, 0, 0, 0]); // MOV AH,[255]
    cpu.state().eax().write(0x12345678);
    cpu.data()[255] = 0xab;
    done(&mut cpu);
    assert_eq!(cpu.state().eax().read(), 0x1234ab78);
    1
}

pub fn leave_fault_boundary() -> u32 {
    let mut cpu = machine(&[0xc9]);
    cpu.state().esp().write(128);
    cpu.state().ebp().write(254);
    cpu.write_memory32(252, 0x12345678).unwrap();
    assert!(matches!(cpu.execute(), Err(Exception::StackFault)));
    assert_eq!(
        (
            cpu.state().esp().read(),
            cpu.state().ebp().read(),
            cpu.state().eip().read()
        ),
        (128, 254, 0)
    );
    cpu.state().ebp().write(252);
    done(&mut cpu);
    assert_eq!(
        (cpu.state().esp().read(), cpu.state().ebp().read()),
        (256, 0x12345678)
    );
    1
}
