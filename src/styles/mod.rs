#![allow(non_snake_case)]
#![allow(unused_imports)]
mod brt_border;
mod brt_color;
mod brt_fill;
mod brt_fmt;
mod brt_font;
mod brt_xf;
mod style_holder;

use std::{
    collections::BTreeSet,
    io::{Read, Seek, Write},
};

pub(super) use crate::util::{Xnum, nullable_utf16_read, utf16_read, utf16_write};
use crate::util::{raw_bit_read, raw_bit_write};

use bitflags::bitflags;
pub(crate) use brt_border::{Blxf, BorderType, BrtBorder};
pub(crate) use brt_color::{BrtColor, BrtColorTheme, ColorType, Icv};
pub(crate) use brt_fill::{BrtFill, Fill, FillStyle, GradientStop};
pub(crate) use brt_fmt::{BrtFmt, BuiltinFmt};

use brt_font::FontBuilder;
pub(crate) use brt_font::{
    Boldness, BrtFont, CharSet, FontFamily, FontFlags, FontScheme, ScriptStyle, Underline,
};
pub(crate) use brt_xf::{
    BrtXF, CellStyleXF, CellXF, GrbitAtr, HAlignment, ReadingOrder, VAlignment,
};

use deku::{DekuError, DekuRead, DekuWrite, ctx::BitSize, reader::Reader, writer::Writer};
use zerocopy::transmute_ref;

#[derive(Default, Debug)]
pub(crate) struct StyleHolder {
    fmts: BTreeSet<BrtFmt>,
    fonts: BTreeSet<BrtFont>,
    fills: BTreeSet<BrtFill>,
    borders: BTreeSet<BrtBorder>,
    xfs: BTreeSet<BrtXF>,
}

pub fn get_idx_of<T: std::hash::Hash + Ord + Clone>(s: &mut BTreeSet<T>, v: &T) -> usize {
    if s.insert(v.clone()) {
        s.len() - 1
    } else {
        s.iter().position(|f| *f == *v).unwrap()
    }
}

pub enum StyleOption {
    Format(BrtFmt),
    Font(BrtFont),
}

impl StyleHolder {
    pub fn style_from_opts(opts: &[StyleOption]) -> Self {
        let mut holder = Self::default();
        let mut brt_xf = BrtXF {
            ixfeParent: 0xffff,
            ..Default::default()
        };

        let is_fmt_set: bool = false;
        let is_font_set: bool = false;

        for opt in opts {
            match opt {
                StyleOption::Format(fmt) => {
                    if is_fmt_set {
                        panic!("Format already set");
                    }
                    brt_xf.iFmt = get_idx_of(&mut holder.fmts, fmt) as u16;
                }
                StyleOption::Font(font) => {
                    if is_font_set {
                        panic!("Font already set");
                    }
                    brt_xf.iFont = get_idx_of(&mut holder.fonts, font) as u16;
                }
            };
        }

        holder.xfs.insert(brt_xf);

        holder
    }
}
