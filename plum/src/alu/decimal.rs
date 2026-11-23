//! Shared legacy decimal adjustments. Undefined flag bits retain the engine policy.
use super::*;
use crate::gpr::PartialRegister;

impl Alu<'_> {
    #[inline]
    fn decimal_szp8(&mut self, value: u8) {
        let flags = self.0.flags_mut();
        flags.set_dynamic(Flags::SF, value & 0x80 != 0);
        flags.set_dynamic(Flags::ZF, value == 0);
        flags.set_dynamic(Flags::PF, value.count_ones() & 1 == 0);
    }

    /// AAA/AAS, using the modern AX adjustment already used by both user engines.
    #[inline]
    pub(crate) fn ascii_adjust(&mut self, subtract: bool) {
        let ax = self.0.ax().read();
        let flags = self.0.flags_mut().resolve();
        let adjust = ax & 15 > 9 || flags.contains(Flags::AF);
        let result = if adjust {
            if subtract {
                ax.wrapping_sub(0x106)
            } else {
                ax.wrapping_add(0x106)
            }
        } else {
            ax
        };
        self.0.ax().write(result & 0xff0f);
        self.0.flags_mut().set_dynamic(Flags::AF, adjust);
        self.0.flags_mut().set_dynamic(Flags::CF, adjust);
    }

    #[inline]
    pub(crate) fn aad(&mut self, base: u8) {
        let result = self
            .0
            .al()
            .read()
            .wrapping_add(self.0.ah().read().wrapping_mul(base));
        self.0.flags_mut().resolve();
        self.0.ax().write(u16::from(result));
        self.decimal_szp8(result);
    }

    /// None denotes #DE. No registers or flags are changed for a zero radix.
    #[inline]
    pub(crate) fn aam(&mut self, base: u8) -> Option<()> {
        if base == 0 {
            return None;
        }
        let al = self.0.al().read();
        self.0.flags_mut().resolve();
        self.0.ah().write(al / base);
        self.0.al().write(al % base);
        self.decimal_szp8(al % base);
        Some(())
    }

    /// DAA/DAS. Capture the original carry before applying either adjustment.
    #[inline]
    pub(crate) fn decimal_adjust(&mut self, subtract: bool) {
        let old = self.0.al().read();
        let flags = self.0.flags_mut().resolve();
        let old_cf = flags.contains(Flags::CF);
        let adjust = old & 15 > 9 || flags.contains(Flags::AF);
        let mut value = old;
        let mut cf = false;
        if adjust {
            if subtract {
                cf = old_cf || old < 6;
                value = value.wrapping_sub(6);
            } else {
                value = value.wrapping_add(6);
            }
        }
        if old > 0x99 || old_cf {
            value = if subtract {
                value.wrapping_sub(0x60)
            } else {
                value.wrapping_add(0x60)
            };
            cf = true;
        }
        self.0.al().write(value);
        self.decimal_szp8(value);
        self.0.flags_mut().set_dynamic(Flags::AF, adjust);
        self.0.flags_mut().set_dynamic(Flags::CF, cf);
    }
}
