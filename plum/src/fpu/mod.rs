//! Optional x87 state: exact value transport and an explicitly approximate f64 backend.
pub(crate) mod arithmetic;
pub(crate) mod bcd;
pub(crate) mod codec;
mod environment;
pub(crate) mod ops;
pub(crate) mod register_ops;

#[derive(Debug)]
pub(crate) struct UnsupportedFpu;
pub(crate) mod exponent;
pub(crate) mod logarithm;
pub(crate) mod remainder;
pub(crate) mod trig;
mod value;
pub(crate) use value::{
    DENORMAL, INDEFINITE, INVALID, OVERFLOW, PRECISION, UNDERFLOW, ZERO_DIVIDE,
};
pub use value::{FpuValue, Raw80};
pub(crate) mod uop;
pub use uop::FpuMinorUop;

/// Architectural x87 state. Internal representation tags are separate from x87 tags.
#[derive(Debug, Clone)]
pub struct FpuState {
    pub(crate) registers: [FpuValue; 8],
    pub(crate) tags: u16,
    pub(crate) instruction_pointer: u32,
    pub(crate) instruction_selector: u16,
    pub(crate) data_pointer: u32,
    pub(crate) data_selector: u16,
    pub(crate) last_opcode: u16,
    // HarvardUME code is immutable, so it decodes FOP only when the environment
    // is saved. FlatUME captures `last_opcode` eagerly and never sets this.
    pub(crate) opcode_pending: bool,
    pub(crate) top: u8,
    pub(crate) control: u16,
    pub(crate) status: u16,
}

impl Default for FpuState {
    fn default() -> Self {
        Self {
            registers: [FpuValue::F64(0.0); 8],
            tags: 0xffff,
            instruction_pointer: 0,
            instruction_selector: 0,
            data_pointer: 0,
            data_selector: 0,
            last_opcode: 0,
            opcode_pending: false,
            top: 0,
            control: 0x037f,
            status: 0,
        }
    }
}
impl FpuState {
    /// Nearest PC=53/64 arithmetic uses f64; PC=24 and directed rounding use
    /// an integer path. Transport and integer conversions retain raw precision.
    pub const fn backend_name(&self) -> &'static str {
        "approx64"
    }

    pub fn control_word(&self) -> u16 {
        self.control
    }
    pub fn status_word(&self) -> u16 {
        (self.status & !0x3800) | (u16::from(self.top) << 11)
    }
    pub fn st(&self, index: u8) -> Option<FpuValue> {
        if index >= 8 {
            return None;
        }
        let physical = self.physical(index);
        (!self.is_empty(physical)).then_some(self.registers[physical])
    }
    pub fn tag_word(&self) -> u16 {
        self.tags
    }
    pub fn instruction_pointer(&self) -> u32 {
        self.instruction_pointer
    }
    pub fn data_pointer(&self) -> u32 {
        self.data_pointer
    }
    fn is_empty(&self, physical: usize) -> bool {
        (self.tags >> (physical * 2)) & 3 == 3
    }
    pub(crate) fn free(&mut self, index: u8) {
        self.tags |= 3 << (self.physical(index) * 2);
    }
    pub(crate) fn physical(&self, index: u8) -> usize {
        ((self.top + index) & 7) as usize
    }
    pub(crate) fn read(&mut self, index: u8) -> FpuValue {
        if let Some(value) = self.st(index) {
            if !matches!(value, FpuValue::Raw80(raw) if !raw.is_supported()) {
                return value;
            }
            self.status |= INVALID;
            return FpuValue::Raw80(INDEFINITE);
        }
        self.status = (self.status & !0x200) | INVALID | 0x40;
        FpuValue::Raw80(INDEFINITE)
    }
    pub(crate) fn set(&mut self, index: u8, value: FpuValue) {
        let physical = self.physical(index);
        self.registers[physical] = value;
        let tag = value.tag();
        self.tags = (self.tags & !(3 << (physical * 2))) | (tag << (physical * 2));
    }
    pub(crate) fn push(&mut self, mut value: FpuValue) {
        self.top = (self.top.wrapping_sub(1)) & 7;
        if !self.is_empty(self.top as usize) {
            self.status |= INVALID | 0x40 | 0x200;
            value = FpuValue::Raw80(INDEFINITE);
        } else {
            self.status &= !0x200;
        }
        self.set(0, value);
    }
    pub(crate) fn pop(&mut self) {
        self.free(0);
        self.top = (self.top + 1) & 7;
    }
    pub(crate) fn rc(&self) -> u16 {
        (self.control >> 10) & 3
    }
    pub(crate) fn supports_arithmetic(&self) -> bool {
        matches!((self.control >> 8) & 3, 0 | 2 | 3)
    }
}
