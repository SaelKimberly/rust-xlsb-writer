use deku::{DekuRead, DekuWrite};

use crate::Unchecked;
use crate::assign_id;
use crate::prelude::UncheckedSqRfX;

/// `2.4.720`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtSel {
    pub pnn: u32,
    pub rwAct: u32,
    pub colAct: u32,
    pub dwRfxAct: u32,
    pub sqrfx: UncheckedSqRfX,
}

impl Unchecked for BrtSel {}

assign_id!(BrtSel);

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;
    use crate::prelude::{UncheckedRfX, UncheckedSqRfX};

    #[test]
    fn roundtrips_one_selection_range() {
        let selection = BrtSel {
            pnn: 3,
            rwAct: 2,
            colAct: 1,
            dwRfxAct: 0,
            sqrfx: UncheckedSqRfX {
                crfx: 1,
                rgRefx: vec![UncheckedRfX {
                    rwFirst: 2,
                    rwLast: 4,
                    colFirst: 1,
                    colLast: 3,
                }],
            },
        };

        let bytes = selection.to_bytes().expect("serializes");
        assert_eq!(bytes.len(), 36);
        let (rest, decoded) = BrtSel::from_bytes((&bytes, 0)).expect("decodes");
        assert_eq!(rest.0, []);
        assert_eq!(decoded, selection);
    }
}
