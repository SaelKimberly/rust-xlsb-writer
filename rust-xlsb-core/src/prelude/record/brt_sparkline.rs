//! Sparkline group and per-sparkline future records.

use deku::{DekuRead, DekuWrite};

use crate::assign_id;
use crate::prelude::{BrtColor, Checked, FrtHeader, Unchecked, Xnum};

/// `2.4.228`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the sparkline flags are independent wire-format bits"
)]
pub struct BrtBeginSparklineGroup {
    pub frt: FrtHeader,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fDateAxis: bool,
    #[deku(bits = 2, bit_order = "lsb")]
    pub fShowEmptyCellAsZero: u8,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fMarkers: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fHigh: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fLow: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fFirst: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fLast: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fNegative: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fAxis: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fDisplayHidden: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fIndividualAutoMax: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fIndividualAutoMin: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fGroupAutoMax: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fGroupAutoMin: bool,
    #[deku(bits = 1, bit_order = "lsb")]
    pub fRTL: bool,
    pub brtcolorSeries: BrtColor,
    pub brtcolorNegative: BrtColor,
    pub brtcolorAxis: BrtColor,
    pub brtcolorMarkers: BrtColor,
    pub brtcolorFirst: BrtColor,
    pub brtcolorLast: BrtColor,
    pub brtcolorHigh: BrtColor,
    pub brtcolorLow: BrtColor,
    pub dManualMax: Xnum,
    pub dManualMin: Xnum,
    pub dLineWeight: Xnum,
    pub isltype: u32,
}

impl Checked for BrtBeginSparklineGroup {
    fn check(&self) -> Result<(), &'static str> {
        self.frt.check()?;
        if self.fShowEmptyCellAsZero > 2 {
            return Err("sparkline empty-cell mode is outside 0..=2");
        }
        if self.fIndividualAutoMax && self.fGroupAutoMax {
            return Err("sparkline individual and group max cannot both be automatic");
        }
        if self.fIndividualAutoMin && self.fGroupAutoMin {
            return Err("sparkline individual and group min cannot both be automatic");
        }
        let weight: f64 = self.dLineWeight.into();
        if !(0.0..=1584.0).contains(&weight) {
            return Err("sparkline line weight is outside 0..=1584");
        }
        if self.isltype > 2 {
            return Err("sparkline type is outside 0..=2");
        }
        for color in [
            &self.brtcolorSeries,
            &self.brtcolorNegative,
            &self.brtcolorAxis,
            &self.brtcolorMarkers,
            &self.brtcolorFirst,
            &self.brtcolorLast,
            &self.brtcolorHigh,
            &self.brtcolorLow,
        ] {
            color.check()?;
        }
        Ok(())
    }
}

assign_id!(BrtBeginSparklineGroup);

/// `2.4.806`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtSparkline {
    pub frt: FrtHeader,
}

impl Checked for BrtSparkline {
    fn check(&self) -> Result<(), &'static str> {
        self.frt.check()
    }
}

assign_id!(BrtSparkline);

/// `2.4.581`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndSparklineGroup {}

impl Unchecked for BrtEndSparklineGroup {}

assign_id!(BrtEndSparklineGroup);

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;
    use crate::prelude::{
        FrtFormulas, FrtHeader, FrtParsedFormula, FrtSqref, FrtSqrefs, UncheckedRfX,
        UncheckedSqRfX, Xnum,
    };

    #[test]
    fn sparkline_group_roundtrips_at_oracle_size() {
        let group = BrtBeginSparklineGroup {
            fShowEmptyCellAsZero: 1,
            fMarkers: true,
            fIndividualAutoMax: true,
            fIndividualAutoMin: true,
            dLineWeight: Xnum::from(0.75),
            ..BrtBeginSparklineGroup::default()
        };
        let bytes = group.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 98);
        assert_eq!(&bytes[4..6], &[0x0A, 0x18]);
        let (_, parsed) = BrtBeginSparklineGroup::from_bytes((&bytes, 0)).expect("decodes");
        assert_eq!(parsed, group);
        group.check().expect("valid");
    }

    #[test]
    fn frt_sqref_default_flags_use_lsb_order() {
        let sqref = FrtSqref::default();
        let bytes = sqref.to_bytes().expect("serializes");
        assert_eq!(&bytes[..4], &[0x02, 0, 0, 0]);
    }

    #[test]
    fn sparkline_formula_orders_cb_before_rgce() {
        let formula = FrtParsedFormula {
            cce: 15,
            cb: 0,
            rgce: vec![0x5B, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            rgcb: Vec::new(),
        };
        let formula_bytes = formula.to_bytes().expect("serializes");
        assert_eq!(formula_bytes[..4], [0x0F, 0, 0, 0]);
        assert_eq!(formula_bytes[4..8], [0, 0, 0, 0]);
        assert_eq!(formula_bytes[8], 0x5B);

        let sparkline = BrtSparkline {
            frt: FrtHeader {
                fSqref: true,
                fFormulas: true,
                rgSqrefs: Some(FrtSqrefs {
                    csqref: 1,
                    array: vec![FrtSqref {
                        sqrfx: UncheckedSqRfX {
                            crfx: 1,
                            rgRefx: vec![UncheckedRfX {
                                rwFirst: 0,
                                rwLast: 0,
                                colFirst: 5,
                                colLast: 5,
                            }],
                        },
                        ..FrtSqref::default()
                    }],
                }),
                rgFormulas: Some(FrtFormulas {
                    cformula: 1,
                    array: vec![crate::prelude::FrtFormula {
                        formula,
                        ..crate::prelude::FrtFormula::default()
                    }],
                }),
                ..FrtHeader::default()
            },
        };
        let bytes = sparkline.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 63);
        let (_, parsed) = BrtSparkline::from_bytes((&bytes, 0)).expect("decodes");
        assert_eq!(parsed, sparkline);
    }
}
