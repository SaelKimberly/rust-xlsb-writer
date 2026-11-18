use deku::{DekuRead, DekuWrite};

/// `2.5.123`
///
/// The 32-bit field holds `fx100` in bit 0, `fInt` in bit 1, and the 30-bit
/// `num` payload above them.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default, Clone, Copy)]
pub struct RkNumber {
    raw: u32,
}

impl RkNumber {
    const FX100: u32 = 0b01;
    const FINT: u32 = 0b10;

    /// Builds an integer Rk value. `value` must fit the signed 30-bit `num` field.
    #[must_use]
    pub const fn int(value: i32) -> Option<Self> {
        if value < i32::MIN / 4 || value > i32::MAX / 4 {
            None
        } else {
            Some(Self {
                raw: value.cast_unsigned() << 2 | Self::FINT,
            })
        }
    }

    /// Builds an integer Rk value holding `value / 100`.
    #[must_use]
    pub const fn percent(value: i32) -> Option<Self> {
        match Self::int(value) {
            Some(rk) => Some(Self {
                raw: rk.raw | Self::FX100,
            }),
            None => None,
        }
    }

    /// Builds a floating-point Rk value. The value must have its low 34 bits zero.
    #[must_use]
    pub const fn float(value: f64) -> Option<Self> {
        let bits = value.to_bits();
        if bits & 0x3_ffff_ffff != 0 {
            None
        } else {
            Some(Self {
                raw: (bits >> 32) as u32,
            })
        }
    }

    /// Decodes the stored value, applying `fx100` when it is set.
    #[must_use]
    pub fn value(&self) -> f64 {
        let magnitude = if self.raw & Self::FINT != 0 {
            let int = i32::from_le_bytes(self.raw.to_le_bytes()) >> 2;
            f64::from(int)
        } else {
            f64::from_bits(u64::from(self.raw & !0b11) << 32)
        };

        if self.raw & Self::FX100 != 0 {
            magnitude / 100.0
        } else {
            magnitude
        }
    }
}
