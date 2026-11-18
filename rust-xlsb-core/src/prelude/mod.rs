#![allow(non_snake_case, clippy::upper_case_acronyms)]

pub mod collection;
pub mod record;
pub mod scope;
pub mod structure;

use super::{Checked, Unchecked};

pub use collection::*;
pub use record::{
    BrtBeginSst, BrtBeginTableStyles, BrtBeginWsView, BrtBookView, BrtBorder, BrtBundleSh,
    BrtCellBlank, BrtCellBool, BrtCellError, BrtCellIsst, BrtCellReal, BrtCellRk, BrtCellSt,
    BrtColInfo, BrtColor, BrtEndSst, BrtEndTableStyles, BrtEndWsView, BrtFill, BrtFmt, BrtFont,
    BrtMargins, BrtMergeCell, BrtPageSetup, BrtPane, BrtPrintOptions, BrtRowHdr, BrtSSTItem,
    BrtSel, BrtStyle, BrtWsDim, BrtWsFmtInfo, BrtWsProp, BrtXF, GrbitAtr, HAlign, ReadingOrder,
    VAlign,
};
pub use scope::*;
pub use structure::{
    ACProductVersion, BErr, Blxf, Bold, BorderType, BrtColSpan, Cell, CodeName, FontFlags,
    RkNumber, SimpleRichStr, UncheckedRfX, UncheckedSqRfX, XlNullableWideString, XlStringExt,
    XlWideString, Xnum,
};

#[cfg(test)]
mod tests {
    use crate::Checked;
    use deku::{DekuContainerRead, DekuContainerWrite};
    use rstest::rstest;
    use std::hash::{Hash, Hasher};

    use super::collection::*;
    use super::record::*;
    use super::scope::*;
    use super::structure::*;

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
    #[case::brt_ws_fmt_info(|| checked_test::<BrtWsFmtInfo>(12))]
    #[case::brt_ws_dim(|| checked_test::<BrtWsDim>(16))]
    #[case::brt_row_hdr(|| checked_test::<BrtRowHdr>(17))]
    #[case::brt_col_info(|| checked_test::<BrtColInfo>(18))]
    #[case::brt_cell_st(|| checked_test::<BrtCellSt>(12))]
    #[case::brt_cell_real(|| checked_test::<BrtCellReal>(16))]
    #[case::brt_cell_rk(|| checked_test::<BrtCellRk>(12))]
    #[case::brt_cell_bool(|| checked_test::<BrtCellBool>(9))]
    #[case::brt_cell_error(|| checked_test::<BrtCellError>(9))]
    #[case::brt_cell_blank(|| checked_test::<BrtCellBlank>(8))]
    #[case::brt_fmt(|| checked_test::<BrtFmt>(6))]
    #[case::brt_begin_fmts(|| checked_test::<BrtBeginFmts>(4))]
    #[case::brt_end_fmts(|| checked_test::<BrtEndFmts>(0))]
    #[case::brt_cell_isst(|| checked_test::<BrtCellIsst>(12))]
    #[case::brt_sst_item(|| checked_test::<BrtSSTItem>(5))]
    #[case::brt_begin_sst(|| checked_test::<BrtBeginSst>(8))]
    #[case::brt_end_sst(|| checked_test::<BrtEndSst>(0))]
    #[case::brt_bundle_sh(|| checked_test::<BrtBundleSh>(16))]
    #[case::brt_book_view(|| checked_test::<BrtBookView>(29))]
    #[case::brt_begin_ws_view(|| checked_test::<BrtBeginWsView>(30))]
    #[case::brt_end_ws_view(|| checked_test::<BrtEndWsView>(0))]
    #[case::brt_pane(|| checked_test::<BrtPane>(29))]
    #[case::brt_sel(|| checked_test::<BrtSel>(20))]
    #[case::brt_merge_cell(|| checked_test::<BrtMergeCell>(16))]
    #[case::brt_margins(|| checked_test::<BrtMargins>(48))]
    #[case::brt_page_setup(|| checked_test::<BrtPageSetup>(38))]
    #[case::brt_print_options(|| checked_test::<BrtPrintOptions>(2))]
    #[case::brt_begin_merge_cells(|| checked_test::<BrtBeginMergeCells>(4))]
    #[case::brt_end_merge_cells(|| checked_test::<BrtEndMergeCells>(0))]
    #[case::brt_font(|| checked_test::<BrtFont>(39))]
    #[case::brt_fill(|| checked_test::<BrtFill>(68))]
    #[case::brt_border(|| checked_test::<BrtBorder>(51))]
    #[case::brt_style(|| checked_test::<BrtStyle>(24))]
    #[case::brt_begin_table_styles(|| checked_test::<BrtBeginTableStyles>(12))]
    #[case::brt_begin_book(|| checked_test::<BrtBeginBook>(0))]
    #[case::brt_end_book(|| checked_test::<BrtEndBook>(0))]
    #[case::brt_begin_book_views(|| checked_test::<BrtBeginBookViews>(0))]
    #[case::brt_end_book_views(|| checked_test::<BrtEndBookViews>(0))]
    #[case::brt_begin_bundle_shs(|| checked_test::<BrtBeginBundleShs>(0))]
    #[case::brt_end_bundle_shs(|| checked_test::<BrtEndBundleShs>(0))]
    #[case::brt_begin_style_sheet(|| checked_test::<BrtBeginStyleSheet>(0))]
    #[case::brt_end_style_sheet(|| checked_test::<BrtEndStyleSheet>(0))]
    #[case::brt_begin_borders(|| checked_test::<BrtBeginBorders>(4))]
    #[case::brt_end_borders(|| checked_test::<BrtEndBorders>(0))]
    #[case::brt_begin_cell_style_xfs(|| checked_test::<BrtBeginCellStyleXFs>(4))]
    #[case::brt_end_cell_style_xfs(|| checked_test::<BrtEndCellStyleXFs>(0))]
    #[case::brt_begin_cell_xfs(|| checked_test::<BrtBeginCellXFs>(4))]
    #[case::brt_end_cell_xfs(|| checked_test::<BrtEndCellXFs>(0))]
    #[case::brt_begin_col_infos(|| checked_test::<BrtBeginColInfos>(4))]
    #[case::brt_begin_dxfs(|| checked_test::<BrtBeginDXFs>(4))]
    #[case::brt_end_dxfs(|| checked_test::<BrtEndDXFs>(0))]
    #[case::brt_begin_fills(|| checked_test::<BrtBeginFills>(4))]
    #[case::brt_end_fills(|| checked_test::<BrtEndFills>(0))]
    #[case::brt_begin_fonts(|| checked_test::<BrtBeginFonts>(4))]
    #[case::brt_end_fonts(|| checked_test::<BrtEndFonts>(0))]
    #[case::brt_begin_styles(|| checked_test::<BrtBeginStyles>(4))]
    #[case::brt_end_styles(|| checked_test::<BrtEndStyles>(0))]
    #[case::brt_begin_sheet(|| checked_test::<BrtBeginSheet>(0))]
    #[case::brt_begin_ws_views(|| checked_test::<BrtBeginWsViews>(0))]
    #[case::brt_end_ws_views(|| checked_test::<BrtEndWsViews>(0))]
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
