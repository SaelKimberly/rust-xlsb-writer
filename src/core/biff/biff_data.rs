use std::{io, mem::MaybeUninit};

pub(crate) trait BiffDataCompatible: Sized {
    #[inline]
    fn size_of_type() -> usize {
        size_of::<Self>()
    }
}

pub(crate) trait IntoBiffData: BiffDataCompatible {
    #[inline]
    fn size_of(&self) -> usize {
        size_of::<Self>()
    }

    fn into_biff_data(&self, offset: usize, out_data: &mut Box<[u8]>) -> std::io::Result<usize>;
}

pub(crate) trait FromBiffData: BiffDataCompatible {
    fn from_biff_data(
        data: &Box<[u8]>,
        offset: usize,
        out_data: &mut MaybeUninit<Self>,
    ) -> std::io::Result<usize>;

    fn deserialize(data: &Box<[u8]>) -> std::io::Result<Self> {
        if data.len() != Self::size_of_type() {
            Err(std::io::Error::from(std::io::ErrorKind::InvalidData))
        } else {
            let mut ret = MaybeUninit::uninit();
            match Self::from_biff_data(&data, 0, &mut ret) {
                Err(e) => Err(e),
                Ok(_) => Ok(unsafe { ret.assume_init() }),
            }
        }
    }
}

pub(crate) trait AlignedBiffData: BiffDataCompatible {}

impl<T: AlignedBiffData> IntoBiffData for T {
    fn into_biff_data(&self, offset: usize, out_data: &mut Box<[u8]>) -> std::io::Result<usize> {
        if out_data.len() < offset + self.size_of() {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
        } else {
            let slice = out_data[offset..offset + self.size_of()].as_mut_ptr();
            unsafe {
                core::ptr::copy_nonoverlapping(
                    self as *const T as *const u8,
                    slice,
                    self.size_of(),
                );
            }
            Ok(self.size_of())
        }
    }
}

impl<T: AlignedBiffData + Copy> FromBiffData for T {
    fn from_biff_data(
        data: &Box<[u8]>,
        offset: usize,
        out_data: &mut MaybeUninit<T>,
    ) -> std::io::Result<usize> {
        if data.len() < offset + Self::size_of_type() {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
        } else {
            let slice = data[offset..offset + Self::size_of_type()].as_ptr();
            out_data.write(unsafe { *(slice as *const T) });
            Ok(Self::size_of_type())
        }
    }
}

#[allow(unused_macros)]
pub(crate) mod macros {
    macro_rules! impl_biff_data_compatible {
        ($name: ident$(, $base_types: ty)*) => {
            impl BiffDataCompatible for $name {
                fn size_of_type() -> usize {
                    0 $(+ <$base_types>::size_of_type())*
                }
            }
        };
    }

    macro_rules! aligned_biff_data_impl {
        ($type: ty$(, $rest: ty)*) => {
            impl crate::core::biff::biff_data::BiffDataCompatible for $type {}
            impl crate::core::biff::biff_data::AlignedBiffData for $type {}
            $(
                impl crate::core::biff::biff_data::BiffDataCompatible for $rest {}
                impl crate::core::biff::biff_data::AlignedBiffData for $rest {}
            )*
        };
    }

    macro_rules! pack_biff_data {
        ($expr: expr) => {{
            let mut data = crate::core::biff::biff_record::box_alloc(
                crate::core::biff::biff_data::IntoBiffData::size_of($expr)
            );

            match crate::core::biff::biff_data::IntoBiffData::into_biff_data($expr, 0, &mut data) {
                Ok(_) => Ok(data),
                Err(e) => Err(e),
            }
        }};
        ($expr: expr$(, $rest: expr)+) => {{
            let mut data = crate::core::biff::biff_record::box_alloc(
                crate::core::biff::biff_data::IntoBiffData::size_of($expr) $(
                    + crate::core::biff::biff_data::IntoBiffData::size_of($rest)
                )*
            );

            loop {
                let mut off = 0;
                match crate::core::biff::biff_data::IntoBiffData::into_biff_data($expr, off, &mut data) {
                    Ok(data_size) => {
                        off += data_size;
                    },
                    Err(e) => {
                        break Err(e);
                    }
                }
                $(
                    match crate::core::biff::biff_data::IntoBiffData::into_biff_data($rest, off, &mut data) {
                        Ok(data_size) => {
                            off += data_size;
                        },
                        Err(e) => {
                            break Err(e);
                        }
                    }
                )+

                debug_assert!(off == data.len());

                break Ok(data);
            }
        }};
    }

