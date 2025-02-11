use std::io::{Read, Seek, Write};

use deku::{DekuError, DekuRead, DekuWrite, ctx::BitSize, reader::Reader, writer::Writer};

use crate::util::{raw_bit_read, raw_bit_write};
use bitflags::bitflags;

#[derive(DekuRead, DekuWrite, Hash, Default, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
#[deku(id_type = "u8", bits = 3)]
#[repr(u8)]
pub(crate) enum HAlignment {
    #[default]
    #[deku(id = 0x00, default)]
    General = 0x00,
    #[deku(id = 0x01)]
    Left = 0x01,
    #[deku(id = 0x02)]
    Center = 0x02,
    #[deku(id = 0x03)]
    Right = 0x03,
    #[deku(id = 0x04)]
    Fill = 0x04,
    #[deku(id = 0x05)]
    Justify = 0x05,
    #[deku(id = 0x06)]
    CenterAcrossSelection = 0x06,
    #[deku(id = 0x07)]
    Distribute = 0x07,
}

#[derive(DekuRead, DekuWrite, Hash, Default, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
#[deku(id_type = "u8", bits = 3)]
#[repr(u8)]
pub(crate) enum VAlignment {
    #[deku(id = 0x00)]
    Top = 0x00,
    #[default]
    #[deku(id = 0x01, default)]
    Center = 0x01,
    #[deku(id = 0x02)]
    Bottom = 0x02,
    #[deku(id = 0x03)]
    Justify = 0x03,
    #[deku(id = 0x04)]
    Distributed = 0x04,
}

#[derive(DekuRead, DekuWrite, Hash, Default, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
#[deku(id_type = "u8", bits = 2)]
#[repr(u8)]
pub(crate) enum ReadingOrder {
    #[deku(id = 0x00)]
    ContextDependent = 0x00,
    #[default]
    #[deku(id = 0x01, default)]
    LeftToRight = 0x01,
    #[deku(id = 0x02)]
    RightToLeft = 0x02,
}

bitflags! {
    #[derive(Hash, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Clone)]
    pub(crate) struct CellStyleXF: u8 {
        const Fmt = 0x01;
    }

    #[derive(Hash, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Clone)]
    pub(crate) struct CellXF: u8 {
        const Fmt = 0x01;
    }
}

#[derive(Hash, Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub(crate) enum GrbitAtr {
    CellStyle(CellStyleXF),
    Cell(CellXF),
}

impl Default for GrbitAtr {
    fn default() -> Self {
        Self::CellStyle(CellStyleXF::default())
    }
}

impl GrbitAtr {
    pub const fn inner(&self) -> u8 {
        match self {
            GrbitAtr::CellStyle(cell_style) => cell_style.bits(),
            GrbitAtr::Cell(cell) => cell.bits(),
        }
    }
    pub fn read<R: Read + Seek>(ixfe: u16, reader: &mut Reader<R>) -> Result<Self, DekuError> {
        if ixfe == 0xffff {
            Ok(Self::CellStyle(raw_bit_read(reader, Some(BitSize(6)))?))
        } else {
            Ok(Self::Cell(raw_bit_read(reader, Some(BitSize(6)))?))
        }
    }

    pub fn write<W: Write + Seek>(&self, writer: &mut Writer<W>) -> Result<(), DekuError> {
        match self {
            GrbitAtr::CellStyle(cell_style) => raw_bit_write(cell_style, writer, Some(BitSize(6)))?,
            GrbitAtr::Cell(cell) => raw_bit_write(cell, writer, Some(BitSize(6)))?,
        }
        Ok(())
    }
}

#[derive(DekuRead, DekuWrite, Hash, Debug, Default, PartialEq, PartialOrd, Eq, Ord, Clone)]
/// 2.4.862
pub(crate) struct BrtXF {
    pub(super) ixfeParent: u16,
    pub(super) iFmt: u16,
    pub(super) iFont: u16,
    pub(super) iFill: u16,
    pub(super) ixBorder: u16,
    pub(super) trot: u8,
    pub(super) indent: u8,
    pub(super) alc: HAlignment,
    pub(super) alcv: VAlignment,
    #[deku(bits = 1)]
    pub(super) fWrap: bool,
    #[deku(bits = 1)]
    pub(super) fJustLast: bool,
    #[deku(bits = 1)]
    pub(super) fShrinkToFit: bool,
    #[deku(bits = 1)]
    pub(super) fMergeCell: bool,
    pub(super) iReadingOrder: ReadingOrder,
    #[deku(bits = 1)]
    pub(super) fLocked: bool,
    #[deku(bits = 1)]
    pub(super) fHidden: bool,
    #[deku(bits = 1)]
    pub(super) fSxButton: bool,
    #[deku(bits = 1)]
    pub(super) f123Prefix: bool,
    #[deku(
        reader = "GrbitAtr::read(*ixfeParent, deku::reader)",
        writer = "GrbitAtr::write(xfGrbitAtr, deku::writer)",
        pad_bits_after = "10"
    )]
    pub(super) xfGrbitAtr: GrbitAtr,
    // padding
}

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::{CellStyleXF, GrbitAtr};

    use super::BrtXF;

    #[test]
    #[allow(clippy::unusual_byte_groupings)]
    fn brt_xf() {
        let buf: &[u8] = &[
            // ixfeParent: 2
            0xff,
            0xff,
            // iFmt: 2
            0x00,
            0x00,
            // iFont: 2
            0x00,
            0x00,
            // iFill: 2
            0x00,
            0x00,
            // ixBorder: 2
            0x00,
            0x00,
            // trot: 1
            0x00,
            // indent: 1
            0x00,
            // alc: 3, alcv: 3, fWrap: 1, fJustLast: 1
            0b_000_000_0_0,
            // fShrinkToFit: 1, fMergeCell: 1, iReadingOrder: 2, fLocked: 1, fHidden: 1, fSxButton: 1, f123Prefix: 1
            0b_0_0_00_0_0_0_0,
            //   grbitAtr: 6
            // v---------v
            0b_1_1_1_1_1_1_00,
            // padding v---^^
            0b_0000_0000,
        ];
        assert_eq!(buf.len(), 16);

        let brt_xf = BrtXF {
            ixfeParent: 0x0000,
            xfGrbitAtr: GrbitAtr::CellStyle(CellStyleXF::Fmt),
            ..Default::default()
        };
        println!("{:?}", brt_xf.to_bytes());

        println!("{:?}", BrtXF::from_bytes((buf, 0)));

        // let ((_, _), cxf) = BrtXF::from_bytes((buf, buf.len())).expect("Cannot read BrtXF");
        // println!("{:?}", cxf);
    }
}
