use crate::err::{Error, Result};
use std::{
    intrinsics::{likely, offset, unlikely},
    io::{Read, Write},
    ops::Deref,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BiffHead {
    pub id: u16,
    pub size: u32,
}

impl BiffHead {
    const MAX_ID: u16 = 0x7F_FF;
    const MAX_SIZE: u32 = 0x0F_FF_FF_FF;

    pub const fn new(id: u16, size: u32) -> Result<Self> {
        if unlikely(id.swap_bytes() > Self::MAX_ID) {
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

    pub fn from_data<T: Deref<Target = [u8]>>(data: &T) -> Result<BiffHead> {
        BiffHead::const_from_data(data.deref())
    }

    pub const fn const_from_data(data: &[u8]) -> Result<BiffHead> {
        let mut p: *const u8 = data.as_ptr();
        if unlikely(data.len() < 2) {
            Err(Error::Incomplete(data.len(), 2))
        } else {
            unsafe {
                let id = if *p > 0x7f {
                    let id = *(p as *const u16);
                    p = offset(p, 2_isize);
                    if data.len() == 2 {
                        return Err(Error::Incomplete(2, 3));
                    }
                    id
                } else {
                    let id = *p as u16;
                    p = offset(p, 1_isize);
                    id
                };
                let sz = {
                    let mut sz = *p as u32;

                    if *p > 0x7f {
                        if data.len() == 3 {
                            return Err(Error::Incomplete(3, 4));
                        }
                        p = offset(p, 1_isize);
                        sz ^= ((!*p) as u32) << 7;

                        if *p > 0x7f {
                            if data.len() == 4 {
                                return Err(Error::Incomplete(4, 5));
                            }
                            p = offset(p, 1_isize);
                            sz ^= ((!*p) as u32) << 14;

                            if *p > 0x7f {
                                if data.len() == 5 {
                                    return Err(Error::Incomplete(5, 6));
                                }
                                p = offset(p, 1_isize);
                                sz ^= ((!*p) as u32) << 21;
                            }
                        }
                    }

                    sz
                };
                BiffHead::new(id, sz)
            }
        }
    }

    fn push_raw_size(&self, data: &mut Vec<u8>) -> Result<usize> {
        let expected = match self.size {
            n if likely(n < 0x80) => 1,
            n if n < 0x4000 => 2,
            n if unlikely(n < 0x200000) => 3,
            n if unlikely(n < 0x10000000) => 4,
            too_large => {
                return Err(Error::TooLargeRecordBody(
                    too_large as usize,
                    Self::MAX_SIZE as usize,
                ));
            }
        };

        let mut i = 0_usize;
        loop {
            if i == (expected - 1) {
                data.push((self.size >> (7 * i) & 0x7f) as u8);
                break;
            } else {
                data.push((self.size >> (7 * i) & 0x7f) as u8 | 0x80);
                i += 1;
            }
        }
        Ok(i)
    }

    /// Biff ID don't need special encoding, because it's value not used in computing.
    ///
    /// # Errors
    ///
    /// This function will return an error if `data` is empty, or have not enough space.
    fn push_raw_id(&self, data: &mut Vec<u8>) -> Result<usize> {
        if self.id > 0x7f {
            if unlikely(self.id.swap_bytes() > Self::MAX_ID) {
                Err(Error::TooLargeRecordId(self.id.swap_bytes(), Self::MAX_ID))
            } else {
                let buf = self.id.to_le_bytes();
                data.extend_from_slice(&buf);
                Ok(2)
            }
        } else {
            data.push((self.id & 0x7f) as u8);
            Ok(1)
        }
    }

    pub fn write_to(&self, writer: &mut dyn Write) -> Result<usize> {
        let mut buf = Vec::with_capacity(6);

        let result = self.push_raw_id(&mut buf)? + self.push_raw_size(&mut buf)?;

        Ok(writer.write(&buf[..result])?)
    }

    pub fn read_from(reader: &mut dyn Read) -> Result<Self> {
        let mut id_buf = [0u8; 2];
        let mut sz_buf = [0u8; 1];

        reader.read_exact(&mut id_buf)?;

        let id = if likely(id_buf[0] < 0x80) {
            sz_buf[0] = id_buf[1];
            id_buf[0] as u16
        } else {
            reader.read_exact(&mut sz_buf)?;
            u16::from_le_bytes(id_buf)
        };

        let mut size: u32 = sz_buf[0] as u32;

        if sz_buf[0] > 0x80 {
            reader.read_exact(&mut sz_buf)?;
            size ^= (!sz_buf[0] as u32) << 7;

            if sz_buf[0] > 0x80 {
                reader.read_exact(&mut sz_buf)?;
                size ^= (!sz_buf[0] as u32) << 14;

                if sz_buf[0] > 0x80 {
                    reader.read_exact(&mut sz_buf)?;
                    size ^= (!sz_buf[0] as u32) << 21;
                }
            }
        }

        Ok(BiffHead { id, size })
    }
}
