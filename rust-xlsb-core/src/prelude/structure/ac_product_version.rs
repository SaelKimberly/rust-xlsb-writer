use deku::{DekuRead, DekuWrite};

use crate::prelude::check::Unchecked;

#[derive(Debug, DekuRead, DekuWrite, Hash, PartialEq)]
/// `2.5.1`
pub struct ACProductVersion {
    fileVersion: u16,
    #[deku(bits = 15)]
    fileProduct: u16,
    #[deku(bits = 1)]
    fileExtension: bool,
}

impl Default for ACProductVersion {
    fn default() -> Self {
        Self {
            fileVersion: 0x0e02,
            fileProduct: 0x0000,
            fileExtension: true,
        }
    }
}

impl Unchecked for ACProductVersion {}