    macro_rules! internal_impl_packable_for {
        ($name: ident, $check_expr: expr) => {
            use crate::core::biff::biff_data::*;

            impl_biff_data_compatible!($name);

            impl crate::core::biff::biff_data::IntoBiffData for $name {
                #[allow(unused_variables)]
                fn into_biff_data(
                    &self,
                    offset: usize,
                    out_data: &mut Box<[u8]>,
                ) -> std::io::Result<usize> {
                    Ok(0)
                }
            }

            impl crate::core::biff::biff_data::FromBiffData for $name {
                #[allow(unused_variables)]
                fn from_biff_data(
                    data: &Box<[u8]>,
                    offset: usize,
                    out_data: &mut MaybeUninit<Self>,
                ) -> std::io::Result<usize> {
                    out_data.write($name {});
                    Ok(0)
                }
            }

            impl CheckBiff for $name {
                fn validated<'a>(&'a self) -> std::io::Result<&'a Self> {
                    if $check_expr(self) {
                        Ok(self)
                    } else {
                        Err(std::io::Error::from(std::io::ErrorKind::InvalidData))
                    }
                }
            }
        };

        ($name: ident, $check_expr: expr, $(, $field: ident, $type: ty)+) => {
            impl_biff_data_compatible!($name $(, $type)*);

            impl crate::core::biff::biff_data::IntoBiffData for $name {
                fn size_of(&self) -> usize {
                    0 $(+ <$type as crate::core::biff::biff_data::BiffDataCompatible>::size_of_type())*
                }
                #[inline]
                #[allow(unused_variables)]
                fn into_biff_data(
                    &self,
                    offset: usize,
                    out_data: &mut Box<[u8]>,
                ) -> std::io::Result<usize> {
                    if self.size_of() == 0 {
                        Ok(0)
                    } else {
                        if out_data.len() < offset + self.size_of() {
                            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
                        }
                        let mut offset = offset;
                        $(
                            match crate::core::biff::biff_data::IntoBiffData::into_biff_data(&self.$field, offset, out_data) {
                                Ok(ref data_size) => {
                                    offset += data_size;
                                }
                                Err(e) => {
                                    return Err(e);
                                }
                            }
                        )+

                        Ok(offset)
                    }
                }
            }

            impl crate::core::biff::biff_data::FromBiffData for $name {
                #[allow(unused_variables)]
                fn from_biff_data(
                    data: &Box<[u8]>,
                    offset: usize,
                    out_data: &mut std::mem::MaybeUninit<Self>,
                ) -> std::io::Result<usize> {
                    if data.len() < offset + Self::size_of_type() {
                        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
                    } else {
                        let mut offset = offset;
                        out_data.write(Self {
                            $(
                                $field: {
                                    let mut field_value = std::mem::MaybeUninit::uninit();
                                    match crate::core::biff::biff_data::FromBiffData::from_biff_data(&data, offset, &mut field_value) {
                                        Ok(ref data_size) => {
                                            offset += data_size;
                                            unsafe { field_value.assume_init() }
                                        }
                                        Err(e) => {
                                            return Err(e);
                                        }
                                    }
                                },
                            )+
                        });
                        Ok(offset)
                    }
                }
            }

            impl CheckBiff for $name {
                fn validated(&self) -> std::io::Result<&Self> {
                    $(
                        self.$field.validated()?;
                    )+
                    if $check_expr(self) {
                        Ok(self)
                    } else {
                        Err(std::io::Error::from(std::io::ErrorKind::InvalidData))
                    }
                }
            }
        };
    }

    macro_rules! impl_packable_for {
        ($name: ident, $check_expr: expr) => {
            use crate::core::biff::biff_data::*;

            internal_impl_packable_for!($name, $check_expr);
        };
        ($name: ident, $check_expr: expr, $id: ident) => {
            use crate::core::biff::biff_data::*;

            internal_impl_packable_for!($name, $check_expr);

            impl BiffSerializable for $name {
                const ID: crate::core::biff::BiffId = crate::core::biff::BiffId::$id;
            }
        };
        ($name: ident, $check_expr: expr $(, $field: ident, $type: ty)+) => {
            use crate::core::biff::biff_data::*;

            internal_impl_packable_for!($name, $check_expr, $(, $field, $type)+);
        };
        ($name: ident, $check_expr: expr, $id: ident  $(, $field: ident, $type: ty)+) => {
            use crate::core::biff::biff_data::*;

            internal_impl_packable_for!($name, $check_expr, $(, $field, $type)+);

            impl BiffSerializable for $name {
                const ID: crate::core::biff::BiffId = crate::core::biff::BiffId::$id;
            }
        };
    }

    // This macro creates a struct with no fields, one or more fields, with or without an ID
    // Every field must be `BiffDataCompatible` and implements `FromBiffData` and `IntoBiffData`
    macro_rules! declare_packable {
        ($name: ident, $check_expr: expr) => {
            use crate::core::biff::biff_data::*;

            #[derive(Default, PartialEq)]
            #[cfg_attr(feature = "test", derive(Debug))]
            pub(crate) struct $name;

            impl_packable_for!($name, $check_expr);

            impl $name {
                pub(crate) fn new() -> Self {
                    Self {}
                }
            }
        };
        ($name: ident, $check_expr: expr, $id: ident) => {
            use crate::core::biff::biff_data::*;

            #[derive(Default, PartialEq)]
            #[cfg_attr(feature = "test", derive(Debug))]
            pub(crate) struct $name;

            impl_packable_for!($name, $check_expr, $id);

            impl $name {
                pub(crate) fn new() -> Self {
                    Self {}
                }
            }
        };
        ($name: ident, $check_expr: expr, $field: ident, $type: ty) => {
            use crate::core::biff::biff_data::*;

            #[derive(Default, PartialEq)]
            #[cfg_attr(feature = "test", derive(Debug))]
            pub(crate) struct $name {
                pub(crate) $field: $type
            }

            impl_packable_for!($name, $check_expr, $field, $type);

            impl $name {
                pub(crate) fn new($field: $type) -> Self {
                    Self {
                        $field,
                    }
                }
            }

            impl From<$type> for $name {
                fn from(value: $type) -> Self {
                    Self {
                        $field: value,
                    }
                }
            }
        };
        ($name: ident, $check_expr: expr, $id: ident, $field: ident, $type: ty) => {
            use crate::core::biff::biff_data::*;

            #[derive(Default, PartialEq)]
            #[cfg_attr(feature = "test", derive(Debug))]
            pub(crate) struct $name {
                pub(crate) $field: $type
            }

            impl_packable_for!($name, $check_expr, $id, $field, $type);

            impl $name {
                pub(crate) fn new($field: $type) -> Self {
                    Self {
                        $field,
                    }
                }
            }

            impl From<$type> for $name {
                fn from(value: $type) -> Self {
                    Self {
                        $field: value,
                    }
                }
            }
        };
        ($name: ident, $check_expr: expr, $field: ident, $type: ty $(, $other: ident, $other_type: ty)+) => {
            pub(crate) use crate::core::biff::biff_data::*;

            #[derive(Default, PartialEq)]
            #[cfg_attr(feature = "test", derive(Debug))]
            pub(crate) struct $name {
                pub(crate) $field: $type,
                $(pub(crate) $other: $other_type,)+
            }

            impl_packable_for!($name, $check_expr, $field, $type $(, $other, $other_type)+);

            impl $name {
                pub(crate) fn new($field: $type $(,$other: $other_type)+) -> Self {
                    Self {
                        $field,
                        $( $other, )+
                    }
                }
            }
        };
        ($name: ident, $check_expr: expr, $id: ident, $field: ident, $type: ty $(, $other: ident, $other_type: ty)+) => {
            pub(crate) use crate::core::biff::biff_data::*;

            #[derive(Default, PartialEq)]
            #[cfg_attr(feature = "test", derive(Debug))]
            pub(crate) struct $name {
                pub(crate) $field: $type,
                $(pub(crate) $other: $other_type,)+
            }

            impl_packable_for!($name, $check_expr, $id, $field, $type $(, $other, $other_type)+);

            impl $name {
                pub(crate) fn new($field: $type $(,$other: $other_type)+) -> Self {
                    Self {
                        $field,
                        $( $other, )+
                    }
                }
            }
        };
    }

    pub(crate) use aligned_biff_data_impl;
    pub(crate) use declare_packable;
    pub(crate) use impl_biff_data_compatible;
    pub(crate) use impl_packable_for;
    pub(crate) use internal_impl_packable_for;
    pub(crate) use pack_biff_data;
}

