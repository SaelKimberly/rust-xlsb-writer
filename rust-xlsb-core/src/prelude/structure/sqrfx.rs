#![allow(
    clippy::cast_possible_truncation,
    reason = "the spec count field is 32-bit; deku expands the cast into a generated impl"
)]

use deku::{DekuRead, DekuWrite};

use crate::Checked;
use crate::prelude::UncheckedRfX;

/// `2.5.156`
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default, Clone)]
pub struct UncheckedSqRfX {
    #[deku(update = "self.rgRefx.len() as u32")]
    pub crfx: u32,
    #[deku(count = "crfx")]
    pub rgRefx: Vec<UncheckedRfX>,
}

impl Checked for UncheckedSqRfX {
    fn check(&self) -> Result<(), &'static str> {
        if self.crfx > 32_767 {
            return Err("selection range count exceeds 32767");
        }
        if usize::try_from(self.crfx).ok() != Some(self.rgRefx.len()) {
            return Err("selection range count does not match the range list");
        }
        for range in &self.rgRefx {
            range.check()?;
        }
        Ok(())
    }
}
