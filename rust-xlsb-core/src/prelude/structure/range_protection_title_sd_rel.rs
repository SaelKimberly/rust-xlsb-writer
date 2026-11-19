#![allow(
    clippy::cast_possible_truncation,
    reason = "the spec descriptor length is 32-bit; deku expands the cast into a generated impl"
)]

use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::prelude::{XlStringExt, XlWideString};

/// `2.5.113`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq)]
pub struct RangeProtectionTitleSDRel {
    pub rgchTitle: XlWideString,
    #[deku(update = "self.pSDRel.len() as u32")]
    pub dwsdRel: u32,
    #[deku(count = "dwsdRel")]
    pub pSDRel: Vec<u8>,
}

impl Default for RangeProtectionTitleSDRel {
    fn default() -> Self {
        Self {
            rgchTitle: XlWideString::construct(Some("A")),
            dwsdRel: 0,
            pSDRel: Vec::new(),
        }
    }
}

impl Checked for RangeProtectionTitleSDRel {
    fn check(&self) -> Result<(), &'static str> {
        if !(1..=255).contains(&self.rgchTitle.len()) {
            return Err("range title must contain 1 to 255 UTF-16 code units");
        }
        if usize::try_from(self.dwsdRel).ok() != Some(self.pSDRel.len()) {
            return Err("security descriptor length does not match its bytes");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use super::*;

    #[test]
    fn encodes_title_length_and_utf16_bytes() {
        let title = RangeProtectionTitleSDRel {
            rgchTitle: XlWideString::construct(Some("Range1")),
            dwsdRel: 0,
            pSDRel: Vec::new(),
        };

        let bytes = title.to_bytes().expect("serializes");
        assert_eq!(&bytes[..4], [6, 0, 0, 0]);
        assert_eq!(bytes.len(), 4 + 12 + 4);
        assert_eq!(&bytes[4..6], [b'R', 0]);
        assert_eq!(&bytes[bytes.len() - 4..], [0, 0, 0, 0]);

        let (rest, decoded) = RangeProtectionTitleSDRel::from_bytes((&bytes, 0)).expect("decodes");
        assert_eq!(rest.0.len(), 0);
        assert_eq!(decoded, title);
    }

    #[test]
    fn rejects_empty_title_and_bad_descriptor_length() {
        let title = RangeProtectionTitleSDRel {
            rgchTitle: XlWideString::construct(Some("")),
            ..RangeProtectionTitleSDRel::default()
        };
        assert!(title.check().is_err());

        let title = RangeProtectionTitleSDRel {
            dwsdRel: 1,
            ..RangeProtectionTitleSDRel::default()
        };
        assert!(title.check().is_err());
    }
}
