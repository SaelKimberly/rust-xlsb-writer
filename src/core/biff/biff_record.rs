use super::{BiffId, BiffScanner, BiffSize};
use std::{
    collections::HashSet,
    fmt::Debug,
    io::{self, Read, Write},
    mem::size_of,
    rc::Rc,
    slice,
};

#[derive(Debug)]
/// The error type for the `BiffRecord` struct for
pub(crate) enum BiffError {
    Empty,
    SizeMismatch,
}

#[inline]
pub(crate) fn invalid_data<T>(msg: &str) -> std::io::Result<T> {
    Err(std::io::Error::new(std::io::ErrorKind::InvalidData, msg))
}

#[inline]
pub(crate) const fn try_to_sized<T: Sized>(offset: usize, data: &[u8]) -> Result<T, BiffError> {
    if std::mem::size_of::<T>() <= (data.len() - offset) {
        Ok(unsafe { data.as_ptr().offset(offset as isize).cast::<T>().read() })
    } else {
        Err(BiffError::SizeMismatch)
    }
}

#[cfg_attr(feature = "test", derive(PartialEq))]
pub(crate) enum BiffRecord {
    Sized { id: BiffId, data: Box<[u8]> },
    Empty { id: BiffId },
}

impl BiffRecord {
    #[inline]
    pub fn from_data(id: u16, data: impl AsRef<[u8]>) -> Self {
        let data_ref = data.as_ref();
        if data_ref.len() == 0 {
            Self::Empty {
                id: BiffId::from(id),
            }
        } else {
            Self::Sized {
                id: BiffId::from(id),
                data: Box::from(data_ref),
            }
        }
    }

    #[inline]
    pub const fn id(&self) -> BiffId {
        *(match self {
            Self::Sized { id, .. } => id,
            Self::Empty { id } => id,
        })
    }

    #[inline]
    pub(crate) const fn from_id(id: u16) -> Self {
        // duplicate of `BiffId::from`, but with the `const`
        Self::Empty {
            id: unsafe { std::mem::transmute(id) },
        }
    }

    #[inline]
    pub fn copy_with_data(&self, data: &[u8]) -> Self {
        Self::Sized {
            id: self.id(),
            data: Box::from(data),
        }
    }

    #[inline]
    pub const fn try_to_sized<T: Sized>(&self, offset: usize) -> Result<T, BiffError> {
        match self {
            Self::Sized { id: _, data } => try_to_sized(offset, data),
            _ => Err(BiffError::Empty),
        }
    }

    pub fn from_sized<'a, T: Sized>(id: u16, data: &'a T) -> Self {
        Self::Sized {
            id: BiffId::from(id),
            data: Box::from(unsafe {
                slice::from_raw_parts(data as *const T as *const u8, std::mem::size_of::<T>())
            }),
        }
    }

    #[inline]
    pub const fn into_inner(&self) -> &[u8] {
        match self {
            Self::Sized { id: _, data } => data,
            Self::Empty { id: _ } => &[],
        }
    }

    pub fn push(&self, writer: &mut impl Write) -> io::Result<usize> {
        let mut pushed: usize = 0;
        match self {
            BiffRecord::Sized { id, data } => {
                if ((*id as u16) & 0x80) != 0 {
                    pushed += writer.write(&(*id as u16).to_le_bytes())?;
                } else {
                    pushed += writer.write(&(((*id as u16) & 0x7f) as u8).to_le_bytes())?;
                }

                if data.len() == 0 {
                    pushed += writer.write(&[0u8; 1])?;
                } else {
                    let biff_sz = BiffSize::from_size(data.len() as u32);
                    pushed += writer.write(biff_sz.inner())?;
                    pushed += writer.write(data)?;
                }
            }
            BiffRecord::Empty { id } => {
                if ((*id as u16) & 0x80) != 0 {
                    pushed += writer.write(&(*id as u16).to_le_bytes())?;
                } else {
                    pushed += writer.write(&(((*id as u16) & 0x7f) as u8).to_le_bytes())?;
                }
                pushed += writer.write(b"\x00")?;
            }
        }
        Ok(pushed)
    }

    pub fn scan<'a, R: Read + ?Sized>(
        reader: &'a mut R,
        ids_only: Option<&[u16]>,
        ids_skip: Option<&[u16]>,
        break_id: Option<u16>,
        max_scan: Option<usize>,
    ) -> impl Iterator<Item = Option<io::Result<BiffRecord>>> + 'a {
        BiffScanner {
            reader,
            ids_only: ids_only
                .map(|slice| HashSet::<u16>::from_iter(slice.iter().copied()))
                .map(Rc::from),
            ids_skip: ids_skip
                .map(|slice| HashSet::<u16>::from_iter(slice.iter().copied()))
                .map(Rc::from),
            break_id,
            max_scan,
            scan_cnt: 0,
        }
    }
}
pub trait VarSizeBiffRecord: Sized + Default + PartialEq {
    const ID: Option<BiffId>;

    fn peek(offset: usize, data: &[u8]) -> Result<Self, BiffError>;
    fn push(&self, writer: &mut impl Write) -> io::Result<usize>;

