#![allow(
    clippy::cast_possible_truncation,
    reason = "the spec count fields are 16-bit; deku expands the cast into a generated impl"
)]

use deku::{DekuRead, DekuWrite};

use crate::{Checked, assign_id};

/// `2.5.159`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash)]
pub struct XFProp {
    pub xfPropType: u16,
    #[deku(update = "self.data.len() as u16 + 4")]
    pub cb: u16,
    #[deku(count = "(cb.saturating_sub(4)) as usize")]
    pub data: Vec<u8>,
}

impl Default for XFProp {
    fn default() -> Self {
        Self {
            xfPropType: 0,
            cb: 4,
            data: Vec::new(),
        }
    }
}

impl XFProp {
    #[must_use]
    pub const fn new(xf_prop_type: u16, data: Vec<u8>) -> Self {
        Self {
            xfPropType: xf_prop_type,
            cb: data.len() as u16 + 4,
            data,
        }
    }
}

impl Checked for XFProp {
    fn check(&self) -> Result<(), &'static str> {
        if usize::from(self.cb) != self.data.len() + 4 {
            return Err("XFProp length does not match its data");
        }
        Ok(())
    }
}

/// `2.5.164`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct XFProps {
    pub reserved: u16,
    #[deku(update = "self.props.len() as u16")]
    pub cprops: u16,
    #[deku(count = "cprops")]
    pub props: Vec<XFProp>,
}

impl XFProps {
    #[must_use]
    pub const fn new(props: Vec<XFProp>) -> Self {
        Self {
            reserved: 0,
            cprops: props.len() as u16,
            props,
        }
    }
}

impl Checked for XFProps {
    fn check(&self) -> Result<(), &'static str> {
        if usize::from(self.cprops) != self.props.len() {
            return Err("XFProps count does not match its property list");
        }
        for prop in &self.props {
            prop.check()?;
        }
        Ok(())
    }
}

/// `2.4.359`
#[derive(DekuRead, DekuWrite, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct BrtDXF {
    /// Bits 0..14 are reserved; bit 15 is `fNewBorder`.
    pub flags: u16,
    pub xfprops: XFProps,
}

impl BrtDXF {
    #[must_use]
    pub const fn new(xfprops: XFProps) -> Self {
        Self { flags: 0, xfprops }
    }
}

impl Checked for BrtDXF {
    fn check(&self) -> Result<(), &'static str> {
        self.xfprops.check()
    }
}

assign_id!(BrtDXF);

#[cfg(test)]
mod tests {
    use deku::DekuContainerWrite;

    use super::*;
    use crate::prelude::BrtColor;

    #[test]
    fn empty_dxf_is_flags_reserved_and_zero_properties() {
        let bytes = BrtDXF::default().to_bytes().expect("serializes");
        assert_eq!(bytes, [0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn background_color_property_has_exact_bytes() {
        let color = BrtColor::rgb(0xFF, 0x00, 0x00);
        let dxf = BrtDXF::new(XFProps::new(vec![XFProp::new(
            0x0002,
            color.xf_prop_bytes().to_vec(),
        )]));
        dxf.check().expect("valid dxf");
        let bytes = dxf.to_bytes().expect("serializes");

        assert_eq!(
            bytes,
            [0, 0, 0, 0, 1, 0, 2, 0, 12, 0, 5, 0, 0, 0, 0xFF, 0, 0, 0xFF]
        );
    }
}
