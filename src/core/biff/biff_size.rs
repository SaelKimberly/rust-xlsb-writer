pub(crate) enum BiffSize {
    U8([u8; 1]),
    U16([u8; 2]),
    U24([u8; 3]),
    U32([u8; 4]),
}


impl BiffSize {
    #[inline]
    pub(super) const fn from_size(sz: u32) -> Self {
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
    pub const fn inner<'a>(&'a self) -> &'a [u8] {
        match self {
            Self::U8(sz) => sz,
            Self::U16(sz) => sz,
            Self::U24(sz) => sz,
            Self::U32(sz) => sz,
        }
    }
}