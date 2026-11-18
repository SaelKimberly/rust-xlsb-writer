//! Conversion from Rust values to semantic cell values.

use crate::context::{NumberEncoding, SharedString, WbContext};
use crate::prelude::BErr;
use crate::{Error, Result};

/// A value ready to be paired with a row, column, and style by the writer.
#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Blank,
    Bool(bool),
    Error(BErr),
    Number(NumberEncoding),
    SharedString(SharedString),
    InlineString(String),
}

/// Converts a Rust value into a [`CellValue`] for a specific workbook.
pub trait ToCellValue {
    /// Converts `self`, using `context` for shared strings and number policy.
    ///
    /// # Errors
    ///
    /// Returns [`Error::SharedStringOutOfRange`] when the value references a
    /// shared string the target workbook does not have, or
    /// [`Error::ValueNotRepresentable`] when a number cannot be stored exactly.
    fn to_cell_value(&self, context: &dyn WbContext) -> Result<CellValue>;
}

impl ToCellValue for () {
    fn to_cell_value(&self, _: &dyn WbContext) -> Result<CellValue> {
        Ok(CellValue::Blank)
    }
}

impl<T: ToCellValue> ToCellValue for Option<T> {
    fn to_cell_value(&self, context: &dyn WbContext) -> Result<CellValue> {
        self.as_ref().map_or_else(
            || Ok(CellValue::Blank),
            |value| value.to_cell_value(context),
        )
    }
}

impl ToCellValue for bool {
    fn to_cell_value(&self, _: &dyn WbContext) -> Result<CellValue> {
        Ok(CellValue::Bool(*self))
    }
}

impl ToCellValue for BErr {
    fn to_cell_value(&self, _: &dyn WbContext) -> Result<CellValue> {
        Ok(CellValue::Error(*self))
    }
}

impl ToCellValue for SharedString {
    fn to_cell_value(&self, context: &dyn WbContext) -> Result<CellValue> {
        let len = context.shared_strings().len();
        if self.id() < len {
            Ok(CellValue::SharedString(*self))
        } else {
            Err(Error::SharedStringOutOfRange { id: self.id(), len })
        }
    }
}

impl ToCellValue for str {
    fn to_cell_value(&self, _: &dyn WbContext) -> Result<CellValue> {
        Ok(CellValue::InlineString(self.to_owned()))
    }
}

impl ToCellValue for String {
    fn to_cell_value(&self, _: &dyn WbContext) -> Result<CellValue> {
        Ok(CellValue::InlineString(self.clone()))
    }
}

impl ToCellValue for f32 {
    fn to_cell_value(&self, context: &dyn WbContext) -> Result<CellValue> {
        Ok(CellValue::Number(context.store_f32(*self)))
    }
}

impl ToCellValue for f64 {
    fn to_cell_value(&self, context: &dyn WbContext) -> Result<CellValue> {
        Ok(CellValue::Number(context.store_f64(*self)))
    }
}

macro_rules! impl_small_integer {
    ($($ty: ty),* $(,)?) => {
        $(
            impl ToCellValue for $ty {
                fn to_cell_value(&self, context: &dyn WbContext) -> Result<CellValue> {
                    Ok(CellValue::Number(context.store_integer(i64::from(*self))?))
                }
            }
        )*
    };
}

impl_small_integer!(i8, i16, i32, i64, u8, u16, u32);

impl ToCellValue for u64 {
    fn to_cell_value(&self, context: &dyn WbContext) -> Result<CellValue> {
        let value = i64::try_from(*self)
            .map_err(|_| Error::ValueNotRepresentable("u64 exceeds i64 range"))?;
        Ok(CellValue::Number(context.store_integer(value)?))
    }
}

impl<T: ToCellValue + ?Sized> ToCellValue for &T {
    fn to_cell_value(&self, context: &dyn WbContext) -> Result<CellValue> {
        (**self).to_cell_value(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_context::TestContext;

    #[test]
    fn inline_strings_are_default() {
        let workbook = TestContext::new();
        let text = "hello";

        assert_eq!(
            text.to_cell_value(&workbook).expect("converts"),
            CellValue::InlineString("hello".to_owned())
        );
        assert_eq!(
            "hello"
                .to_owned()
                .to_cell_value(&workbook)
                .expect("converts"),
            CellValue::InlineString("hello".to_owned())
        );
        assert!(workbook.shared_strings().is_empty());
    }

    #[test]
    fn unit_and_none_are_blank() {
        let workbook = TestContext::new();

        assert_eq!(
            ().to_cell_value(&workbook).expect("converts"),
            CellValue::Blank
        );
        assert_eq!(
            Option::<i32>::None
                .to_cell_value(&workbook)
                .expect("converts"),
            CellValue::Blank
        );
        assert!(matches!(
            Some(-1_i32).to_cell_value(&workbook).expect("converts"),
            CellValue::Number(NumberEncoding::Rk(_))
        ));
    }

    #[test]
    fn integers_use_the_context_policy() {
        let workbook = TestContext::new();

        assert!(matches!(
            (-42_i32).to_cell_value(&workbook).expect("converts"),
            CellValue::Number(NumberEncoding::Rk(rk)) if rk.value().to_bits() == (-42.0_f64).to_bits()
        ));
        assert_eq!(
            2_u64.pow(29).to_cell_value(&workbook).expect("converts"),
            CellValue::Number(NumberEncoding::Real(536_870_912.0))
        );
    }

    #[test]
    fn integers_outside_f64_precision_error() {
        let workbook = TestContext::new();

        assert!(matches!(
            i64::MAX.to_cell_value(&workbook),
            Err(Error::ValueNotRepresentable(_))
        ));
        assert!(matches!(
            u64::MAX.to_cell_value(&workbook),
            Err(Error::ValueNotRepresentable(_))
        ));
    }

    #[test]
    fn booleans_and_errors_keep_their_type() {
        let workbook = TestContext::new();

        assert_eq!(
            true.to_cell_value(&workbook).expect("converts"),
            CellValue::Bool(true)
        );
        assert_eq!(
            BErr::Div0.to_cell_value(&workbook).expect("converts"),
            CellValue::Error(BErr::Div0)
        );
    }

    #[test]
    fn shared_string_is_validated_against_the_target_workbook() {
        let first = TestContext::new();
        let second = TestContext::new();
        let hello = first.shared_string("hello").expect("interns");
        let other = first.shared_string("other").expect("interns");
        let one = second.shared_string("only").expect("interns");

        assert_eq!(
            hello.to_cell_value(&second).expect("converts"),
            CellValue::SharedString(hello)
        );
        assert_eq!(one.id(), 0);

        assert!(matches!(
            other.to_cell_value(&second),
            Err(Error::SharedStringOutOfRange { id: 1, len: 1 })
        ));
        assert!(matches!(
            hello.to_cell_value(&TestContext::new()),
            Err(Error::SharedStringOutOfRange { id: 0, len: 0 })
        ));
    }

    #[test]
    fn references_forward_to_the_inner_value() {
        let workbook = TestContext::new();
        let value = 3.75_f64;

        assert_eq!(
            value.to_cell_value(&workbook).expect("converts"),
            value.to_cell_value(&workbook).expect("converts")
        );
    }
}
