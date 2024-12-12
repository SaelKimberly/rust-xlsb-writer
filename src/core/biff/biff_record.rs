#![allow(unused_macros, unused_imports)]
use std::{
    alloc::{alloc, Layout},
    borrow::Cow,
    collections::HashSet,
    io::{self, Read, Write},
    mem::{size_of, transmute, MaybeUninit},
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
    slice,
};

use super::{BiffDataCompatible, BiffId, BiffSerializable, BiffSize, CheckBiff, FromBiffData};

/// The error type for the `BiffRecord` struct
#[derive(Debug)]
pub(crate) enum BiffError {
    Empty,
    SizeMismatch,
    InvalidCast,
}

pub(crate) struct DataSlice<'a>(&'a [u8]);
impl<'a> DataSlice<'a> {
    pub(crate) const fn new(data: &'a [u8]) -> Self {
        Self(data)
    }
    pub(crate) const fn peek<T: Sized>(&self, offset: usize) -> Result<T, BiffError> {
        if std::mem::size_of::<T>() <= (self.0.len() - offset) {
            Ok(unsafe { self.0.as_ptr().offset(offset as isize).cast::<T>().read() })
        } else {
            Err(BiffError::SizeMismatch)
        }
    }
    pub(crate) const fn peek_many<T: Sized>(
        &self,
        offset: usize,
        count: usize,
    ) -> Result<&[T], BiffError> {
        if count * std::mem::size_of::<T>() <= (self.0.len() - offset) {
            Ok(unsafe {
                std::slice::from_raw_parts(
                    self.0.as_ptr().offset(offset as isize).cast::<T>(),
                    count,
                )
            })
        } else {
            Err(BiffError::SizeMismatch)
        }
    }
}

#[derive(Clone)]
pub(crate) struct BiffRecord {
    pub(crate) id: BiffId,
    pub(crate) data: Box<[u8]>,
}

/// Try to read the `T` from the given `data` at the given `offset`
///
/// Function is, actually, unsafe, but it cannot panic, because of data size check.
///
/// Function is implemented to avoid copying data on reading.
#[inline]
pub(crate) const fn try_to_sized<T: Sized>(offset: usize, data: &[u8]) -> Result<T, BiffError> {
    if std::mem::size_of::<T>() <= (data.len() - offset) {
        Ok(unsafe { data.as_ptr().offset(offset as isize).cast::<T>().read() })
    } else {
        Err(BiffError::SizeMismatch)
    }
}

#[inline]
pub(crate) fn box_alloc(size: usize) -> Box<[u8]> {
    match size {
        0 => return Box::new([]),
        n => unsafe {
            Box::from_raw(slice_from_raw_parts_mut(
                alloc(Layout::from_size_align_unchecked(n, 1)),
                n,
            ))
        },
    }
}

enum ReadResult {
    Accepted(BiffRecord),
    Rejected,
    Error(io::Error),
}