pub(crate) use self::macros::{
    aligned_biff_data_impl, declare_packable, impl_biff_data_compatible, impl_packable_for,
    internal_impl_packable_for, pack_biff_data,
};
pub(crate) use super::biff_id::BiffId;
pub(crate) use crate::core::biff::biff_record::{box_alloc, try_to_sized, BiffError, BiffRecord};

pub(crate) trait CheckBiff: FromBiffData + IntoBiffData {
    fn validated<'a>(&'a self) -> std::io::Result<&'a Self> {
        Err(std::io::Error::from(std::io::ErrorKind::InvalidData))
    }

    fn is_valid(&self) -> bool {
        self.validated().is_ok()
    }
}

pub(crate) trait ValidBiff: FromBiffData + IntoBiffData {}

impl<T: ValidBiff> CheckBiff for T {
    fn validated<'a>(&'a self) -> std::io::Result<&'a Self> {
        Ok(self)
    }
}

pub(crate) trait BiffSerializable: CheckBiff {
    const ID: super::BiffId;

    fn into_biff(&self) -> std::io::Result<super::BiffRecord> {
        match self.validated() {
            Ok(me) => Ok(super::BiffRecord {
                id: Self::ID,
                data: pack_biff_data!(me)?,
            }),
            Err(e) => Err(e),
        }
    }

