#![allow(unused_imports)]
#![allow(non_snake_case)]
pub(crate) use crate::styles::{BrtBorder, BrtFill, BrtFmt, BrtFont, BrtXF, Icv};
use crate::util::{utf16_read, utf16_write};
use crate::{arr::BrtColSpan, styles::BrtColor, util::Xnum};
use chrono::{Days, NaiveDate, NaiveDateTime, NaiveTime};
use deku::{DekuRead, DekuWrite};
use std::{fmt::Debug, mem::transmute};

macro_rules! declare_markers {
    ($id: ident$(, $o_id: ident)*) => {
        #[derive(deku::DekuRead, deku::DekuWrite, std::fmt::Debug)]
        pub(crate) struct $id {}

        $(
            #[derive(deku::DekuRead, deku::DekuWrite, std::fmt::Debug)]
            pub(crate) struct $o_id {}
        )*
    };
}

macro_rules! declare_collection_headers {
    ($beg: ident, $end: ident$(,$o_beg: ident, $o_end: ident)*) => {
        #[derive(deku::DekuRead, deku::DekuWrite, std::fmt::Debug)]
        pub(crate) struct $beg {
            pub(crate) size: u32,
        }

        #[derive(deku::DekuRead, deku::DekuWrite, std::fmt::Debug)]
        pub(crate) struct $end {}

        $(
            #[derive(deku::DekuRead, deku::DekuWrite, std::fmt::Debug)]
            pub(crate) struct $o_beg {
                pub(crate) size: u32,
            }

            #[derive(deku::DekuRead, deku::DekuWrite, std::fmt::Debug)]
            pub(crate) struct $o_end {}
        )*
    };
}

declare_collection_headers!(
    BrtBeginFonts,
    BrtEndFonts,
    BrtBeginFills,
    BrtEndFills,
    BrtBeginBorders,
    BrtEndBorders,
    BrtBeginCellStyleXFs,
    BrtEndCellStyleXFs,
    BrtBeginCellXFs,
    BrtEndCellXFs,
    BrtBeginStyles,
    BrtEndStyles,
    BrtBeginDXFs,
    BrtEndDXFs
);

