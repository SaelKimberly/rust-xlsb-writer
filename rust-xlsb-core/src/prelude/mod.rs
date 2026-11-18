#![allow(non_snake_case, clippy::upper_case_acronyms)]

pub mod collection;
pub mod record;
pub mod scope;
pub mod structure;

use super::{Checked, Unchecked};

pub use collection::BrtBeginColInfos;
pub use record::{BrtColor, BrtWsProp, BrtXF};
pub use structure::{
    ACProductVersion, Blxf, Bold, BorderType, CodeName, FontFlags, SimpleRichStr,
    XlNullableWideString, XlWideString,
};

pub use scope::{
    BrtBeginSheet, BrtBeginSheetData, BrtBeginSingleCells, BrtBeginSlicerCacheIDs,
    BrtBeginSlicerCacheSiRanges, BrtBeginSlicerCachesPivotCacheIDs, BrtBeginSlicerStyleElements,
    BrtBeginSlicers, BrtBeginSlicersEx, BrtEndSheet, BrtEndSheetData, BrtEndSingleCells,
    BrtEndSlicerCacheIDs, BrtEndSlicerCacheSiRanges, BrtEndSlicerCachesPivotCacheIDs,
    BrtEndSlicerStyleElements, BrtEndSlicers, BrtEndSlicersEx,
};

#[cfg(test)]
mod tests {
    use crate::Checked;
    use deku::{DekuContainerRead, DekuContainerWrite};
    use rstest::rstest;
    use std::hash::{Hash, Hasher};

    use super::collection::BrtBeginColInfos;
    use super::record::{BrtColor, BrtWsProp, BrtXF};
    use super::scope::{
        BrtBeginSheet, BrtBeginSheetData, BrtBeginSingleCells, BrtBeginSlicerCacheIDs,
        BrtBeginSlicerCacheSiRanges, BrtBeginSlicerCachesPivotCacheIDs,
        BrtBeginSlicerStyleElements, BrtBeginSlicers, BrtBeginSlicersEx, BrtEndSheet,
        BrtEndSheetData, BrtEndSingleCells, BrtEndSlicerCacheIDs, BrtEndSlicerCacheSiRanges,
        BrtEndSlicerCachesPivotCacheIDs, BrtEndSlicerStyleElements, BrtEndSlicers, BrtEndSlicersEx,
    };
    use super::structure::{
        ACProductVersion, Blxf, Bold, BorderType, CodeName, FontFlags, SimpleRichStr,
        XlNullableWideString, XlWideString, Xnum,
    };

    fn checked_test<T>(size: usize)
    where
        T: for<'a> DekuContainerRead<'a>
            + Default
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
        println!("{b:?}");
        assert_eq!(b.len(), size);
        let (_, p) = T::from_bytes((b.as_slice(), 0)).expect("Cannot deserialize");
        s.check().expect("Default must be valid");
        p.check().expect("Parsed must be valid");
        assert_eq!(s, p);

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
        assert_eq!(format!("{s:?}"), format!("{p:?}"));
    }

    #[rstest]
    #[case::ac_product_version(|| checked_test::<ACProductVersion>(4))]
    #[case::bold(|| checked_test::<Bold>(2))]
    #[case::border_type(|| checked_test::<BorderType>(1))]
    #[case::blxf(|| checked_test::<Blxf>(10))]
    #[case::font_flags(|| checked_test::<FontFlags>(2))]
    #[case::xnum(|| checked_test::<Xnum>(8))]
    #[case::code_name(|| checked_test::<CodeName>(4))]
    #[case::xl_wide_string(|| checked_test::<XlWideString>(4))]
    #[case::xl_nullable_wide_string(|| checked_test::<XlNullableWideString>(4))]
    #[case::simple_rich_str(|| checked_test::<SimpleRichStr>(5))]
    #[case::brt_color(|| checked_test::<BrtColor>(8))]
    #[case::brt_xf(|| checked_test::<BrtXF>(16))]
    #[case::brt_ws_prop(|| checked_test::<BrtWsProp>(23))]
    #[case::brt_begin_col_infos(|| checked_test::<BrtBeginColInfos>(4))]
    #[case::brt_begin_sheet(|| checked_test::<BrtBeginSheet>(0))]
    #[case::brt_begin_sheet_data(|| checked_test::<BrtBeginSheetData>(0))]
    #[case::brt_begin_single_cells(|| checked_test::<BrtBeginSingleCells>(0))]
    #[case::brt_begin_slicer_cache_ids(|| checked_test::<BrtBeginSlicerCacheIDs>(0))]
    #[case::brt_begin_slicer_cache_si_ranges(|| checked_test::<BrtBeginSlicerCacheSiRanges>(0))]
    #[case::brt_begin_slicer_caches_pivot_cache_ids(
        || checked_test::<BrtBeginSlicerCachesPivotCacheIDs>(0)
    )]
    #[case::brt_begin_slicer_style_elements(|| checked_test::<BrtBeginSlicerStyleElements>(0))]
    #[case::brt_begin_slicers(|| checked_test::<BrtBeginSlicers>(0))]
    #[case::brt_begin_slicers_ex(|| checked_test::<BrtBeginSlicersEx>(0))]
    #[case::brt_end_sheet(|| checked_test::<BrtEndSheet>(0))]
    #[case::brt_end_sheet_data(|| checked_test::<BrtEndSheetData>(0))]
    #[case::brt_end_single_cells(|| checked_test::<BrtEndSingleCells>(0))]
    #[case::brt_end_slicer_cache_ids(|| checked_test::<BrtEndSlicerCacheIDs>(0))]
    #[case::brt_end_slicer_cache_si_ranges(|| checked_test::<BrtEndSlicerCacheSiRanges>(0))]
    #[case::brt_end_slicer_caches_pivot_cache_ids(
        || checked_test::<BrtEndSlicerCachesPivotCacheIDs>(0)
    )]
    #[case::brt_end_slicer_style_elements(|| checked_test::<BrtEndSlicerStyleElements>(0))]
    #[case::brt_end_slicers(|| checked_test::<BrtEndSlicers>(0))]
    #[case::brt_end_slicers_ex(|| checked_test::<BrtEndSlicersEx>(0))]
    fn test_biff_export(#[case] run: fn()) {
        run();
    }
}
