#![allow(unused_imports)]
mod biff_data;
mod biff_id;
mod biff_record;
mod biff_size;
mod biff_traits;
mod cell;
mod common;
mod internal;
mod prelude;

pub(crate) type BiffSize = biff_size::BiffSize;

use std::mem::MaybeUninit;

pub(crate) use biff_data::{
    aligned_biff_data_impl, box_alloc, checked, declare_packable, impl_biff_data_compatible,
    impl_packable_for, internal_impl_packable_for, pack_biff_data, try_to_sized,
    BiffDataCompatible, BiffId, BiffRecord, BiffSerializable, FromBiffData, IntoBiffData,
};

mod records;

#[cfg_attr(feature = "test", derive(Debug))]
#[derive(Default, PartialEq)]
pub(crate) struct I24Adapter {
    inner: i32,
}

impl BiffDataCompatible for I24Adapter {
    fn size_of_type() -> usize {
        3
    }
}

impl IntoBiffData for I24Adapter {
    fn size_of(&self) -> usize {
        3
    }

    fn into_biff_data(&self, offset: usize, out_data: &mut Box<[u8]>) -> std::io::Result<usize> {
        if out_data.len() < offset + 3 {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
        }
        out_data[offset + 2] = (self.inner >> 16) as u8;
        out_data[offset + 1] = (self.inner >> 8) as u8;
        out_data[offset] = self.inner as u8;
        Ok(3)
    }
}

impl FromBiffData for I24Adapter {
    fn from_biff_data(
        data: &Box<[u8]>,
        offset: usize,
        out_data: &mut MaybeUninit<Self>,
    ) -> std::io::Result<usize> {
        if data.len() < offset + 3 {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
        }
        out_data.write(I24Adapter {
            inner: i32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], 0]),
        });
        Ok(3)
    }
}

impl From<i32> for I24Adapter {
    fn from(value: i32) -> Self {
        I24Adapter { inner: value }
    }
}

impl Into<i32> for I24Adapter {
    fn into(self) -> i32 {
        self.inner
    }
}

impl ValidBiff for I24Adapter {}

declare_packable!(
    UncheckedCol,
    |x: &Self| x.inner >= 0 && x.inner <= 16383,
    inner,
    i32
);

declare_packable!(
    Cell,
    checked,
    column,
    UncheckedCol,
    i_style_ref,
    I24Adapter,
    f_ph_show,
    u8
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell() {
        let cell = Cell::new(0x12.into(), 0x34.into(), 0x56);
        let mut data = box_alloc(8);
        let r = cell.into_biff_data(0, &mut data);
        assert!(r.is_ok());

        let some = pack_biff_data!(&cell).unwrap();
        assert_eq!(some, data);

        println!("{:?}", data);
        let cnew = Cell::deserialize(&some);
        assert!(cnew.is_ok());
        assert_eq!(cnew.unwrap(), cell);

        println!("{:?}", cell);
        println!("{:?}", data);
    }
}
