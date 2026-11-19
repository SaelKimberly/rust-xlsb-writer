//! Worksheet tables and Table part serialization.

use std::fmt::{self, Display};

use rust_xlsb_core::prelude::{
    BrtBeginAFilter, BrtBeginList, BrtBeginListCol, BrtBeginListCols, BrtEndAFilter, BrtEndList,
    BrtEndListCol, BrtEndListCols, BrtListCCFmla, BrtListTrFmla, BrtTableStyleClient,
    CellParsedFormula, UncheckedRfX, XlNullableWideString, XlStringExt,
};
use rust_xlsb_core::push_record;

use crate::formula::{TableInfo, compile, compile_table_formula};
use crate::{Error, Format, Formula, Result};

const NO_DXF: u32 = 0xffff_ffff;

/// A worksheet table.
#[derive(Debug, Clone)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "table flags map directly to independent style and visibility settings"
)]
pub struct Table {
    pub(crate) name: String,
    pub(crate) id: u32,
    pub(crate) style: TableStyle,
    pub(crate) columns: Vec<TableColumn>,
    pub(crate) show_header_row: bool,
    pub(crate) show_total_row: bool,
    pub(crate) show_autofilter: bool,
    pub(crate) show_banded_rows: bool,
    pub(crate) show_banded_columns: bool,
    pub(crate) show_first_column: bool,
    pub(crate) show_last_column: bool,
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

impl Table {
    /// Creates a table with default header, `AutoFilter`, and style settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            name: String::new(),
            id: 0,
            style: TableStyle::Medium9,
            columns: Vec::new(),
            show_header_row: true,
            show_total_row: false,
            show_autofilter: true,
            show_banded_rows: true,
            show_banded_columns: false,
            show_first_column: false,
            show_last_column: false,
        }
    }

    /// Sets the table name used in formulas.
    #[must_use]
    pub fn set_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Sets the built-in table style.
    #[must_use]
    pub const fn set_style(mut self, style: TableStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the table columns. Missing columns are filled in by position.
    #[must_use]
    pub fn set_columns(mut self, columns: &[TableColumn]) -> Self {
        self.columns = columns.to_vec();
        self
    }

    /// Turns the header row on or off.
    #[must_use]
    pub const fn set_header_row(mut self, enable: bool) -> Self {
        self.show_header_row = enable;
        self
    }

    /// Turns the total row on or off.
    ///
    /// The total row is the table's last row. Populate it with
    /// [`TableColumn::set_total_label`] or [`TableColumn::set_total_function`].
    #[must_use]
    pub const fn set_total_row(mut self, enable: bool) -> Self {
        self.show_total_row = enable;
        self
    }

    /// Turns the table `AutoFilter` on or off.
    #[must_use]
    pub const fn set_autofilter(mut self, enable: bool) -> Self {
        self.show_autofilter = enable;
        self
    }

    /// Turns row banding on or off.
    #[must_use]
    pub const fn set_banded_rows(mut self, enable: bool) -> Self {
        self.show_banded_rows = enable;
        self
    }

    /// Turns column banding on or off.
    #[must_use]
    pub const fn set_banded_columns(mut self, enable: bool) -> Self {
        self.show_banded_columns = enable;
        self
    }

    /// Highlights the first column.
    #[must_use]
    pub const fn set_first_column(mut self, enable: bool) -> Self {
        self.show_first_column = enable;
        self
    }

    /// Highlights the last column.
    #[must_use]
    pub const fn set_last_column(mut self, enable: bool) -> Self {
        self.show_last_column = enable;
        self
    }

    pub(crate) fn validate_name(&self) -> Result<()> {
        if self.name.is_empty() {
            return Ok(());
        }
        if self
            .name
            .get(..3)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("_xl"))
        {
            return Err(Error::TableError(
                "table name cannot start with '_xl'".to_owned(),
            ));
        }
        crate::defined_name::check_name(&self.name)
            .map_err(|error| Error::TableError(error.to_string()))
    }
}

/// One table column.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableColumn {
    pub(crate) header: String,
    pub(crate) total_label: String,
    pub(crate) total_function: TableFunction,
    pub(crate) format: Option<Format>,
    pub(crate) header_format: Option<Format>,
    pub(crate) total_formula: Option<CellParsedFormula>,
    pub(crate) total_result: Option<Box<str>>,
    pub(crate) formula: Option<Formula>,
}

