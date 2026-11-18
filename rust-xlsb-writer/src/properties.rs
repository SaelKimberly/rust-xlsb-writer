//! Workbook document metadata.

/// Core, extended, and custom document properties written to `docProps`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DocProperties {
    pub(crate) title: String,
    pub(crate) subject: String,
    pub(crate) author: String,
    pub(crate) manager: String,
    pub(crate) company: String,
    pub(crate) category: String,
    pub(crate) keywords: String,
    pub(crate) comment: String,
    pub(crate) status: String,
    pub(crate) hyperlink_base: String,
    pub(crate) creation_time: Option<String>,
    pub(crate) custom_properties: Vec<CustomProperty>,
}

impl DocProperties {
    /// Creates an empty set of document properties.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            title: String::new(),
            subject: String::new(),
            author: String::new(),
            manager: String::new(),
            company: String::new(),
            category: String::new(),
            keywords: String::new(),
            comment: String::new(),
            status: String::new(),
            hyperlink_base: String::new(),
            creation_time: None,
            custom_properties: Vec::new(),
        }
    }

    /// Sets the document title.
    #[must_use]
    pub fn set_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Sets the document subject.
    #[must_use]
    pub fn set_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    /// Sets the document author.
    #[must_use]
    pub fn set_author(mut self, author: impl Into<String>) -> Self {
        self.author = author.into();
        self
    }

    /// Sets the manager name.
    #[must_use]
    pub fn set_manager(mut self, manager: impl Into<String>) -> Self {
        self.manager = manager.into();
        self
    }

    /// Sets the company name.
    #[must_use]
    pub fn set_company(mut self, company: impl Into<String>) -> Self {
        self.company = company.into();
        self
    }

    /// Sets the document category.
    #[must_use]
    pub fn set_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    /// Sets the document keywords.
    #[must_use]
    pub fn set_keywords(mut self, keywords: impl Into<String>) -> Self {
        self.keywords = keywords.into();
        self
    }

    /// Sets the document comment.
    #[must_use]
    pub fn set_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = comment.into();
        self
    }

    /// Sets the document status, such as `Draft` or `Final`.
    #[must_use]
    pub fn set_status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }

    /// Sets the hyperlink base URL.
    #[must_use]
    pub fn set_hyperlink_base(mut self, hyperlink_base: impl Into<String>) -> Self {
        self.hyperlink_base = hyperlink_base.into();
        self
    }

    /// Sets the document creation time in UTC.
    ///
    /// The same value is written to `dcterms:created` and `dcterms:modified`.
    #[must_use]
    pub fn set_creation_datetime(mut self, create_time: impl IntoCustomDateTime) -> Self {
        self.creation_time = Some(create_time.utc_datetime());
        self
    }

    /// Sets a custom document property.
    ///
    /// Values can be text, `i32`, `f64`, `bool`, or a supported chrono date
    /// type.
    #[must_use]
    pub fn set_custom_property(
        mut self,
        name: impl Into<String>,
        value: impl IntoCustomProperty,
    ) -> Self {
        self.custom_properties.push(value.new_custom_property(name));
        self
    }
}

/// A custom document property stored with its user-defined name.
#[derive(Debug, Clone, PartialEq)]
pub struct CustomProperty {
    pub(crate) name: String,
    pub(crate) value: CustomPropertyValue,
}

impl CustomProperty {
    fn text(name: impl Into<String>, value: String) -> Self {
        Self {
            name: name.into(),
            value: CustomPropertyValue::Text(value),
        }
    }

    const fn int(name: String, value: i32) -> Self {
        Self {
            name,
            value: CustomPropertyValue::Int(value),
        }
    }

    const fn real(name: String, value: f64) -> Self {
        Self {
            name,
            value: CustomPropertyValue::Real(value),
        }
    }

    const fn boolean(name: String, value: bool) -> Self {
        Self {
            name,
            value: CustomPropertyValue::Bool(value),
        }
    }

