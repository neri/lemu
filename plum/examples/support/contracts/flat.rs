//! Small Flat code-update and LEAVE contracts shared by native and Wasm.
#![allow(dead_code)]
use plum::gpr::PartialRegister;
use plum::prelude::*;

pub fn code_update() -> u32 {
    let mut memory = vec![0; 256];
    memory[..9].copy_from_slice(&[0xb8, 1, 0, 0, 0, 0xcd, 0x40, 0xeb, 0xf7]);
    let mut cpu =
        plum::ume::flat(memory, Offset32(0x1000), Offset32(0x1000), Offset32(0x1100)).unwrap();
    assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
    assert_eq!(cpu.state().eax().read(), 1);
    cpu.write_bytes(0x1001, &2u32.to_le_bytes()).unwrap();
    cpu.resume_next();
    assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
    assert_eq!(cpu.state().eax().read(), 2);
    1
}

pub fn leave_fault_boundary() -> u32 {
    let mut memory = vec![0; 256];
    memory[..3].copy_from_slice(&[0xc9, 0xcd, 0x40]);
    let mut cpu =
        plum::ume::flat(memory, Offset32(0x1000), Offset32(0x1000), Offset32(0x1100)).unwrap();
    cpu.state().esp().write(0x1080);
    cpu.state().ebp().write(0x10fe);
    cpu.write_memory32(0x10fc, 0x12345678).unwrap();
    assert!(matches!(cpu.execute(), Err(Exception::StackFault)));
    assert_eq!(
        (
            cpu.state().esp().read(),
            cpu.state().ebp().read(),
            cpu.state().eip().read()
        ),
        (0x1080, 0x10fe, 0x1000)
    );
    cpu.state().ebp().write(0x10fc);
    assert!(matches!(cpu.execute(), Err(Exception::Swi(0x40))));
    assert_eq!(
        (cpu.state().esp().read(), cpu.state().ebp().read()),
        (0x1100, 0x12345678)
    );
    1
}
