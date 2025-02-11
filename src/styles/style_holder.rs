use super::{BrtColor, BrtFill, BrtFmt, BrtFont, BuiltinFmt, Fill};

pub type StyleRef = u32;

#[derive(Default)]
pub struct StyleBuilder {
    fmt: BrtFmt,
    font: BrtFont,
    fill: BrtFill,
}

impl StyleBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtin_format(&mut self, format: BuiltinFmt) -> &mut Self {
        self.fmt = format.into();
        self
    }

    pub fn with_fill(&mut self, fill: Fill) -> &mut Self {
        self.fill = fill.into();
        self
    }
}
