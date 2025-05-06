use crate::{
    KnownID,
    err::{Error, Result},
};
use std::{
    intrinsics::{cold_path, unlikely},
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

    pub const fn id(&self) -> u16 {
        self.id
    }

    pub fn known_id(&self) -> Option<KnownID> {
        KnownID::try_from(self.id).ok()
    }

    pub const fn size(&self) -> u32 {
        self.size
    }

    pub const fn new(id: u16, size: u32) -> Result<Self> {
        if unlikely(id & 0x80_00 != 0) {
            Err(Error::TooLargeRecordId(id.swap_bytes(), Self::MAX_ID))
        } else if unlikely(size > Self::MAX_SIZE) {
            Err(Error::TooLargeRecordBody(
                size as usize,
                Self::MAX_SIZE as usize,
            ))
        } else {
            Ok(Self { id, size })
        }
    }

    pub fn from_data<T: Deref<Target = [u8]>>(data: T) -> Result<(usize, BiffHead)> {
        BiffHead::const_from_data(data.deref())
    }

    pub fn consume_data<T: Deref<Target = [u8]>>(data: T) -> Result<BiffHead> {
        let buf = data.deref();
        let (consumed, record) = BiffHead::const_from_data(buf)?;
        if consumed < buf.len() {
            Err(Error::ConsumeFailed(buf.len() - consumed))
        } else {
            Ok(record)
        }
    }

    pub const fn const_from_data(data: &[u8]) -> Result<(usize, BiffHead)> {
        let len = data.len();

        if unlikely(len < 2) {
            Err(Error::Incomplete(len, 2))
        } else {
            unsafe {
                let (id, mut p) = match [data[0], data[1]] {
                    [id @ 0x00..0x80, sz @ 0x00..0x80] => {
                        return Ok((
                            2,
                            BiffHead {
                                id: id as u16,
                                size: sz as u32,
                            },
                        ));
                    }
                    [id @ 0x00..0x80, _] => {
                        if unlikely(len == 2) {
                            return Err(Error::Incomplete(2, 3));
                        }
                        (id as u16, data.as_ptr().offset(1))
                    }
                    id @ [0x80..=u8::MAX, 0x00..0x80] => {
                        if unlikely(len == 2) {
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

                            if unlikely(*p > 0x7f) {
                                if data.len() == 5 {
                                    return Err(Error::Incomplete(5, 6));
                                }
                                p = p.offset(1);
                                sz ^= ((!*p) as u32) << 21;

                                if unlikely(*p > 0x7f) {
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

    fn push_raw_size(&self, out: &mut Vec<u8>) -> Result<usize> {
        let pushed = match self.size {
            0..0x80 => {
                out.push(
                    // one byte
                    self.size as u8,
                );
                1
            }
            0x80..0x4000 => {
                out.extend_from_slice(&[
                    // two bytes
                    (self.size & 0x0000_007f | 0x00_80) as u8,
                    ((self.size >> 0x07) & 0x0000_007f) as u8,
                ]);
                2
            }
            0x4000..0x200000 => {
                cold_path();
                out.extend_from_slice(&[
                    // three bytes
                    (self.size & 0x0000_007f | 0x00_80) as u8,
                    ((self.size >> 0x07) & 0x7f | 0x80) as u8,
                    ((self.size >> 0x0e) & 0x0000_007f) as u8,
                ]);
                3
            }
            _ => {
                cold_path();
                out.extend_from_slice(&[
                    // four bytes
                    (self.size & 0x0000_007f | 0x00_80) as u8,
                    ((self.size >> 0x07) & 0x7f | 0x80) as u8,
                    ((self.size >> 0x0e) & 0x7f | 0x80) as u8,
                    ((self.size >> 0x15) & 0x0000_007f) as u8,
                ]);
                4
            }
        };

        Ok(pushed)
    }

    /// Biff ID don't need special encoding, because it's value not used in computing.
    ///
    /// # Errors
    ///
    /// This function will return an error if `data` is empty, or have not enough space.
    fn push_raw_id(&self, data: &mut Vec<u8>) -> Result<usize> {
        let pushed = match self.id {
            0x00..0x80 => {
                data.push((self.id & 0x7f) as u8);
                1
            }
            _ => {
                let buf = self.id.to_le_bytes();
                data.extend_from_slice(&buf);
                2
            }
        };
        Ok(pushed)
    }

    pub fn write_to(&self, writer: &mut dyn Write) -> Result<usize> {
        let mut buf = Vec::with_capacity(6);

        let pushed = self.push_raw_id(&mut buf)? + self.push_raw_size(&mut buf)?;

        writer.write(&buf[..pushed]).map_err(Error::BiffWriteFailed)
    }

    pub fn read_from(reader: &mut dyn BufRead) -> crate::Result<Self> {
        let buf = reader.fill_buf().map_err(Error::InputReaderError)?;
        let (consumed, record) = Self::const_from_data(buf)?;
        reader.consume(consumed);
        Ok(record)
    }
}
