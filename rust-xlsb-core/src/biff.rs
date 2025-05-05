use std::io::Write;

use deku::{DekuContainerRead, DekuContainerWrite};

use crate::{CheckedBiff, Error, Result};

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