    fn date(name: impl Into<String>, value: String) -> Self {
        Self {
            name: name.into(),
            value: CustomPropertyValue::DateTime(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CustomPropertyValue {
    Text(String),
    Int(i32),
    Real(f64),
    Bool(bool),
    DateTime(String),
}

/// Types that can be stored as a custom document property.
pub trait IntoCustomProperty {
    /// Converts the value into a named custom property.
    fn new_custom_property(self, name: impl Into<String>) -> CustomProperty;
}

impl IntoCustomProperty for &str {
    fn new_custom_property(self, name: impl Into<String>) -> CustomProperty {
        CustomProperty::text(name, self.to_owned())
    }
}

impl IntoCustomProperty for String {
    fn new_custom_property(self, name: impl Into<Self>) -> CustomProperty {
        CustomProperty::text(name, self)
    }
}

impl IntoCustomProperty for &String {
    fn new_custom_property(self, name: impl Into<String>) -> CustomProperty {
        CustomProperty::text(name, self.clone())
    }
}

impl IntoCustomProperty for i32 {
    fn new_custom_property(self, name: impl Into<String>) -> CustomProperty {
        CustomProperty::int(name.into(), self)
    }
}

impl IntoCustomProperty for f64 {
    fn new_custom_property(self, name: impl Into<String>) -> CustomProperty {
        CustomProperty::real(name.into(), self)
    }
}

impl IntoCustomProperty for bool {
    fn new_custom_property(self, name: impl Into<String>) -> CustomProperty {
        CustomProperty::boolean(name.into(), self)
    }
}

impl IntoCustomProperty for &chrono::NaiveDateTime {
    fn new_custom_property(self, name: impl Into<String>) -> CustomProperty {
        CustomProperty::date(name, self.utc_datetime())
    }
}

impl IntoCustomProperty for &chrono::NaiveDate {
    fn new_custom_property(self, name: impl Into<String>) -> CustomProperty {
        CustomProperty::date(name, self.utc_datetime())
    }
}

impl IntoCustomProperty for &chrono::DateTime<chrono::Utc> {
    fn new_custom_property(self, name: impl Into<String>) -> CustomProperty {
        CustomProperty::date(name, self.utc_datetime())
    }
}

/// Types that convert to an Excel UTC creation timestamp.
pub trait IntoCustomDateTime {
    /// Returns an ISO 8601 UTC timestamp.
    fn utc_datetime(self) -> String;
}

impl IntoCustomDateTime for &chrono::NaiveDateTime {
    fn utc_datetime(self) -> String {
        self.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }
}

impl IntoCustomDateTime for &chrono::NaiveDate {
    fn utc_datetime(self) -> String {
        self.format("%Y-%m-%dT00:00:00Z").to_string()
    }
}

impl IntoCustomDateTime for &chrono::DateTime<chrono::Utc> {
    fn utc_datetime(self) -> String {
        self.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDate, Utc};

    use super::*;

    #[test]
    fn formats_supported_creation_times() {
        let naive = NaiveDate::from_ymd_opt(2026, 11, 18)
            .expect("date")
            .and_hms_opt(12, 34, 56)
            .expect("time");
        assert_eq!((&naive).utc_datetime(), "2026-11-18T12:34:56Z");

        let date = NaiveDate::from_ymd_opt(2026, 11, 18).expect("date");
        assert_eq!((&date).utc_datetime(), "2026-11-18T00:00:00Z");

        let utc: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive, Utc);
        assert_eq!((&utc).utc_datetime(), "2026-11-18T12:34:56Z");
    }

    #[test]
    fn stores_custom_property_types_without_reformatting() {
        let date = NaiveDate::from_ymd_opt(2026, 11, 18).expect("date");
        let properties = DocProperties::new()
            .set_custom_property(String::from("text"), String::from("value"))
            .set_custom_property("int", 7_i32)
            .set_custom_property("real", 1.25_f64)
            .set_custom_property("bool", false)
            .set_custom_property("date", &date);

        assert_eq!(properties.custom_properties.len(), 5);
        assert_eq!(
            properties.custom_properties[0].value,
            CustomPropertyValue::Text(String::from("value"))
        );
        assert_eq!(
            properties.custom_properties[1].value,
            CustomPropertyValue::Int(7)
        );
        assert_eq!(
            properties.custom_properties[2].value,
            CustomPropertyValue::Real(1.25)
        );
        assert_eq!(
            properties.custom_properties[3].value,
            CustomPropertyValue::Bool(false)
        );
        assert_eq!(
            properties.custom_properties[4].value,
            CustomPropertyValue::DateTime(String::from("2026-11-18T00:00:00Z"))
        );
    }
}
