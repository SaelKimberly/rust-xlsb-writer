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
        let Ok(r) = self.to_bytes() else {
            return Err(Error::ValidationError("Invalid BIFF record".to_owned()));
        };
        out.extend_from_slice(&r);
        Ok(r.len())
    }

    fn biff_push_to_writer(&self, out: &mut dyn Write) -> Result<usize> {
        let Ok(r) = self.to_bytes() else {
            return Err(Error::ValidationError("Invalid BIFF record".to_owned()));
        };
        out.write(&r).map_err(Error::BiffWriteFailed)
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
    let data = &*data;
    let size =
        u32::try_from(data.len()).map_err(|_| io::Error::other("Record body exceeds u32 range"))?;
    let biff_head = BiffHead::new(id as u16, size).map_err(io::Error::other)?;
    let (raw, len) = biff_head.as_raw_data();
    Ok(writer.write(&raw[..len])? + writer.write(data)?)
}

pub fn push_record<T: BiffWrite>(out: &mut Vec<u8>, record: &T) -> Result<()> {
    let mut body = Vec::new();
    record.biff_push_to(&mut body)?;
    write_as_biff(record.id(), body, out).map_err(Error::BiffWriteFailed)?;
    Ok(())
}
