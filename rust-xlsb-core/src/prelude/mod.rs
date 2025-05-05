#![allow(non_snake_case, clippy::upper_case_acronyms)]

pub mod collection;
pub mod record;
pub mod scope;
pub mod structure;

use super::{Checked, Unchecked};
// #[allow(dead_code)]
// pub(crate) mod util;
// #[allow(unused_imports)]
// pub(crate) use util::{
//     nullable_utf16_read, nullable_utf16_write, raw_bit_read, raw_bit_write, utf16_read, utf16_write,
// };

pub use collection::BrtBeginColInfos;
pub use record::BrtColor;
pub use structure::{ACProductVersion, Blxf, Bold, BorderType, FontFlags};

pub use scope::{
    BrtBeginSheet, BrtBeginSheetData, BrtBeginSingleCells, BrtBeginSlicerCacheIDs,
    BrtBeginSlicerCacheSiRanges, BrtBeginSlicerCachesPivotCacheIDs, BrtBeginSlicerStyleElements,
    BrtBeginSlicers, BrtBeginSlicersEx, BrtEndSheet, BrtEndSheetData, BrtEndSingleCells,
    BrtEndSlicerCacheIDs, BrtEndSlicerCacheSiRanges, BrtEndSlicerCachesPivotCacheIDs,
    BrtEndSlicerStyleElements, BrtEndSlicers, BrtEndSlicersEx,
};

#[cfg(test)]
mod tests {
    use super::check::Checked;
    use deku::{DekuContainerRead, DekuContainerWrite};
    use std::hash::{Hash, Hasher};

    fn checked_test<'a, T>(size: Option<usize>)
    where
        T: Default
            + DekuContainerRead<'a>
            + DekuContainerWrite
            + Hash
            + PartialEq
            + std::fmt::Debug
            + Checked
            + Send
            + Sync
            + 'static,
    {
        let s = T::default();
        let b = s.to_bytes().expect("Cannot serialize");
        let a: &'a [u8] = Box::leak(b.into_boxed_slice());
        let (_, p) = T::from_bytes((a, 0)).expect("Cannot deserialize");
        s.check().expect("Default must be valid");
        p.check().expect("Parsed must be valid");
        assert_eq!(s, p);

        if let Some(size) = size {
            assert_eq!(a.len(), size);
        }
        let sh = {
            let mut hasher = std::hash::DefaultHasher::new();
            s.hash(&mut hasher);
            hasher.finish()
        };
        let ph = {
            let mut hasher = std::hash::DefaultHasher::new();
            p.hash(&mut hasher);
            hasher.finish()
        };

        assert_eq!(sh, ph);
        assert_eq!(format!("{:?}", s), format!("{:?}", p));
    }

    macro_rules! test_biff_export {
        ($mod: ident, $struct: ident) => {
            concat_idents::concat_idents!(test_name = $mod, _, $struct {
            #[test]
            fn test_name() { checked_test::<super::$mod::$struct>(None); }
            });
        };

        ($mod: ident, $struct: ident = $size: literal) => {
            concat_idents::concat_idents!(test_name = $mod, _, $struct {
            #[test]
            fn test_name() { checked_test::<super::$mod::$struct>(Some($size)); }
            });
        };
    }

    macro_rules! test_scopes {
        ($struct: ident$(, $struct_other: ident)*) => {
            test_biff_export!(scope, $struct = 0);
            $(test_biff_export!(scope, $struct_other = 0);)*
        };
    }

    test_biff_export!(structure, ACProductVersion = 4);
    test_biff_export!(structure, Bold = 2);
    test_biff_export!(structure, BorderType = 1);
    test_biff_export!(structure, Blxf = 10);
    test_biff_export!(structure, FontFlags = 2);

    test_biff_export!(structure, Xnum = 8);

    test_biff_export!(record, BrtColor = 8);

    test_biff_export!(collection, BrtBeginColInfos = 4);

    test_scopes!(
        BrtBeginSheet,
        BrtBeginSheetData,
        BrtBeginSingleCells,
        BrtBeginSlicerCacheIDs,
        BrtBeginSlicerCacheSiRanges,
        BrtBeginSlicerCachesPivotCacheIDs,
        BrtBeginSlicerStyleElements,
        BrtBeginSlicers,
        BrtBeginSlicersEx,
        BrtEndSheet,
        BrtEndSheetData,
        BrtEndSingleCells,
        BrtEndSlicerCacheIDs,
        BrtEndSlicerCacheSiRanges,
        BrtEndSlicerCachesPivotCacheIDs,
        BrtEndSlicerStyleElements,
        BrtEndSlicers,
        BrtEndSlicersEx
    );
}
