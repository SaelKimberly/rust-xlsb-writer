//! Test-only workbook context for low-level conversion tests.

use crate::{SSTHolder, WbContext};

pub struct TestContext {
    sst: SSTHolder,
}

impl TestContext {
    pub(crate) fn new() -> Self {
        Self {
            sst: SSTHolder::create_in_memory().expect("in-memory shared strings"),
        }
    }
}

impl WbContext for TestContext {
    fn shared_strings(&self) -> &SSTHolder {
        &self.sst
    }
}