impl BiffRecord {
    pub(crate) fn read<R: Read + ?Sized>(reader: &mut R) -> io::Result<Option<BiffRecord>> {
        let mut id_buf = [0u8; 2];
        let mut sz = 0u32;
        let mut sz_idx = 0u32;

        // If we cannot read next two bytes, there is EOF
        if reader.read_exact(&mut id_buf).is_err() {
            return Ok(None);
        }

        let id: u16;

        match (id_buf[0], id_buf[1]) {
            (biff_id, 0x00) => {
                id = biff_id as u16;
                sz = 0;
            }
            (id_part, _) if id_part & 0x80 != 0 => {
                id = u16::from_le_bytes(id_buf);
                for b in reader.bytes().take(4) {
                    match b {
                        Ok(b) => {
                            sz |= ((b & 0x7f) as u32) << (sz_idx * 7);
                            sz_idx += 1;

                            if b & 0x80 == 0 {
                                break;
                            }
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
            (biff_id, sz_part) => {
                id = biff_id as u16;
                sz = (sz_part & 0x7f) as u32;
                sz_idx += 1;
                if sz_part & 0x80 != 0 {
                    for b in reader.bytes().take(3) {
                        if let Ok(b) = b {
                            sz |= ((b & 0x7f) as u32) << (sz_idx * 7);
                            sz_idx += 1;

                            if b & 0x80 == 0 {
                                break;
                            }
                        }
                    }
                }
            }
        };
        if sz > 0 {
            let mut data = box_alloc(sz as usize);
            reader.read_exact(data.as_mut())?;
            Ok(Some(BiffRecord {
                id: unsafe { transmute(id) },
                data,
            }))
        } else {
            Ok(Some(BiffRecord {
                id: unsafe { transmute(id) },
                data: box_alloc(0),
            }))
        }
    }

    pub(crate) fn scan<'a, R: Read + ?Sized>(
        reader: &'a mut R,
        ids_only: Option<&[u16]>,
        ids_skip: Option<&[u16]>,
        break_id: Option<u16>,
        max_scan: Option<usize>,
    ) -> impl Iterator<Item = io::Result<BiffRecord>> + 'a {
        let ids_only = ids_only.map(|slice| HashSet::<u16>::from_iter(slice.iter().copied()));
        let ids_skip = ids_skip.map(|slice| HashSet::<u16>::from_iter(slice.iter().copied()));
        let mut scanned = 0_usize;
        let mut lastrec: Option<BiffRecord> = None;

        std::iter::from_fn(move || {
            if let Some(break_id) = break_id {
                if let Some(ref lastrec) = lastrec {
                    if lastrec.id as u16 == break_id {
                        return None;
                    }
                }
            }
            if let Some(max_scan) = max_scan {
                if scanned >= max_scan {
                    return None;
                }
            }
            match BiffRecord::read(reader) {
                Ok(Some(rec)) => {
                    scanned += 1;
                    lastrec.replace(rec.clone());
                    if let Some(ref ids_only) = ids_only {
                        if ids_only.contains(&(rec.id as u16)) {
                            Some(ReadResult::Accepted(rec))
                        } else {
                            Some(ReadResult::Rejected)
                        }
                    } else {
                        if let Some(ref ids_skip) = ids_skip {
                            if ids_skip.contains(&(rec.id as u16)) {
                                Some(ReadResult::Rejected)
                            } else {
                                Some(ReadResult::Accepted(rec))
                            }
                        } else {
                            Some(ReadResult::Accepted(rec))
                        }
                    }
                }
                Ok(None) => None,
                Err(e) => Some(ReadResult::Error(e)),
            }
        })
        .filter_map(|res| match res {
            ReadResult::Accepted(rec) => Some(Ok(rec)),
            _ => None,
        })
    }

    pub const fn size_raw(&self) -> usize {
        1 + ((self.id as u16) & 0x80 != 0) as usize
            + match BiffSize::from_size(self.data.len() as u32) {
                BiffSize::U8(_) => 1,
                BiffSize::U16(_) => 2,
                BiffSize::U24(_) => 3,
                BiffSize::U32(_) => 4,
            }
            + self.data.len()
    }

    pub const fn size(&self) -> usize {
        self.data.len()
    }

    pub const fn data<'a>(&'a self) -> DataSlice<'a> {
        DataSlice::new(unsafe { slice::from_raw_parts(self.data.as_ptr(), self.data.len()) })
    }

