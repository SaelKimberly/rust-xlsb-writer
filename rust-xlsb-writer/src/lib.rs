/*
 Copyright 2024 Sael Kimberly

 Licensed under the Apache License, Version 2.0 (the "License");
 you may not use this file except in compliance with the License.
 You may obtain a copy of the License at

 http://www.apache.org/licenses/LICENSE-2.0

 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.
*/
#[allow(unused_imports)]
use rust_xlsb_core::{BiffHead, Error, KnownID, prelude::*};
// mod biff;
// mod err;
// mod styles;
// use err::BiffError;
// mod arr;
// mod cell;
// mod ids;
// mod shs;
// mod util;
// use ids::KnownID;
use mimalloc::MiMalloc;
// mod fmla;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

// pub(crate) struct BiffPointer<R: Read> {
//     inner: BufReader<R>,
// }

// impl<R: Read> BiffPointer<R> {
//     pub fn new(inner: R) -> Self {
//         Self {
//             inner: BufReader::new(inner),
//         }
//     }
// }

// impl<R: Read> Iterator for BiffPointer<R> {
//     type Item = Result<(KnownID, Box<[u8]>)>;

//     fn next(&mut self) -> Option<Self::Item> {
//         let mut id_buf = [0u8; 2];
//         let mut sz_buf = [0u8; 1];
//         let sz_ptr: *const u8 = sz_buf.as_ptr();

//         macro_rules! ret_on_err {
//             ($e: expr) => {
//                 if let Err(e) = $e {
//                     return Some(Err(e.into()));
//                 }
//             };
//         }

//         ret_on_err!(self.inner.read_exact(&mut id_buf));

//         let id = if id_buf[0] > 0x7f {
//             ret_on_err!(self.inner.read_exact(&mut sz_buf));
//             u16::from_le_bytes(id_buf)
//         } else {
//             sz_buf[0] = id_buf[1];
//             id_buf[0] as u16
//         };

//         let mut sz = sz_buf[0] as u32;
//         unsafe {
//             if sz_buf[0] > 0x7f {
//                 ret_on_err!(self.inner.read_exact(&mut sz_buf));
//                 sz ^= ((!*sz_ptr) as u32) << 7;
//             }
//             if sz_buf[0] > 0x7f {
//                 ret_on_err!(self.inner.read_exact(&mut sz_buf));
//                 sz ^= ((!*sz_ptr) as u32) << 14;
//             }

//             if sz_buf[0] > 0x7f {
//                 ret_on_err!(self.inner.read_exact(&mut sz_buf));
//                 sz ^= ((!*sz_ptr) as u32) << 21;
//             }

//             if unlikely(sz_buf[0] > 0x7f) {
//                 panic!("Invalid BIFF")
//             }
//         }
//         if sz > 0 {
//             let mut data = unsafe { Box::<[u8]>::new_uninit_slice(sz as usize).assume_init() };
//             if let Err(e) = self.inner.read_exact(data.as_mut()) {
//                 Some(Err(e.into()))
//             } else {
//                 Some(Ok((unsafe { transmute::<u16, ids::KnownID>(id) }, data)))
//             }
//         } else {
//             Some(Ok((
//                 unsafe { transmute::<u16, ids::KnownID>(id) },
//                 Box::new([]),
//             )))
//         }
//     }
// }

// type Result<T> = std::result::Result<T, err::BiffError>;

// #[derive(Debug, Default)]
// pub struct BiffHead {
//     pub id: u16,
//     pub size: u32,
// }

// impl BiffHead {
//     const MAX_ID: u16 = ((0x7f | 0x80) << 8) | 0x7f;
//     const MAX_SIZE: u32 = (0x7f | (0x7f << 7) | (0x7f << 14) | (0x7f << 21)) as u32;

//     pub const fn new(id: u16, size: u32) -> Result<Self> {
//         if id > Self::MAX_ID {
//             Err(BiffError::Textual("Too large id"))
//         } else if size > Self::MAX_SIZE {
//             Err(BiffError::Textual("Too large size"))
//         } else {
//             Ok(Self { id, size })
//         }
//     }

