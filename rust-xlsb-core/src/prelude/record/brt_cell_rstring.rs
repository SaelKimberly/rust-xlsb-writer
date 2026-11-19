use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::assign_id;
use crate::prelude::{Cell, RichStr};

/// A cell containing a rich string; MS-XLSB `2.4.328`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtCellRString {
    pub cell: Cell,
    pub value: RichStr,
}

impl Checked for BrtCellRString {
    fn check(&self) -> Result<(), &'static str> {
        self.value.check()?;
        self.cell.check_range()
    }
}

assign_id!(BrtCellRString);

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use crate::Checked;
    use crate::prelude::{Cell, RichStr, StrRun, StrRuns, XlStringExt, XlWideString};

    use super::BrtCellRString;

    #[test]
    fn rich_cell_roundtrips_text_and_run_offsets() {
        let record = BrtCellRString {
            cell: Cell {
                column: 3,
                iStyleRef: 1,
                fPhShow: false,
            },
            value: RichStr {
                flags: RichStr::F_RICH_STR,
                str: XlWideString::construct(Some("AB")),
                rich: Some(StrRuns {
                    dwSizeStrRun: 2,
                    rgsStrRun: vec![StrRun { ich: 0, ifnt: 0 }, StrRun { ich: 1, ifnt: 1 }],
                }),
                ..RichStr::default()
            },
        };

        let bytes = record.to_bytes().expect("serialize rich cell");
        let (rest, decoded) =
            BrtCellRString::from_bytes((&bytes, 0)).expect("deserialize rich cell");

        assert_eq!(
            bytes,
            [
                3, 0, 0, 0, 1, 0, 0, 0, 1, 2, 0, 0, 0, b'A', 0, b'B', 0, 2, 0, 0, 0, 0, 0, 0, 0, 1,
                0, 1, 0,
            ]
        );
        assert_eq!(rest.0.len(), 0);
        assert_eq!(decoded, record);
        record.check().expect("valid rich cell");
    }
}