    fn push_many(
        writer: &mut impl Write,
        elements: impl IntoIterator<Item = Self>,
    ) -> io::Result<usize> {
        elements.into_iter().map(|e| e.push(writer)).sum()
    }

    #[allow(unused_variables)]
    fn peek_many(offset: usize, data: &[u8], count: usize) -> Result<Box<[Self]>, BiffError> {
        Err(BiffError::SizeMismatch)
    }
}

pub trait FixSizeBiffRecord: Sized + Default + PartialEq {
    const ID: Option<BiffId> = None;
}

impl<T: FixSizeBiffRecord> VarSizeBiffRecord for T {
    const ID: Option<BiffId> = T::ID;

    #[inline]
    fn peek(offset: usize, data: &[u8]) -> Result<Self, BiffError> {
        try_to_sized(offset, data)
    }

    #[inline]
    fn push(&self, writer: &mut impl Write) -> io::Result<usize> {
        match Self::ID {
            Some(id) => BiffRecord::from_sized(id as u16, self).push(writer),
            None => writer.write(unsafe {
                slice::from_raw_parts(self as *const T as *const u8, size_of::<Self>())
            }),
        }
    }

    fn peek_many(offset: usize, data: &[u8], count: usize) -> Result<Box<[Self]>, BiffError> {
        if count == 0 {
            return Ok(Box::from([]));
        } else {
            let mut ret = Vec::<Self>::with_capacity(count);

            for i in 0..count {
                ret.push(Self::peek(0, &data[(offset + i * size_of::<Self>())..])?);
            }
            Ok(Box::from(ret))
        }
    }
}

pub trait CheckedBiffRecord: VarSizeBiffRecord {
    fn validate<'a>(&'a self) -> io::Result<&'a Self>;

    fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

pub(crate) mod macros {
    #[allow(unused_imports)]
    pub(crate) use super::{super::biff_record::*, super::BiffId};

    macro_rules! fixed_size_biff {
        ($name: ident, $id: expr, $($check_expr: expr, $message: expr)?) => {

            impl FixSizeBiffRecord for $name {
                const ID: Option<BiffId> = $id;
            }

            impl CheckedBiffRecord for $name {
                fn validate<'a>(&'a self) -> std::io::Result<&'a Self> {
                    $(
                        if !($check_expr)(&self) {
                            return invalid_data($message);
                        }
                    )*
                    return Ok(self);
                }
            }
        };
    }

    macro_rules! single_field_fixed_size_biff {
        ($name: ident, $id: expr, $field_name: ident, $field_type: ty, $($check_expr: expr, $message: expr)?) => {
            #[cfg_attr(feature = "test", derive(Debug))]
            #[derive(Default, PartialEq, Clone)]
            pub struct $name {
                pub $field_name: $field_type,
            }

            impl FixSizeBiffRecord for $name {
                const ID: Option<BiffId> = $id;
            }

            impl CheckedBiffRecord for $name {
                fn validate<'a>(&'a self) -> std::io::Result<&'a Self> {
                    $(
                        if !($check_expr)(&self) {
                            return invalid_data($message);
                        }
                    )*
                    return Ok(self);
                }
            }

            impl TryFrom<$field_type> for $name {
                type Error = std::io::Error;

                fn try_from(value: $field_type) -> Result<Self, Self::Error> {
                    let value = $name { $field_name: value };
                    $(
                        if !($check_expr)(&value) {
                            return invalid_data($message);
                        }
                    )*
                    Ok(value)
                }
            }

            impl Into<$field_type> for $name {
                fn into(self) -> $field_type {
                    self.$field_name
                }
            }
        };
    }

    macro_rules! push_many {
        ($writer: ident, $($data: expr,)*) => {
            {
                let mut pushed: usize = 0;
                $(
                    pushed += $data.push($writer)?;
                )*
                pushed
            }
        };
    }

    pub(crate) use {fixed_size_biff, push_many, single_field_fixed_size_biff};
}

pub(crate) use macros::{fixed_size_biff, push_many, single_field_fixed_size_biff};

#[cfg(test)]
impl Debug for BiffRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sized { id, data } => {
                write!(
                    f,
                    "{:_>4} ({:_>4X}) <{:-^4}> {:-<20}> [{}]",
                    {
                        if (*id as u16) & 0x80 != 0 {
                            ((*id as u16) & 0b0111_1111)
                                | (((*id as u16) & 0b0111_1111_0000_0000) >> 1)
                        } else {
                            *id as u16
                        }
                    },
                    *id as u16,
                    data.len(),
                    id,
                    data.iter()
                        .map(|b| format!("{:02X}", b))
                        .collect::<Vec<_>>()
                        .join(" "),
                )
            }
            Self::Empty { id } => {
                write!(
                    f,
                    "{:_>4} ({:_>4X}) <{:-^4}> {: <20}",
                    {
                        if (*id as u16) & 0x80 != 0 {
                            ((*id as u16) & 0b0111_1111)
                                | (((*id as u16) & 0b0111_1111_0000_0000) >> 1)
                        } else {
                            *id as u16
                        }
                    },
                    *id as u16,
                    0,
                    id
                )
            }
        }
    }
}
