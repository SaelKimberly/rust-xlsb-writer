#![deny(dead_code)]
#![allow(internal_features)]
#![feature(maybe_uninit_write_slice)]
#![feature(core_intrinsics)]
#![feature(str_from_utf16_endian)]

mod biff_head;
mod biff_id;
mod err;
pub mod prelude;

pub use biff_head::BiffHead;
pub use biff_id::KnownID;
pub use err::{Error, Result};