declare_markers!(
    BrtBeginSheet,
    BrtBeginWsViews,
    BrtEndWsView,
    BrtEndWsViews,
    BrtACEnd,
    BrtBeginColInfos,
    BrtEndColInfos,
    BrtBeginSheetData,
    BrtEndSheetData,
    BrtEndSheet
);

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct Cell {
    column: u32,
    #[deku(bits = 24)]
    iStyleRef: u32,
    #[deku(bits = 1)]
    fPhShow: bool,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct BrtCellIsst {
    cell: Cell,
    isst: u32,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct BrtCellReal {
    cell: Cell,
    xnum: Xnum,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct RkNumber {
    #[deku(bits = 1)]
    fx100: bool,
    #[deku(bits = 1)]
    fInt: bool,
    #[deku(bits = 30)]
    num: u32,
}

#[derive(DekuRead, DekuWrite)]
pub(crate) struct BrtCellRk {
    cell: Cell,
    value: RkNumber,
}

impl Debug for BrtCellRk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrtCellRk")
            .field("cell", &self.cell)
            .field_with("value", |f| {
                if self.value.fInt {
                    f.debug_tuple("Int").field(&self.value.num).finish()
                } else {
                    f.debug_tuple("Float")
                        .field(&self.value.as_float64())
                        .finish()
                }
            })
            .finish()
    }
}

impl RkNumber {
    pub const fn as_date(&self) -> NaiveDate {
        NaiveDate::from_ymd_opt(1899, 12, 31)
            .unwrap()
            .checked_add_days(Days::new(self.as_int32() as u64))
            .unwrap()
    }

    pub const fn as_datetime(&self) -> NaiveDateTime {
        NaiveDateTime::new(self.as_date(), NaiveTime::MIN)
    }

    pub const fn as_float64(&self) -> f64 {
        let ret = if self.fInt {
            self.num as f64
        } else {
            f64::from_bits((self.num as u64) << 34)
        };
        if self.fx100 { ret / 100.0 } else { ret }
    }

    pub const fn as_int32(&self) -> i32 {
        unimplemented!()
    }
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct BrtColInfo {
    colFirst: u32,
    colLast: u32,
    coldx: u32,
    ixfe: u32,
    #[deku(bits = 1)]
    fHidden: bool,
    #[deku(bits = 1)]
    fUserSet: bool,
    #[deku(bits = 1)]
    fBestFit: bool,
    #[deku(bits = 1, pad_bits_after = "1")]
    fPhonetic: bool,
    #[deku(bits = 3, pad_bits_after = "1")]
    iOutLevel: u8,
    #[deku(bits = 1, pad_bits_after = "3")]
    fCollapsed: bool,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct BrtRowHdr {
    rw: u32,
    ixfe: u32,
    miyRw: u16,
    #[deku(bits = 1)]
    fExtraAsc: bool,
    #[deku(bits = 1, pad_bits_after = "6")]
    fExtraDsc: bool,
    #[deku(bits = 3)]
    iOutLevel: u8,
    #[deku(bits = 1)]
    fCollapsed: bool,
    #[deku(bits = 1)]
    fDyZero: bool,
    #[deku(bits = 1)]
    fUnsynced: bool,
    #[deku(bits = 1, pad_bits_after = "1")]
    fGhostDirty: bool,
    #[deku(bits = 1, pad_bits_after = "7")]
    fPhShow: bool,
    #[deku(update = "self.rgBrtColSpan.len() as u32")]
    ccolspan: u32,
    #[deku(count = "ccolspan")]
    rgBrtColSpan: Vec<BrtColSpan>,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct BrtRwDescent {
    dyDescent: u16,
}

#[derive(DekuRead, DekuWrite, Debug)]
/// `2.5.1`
pub(crate) struct ACProductVersion {
    fileVersion: u16,
    #[deku(bits = 15)]
    fileProduct: u16,
    #[deku(bits = 1)]
    fileExtension: bool,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct BrtACBegin {
    #[deku(update = "self.RgACVer.len()")]
    cver: u16,
    #[deku(count = "cver")]
    RgACVer: Vec<ACProductVersion>,
}

#[derive(DekuRead, DekuWrite, Debug)]
/// `2.4.861`
pub(crate) struct BrtWsProp {
    #[deku(bits = 1, pad_bits_after = "2")]
    fShowAutoBreaks: bool,
    #[deku(bits = 1)]
    fPublish: bool,
    #[deku(bits = 1)]
    fDialog: bool,
    #[deku(bits = 1)]
    fApplyStyles: bool,
    #[deku(bits = 1)]
    fRowSumsBelow: bool,
    #[deku(bits = 1)]
    fColSumsRight: bool,
    #[deku(bits = 1, pad_bits_after = "1")]
    fFitToPage: bool,
    #[deku(bits = 1, pad_bits_after = "1")]
    fShowOutlineSymbols: bool,
    #[deku(bits = 1)]
    fSyncHoriz: bool,
    #[deku(bits = 1)]
    fSyncVert: bool,
    #[deku(bits = 1)]
    fAltExprEval: bool,
    #[deku(bits = 1)]
    fAltFormulaEntry: bool,
    #[deku(bits = 1)]
    fFilterMode: bool,
    #[deku(bits = 1, pad_bits_after = "6")]
    fCondFmtCalc: bool,
    brtcolorTab: BrtColor,
    rwSync: u32,
    colSync: u32,

    #[deku(
        reader = "utf16_read(deku::reader)",
        writer = "utf16_write(strName, deku::writer)"
    )]
    strName: String,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct UncheckedRfX {
    rwFirst: u32,
    rwLast: u32,
    colFirst: u32,
    colLast: u32,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct BrtWsDim {
    rfx: UncheckedRfX,
}

#[derive(DekuRead, DekuWrite, Debug)]
/// `2.4.859`
pub(crate) struct BrtWsFmtInfo {
    dxGCol: u32,
    cchDefColWidth: u16,
    miyDefRwHeight: u16,
    #[deku(bits = 1)]
    fUnsynced: bool,
    #[deku(bits = 1)]
    fDyZero: bool,
    #[deku(bits = 1)]
    fExAsc: bool,
    #[deku(bits = 1, pad_bits_after = "12")]
    fExDesc: bool,

    iOutLevelRw: u8,
    iOutLevelCol: u8,
}

#[derive(DekuRead, DekuWrite, Debug)]
/// `2.4.860`
pub(crate) struct BrtWsFmtInfoEx14 {
    dyDescent: u16,
}

#[derive(DekuRead, DekuWrite, Debug)]
#[repr(u32)]
#[deku(id_type = "u32")]
pub(crate) enum Pnn {
    #[deku(id = 0x0000_0000)]
    BotRight = 0x0000_0000,
    #[deku(id = 0x0000_0001)]
    TopRight = 0x0000_0001,
    #[deku(id = 0x0000_0002)]
    TopLeft = 0x0000_0002,
    #[deku(id = 0x0000_0003)]
    BotLeft = 0x0000_0003,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct UncheckedSqRfX {
    #[deku(update = "self.rgrfx.len() as u32")]
    crfx: u32,
    #[deku(count = "crfx")]
    rgrfx: Vec<UncheckedRfX>,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct BrtSel {
    pnn: Pnn,
    rwAct: u32,
    colAct: u32,
    dwRfxAct: u32,
    sqrfx: UncheckedSqRfX,
}

#[derive(DekuRead, DekuWrite, Debug)]
#[deku(id_type = "u32")]
#[repr(u32)]
pub(crate) enum XLView {
    #[deku(id = 0x0000_0000)]
    Normal = 0x0000_0000,
    #[deku(id = 0x0000_0001)]
    SheetLayoutView = 0x0000_0001,
    #[deku(id = 0x0000_0002)]
    PageLayoutView = 0x0000_0002,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct BrtBeginWsView {
    #[deku(bits = 1)]
    fWnProt: bool,
    #[deku(bits = 1)]
    fDspFmla: bool,
    #[deku(bits = 1)]
    fDspGrid: bool,
    #[deku(bits = 1)]
    fDspRwCol: bool,
    #[deku(bits = 1)]
    fDspZeros: bool,
    #[deku(bits = 1)]
    fRightToLeft: bool,
    #[deku(bits = 1)]
    fSelected: bool,
    #[deku(bits = 1)]
    fDspRuler: bool,
    #[deku(bits = 1)]
    fDspGuts: bool,
    #[deku(bits = 1)]
    fDefaultHdr: bool,
    #[deku(bits = 1, pad_bits_after = "5")]
    fWhitespaceHidden: bool,
    xlView: XLView,
    rwTop: u32,
    colLeft: u32,
    #[deku(pad_bytes_after = "3")]
    icvHdr: Icv,
    wScale: u16,
    wScaleNormal: u16,
    wScaleSLV: u16,
    wScalePLV: u16,
    iWbkView: u32,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct BrtPrintOptions {
    #[deku(bits = 1)]
    fHCenter: bool,
    #[deku(bits = 1)]
    fVCenter: bool,
    #[deku(bits = 1)]
    fPrintHeaders: bool,
    #[deku(bits = 1, pad_bits_after = "12")]
    fPrintGrid: bool,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct BrtMargins {
    xnumLeft: f64,
    xnumRight: f64,
    xnumTop: f64,
    xnumBottom: f64,
    xnumHeader: f64,
    xnumFooter: f64,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct FRTBlank {
    reserved: u32,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct FRTRef {
    #[deku(bits = 1)]
    fAdjDelete: bool,
    #[deku(bits = 1)]
    fDoAdjust: bool,
    #[deku(bits = 1)]
    fAdjChange: bool,
    #[deku(bits = 1, pad_bits_after = "28")]
    fEdit: bool,
    rfx: UncheckedRfX,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct FRTRefs {
    #[deku(update = "self.array.len() as u32")]
    cref: u32,
    #[deku(count = "cref")]
    array: Vec<FRTRef>,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct FRTSqref {
    #[deku(bits = 1)]
    fAdjDelete: bool,
    #[deku(bits = 1)]
    fDoAdjust: bool,
    #[deku(bits = 1)]
    fAdjChange: bool,
    #[deku(bits = 1, pad_bits_after = "28")]
    fEdit: bool,
    sqrfx: UncheckedSqRfX,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct FRTSqrefs {
    #[deku(update = "self.array.len() as u32")]
    csqref: u32,
    #[deku(count = "csqref")]
    array: Vec<FRTSqref>,
}
pub(crate) mod ptg {
    use deku::{DekuRead, DekuWrite};

    #[derive(DekuRead, DekuWrite, Debug)]
    #[repr(u8)]
    #[deku(id_type = "u8")]
    /// `2.5.98.36`
    pub(crate) enum PtgDataType {
        #[deku(id = 0x1)]
        Reference = 0x1,
        #[deku(id = 0x2)]
        Value = 0x2,
        #[deku(id = 0x3)]
        Array = 0x3,
    }

    #[derive(DekuRead, DekuWrite, Debug)]
    /// `2.5.98.17`
    pub(crate) struct PtgAdd {
        #[deku(bits = 7)]
        ptg: u8,
    }
}

pub(crate) enum Ptg {
    PtgAdd,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct FRTHeader {
    #[deku(bits = 1)]
    fRef: bool,
    #[deku(bits = 1)]
    fSqref: bool,
    #[deku(bits = 1)]
    fFormula: bool,
    #[deku(bits = 1, pad_bits_after = "28")]
    fRelID: bool,
    rgRefs: FRTRefs,
}

#[derive(DekuRead, DekuWrite, Debug)]
pub(crate) struct BrtPivotCacheAutoRefresh {
    FRTHeader: FRTBlank,
    fAutoRefresh: bool,
}
