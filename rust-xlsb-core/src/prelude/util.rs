use core::slice;
use std::{
    borrow::Cow,
    hash::Hash,
    io::{ErrorKind, Read, Seek, Write},
    ops::Deref,
};

use bitflags::{Bits, Flags};
use deku::{
    DekuError, DekuRead, DekuWrite, bitvec::BitSlice, ctx::BitSize, error::NeedSize,
    reader::Reader, writer::Writer,
};
use zerocopy::{FromBytes, Immutable, IntoBytes};

pub(crate) type Result<T> = std::result::Result<T, DekuError>;

#[derive(DekuRead, DekuWrite, Debug, Default, Clone, Copy)]
pub struct Xnum(f64);

impl Deref for Xnum {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Xnum {
    pub const fn to_le_bytes(self) -> [u8; 8] {
        self.0.to_le_bytes()
    }
}

impl From<f64> for Xnum {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<Xnum> for f64 {
    fn from(value: Xnum) -> Self {
        value.0
    }
}

impl PartialEq for Xnum {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl PartialOrd for Xnum {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(std::cmp::Ord::cmp(self, other))
    }
}

impl Ord for Xnum {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Eq for Xnum {}

impl Hash for Xnum {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

#[allow(clippy::uninit_vec)]
pub(crate) fn nullable_utf16_read<R: Read + Seek>(
    reader: &mut Reader<R>,
) -> Result<Option<String>> {
    let mut buf = [0u8; 4];
    reader.read_bytes_const(&mut buf)?;
    Ok(match u32::from_le_bytes(buf) {
        0xff_ff_ff_ff => None,
        0x00_00_00_00 => Some(String::new()),
        n => {
            let mut buf = Vec::<u8>::with_capacity(n as usize * 2);
            unsafe { buf.set_len(n as usize * 2) };
            Some(
                String::from_utf16le(buf.as_slice())
                    .map_err(|_| DekuError::Io(ErrorKind::InvalidData))?,
            )
        }
    })
}

#[allow(clippy::uninit_vec)]
pub(crate) fn utf16_read<R: Read + Seek>(reader: &mut Reader<R>) -> Result<String> {
    let mut buf = [0u8; 4];
    reader.read_bytes_const(&mut buf)?;
    Ok(match u32::from_le_bytes(buf) {
        0xff_ff_ff_ff => return Err(DekuError::Io(ErrorKind::InvalidData)),
        0x00_00_00_00 => String::new(),
        n => {
            let mut buf = Vec::<u8>::with_capacity(n as usize * 2);
            unsafe { buf.set_len(n as usize * 2) };
            reader.read_bytes(n as usize * 2, &mut buf)?;

            String::from_utf16le(buf.as_slice())
                .map_err(|_| DekuError::Io(std::io::ErrorKind::InvalidData))?
        }
    })
}

pub(crate) fn utf16_write<W: Write + Seek>(field: &str, writer: &mut Writer<W>) -> Result<()> {
    writer.write_bytes(&(field.len() as u32).to_le_bytes())?;
    if !field.is_empty() {
        let a = field.encode_utf16().collect::<Vec<_>>();
        let b = unsafe { slice::from_raw_parts(a.as_ptr() as *const u8, field.len() * 2) };
        writer.write_bytes(b)?;
    }
    Ok(())
}

pub(crate) fn nullable_utf16_write<W: Write + Seek, S: Deref<Target = str>>(
    field: Option<S>,
    writer: &mut Writer<W>,
) -> Result<()> {
    if let Some(field) = field.as_deref() {
        utf16_write(field, writer)
    } else {
        writer.write_bytes(&0xff_ff_ff_ff_u32.to_le_bytes())?;
        Ok(())
    }
}

pub(crate) trait DekuBitFlag: Sized + Default + Hash + Flags {}

#[allow(clippy::uninit_vec)]
pub(crate) fn raw_bit_read<R: Read + Seek, T: Bits + FromBytes, F: Flags<Bits = T>>(
    reader: &mut Reader<R>,
    bit_size: Option<BitSize>,
) -> Result<F> {
    let flags = if let Some(bit_size) = bit_size {
        let bits = reader
            .read_bits(bit_size.0)?
            .ok_or(DekuError::Incomplete(NeedSize::new(bit_size.0)))?;
        F::from_bits_truncate(
            T::read_from_bytes(bits.as_raw_slice())
                .map_err(|e| DekuError::Parse(Cow::Owned(format!("{:?}", e))))?,
        )
    } else {
        let mut buf = Vec::<u8>::with_capacity(size_of::<T>());
        unsafe { buf.set_len(size_of::<T>()) };
        reader.read_bytes(size_of::<T>(), buf.as_mut_slice())?;

        F::from_bits_truncate(T::read_from_bytes(buf.as_slice()).unwrap())
    };
    Ok(flags)
}

#[allow(clippy::uninit_vec)]
pub(crate) fn raw_bit_write<
    W: Write + Seek,
    T: Bits + Immutable + IntoBytes,
    F: Flags<Bits = T>,
>(
    field: &F,
    writer: &mut Writer<W>,
    bit_size: Option<BitSize>,
) -> Result<()> {
    let buf = field.bits();
    if let Some(bit_size) = bit_size {
        if bit_size.0 > deku::writer::MAX_BITS_AMT {
            return Err(DekuError::Parse(Cow::Owned(format!(
                "BitSlice max size exceeded: {} > {}",
                bit_size.0,
                deku::writer::MAX_BITS_AMT
            ))));
        }
        let bs = unsafe { BitSlice::from_slice_unchecked(buf.as_bytes()) };
        writer.write_bits(unsafe { bs.get_unchecked(0..bit_size.0) })?;
    } else {
        writer.write_bytes(buf.as_bytes())?;
    }
    Ok(())
}