impl TableColumn {
    /// Creates an empty table column.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            header: String::new(),
            total_label: String::new(),
            total_function: TableFunction::None,
            format: None,
            header_format: None,
            total_formula: None,
            total_result: None,
            formula: None,
        }
    }

    /// Sets the column header text.
    #[must_use]
    pub fn set_header(mut self, header: impl Into<String>) -> Self {
        self.header = header.into();
        self
    }

    /// Sets the label displayed in the table total row for this column.
    #[must_use]
    pub fn set_total_label(mut self, label: impl Into<String>) -> Self {
        self.total_label = label.into();
        self
    }

    /// Sets the `SUBTOTAL` function for this column's total row cell.
    #[must_use]
    pub fn set_total_function(mut self, function: TableFunction) -> Self {
        self.total_function = function;
        self
    }

    /// Sets a calculated-column formula.
    ///
    /// The formula may use structured references to the table, such as
    /// `SUM(Table1[@[Q1]:[Q4]])`. The formula is stored in the Table part; it
    /// does not write calculated cell values.
    #[must_use]
    pub fn set_formula(mut self, formula: impl Into<Formula>) -> Self {
        self.formula = Some(formula.into());
        self
    }

    /// Sets the format for this column's data cells.
    #[must_use]
    pub fn set_format(mut self, format: impl Into<Format>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Sets the format for this column's header cell.
    #[must_use]
    pub fn set_header_format(mut self, format: impl Into<Format>) -> Self {
        self.header_format = Some(format.into());
        self
    }
}

/// A function for a table total row column.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum TableFunction {
    /// No total function.
    #[default]
    None,
    /// Arithmetic mean.
    Average,
    /// Count of non-empty cells.
    Count,
    /// Count of numeric cells.
    CountNumbers,
    /// Largest value.
    Max,
    /// Smallest value.
    Min,
    /// Arithmetic sum.
    Sum,
    /// Estimated standard deviation.
    StdDev,
    /// Estimated variance.
    Var,
    /// User supplied same-sheet A1 formula.
    Custom(Formula),
}

impl TableFunction {
    pub(crate) const fn ilta(&self) -> u32 {
        match self {
            Self::None => 0,
            Self::Average => 1,
            Self::Count => 2,
            Self::CountNumbers => 3,
            Self::Max => 4,
            Self::Min => 5,
            Self::Sum => 6,
            Self::StdDev => 7,
            Self::Var => 8,
            Self::Custom(_) => 9,
        }
    }

    pub(crate) const fn subtotal_number(&self) -> Option<u16> {
        match self {
            Self::None | Self::Custom(_) => None,
            Self::Average => Some(101),
            Self::CountNumbers => Some(102),
            Self::Count => Some(103),
            Self::Max => Some(104),
            Self::Min => Some(105),
            Self::StdDev => Some(107),
            Self::Sum => Some(109),
            Self::Var => Some(110),
        }
    }
}

macro_rules! table_styles {
    (
        default $default:ident => $default_name:literal,
        $($variant:ident => $name:literal),+ $(,)?
    ) => {
        /// A built-in Excel table style.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        pub enum TableStyle {
            /// No table style.
            None,
            /// Built-in table style.
            #[default]
            $default,
            $(
                /// Built-in table style.
                $variant,
            )+
        }

        impl TableStyle {
            pub(crate) const fn name(self) -> Option<&'static str> {
                match self {
                    Self::None => None,
                    Self::$default => Some($default_name),
                    $(Self::$variant => Some($name),)+
                }
            }
        }

        impl Display for TableStyle {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.name().unwrap_or("TableStyleNone"))
            }
        }
    };
}