    fn from_biff(data: &super::BiffRecord) -> std::io::Result<Self> {
        if Self::ID == data.id {
            FromBiffData::deserialize(&data.data)
        } else {
            Err(std::io::Error::from(std::io::ErrorKind::InvalidData))
        }
    }
}

impl<T: AlignedBiffData + FromBiffData + IntoBiffData> ValidBiff for T {}

#[inline]
pub(crate) const fn checked<T: BiffDataCompatible>(_: &T) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use std::assert_matches::{self, assert_matches};

    use arrow::datatypes::ToByteSlice;

    use super::{declare_packable, pack_biff_data};

    #[inline]
    const fn bypass<T: BiffDataCompatible>(_: &T) -> bool {
        true
    }

    declare_packable!(SomeNoIdNoData, bypass);
    declare_packable!(SomeNoIdOneField, bypass, inner, u32);
    declare_packable!(SomeNoIdManyField, bypass, f1, u32, f2, u32);

    declare_packable!(SomeWithIdNoData, bypass, BrtACBegin);
    declare_packable!(SomeWithIdOneField, bypass, BrtACBegin, inner, u32);
    declare_packable!(SomeWithIdManyField, checked, BrtACBegin, f1, u32, f2, u32);

    declare_packable!(
        UncheckedRw,
        |s: &Self| { s.inner >= 0 && s.inner <= 1048576 },
        inner,
        i32
    );

    declare_packable!(
        SomeWithIdCheckedField,
        bypass,
        BrtACBegin,
        inner,
        UncheckedRw
    );

    #[test]
    fn test_aligned_biff_data_impl() {
        let _ = SomeNoIdNoData::new();
        let a = SomeNoIdOneField::from(0);
        let b = SomeNoIdOneField::new(0);
        let c = SomeNoIdOneField { inner: 0 };
        let d = SomeNoIdOneField::default();

        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(a, d);

        let a = SomeNoIdManyField::new(0, 0);
        let b = SomeNoIdManyField { f1: 0, f2: 0 };
        let c = SomeNoIdManyField::default();

        assert_eq!(a, b);
        assert_eq!(a, c);

        let some_data = pack_biff_data!(&0u32, &1u16);
        assert_matches!(some_data, Ok(_));

        let a = SomeWithIdNoData::new();
        let r = a.into_biff();
        assert!(r.is_ok());

        let x = UncheckedRw::new(0);
        assert!(x.is_valid());
        let x = UncheckedRw::new(-1);
        assert!(!x.is_valid());

        let a = SomeWithIdCheckedField::new(UncheckedRw::new(0));
        assert!(a.is_valid());

        let a = SomeWithIdCheckedField::new(UncheckedRw::new(-1));
        assert!(!a.is_valid());
    }
}
