use crate::{
    KnownID,
    err::{Error, Result},
};
use std::{
    hint::cold_path,
    io::{BufRead, Write},
    ops::Deref,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BiffHead {
    id: u16,
    size: u32,
}

impl BiffHead {
    const MAX_ID: u16 = 0x7F_FF;
    const MAX_SIZE: u32 = 0x0F_FF_FF_FF;

    #[must_use]
    pub const fn id(&self) -> u16 {
        self.id
    }

    #[must_use]
    pub fn known_id(&self) -> Option<KnownID> {
        KnownID::try_from(self.id).ok()
    }

    #[must_use]
    pub const fn size(&self) -> u32 {
        self.size
    }

    pub const fn new(id: u16, size: u32) -> Result<Self> {
        if id & 0x80_00 != 0 {
            Err(Error::TooLargeRecordId(id.swap_bytes(), Self::MAX_ID))
        } else if size > Self::MAX_SIZE {
            Err(Error::TooLargeRecordBody(
                size as usize,
                Self::MAX_SIZE as usize,
            ))
        } else {
            Ok(Self { id, size })
        }
    }

    pub fn from_data<T: Deref<Target = [u8]>>(data: T) -> Result<(usize, Self)> {
        Self::const_from_data(&data)
    }

    pub fn consume_data<T: Deref<Target = [u8]>>(data: T) -> Result<Self> {
        let buf = &*data;
        let (consumed, record) = Self::const_from_data(buf)?;
        if consumed < buf.len() {
            Err(Error::ConsumeFailed(buf.len() - consumed))
        } else {
            Ok(record)
        }
    }

    #[expect(
        clippy::ptr_offset_by_literal,
        reason = "each header byte is reached by stepping one byte past the previous varint field"
    )]
    pub const fn const_from_data(data: &[u8]) -> Result<(usize, Self)> {
        let len = data.len();

        if len < 2 {
            Err(Error::Incomplete(len, 2))
        } else {
            unsafe {
                let (id, mut p) = match [data[0], data[1]] {
                    [id @ 0x00..0x80, sz @ 0x00..0x80] => {
                        return Ok((
                            2,
                            Self {
                                id: id as u16,
                                size: sz as u32,
                            },
                        ));
                    }
                    [id @ 0x00..0x80, _] => {
                        if len == 2 {
                            return Err(Error::Incomplete(2, 3));
                        }
                        (id as u16, data.as_ptr().offset(1))
                    }
                    id @ [0x80..=u8::MAX, 0x00..0x80] => {
                        if len == 2 {
                            return Err(Error::Incomplete(2, 3));
                        }
                        (u16::from_le_bytes(id), data.as_ptr().offset(2))
                    }
                    id => {
                        cold_path();
                        return Err(Error::TooLargeRecordId(
                            u16::from_le_bytes(id),
                            Self::MAX_ID,
                        ));
                    }
                };

                let size = {
                    let mut sz = *p as u32;

                    if *p > 0x7f {
                        if data.len() == 3 {
                            return Err(Error::Incomplete(3, 4));
                        }
                        p = p.offset(1);
                        sz ^= ((!*p) as u32) << 7;

                        if *p > 0x7f {
                            if data.len() == 4 {
                                return Err(Error::Incomplete(4, 5));
                            }
                            p = p.offset(1);
                            sz ^= ((!*p) as u32) << 14;

                            if *p > 0x7f {
                                if data.len() == 5 {
                                    return Err(Error::Incomplete(5, 6));
                                }
                                p = p.offset(1);
                                sz ^= ((!*p) as u32) << 21;

                                if *p > 0x7f {
                                    return Err(Error::TooLargeRecordBody(
                                        sz as usize,
                                        Self::MAX_SIZE as usize,
                                    ));
                                }
                            }
                        }
                    }

                    sz
                };

                Ok((p.offset_from_unsigned(data.as_ptr()) + 1, Self { id, size }))
            }
        }
    }

    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "every casted byte is masked to 7 bits or is an already-in-range low byte"
    )]
    pub const fn as_raw_data(&self) -> ([u8; 6], usize) {
        let mut out = [0u8; 6];

        let off: usize = if let 0x00..0x80 = self.id {
            out[0] = (self.id & 0x7f) as u8;
            1
        } else {
            let buf = self.id.to_le_bytes();
            out[0] = buf[0];
            out[1] = buf[1];
            2
        };
        let end: usize = off
            + match self.size {
                0..0x80 => {
                    out[off] = {
                        // one byte
                        self.size as u8
                    };
                    1
                }
                0x80..0x4000 => {
                    out[off] = (self.size & 0x0000_007f | 0x0000_0080) as u8;
                    out[off + 1] = ((self.size >> 0x07) & 0x0000_007f) as u8;
                    2
                }
                0x4000..0x0020_0000 => {
                    cold_path();
                    out[off] = (self.size & 0x0000_007f | 0x0000_0080) as u8;
                    out[off + 1] = ((self.size >> 0x07) & 0x7f | 0x80) as u8;
                    out[off + 2] = ((self.size >> 0x0e) & 0x0000_007f) as u8;
                    3
                }
                _ => {
                    cold_path();
                    out[off] = (self.size & 0x0000_007f | 0x0000_0080) as u8;
                    out[off + 1] = ((self.size >> 0x07) & 0x7f | 0x80) as u8;
                    out[off + 2] = ((self.size >> 0x0e) & 0x7f | 0x80) as u8;
                    out[off + 3] = ((self.size >> 0x15) & 0x0000_007f) as u8;
                    4
                }
            };

        (out, end)
    }

    pub fn push_to(&self, out: &mut Vec<u8>) -> Result<usize> {
        let (raw, len) = self.as_raw_data();
        out.extend_from_slice(&raw[..len]);
        Ok(len)
    }

    pub fn write_to(&self, writer: &mut dyn Write) -> Result<usize> {
        let (raw, len) = self.as_raw_data();
        writer.write(&raw[..len]).map_err(Error::BiffWriteFailed)
    }

    pub fn read_from(reader: &mut dyn BufRead) -> crate::Result<Self> {
        let buf = reader.fill_buf().map_err(Error::InputReaderError)?;
        let (consumed, record) = Self::const_from_data(buf)?;
        reader.consume(consumed);
        Ok(record)
    }

    #[allow(clippy::uninit_vec, clippy::read_zero_byte_vec)]
    pub fn read_from_with_data(
        reader: &mut dyn BufRead,
    ) -> crate::Result<(Self, Option<Box<[u8]>>)> {
        let head = Self::read_from(reader)?;
        if head.size > 0 {
            let mut buf = Vec::with_capacity(head.size as usize);
            unsafe { buf.set_len(head.size as usize) };
            reader
                .read_exact(&mut buf)
                .map_err(Error::InputReaderError)?;
            Ok((head, Some(buf.into_boxed_slice())))
        } else {
            Ok((head, None))
        }
    }
}
