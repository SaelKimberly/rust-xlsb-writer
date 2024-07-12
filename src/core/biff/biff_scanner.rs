use super::{BiffId, BiffRecord};
use std::{
    collections::HashSet,
    io::{self, Read},
    rc::Rc,
};

// Single threaded scanner for BIFF files.
pub(super) struct BiffScanner<'a, R: Read + ?Sized> {
    // Underlying reader
    pub reader: &'a mut R,
    // Reads only the specified IDs, all other will be ignored
    pub ids_only: Option<Rc<HashSet<u16>>>,
    // Skips the specified IDs, all other will be read
    pub ids_skip: Option<Rc<HashSet<u16>>>,
    // Break on the specified ID
    pub break_id: Option<u16>,
    // Maximum number of records to read
    pub max_scan: Option<usize>,

    // Scan counter.
    pub scan_cnt: usize,
}

impl<'a, R: Read + ?Sized> BiffScanner<'a, R> {
    /// Read the next BIFF record header from the underlying reader (record ID and record size).
    pub fn read_header(&mut self) -> io::Result<(u16, i64)> {
        let mut id_buf = [0u8; 2];
        let mut sz_buf = [0u8; 1];

        self.reader.read_exact(&mut id_buf)?;

        let id: u16;
        let mut sz: u32;
        if id_buf[0] & 0x80 != 0 {
            id = u16::from_le_bytes(id_buf);

            self.reader.read_exact(&mut sz_buf)?;

            sz = sz_buf[0] as u32;
        } else {
            id = id_buf[0] as u16;
            sz = id_buf[1] as u32;
        }

        if sz == 0 {
            Ok((id, sz as i64))
        } else {
            if sz & 0x80 != 0 {
                self.reader.read_exact(&mut sz_buf)?;

                sz = sz ^ 0x80 | (sz_buf[0] as u32) << 7;

                if sz & 0x4000 != 0 {
                    self.reader.read_exact(&mut sz_buf)?;

                    sz = sz ^ 0x4000 | (sz_buf[0] as u32) << 14;

                    if sz & 0x200000 != 0 {
                        self.reader.read_exact(&mut sz_buf)?;

                        sz = sz ^ 0x200000 | (sz_buf[0] as u32) << 21;
                    }
                }
            }

            Ok((id, sz as i64))
        }
    }
}

// Iterator implementation for BiffScanner
impl<'a, R: Read + ?Sized> Iterator for BiffScanner<'a, R> {
    type Item = Option<io::Result<BiffRecord>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(true) = self.max_scan.map(|x| self.scan_cnt == x) {
            return None;
        }
        match self.read_header() {
            Ok((id, sz)) => {
                if self.break_id.map(|u| u == id).unwrap_or(false) {
                    None
                } else if self
                    .ids_only
                    .as_ref()
                    .map(|set| !set.contains(&id))
                    .unwrap_or(false)
                    || self
                        .ids_skip
                        .as_ref()
                        .map(|set| set.contains(&id))
                        .unwrap_or(false)
                {
                    if sz > 0 {
                        if let Err(e) = io::copy(&mut self.reader.take(sz as u64), &mut io::sink())
                        {
                            Some(Some(Err(e)))
                        } else {
                            Some(None)
                        }
                    } else {
                        Some(None)
                    }
                } else if sz > 0 {
                    let mut data = vec![0u8; sz as usize];

                    match self.reader.read_exact(&mut data) {
                        Ok(()) => {
                            self.scan_cnt += 1;
                            return Some(Some(Ok(BiffRecord::Sized {
                                id: BiffId::from(id),
                                data: data.into_boxed_slice(),
                            })));
                        }
                        Err(e) => return Some(Some(Err(e))),
                    }
                } else {
                    self.scan_cnt += 1;
                    return Some(Some(Ok(BiffRecord::Empty {
                        id: BiffId::from(id),
                    })));
                }
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    None
                } else {
                    Some(Some(Err(e)))
                }
            }
        }
    }
}