//     pub const fn from_data(data: &[u8]) -> Result<BiffHead> {
//         let mut p: *const u8 = data.as_ptr();
//         if data.len() < 2 {
//             Err(BiffError::InvalidData)
//         } else {
//             unsafe {
//                 let id = if *p > 0x7f {
//                     let id = *(p as *const u16);
//                     p = offset(p, 2_isize);
//                     id
//                 } else {
//                     let id = *p as u16;
//                     p = offset(p, 1_isize);
//                     id
//                 };
//                 let sz = if data.len() == 2 {
//                     return Err(BiffError::InvalidData);
//                 } else {
//                     let mut sz = *p as u32;

//                     if *p > 0x7f {
//                         if data.len() == 3 {
//                             return Err(BiffError::InvalidData);
//                         }
//                         p = offset(p, 1_isize);
//                         sz ^= ((!*p) as u32) << 7;

//                         if *p > 0x7f {
//                             if data.len() == 4 {
//                                 return Err(BiffError::InvalidData);
//                             }
//                             p = offset(p, 1_isize);
//                             sz ^= ((!*p) as u32) << 14;

//                             if *p > 0x7f {
//                                 if data.len() == 5 {
//                                     return Err(BiffError::InvalidData);
//                                 }
//                                 p = offset(p, 1_isize);
//                                 sz ^= ((!*p) as u32) << 21;
//                             }
//                         }
//                     }

//                     sz
//                 };
//                 BiffHead::new(id, sz)
//             }
//         }
//     }

//     const fn expect_biff_data_size(raw_size: usize) -> usize {
//         if raw_size < 0x80 {
//             1
//         } else if raw_size < 0x4000 {
//             2
//         } else if raw_size < 0x200000 {
//             3
//         } else if raw_size < 0x10000000 {
//             4
//         } else {
//             panic!("Too large BIFF record size!")
//         }
//     }

//     const fn expect_biff_id_size(raw_id: u16) -> usize {
//         if raw_id > 0x7f { 2 } else { 1 }
//     }

//     pub const fn raw_size(&self, data: &mut [u8]) -> Result<usize> {
//         if data.is_empty() {
//             Err(crate::err::BiffError::InsufficientData)
//         } else {
//             let buf = self.size.to_le_bytes();
//             if buf[3] != 0 {
//                 if data.len() < 4 {
//                     Err(crate::err::BiffError::InsufficientData)
//                 } else {
//                     data[0] = buf[0] | 0x80;
//                     data[1] = (buf[1] << 1) | (buf[0] >> 7) | 0x80;
//                     data[2] = (buf[2] << 2) | (buf[1] >> 6) | 0x80;
//                     data[3] = (buf[3] << 3) | (buf[2] >> 5);
//                     Ok(4)
//                 }
//             } else if buf[2] != 0 {
//                 if data.len() < 3 {
//                     Err(crate::err::BiffError::InsufficientData)
//                 } else {
//                     data[0] = buf[0] | 0x80;
//                     data[1] = (buf[1] << 1) | (buf[0] >> 7) | 0x80;
//                     data[2] = (buf[2] << 2) | (buf[1] >> 6);
//                     Ok(3)
//                 }
//             } else if buf[1] != 0 {
//                 if data.len() < 2 {
//                     Err(crate::err::BiffError::InsufficientData)
//                 } else {
//                     data[0] = buf[0] | 0x80;
//                     data[1] = (buf[1] << 1) | (buf[0] >> 7);
//                     Ok(2)
//                 }
//             } else if data.is_empty() {
//                 Err(crate::err::BiffError::InsufficientData)
//             } else {
//                 data[0] = buf[0];
//                 Ok(1)
//             }
//         }
//     }

//     /// Biff ID don't need special encoding, because it's value not used in computing.
//     ///
//     /// # Errors
//     ///
//     /// This function will return an error if `data` is empty, or have not enough space.
//     pub const fn raw_id(&self, data: &mut [u8]) -> Result<usize> {
//         if data.is_empty() {
//             Err(crate::err::BiffError::InsufficientData)
//         } else {
//             let buf = self.id.to_le_bytes();
//             if buf[1] != 0 {
//                 if data.len() < 2 {
//                     Err(crate::err::BiffError::InsufficientData)
//                 } else {
//                     data[0] = buf[0];
//                     data[1] = buf[1];
//                     Ok(2)
//                 }
//             } else if data.is_empty() {
//                 Err(crate::err::BiffError::InsufficientData)
//             } else {
//                 data[0] = self.id as u8;
//                 Ok(1)
//             }
//         }
//     }

