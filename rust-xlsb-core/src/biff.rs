use std::{
    io::{self, Write},
    ops::Deref,
};

use deku::{DekuContainerRead, DekuContainerWrite};

use crate::{BiffHead, CheckedBiff, Error, KnownID, Result};

pub trait BiffWrite: CheckedBiff {
    fn biff_push_to(&self, out: &mut Vec<u8>) -> Result<usize>;
    fn biff_push_to_writer(&self, out: &mut dyn Write) -> Result<usize>;
}

impl<T: for<'a> DekuContainerRead<'a> + DekuContainerWrite + CheckedBiff> BiffWrite for T {
    fn biff_push_to(&self, out: &mut Vec<u8>) -> Result<usize> {
        if let Ok(r) = self.to_bytes() {
            out.extend_from_slice(&r);
            Ok(r.len())
        } else {
            Err(Error::ValidationError("Invalid BIFF record".to_owned()))
        }
    }

    fn biff_push_to_writer(&self, out: &mut dyn Write) -> Result<usize> {
        if let Ok(r) = self.to_bytes() {
            out.write(&r).map_err(Error::BiffWriteFailed)
        } else {
            Err(Error::ValidationError("Invalid BIFF record".to_owned()))
        }
    }
}

pub fn write_as_empty_biff(id: KnownID, writer: &mut dyn Write) -> io::Result<usize> {
    let biff_head = BiffHead::new(id as u16, 0).expect("Should never fail");
    let (raw, len) = biff_head.as_raw_data();
    writer.write(&raw[..len])
}

pub fn write_as_biff<W: Write, S: Deref<Target = [u8]>>(
    id: KnownID,
    data: S,
    writer: &mut W,
) -> io::Result<usize> {
    let data = data.deref();
    let biff_head = BiffHead::new(id as u16, data.len() as u32).expect("Should never fail");
    let (raw, len) = biff_head.as_raw_data();
    Ok(writer.write(&raw[..len])? + writer.write(data)?)
}
