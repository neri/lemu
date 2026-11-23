//! Protected-mode x87 environment layout with segment selectors supplied by each engine.
//! Operand size selects 14/28-byte headers, independently of address size.
use super::{FpuState, FpuValue, Raw80};

fn word(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}
fn dword(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
fn put_word(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}
fn put_dword(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
impl FpuState {
    pub(crate) fn environment(&self, wide: bool, full: bool, opcode: u16) -> ([u8; 108], usize) {
        let mut bytes = [0u8; 108];
        let stride = if wide { 4 } else { 2 };
        let header = stride * 7;
        // Reserved upper halves follow the common x87 all-ones image.
        if wide {
            bytes[..header].fill(0xff);
        }
        put_word(&mut bytes, 0, self.control_word());
        put_word(&mut bytes, stride, self.status_word());
        put_word(&mut bytes, stride * 2, self.tag_word());
        if wide {
            put_dword(&mut bytes, 12, self.instruction_pointer);
            put_word(&mut bytes, 16, self.instruction_selector);
            put_word(&mut bytes, 18, opcode & 0x7ff);
            put_dword(&mut bytes, 20, self.data_pointer);
            put_word(&mut bytes, 24, self.data_selector);
        } else {
            put_word(&mut bytes, 6, self.instruction_pointer as u16);
            put_word(&mut bytes, 8, self.instruction_selector);
            put_word(&mut bytes, 10, self.data_pointer as u16);
            put_word(&mut bytes, 12, self.data_selector);
        }
        if full {
            // Tag bits refer to physical registers, but saved values start at ST0.
            for i in 0..8 {
                let raw = self.registers[self.physical(i)].raw();
                let offset = header + i as usize * 10;
                bytes[offset..offset + 8].copy_from_slice(&raw.significand.to_le_bytes());
                put_word(&mut bytes, offset + 8, raw.sign_exponent);
            }
        }
        (bytes, header + if full { 80 } else { 0 })
    }

    /// Parse into a temporary state so a rejected control word never partly restores.
    pub(crate) fn restore_environment(&mut self, bytes: &[u8], wide: bool, full: bool) -> bool {
        let stride = if wide { 4 } else { 2 };
        let control = word(bytes, 0);
        if control & 0x3f != 0x3f {
            return false;
        }
        self.control = control;
        let status = word(bytes, stride);
        self.top = ((status >> 11) & 7) as u8;
        self.status = status & !0x3800;
        self.tags = word(bytes, stride * 2);
        if wide {
            self.instruction_pointer = dword(bytes, 12);
            self.instruction_selector = word(bytes, 16);
            self.last_opcode = word(bytes, 18) & 0x7ff;
            self.opcode_pending = false;
            self.data_pointer = dword(bytes, 20);
            self.data_selector = word(bytes, 24);
        } else {
            self.instruction_pointer = u32::from(word(bytes, 6));
            self.instruction_selector = word(bytes, 8);
            // The protected-mode 14-byte image has no FOP field.
            self.data_pointer = u32::from(word(bytes, 10));
            self.data_selector = word(bytes, 12);
        }
        if full {
            for i in 0..8 {
                let offset = stride * 7 + i as usize * 10;
                let raw = Raw80 {
                    significand: u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap()),
                    sign_exponent: word(bytes, offset + 8),
                };
                self.registers[self.physical(i)] = FpuValue::Raw80(raw);
            }
        }
        true
    }
}
