//! The first real workbook model: an owner of the shared-string table.
//!
//! Only the parts needed by value conversion exist so far; sheet data and
//! package assembly arrive with the worksheet writer.

use crate::context::WbContext;
use crate::{Result, SSTHolder};

/// Owns the workbook shared-string table for the duration of one write.
pub struct Workbook {
    sst: SSTHolder,
}

impl Workbook {
    /// Creates a workbook backed by an in-memory shared-string table.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Sst`] if the in-memory SQLite table cannot be
    /// initialized.
    pub fn new() -> Result<Self> {
        Ok(Self {
            sst: SSTHolder::create_in_memory()?,
        })
    }

    /// Wraps a caller-owned write-session shared-string table.
    ///
    /// The holder must not be a table reopened from disk; see the design spec.
    #[must_use]
    pub const fn with_sst(sst: SSTHolder) -> Self {
        Self { sst }
    }

    /// Returns a worksheet bound to this workbook's shared strings.
    #[must_use]
    pub const fn worksheet(&self) -> Worksheet<'_> {
        Worksheet { parent: self }
    }
}

impl WbContext for Workbook {
    fn shared_strings(&self) -> &SSTHolder {
        &self.sst
    }
}

/// A worksheet context. Values converted through it intern into the parent
/// workbook's shared-string table.
pub struct Worksheet<'wb> {
    parent: &'wb Workbook,
}

impl WbContext for Worksheet<'_> {
    fn shared_strings(&self) -> &SSTHolder {
        self.parent.shared_strings()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::WbContext;

    #[test]
    fn worksheet_shares_the_workbook_sst() {
        let workbook = Workbook::new().expect("in-memory workbook");
        let worksheet = workbook.worksheet();

        let hello = worksheet.shared_string("hello").expect("interns");
        assert_eq!(hello.id(), 0);
        assert_eq!(workbook.shared_strings().len(), 1);

        let again = workbook.shared_string("hello").expect("interns");
        assert_eq!(again, hello);
        assert_eq!(worksheet.shared_strings().len(), 1);
    }

    #[test]
    fn ids_follow_pushes() {
        let workbook = Workbook::new().expect("in-memory workbook");
        let first = workbook.shared_string("a").expect("interns");
        let second = workbook.shared_string("b").expect("interns");
        assert_eq!((first.id(), second.id()), (0, 1));
        assert_eq!(workbook.shared_strings().len(), 2);
    }

    #[test]
    fn empty_sst_is_reported() {
        let workbook = Workbook::new().expect("in-memory workbook");
        assert!(workbook.shared_strings().is_empty());
    }
}
