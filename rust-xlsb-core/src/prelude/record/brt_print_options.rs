use deku::{DekuRead, DekuWrite};

use crate::{Unchecked, assign_id};

/// `2.4.762`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
#[deku(bit_order = "lsb")]
#[allow(
    clippy::struct_excessive_bools,
    reason = "bit layout mirrors the MS-XLSB record"
)]
pub struct BrtPrintOptions {
    #[deku(bits = 1)]
    pub fHCenter: bool,
    #[deku(bits = 1)]
    pub fVCenter: bool,
    #[deku(bits = 1)]
    pub fPrintHeaders: bool,
    #[deku(bits = 1)]
    pub fPrintGrid: bool,
    #[deku(bits = 1, pad_bits_after = "11")]
    pub unused: bool,
}

impl Unchecked for BrtPrintOptions {}

assign_id!(BrtPrintOptions);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;

    #[test]
    fn print_flags_land_in_the_first_bit_byte() {
        let options = BrtPrintOptions {
            fHCenter: true,
            fPrintGrid: true,
            ..BrtPrintOptions::default()
        };

        let bytes = options.to_bytes().expect("serializes");
        assert_eq!(bytes, [0x09, 0x00]);
    }
}
