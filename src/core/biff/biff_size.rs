/// Size of BIFF records
///
/// Teoretically, chain of `from_size` and `inner` methods
///   can be inlined, and, in fact, `[u8]` buffer will be computed in constant.
pub(crate) enum BiffSize {
    U8([u8; 1]),
    U16([u8; 2]),
    U24([u8; 3]),
    U32([u8; 4]),
}

impl BiffSize {
    #[inline]
    pub(crate) const fn from_size(sz: u32) -> Self {
        match sz {
            n if n < 0x80 => Self::U8([n as u8]),
            n if (n >> 7) < 0x80 => Self::U16([n as u8 | 0x80, (n >> 7) as u8]),
            n if (n >> 14) < 0x80 => {
                Self::U24([n as u8 | 0x80, (n >> 7) as u8 | 0x80, (n >> 14) as u8])
            }
            n if (n >> 21) < 0x80 => Self::U32([
                n as u8 | 0x80,
                (n >> 7) as u8 | 0x80,
                (n >> 14) as u8 | 0x80,
                (n >> 21) as u8,
            ]),
            _ => panic!("Too large BIFF record size!"),
        }
    }

    #[inline]
    pub(crate) const fn inner<'a>(&'a self) -> &'a [u8] {
        match self {
            Self::U8(sz) => sz,
            Self::U16(sz) => sz,
            Self::U24(sz) => sz,
            Self::U32(sz) => sz,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biff_size() {
        assert_eq!(BiffSize::from_size(0x7F).inner(), &[0x7F]);
        assert_eq!(BiffSize::from_size(0x80).inner(), &[0x80, 0x01]);
    }
}