table_styles! {
    default Medium9 => "TableStyleMedium9",
    Light1 => "TableStyleLight1",
    Light2 => "TableStyleLight2",
    Light3 => "TableStyleLight3",
    Light4 => "TableStyleLight4",
    Light5 => "TableStyleLight5",
    Light6 => "TableStyleLight6",
    Light7 => "TableStyleLight7",
    Light8 => "TableStyleLight8",
    Light9 => "TableStyleLight9",
    Light10 => "TableStyleLight10",
    Light11 => "TableStyleLight11",
    Light12 => "TableStyleLight12",
    Light13 => "TableStyleLight13",
    Light14 => "TableStyleLight14",
    Light15 => "TableStyleLight15",
    Light16 => "TableStyleLight16",
    Light17 => "TableStyleLight17",
    Light18 => "TableStyleLight18",
    Light19 => "TableStyleLight19",
    Light20 => "TableStyleLight20",
    Light21 => "TableStyleLight21",
    Medium1 => "TableStyleMedium1",
    Medium2 => "TableStyleMedium2",
    Medium3 => "TableStyleMedium3",
    Medium4 => "TableStyleMedium4",
    Medium5 => "TableStyleMedium5",
    Medium6 => "TableStyleMedium6",
    Medium7 => "TableStyleMedium7",
    Medium8 => "TableStyleMedium8",
    Medium10 => "TableStyleMedium10",
    Medium11 => "TableStyleMedium11",
    Medium12 => "TableStyleMedium12",
    Medium13 => "TableStyleMedium13",
    Medium14 => "TableStyleMedium14",
    Medium15 => "TableStyleMedium15",
    Medium16 => "TableStyleMedium16",
    Medium17 => "TableStyleMedium17",
    Medium18 => "TableStyleMedium18",
    Medium19 => "TableStyleMedium19",
    Medium20 => "TableStyleMedium20",
    Medium21 => "TableStyleMedium21",
    Medium22 => "TableStyleMedium22",
    Medium23 => "TableStyleMedium23",
    Medium24 => "TableStyleMedium24",
    Medium25 => "TableStyleMedium25",
    Medium26 => "TableStyleMedium26",
    Medium27 => "TableStyleMedium27",
    Medium28 => "TableStyleMedium28",
    Dark1 => "TableStyleDark1",
    Dark2 => "TableStyleDark2",
    Dark3 => "TableStyleDark3",
    Dark4 => "TableStyleDark4",
    Dark5 => "TableStyleDark5",
    Dark6 => "TableStyleDark6",
    Dark7 => "TableStyleDark7",
    Dark8 => "TableStyleDark8",
    Dark9 => "TableStyleDark9",
    Dark10 => "TableStyleDark10",
    Dark11 => "TableStyleDark11",
}

/// A table plus its resolved worksheet range and save-time identity.
#[expect(
    clippy::redundant_pub_crate,
    reason = "shared with the worksheet serializer and package writer"
)]
#[derive(Debug, Clone)]
pub(crate) struct TableRange {
    pub(crate) table: Table,
    pub(crate) range: UncheckedRfX,
}

