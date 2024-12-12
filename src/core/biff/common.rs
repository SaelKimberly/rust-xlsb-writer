use super::{
    aligned_biff_data_impl, box_alloc, checked, declare_packable, impl_biff_data_compatible,
    impl_packable_for, internal_impl_packable_for, pack_biff_data, try_to_sized,
    BiffDataCompatible, BiffId, BiffRecord, BiffSerializable, FromBiffData, IntoBiffData,
    ValidBiff,
};

aligned_biff_data_impl!(u16, u32, u64, i16, i32, i64);

impl BiffDataCompatible for u8 {
    fn size_of_type() -> usize {
        1
    }
}

impl BiffDataCompatible for i8 {
    fn size_of_type() -> usize {
        1
    }
}

impl IntoBiffData for u8 {
    fn size_of(&self) -> usize {
        1
    }

    fn into_biff_data(&self, offset: usize, out_data: &mut Box<[u8]>) -> std::io::Result<usize> {
        if out_data.len() < offset + 1 {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
        }
        out_data[offset] = *self;
        Ok(1)
    }
}

impl IntoBiffData for i8 {
    fn size_of(&self) -> usize {
        1
    }

    fn into_biff_data(&self, offset: usize, out_data: &mut Box<[u8]>) -> std::io::Result<usize> {
        if out_data.len() < offset + 1 {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
        }
        out_data[offset] = *self as u8;
        Ok(1)
    }
}

impl FromBiffData for u8 {
    fn from_biff_data(
        data: &Box<[u8]>,
        offset: usize,
        out_data: &mut std::mem::MaybeUninit<Self>,
    ) -> std::io::Result<usize> {
        if data.len() < offset + 1 {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
        }
        out_data.write(data[offset]);
        Ok(1)
    }
}

impl FromBiffData for i8 {
    fn from_biff_data(
        data: &Box<[u8]>,
        offset: usize,
        out_data: &mut std::mem::MaybeUninit<Self>,
    ) -> std::io::Result<usize> {
        if data.len() < offset + 1 {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
        }
        out_data.write(data[offset] as i8);
        Ok(1)
    }
}

impl ValidBiff for u8 {}
impl ValidBiff for i8 {}