//     pub fn from_reader(reader: &mut dyn Read) -> io::Result<BiffHead> {
//         let mut id_buf = [0u8; 2];
//         let mut sz_buf = [0u8; 1];

//         reader.read_exact(&mut id_buf)?;

//         let id = if likely(id_buf[0] < 0x80) {
//             sz_buf[0] = id_buf[1];
//             id_buf[0] as u16
//         } else {
//             reader.read_exact(&mut sz_buf)?;
//             u16::from_le_bytes(id_buf)
//         };

//         let mut sz: u32 = sz_buf[0] as u32;

//         if sz_buf[0] > 0x80 {
//             reader.read_exact(&mut sz_buf)?;
//             sz ^= (!sz_buf[0] as u32) << 7;

//             if sz_buf[0] > 0x80 {
//                 reader.read_exact(&mut sz_buf)?;
//                 sz ^= (!sz_buf[0] as u32) << 14;

//                 if sz_buf[0] > 0x80 {
//                     reader.read_exact(&mut sz_buf)?;
//                     sz ^= (!sz_buf[0] as u32) << 21;
//                 }
//             }
//         }

//         Ok(BiffHead { id, size: sz })
//     }
// }

// #[derive(Debug, Default)]
// pub struct BiffBody<'a> {
//     pub data: Option<&'a mut [u8]>,
// }

// impl BiffBody<'_> {
//     pub const fn new_empty() -> Self {
//         BiffBody { data: None }
//     }

//     pub const fn is_empty(&self) -> bool {
//         self.data.is_none()
//     }

//     pub const fn len(&self) -> usize {
//         match &self.data {
//             Some(data) => data.len(),
//             None => 0,
//         }
//     }
// }

// impl<'a> BiffBody<'a> {
//     pub const fn from_data(data: &'a mut [u8]) -> Self {
//         match data.len() {
//             0 => BiffBody { data: None },
//             _ => BiffBody { data: Some(data) },
//         }
//     }

//     pub const fn into_data(self, id: u16) -> BiffData<'a> {
//         BiffData {
//             head: BiffHead {
//                 id,
//                 size: self.len() as u32,
//             },
//             body: self,
//         }
//     }
// }

// #[derive(Debug, Default)]
// pub struct BiffData<'r> {
//     head: BiffHead,
//     body: BiffBody<'r>,
// }

// impl BiffData<'_> {
//     pub const fn id(&self) -> u16 {
//         self.head.id
//     }

//     pub const fn size(&self) -> u32 {
//         self.head.size
//     }
// }

// impl<'r> BiffData<'r> {
//     pub const fn data_mut(&mut self) -> &mut Option<&'r mut [u8]> {
//         &mut self.body.data
//     }

//     pub fn iter_read(reader: &'r mut dyn Read) -> impl Iterator<Item = Result<BiffData<'r>>> {
//         struct Iter<'r> {
//             reader: &'r mut dyn Read,
//         }

//         impl<'r> Iterator for Iter<'r> {
//             type Item = Result<BiffData<'r>>;

//             #[allow(clippy::uninit_vec)]
//             fn next(&mut self) -> Option<Self::Item> {
//                 match BiffHead::from_reader(self.reader) {
//                     Ok(head) => {
//                         if head.size == 0 {
//                             Some(Ok(BiffData {
//                                 head,
//                                 body: BiffBody::new_empty(),
//                             }))
//                         } else {
//                             let mut buf = Vec::with_capacity(head.size as usize);
//                             unsafe { buf.set_len(head.size as usize) };

//                             match self.reader.read_exact(&mut buf) {
//                                 Err(err) => Some(Err(err.into())),
//                                 Ok(_) => Some(Ok(BiffData {
//                                     head,
//                                     body: BiffBody::from_data(Box::leak(buf.into_boxed_slice())),
//                                 })),
//                             }
//                         }
//                     }
//                     Err(err) => Some(Err(err.into())),
//                 }
//             }
//         }

//         Iter { reader }
//     }
// }

// #[cfg(test)]
// mod tests {

//     use std::fs::File;