/// Serializes one Table part.
#[expect(clippy::redundant_pub_crate, reason = "shared with the package writer")]
pub(crate) fn write_table_part(
    table: &TableRange,
    tables: &[TableInfo<'_>],
    out: &mut Vec<u8>,
) -> Result<()> {
    let table_data = &table.table;
    let range = table.range;
    let name = table_data.name.as_str();
    let total_row = table_data.show_total_row;
    push_record(
        out,
        &BrtBeginList {
            rfxList: range,
            lt: 0,
            idList: table_data.id,
            crwHeader: u32::from(table_data.show_header_row),
            crwTotals: u32::from(total_row),
            flags: if total_row {
                BrtBeginList::F_SHOWN_TOTAL_ROW
            } else {
                0
            },
            nDxfHeader: NO_DXF,
            nDxfData: NO_DXF,
            nDxfAgg: NO_DXF,
            nDxfBorder: NO_DXF,
            nDxfHeaderBorder: NO_DXF,
            nDxfAggBorder: NO_DXF,
            dwConnID: 0,
            stName: XlNullableWideString::construct(Some(name)),
            stDisplayName: XlNullableWideString::construct(Some(name)),
            stComment: XlNullableWideString::default(),
            stStyleHeader: XlNullableWideString::default(),
            stStyleData: XlNullableWideString::default(),
            stStyleAgg: XlNullableWideString::default(),
        },
    )?;

    if table_data.show_header_row && table_data.show_autofilter {
        let mut filter_range = range;
        if total_row {
            filter_range.rwLast = filter_range.rwLast.saturating_sub(1);
        }
        push_record(out, &BrtBeginAFilter { rfx: filter_range })?;
        push_record(out, &BrtEndAFilter::default())?;
    }

    push_record(
        out,
        &BrtBeginListCols {
            nCols: u32::try_from(table_data.columns.len())
                .map_err(|_| Error::ValueNotRepresentable("too many table columns"))?,
        },
    )?;
    for (index, column) in table_data.columns.iter().enumerate() {
        let id = u32::try_from(index + 1)
            .map_err(|_| Error::ValueNotRepresentable("too many table columns"))?;
        let label = (!column.total_label.is_empty()).then_some(column.total_label.as_str());
        let custom = if label.is_none() {
            match &column.total_function {
                TableFunction::Custom(formula) => Some(formula),
                _ => None,
            }
        } else {
            None
        };
        push_record(
            out,
            &BrtBeginListCol {
                idField: id,
                ilta: if label.is_some() {
                    0
                } else {
                    column.total_function.ilta()
                },
                nDxfHdr: NO_DXF,
                nDxfInsertRow: NO_DXF,
                nDxfAgg: NO_DXF,
                idqsif: 0,
                stName: XlNullableWideString::default(),
                stCaption: XlNullableWideString::construct(Some(column.header.as_str())),
                stTotal: label.map_or_else(XlNullableWideString::default, |label| {
                    XlNullableWideString::construct(Some(label))
                }),
                stStyleHeader: XlNullableWideString::default(),
                stStyleInsertRow: XlNullableWideString::default(),
                stStyleAgg: XlNullableWideString::default(),
            },
        )?;
        if let Some(formula) = &column.formula {
            let formula = compile_table_formula(formula.as_str(), tables)?;
            push_record(out, &BrtListCCFmla { flags: 0, formula })?;
        }
        if let Some(formula) = custom {
            let formula = match &column.total_formula {
                Some(compiled) => compiled.clone(),
                None => compile(formula.as_str())?,
            };
            push_record(out, &BrtListTrFmla { flags: 0, formula })?;
        }
        push_record(out, &BrtEndListCol::default())?;
    }
    push_record(out, &BrtEndListCols::default())?;

    let mut flags = 0;
    if table_data.show_first_column {
        flags |= BrtTableStyleClient::F_FIRST_COLUMN;
    }
    if table_data.show_last_column {
        flags |= BrtTableStyleClient::F_LAST_COLUMN;
    }
    if table_data.show_banded_rows {
        flags |= BrtTableStyleClient::F_ROW_STRIPES;
    }
    if table_data.show_banded_columns {
        flags |= BrtTableStyleClient::F_COLUMN_STRIPES;
    }
    push_record(
        out,
        &BrtTableStyleClient {
            flags,
            stStyleName: table_data
                .style
                .name()
                .map_or_else(XlNullableWideString::default, |style| {
                    XlNullableWideString::construct(Some(style))
                }),
        },
    )?;
    push_record(out, &BrtEndList::default())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use deku::DekuContainerRead;
    use rust_xlsb_core::prelude::{
        BrtBeginAFilter, BrtBeginList, BrtBeginListCol, BrtTableStyleClient,
    };
    use rust_xlsb_core::{BiffHead, CheckedBiff, KnownID};

    use super::*;

    fn records<T>(part: &[u8]) -> Vec<T>
    where
        T: for<'a> DekuContainerRead<'a> + Default + CheckedBiff,
    {
        let target = T::default().id();
        let mut reader = std::io::BufReader::new(part);
        let mut records = Vec::new();
        while let Ok((head, body)) = BiffHead::read_from_with_data(&mut reader) {
            if head.known_id() == Some(target)
                && let Some(body) = body
            {
                let (_, record) = T::from_bytes((&body, 0)).expect("record decodes");
                records.push(record);
            }
        }
        records
    }

    fn record_ids(part: &[u8]) -> Vec<KnownID> {
        let mut reader = std::io::BufReader::new(part);
        let mut ids = Vec::new();
        while let Ok((head, _)) = BiffHead::read_from_with_data(&mut reader) {
            if let Some(id) = head.known_id() {
                ids.push(id);
            }
        }
        ids
    }

    fn range() -> UncheckedRfX {
        UncheckedRfX {
            rwFirst: 1,
            rwLast: 5,
            colFirst: 2,
            colLast: 4,
        }
    }

    fn table() -> TableRange {
        TableRange {
            table: Table {
                name: "Table1".to_owned(),
                id: 1,
                columns: vec![
                    TableColumn::new().set_header("Name"),
                    TableColumn::new().set_header("Q1"),
                    TableColumn::new().set_header("Q2"),
                ],
                ..Table::new()
            },
            range: range(),
        }
    }

    #[test]
    fn table_part_writes_calculated_column_formulas() {
        let mut table = table();
        table.table.columns[1] = TableColumn::new()
            .set_header("Q1")
            .set_formula("Table1[@[Q1]:[Q2]]")
            .set_total_function(TableFunction::Custom(Formula::new("SUM(B2:B4)")));
        let tables = [TableInfo {
            name: "Table1",
            id: 1,
            ixti: 0,
            columns: &table.table.columns,
        }];
        let mut part = Vec::new();
        write_table_part(&table, &tables, &mut part).expect("table part");

        let ids = record_ids(&part);
        let calc = ids
            .iter()
            .position(|id| *id == KnownID::BrtListCCFmla)
            .expect("calculated formula");
        let total = ids
            .iter()
            .position(|id| *id == KnownID::BrtListTrFmla)
            .expect("total formula");
        let begin = ids[..calc]
            .iter()
            .rposition(|id| *id == KnownID::BrtBeginListCol)
            .expect("begin column");
        let end = ids[total + 1..]
            .iter()
            .position(|id| *id == KnownID::BrtEndListCol)
            .expect("end column")
            + total
            + 1;
        assert!(begin < calc && calc < total && total < end);

        let calculated = records::<BrtListCCFmla>(&part);
        assert_eq!(calculated.len(), 1);
        assert_eq!(
            calculated[0].formula,
            compile_table_formula("Table1[@[Q1]:[Q2]]", &tables).expect("compiled")
        );
        assert_eq!(calculated[0].flags, 0);
        assert_eq!(records::<BrtListTrFmla>(&part).len(), 1);
    }

    #[test]
    fn table_part_orders_records_and_style_flags() {
        let mut part = Vec::new();
        write_table_part(&table(), &[], &mut part).expect("table part");

        assert_eq!(
            record_ids(&part),
            [
                KnownID::BrtBeginList,
                KnownID::BrtBeginAFilter,
                KnownID::BrtEndAFilter,
                KnownID::BrtBeginListCols,
                KnownID::BrtBeginListCol,
                KnownID::BrtEndListCol,
                KnownID::BrtBeginListCol,
                KnownID::BrtEndListCol,
                KnownID::BrtBeginListCol,
                KnownID::BrtEndListCol,
                KnownID::BrtEndListCols,
                KnownID::BrtTableStyleClient,
                KnownID::BrtEndList,
            ]
        );

        let list = records::<BrtBeginList>(&part);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].idList, 1);
        assert_eq!(list[0].crwHeader, 1);
        assert_eq!(
            list[0].stDisplayName.to_optional_string().as_deref(),
            Some("Table1")
        );
        assert_eq!(list[0].nDxfHeader, NO_DXF);

        let filters = records::<BrtBeginAFilter>(&part);
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0].rfx, range());

        let columns = records::<BrtBeginListCol>(&part);
        assert_eq!(columns.len(), 3);
        assert_eq!(
            columns[2].stCaption.to_optional_string().as_deref(),
            Some("Q2")
        );

        let styles = records::<BrtTableStyleClient>(&part);
        assert_eq!(styles.len(), 1);
        assert_eq!(styles[0].flags, BrtTableStyleClient::F_ROW_STRIPES);
        assert_eq!(
            styles[0].stStyleName.to_optional_string().as_deref(),
            Some("TableStyleMedium9")
        );
    }

    #[test]
    fn table_part_honors_disable_flags_and_no_style() {
        let mut table = table();
        table.table.name = "Plain".to_owned();
        table.table.show_autofilter = false;
        table.table.show_banded_rows = false;
        table.table.show_first_column = true;
        table.table.style = TableStyle::None;

        let mut part = Vec::new();
        write_table_part(&table, &[], &mut part).expect("table part");
        assert_eq!(records::<BrtBeginAFilter>(&part).len(), 0);

        let styles = records::<BrtTableStyleClient>(&part);
        assert_eq!(styles[0].flags, BrtTableStyleClient::F_FIRST_COLUMN);
        assert!(styles[0].stStyleName.is_null());
    }

    #[test]
    fn style_names_render_stable_table_style_names() {
        assert_eq!(TableStyle::Light17.to_string(), "TableStyleLight17");
        assert_eq!(TableStyle::None.to_string(), "TableStyleNone");
    }
}
