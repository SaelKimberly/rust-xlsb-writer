use deku::{DekuRead, DekuWrite};
use std::{hash::Hash, ops::Deref};

use crate::prelude::Unchecked;

#[derive(DekuRead, DekuWrite, Debug, Default, Clone, Copy)]
pub struct Xnum(f64);

impl Deref for Xnum {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Xnum {
    pub const fn to_le_bytes(self) -> [u8; 8] {
        self.0.to_le_bytes()
    }
}

impl From<f64> for Xnum {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

impl From<Xnum> for f64 {
    fn from(value: Xnum) -> Self {
        value.0
    }
}

impl PartialEq for Xnum {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl PartialOrd for Xnum {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(std::cmp::Ord::cmp(self, other))
    }
}

impl Ord for Xnum {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Eq for Xnum {}

impl Hash for Xnum {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl Unchecked for Xnum {}