    pub const fn data_as<'a, T: Sized + 'a>(&'a self) -> Result<&'a T, BiffError> {
        if size_of::<T>() != self.size() {
            Err(BiffError::SizeMismatch)
        } else {
            Ok(self.data_as_unchecked())
        }
    }

    pub fn as_biff_data<T: BiffDataCompatible + FromBiffData>(
        &self,
        offset: usize,
    ) -> std::io::Result<T> {
        let mut out_data = MaybeUninit::uninit();
        match T::from_biff_data(&self.data, offset, &mut out_data) {
            Ok(_) => Ok(unsafe { out_data.assume_init() }),
            Err(e) => Err(e),
        }
    }

    #[inline]
    pub fn as_biff<T: BiffSerializable>(&self) -> std::io::Result<T> {
        T::from_biff(self)
    }

    pub const fn data_as_unchecked<'a, T: Sized + 'a>(&'a self) -> &'a T {
        unsafe { transmute(&*self.data.as_ptr()) }
    }

    pub fn data_as_xlws(&self) -> Result<Option<String>, BiffError> {
        match self.size() {
            0 => Err(BiffError::Empty),
            n if n < 4 || n % 2 != 0 => Err(BiffError::SizeMismatch),
            _ => match u32::from_le_bytes(self.data[0..4].try_into().unwrap()) {
                0xffff_ffff => Ok(None),
                0 => Ok(Some(String::new())),
                n if n * 2 != ((self.size() as u32) - 4) => Err(BiffError::SizeMismatch),
                n => Ok(Some(
                    match encoding_rs::UTF_16LE
                        .decode_without_bom_handling_and_without_replacement(
                            &self.data[4..4 + (n as usize) * 2],
                        )
                        .expect("non utf-16")
                    {
                        Cow::Borrowed(s) => s.to_owned(),
                        Cow::Owned(s) => s,
                    },
                )),
            },
        }
    }

    pub fn from_sized<'a, T: Sized>(id: u16, data: &'a T) -> Self {
        BiffRecord {
            id: BiffId::from(id),
            data: Box::from(unsafe {
                slice::from_raw_parts(data as *const T as *const u8, std::mem::size_of::<T>())
            }),
        }
    }

    pub fn push(&self, writer: &mut impl Write) -> io::Result<usize> {
        let boxed: Box<[u8]> = self.into();
        writer.write(boxed.as_ref())
    }
}

impl Into<Box<[u8]>> for &BiffRecord {
    fn into(self) -> Box<[u8]> {
        let mut ret = box_alloc(self.size_raw());
        let mut idx = 0_usize;

        if (self.id as u16) & 0x80 != 0 {
            ret[idx..idx + 2].copy_from_slice(&(self.id as u16).to_le_bytes());
            idx += 2;
        } else {
            ret[idx] = self.id as u8;
            idx += 1;
        }
        match BiffSize::from_size(self.data.len() as u32) {
            BiffSize::U8(data) => {
                ret[idx] = data[0];
                idx += 1;
            }
            BiffSize::U16(data) => {
                ret[idx..idx + 2].copy_from_slice(&data);
                idx += 2;
            }
            BiffSize::U24(data) => {
                ret[idx..idx + 3].copy_from_slice(&data);
                idx += 3;
            }
            BiffSize::U32(data) => {
                ret[idx..idx + 4].copy_from_slice(&data);
                idx += 4;
            }
        }
        ret[idx..].copy_from_slice(&self.data);
        ret
    }
}

#[cfg(test)]
use std::fmt::Debug;
#[cfg(test)]
impl Debug for BiffRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.size() == 0 {
            write!(
                f,
                "{:_>4} ({:_>4X}) <{:-^4}> {: <20}",
                {
                    if (self.id as u16) & 0x80 != 0 {
                        ((self.id as u16) & 0b0111_1111)
                            | (((self.id as u16) & 0b0111_1111_0000_0000) >> 1)
                    } else {
                        self.id as u16
                    }
                },
                self.id as u16,
                0,
                self.id
            )
        } else {
            write!(
                f,
                "{:_>4} ({:_>4X}) <{:-^4}> {:-<20}> [{}]",
                {
                    if (self.id as u16) & 0x80 != 0 {
                        ((self.id as u16) & 0b0111_1111)
                            | (((self.id as u16) & 0b0111_1111_0000_0000) >> 1)
                    } else {
                        self.id as u16
                    }
                },
                self.id as u16,
                self.size(),
                self.id,
                self.data
                    .iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, ptr::slice_from_raw_parts};

    use super::*;
    use io::{Seek, Write};

    fn const_reader<const I: usize>(data: [u8; I]) -> Cursor<[u8; I]> {
        Cursor::new(data)
    }

    fn string_reader(id: BiffId, data: &str) -> Cursor<Vec<u8>> {
        let mut cur = Cursor::new(Vec::new());
        let id_raw: Box<[u8]> = id.into();
        cur.write(&id_raw).unwrap();
        cur.write(BiffSize::from_size((data.len() * 2 + 4) as u32).inner())
            .unwrap();
        cur.write(&(data.len() as u32).to_le_bytes()).unwrap();

        let chars: Vec<_> = data.encode_utf16().collect();
        cur.write(unsafe { &*slice_from_raw_parts(chars.as_ptr() as *const u8, chars.len() * 2) })
            .unwrap();

        cur.flush().unwrap();
        cur.seek(io::SeekFrom::Start(0)).unwrap();
        cur
    }

    #[test]
    fn test_biff_record() {
        let r = BiffRecord {
            id: 0.into(),
            data: vec![0x00, 0x00, 0x00, 0x00].into_boxed_slice(),
        };
        let i: &u32 = r.data_as_unchecked();
        println!("{:?}", i);

        let mut r = const_reader([0x01, 0x01, 0x01]);
        let rec = BiffRecord::read(&mut r)
            .expect("must be ok")
            .expect("must be not EOF");
        {
            let _: u8 = rec.data_as::<u8>().expect("must be u8").to_owned();
        }
        let data_2: u8 = rec.data_as::<u8>().expect("must be u8").to_owned();

        let mut cur = string_reader(0.into(), "hello!");
        let rec = BiffRecord::read(&mut cur)
            .expect("must be ok")
            .expect("must be not EOF");

        let s = rec
            .data_as_xlws()
            .expect("must be string")
            .expect("must be not empty");
        assert_eq!(s.as_str(), "hello!");

        let boxed: Box<[u8]> = (&rec).into();

        println!("{:?} {:?} {:?}", boxed, data_2, rec);
    }
}