//     use crate::{BiffHead, BiffPointer, ids::KnownID};
//     use deku::DekuContainerRead;
//     use zip::read::ZipArchive;

//     #[test]
//     fn it_works() {
//         fn expect_data(data: &'static [u8], _expected: crate::Result<BiffHead>) -> bool {
//             matches!(crate::BiffHead::from_data(data), _expected)
//         }

//         assert!(expect_data(&[0x00, 0x00], BiffHead::new(0, 0)));
//         assert!(expect_data(&[0x00, 0x01], BiffHead::new(0, 1)));
//         assert!(expect_data(
//             &[0x00, 0b1000_0001, 0b0000_0001],
//             BiffHead::new(0, 0b1000_0001)
//         ));
//     }

//     #[test]
//     fn it_panics() {
//         println!("{:02X?}", KnownID::BrtBeginStyleSheet.as_biff_literal(&0u8));
//         assert!(BiffHead::from_data(&[0x00, 0x80]).is_err());
//     }

//     #[test]
//     #[allow(clippy::uninit_vec)]
//     fn read_some_xlsb() {
//         let file = File::open("covid.xlsb").expect("Not found");

//         let mut zip_arc = ZipArchive::new(file).expect("Cannot read zip archive");

//         // for file in zip_arc.file_names() {
//         //     println!("{}", file);
//         // }
//         let style_reader = zip_arc
//             .by_name("xl/worksheets/sheet1.bin")
//             .expect("Contains no styles");

//         let mut biff_reader = BiffPointer::new(style_reader);
//         let mut i = 0;

//         let from = 8_830_000;

//         let till = from + 10_000;

//         while let Some(Ok((id, data))) = biff_reader.next() {
//             i += 1;
//             if i < from {
//                 continue;
//             }
//             if i > till {
//                 break;
//             }

//             macro_rules! match_id {
//                 ($id: ident $(, $other_ids: ident)*) => {
//                     match id {
//                         crate::ids::KnownID::$id => {
//                             println!(
//                                 "----------\n[x]  {:?}\n----------",
//                                 crate::biff::$id::from_bytes((data.as_ref(), 0))
//                                     .expect("Cannot parse")
//                                     .1
//                             )
//                         }$(
//                             ,crate::ids::KnownID::$other_ids => {
//                                 println!(
//                                     "----------\n[x]  {:?}\n----------",
//                                     crate::biff::$other_ids::from_bytes((data.as_ref(), 0))
//                                         .expect("Cannot parse")
//                                         .1
//                                 )
//                             }
//                         )*,
//                         _ => println!("[ ] {:?}: {:02X?}", id, data)
//                     }
//                 };
//             }

//             match_id!(
//                 BrtFont,
//                 BrtFill,
//                 BrtXF,
//                 BrtBorder,
//                 BrtFmt,
//                 BrtBeginFonts,
//                 BrtEndFonts,
//                 BrtBeginFills,
//                 BrtEndFills,
//                 BrtBeginBorders,
//                 BrtEndBorders,
//                 BrtBeginCellStyleXFs,
//                 BrtEndCellStyleXFs,
//                 BrtBeginCellXFs,
//                 BrtEndCellXFs,
//                 BrtBeginStyles,
//                 BrtEndStyles,
//                 BrtBeginDXFs,
//                 BrtEndDXFs,
//                 BrtCellIsst,
//                 BrtCellReal,
//                 BrtBeginSheet,
//                 BrtBeginWsViews,
//                 BrtEndWsView,
//                 BrtACEnd,
//                 BrtBeginColInfos,
//                 BrtEndColInfos,
//                 BrtBeginSheetData,
//                 BrtColInfo,
//                 BrtRowHdr,
//                 BrtCellRk,
//                 BrtRwDescent,
//                 BrtACBegin,
//                 BrtWsProp,
//                 BrtWsDim,
//                 BrtWsFmtInfo,
//                 BrtEndWsViews,
//                 BrtWsFmtInfoEx14,
//                 BrtSel,
//                 BrtBeginWsView,
//                 BrtEndSheetData,
//                 BrtPrintOptions,
//                 BrtMargins,
//                 BrtEndSheet,
//                 BrtPivotCacheAutoRefresh
//             );
//         }
//     }
// }
