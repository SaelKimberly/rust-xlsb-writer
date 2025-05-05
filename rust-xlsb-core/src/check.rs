use crate::KnownID;

pub trait Checked {
    fn check(&self) -> Result<(), &'static str>;
}

pub trait Unchecked {}

impl<T: Unchecked> Checked for T {
    fn check(&self) -> Result<(), &'static str> {
        Ok(())
    }
}

pub trait CheckedBiff: Checked {
    fn id(&self) -> KnownID;
}
