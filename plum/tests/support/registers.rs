//! Register snapshots only; no engine adapter or reference implementation.
use plum::cpu::{ProcessorState, RtReg};
use plum::gpr::PartialRegister;

const GPRS: [RtReg; 8] = [
    RtReg::EAX,
    RtReg::ECX,
    RtReg::EDX,
    RtReg::EBX,
    RtReg::ESP,
    RtReg::EBP,
    RtReg::ESI,
    RtReg::EDI,
];

pub fn read(state: &ProcessorState) -> [u32; 8] {
    GPRS.map(|reg| state.reg(reg).e().read())
}

pub fn write(state: &ProcessorState, values: [u32; 8]) {
    for (reg, value) in GPRS.into_iter().zip(values) {
        state.reg(reg).e().write(value);
    }
}
