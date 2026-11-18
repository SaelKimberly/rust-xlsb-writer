//! Hyperlink value and URL normalization.

use std::borrow::Cow;
use std::fmt::Write as _;

use crate::{Error, Result};

const MAX_URL_UTF16: usize = 2_080;
const MAX_TOOLTIP_UTF16: usize = 255;

/// A URL or workbook-internal hyperlink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    pub(crate) link: String,
    pub(crate) text: String,
    pub(crate) tooltip: String,
    pub(crate) target: String,
    pub(crate) location: String,
    pub(crate) link_type: HyperlinkType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyperlinkType {
    Url,
    File,
    Internal,
}

impl Url {
    /// Creates a hyperlink from a URL, file path, `mailto:`, or `internal:`
    /// target.
    #[must_use]
    pub fn new(link: impl Into<String>) -> Self {
        let link = link.into();
        Self {
            text: String::new(),
            tooltip: String::new(),
            target: String::new(),
            location: String::new(),
            link_type: HyperlinkType::Url,
            link,
        }
    }

    /// Sets the visible cell text.
    #[must_use]
    pub fn set_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    /// Sets the mouse-over tooltip.
    #[must_use]
    pub fn set_tip(mut self, tip: impl Into<String>) -> Self {
        self.tooltip = tip.into();
        self
    }

    pub(crate) fn initialize(&mut self) -> Result<()> {
        if let Some(rest) = self.link.strip_prefix("internal:") {
            self.link_type = HyperlinkType::Internal;
            self.location = rest.to_owned();
            if self.text.is_empty() {
                self.text.clone_from(&self.location);
            }
        } else if let Some(rest) = self.link.strip_prefix("mailto:") {
            self.link_type = HyperlinkType::Url;
            self.target = escape_url(&self.link).into_owned();
            if self.text.is_empty() {
                self.text = rest.to_owned();
            }
        } else if let Some(path) = self
            .link
            .strip_prefix("file:///")
            .or_else(|| self.link.strip_prefix("file://"))
        {
            self.link_type = HyperlinkType::File;
            let (base, anchor) = path.split_once('#').unwrap_or((path, ""));
            self.location = anchor.to_owned();
            if Self::is_relative_path(path) {
                self.target = escape_url(base).into_owned();
            } else {
                let full = self
                    .link
                    .split_once('#')
                    .map_or(self.link.as_str(), |(v, _)| v);
                self.target = escape_url(full).into_owned();
            }
            if self.text.is_empty() {
                self.text = path.to_owned();
            }
        } else if self.link.contains("://") {
            let (base, anchor) = self
                .link
                .split_once('#')
                .unwrap_or((self.link.as_str(), ""));
            self.target = escape_url(base).into_owned();
            self.location = anchor.to_owned();
            if self.text.is_empty() {
                self.text.clone_from(&self.link);
            }
        } else {
            return Err(Error::UnknownUrlType(self.link.clone()));
        }

        if !self.is_internal() && self.target.is_empty() {
            return Err(Error::UnknownUrlType(self.link.clone()));
        }
        if self.link.encode_utf16().count() > MAX_URL_UTF16
            || self.target.encode_utf16().count() > MAX_URL_UTF16
            || self.location.encode_utf16().count() > MAX_URL_UTF16
        {
            return Err(Error::MaxUrlLengthExceeded);
        }
        if self.tooltip.encode_utf16().count() > MAX_TOOLTIP_UTF16 {
            return Err(Error::TooltipTooLong);
        }
        Ok(())
    }

    pub(crate) const fn is_internal(&self) -> bool {
        matches!(self.link_type, HyperlinkType::Internal)
    }

    fn is_relative_path(path: &str) -> bool {
        if path.starts_with(r"\\") {
            return false;
        }
        !matches!(
            path.find(':'),
            Some(1) if path.is_char_boundary(1) && path.as_bytes()[0].is_ascii()
        )
    }
}

impl From<&str> for Url {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<&Self> for Url {
    fn from(value: &Self) -> Self {
        value.clone()
    }
}

impl From<String> for Url {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

fn escape_url(input: &str) -> Cow<'_, str> {
    if is_escaped(input)
        || !input.chars().any(|ch| {
            matches!(
                ch,
                '%' | '"' | ' ' | '<' | '>' | '[' | ']' | '^' | '`' | '{' | '}'
            )
        })
    {
        return Cow::Borrowed(input);
    }

    let mut escaped = String::with_capacity(input.len() + 8);
    for ch in input.chars() {
        match ch {
            '%' => escaped.push_str("%25"),
            '"' => escaped.push_str("%22"),
            ' ' => escaped.push_str("%20"),
            '<' => escaped.push_str("%3c"),
            '>' => escaped.push_str("%3e"),
            '[' => escaped.push_str("%5b"),
            ']' => escaped.push_str("%5d"),
            '^' => escaped.push_str("%5e"),
            '`' => escaped.push_str("%60"),
            '{' => escaped.push_str("%7b"),
            '}' => escaped.push_str("%7d"),
            _ => escaped.push(ch),
        }
    }
    Cow::Owned(escaped)
}

fn is_escaped(url: &str) -> bool {
    if !url.contains('%') {
        return false;
    }

    url.as_bytes().windows(3).any(|window| {
        matches!(
            window,
            b"%20"
                | b"%25"
                | b"%22"
                | b"%60"
                | b"%3c"
                | b"%3C"
                | b"%3e"
                | b"%3E"
                | b"%5b"
                | b"%5B"
                | b"%5d"
                | b"%5D"
                | b"%5e"
                | b"%5E"
                | b"%7b"
                | b"%7B"
                | b"%7d"
                | b"%7D"
        )
    })
}

/// Escapes text for XML character data, including Excel's `_xHHHH_` form for
/// control characters and Unicode non-characters.
pub fn escape_xml_data(input: &str) -> Cow<'_, str> {
    if !input.chars().any(needs_xml_data_escape) {
        return Cow::Borrowed(input);
    }

    let mut escaped = String::with_capacity(input.len() + 8);
    for ch in input.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\u{0000}'..='\u{0008}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'..='\u{001F}'
            | '\u{FFFE}'
            | '\u{FFFF}' => {
                write!(escaped, "_x{:04X}_", u32::from(ch)).expect("writing to String cannot fail");
            }
            _ => escaped.push(ch),
        }
    }
    Cow::Owned(escaped)
}

const fn needs_xml_data_escape(ch: char) -> bool {
    matches!(
        ch,
        '&' | '<'
            | '>'
            | '\u{0000}'..='\u{0008}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'..='\u{001F}'
            | '\u{FFFE}'
            | '\u{FFFF}'
    )
}

pub fn escape_xml(input: &str) -> Cow<'_, str> {
    if !input.chars().any(|ch| matches!(ch, '&' | '"' | '<' | '>')) {
        return Cow::Borrowed(input);
    }

    let mut escaped = String::with_capacity(input.len() + 8);
    for ch in input.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '"' => escaped.push_str("&quot;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            _ => escaped.push(ch),
        }
    }
    Cow::Owned(escaped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(link: &str) -> Url {
        let mut url = Url::new(link);
        url.initialize().expect("valid url");
        url
    }

    #[test]
    fn escapes_xml_data_control_characters_and_non_characters() {
        assert_eq!(escape_xml_data("plain text"), "plain text");
        assert_eq!(
            escape_xml_data("a\u{1}b\r\nc\u{FFFE}d\u{FFFF}e"),
            "a_x0001_b_x000D_\nc_xFFFE_d_xFFFF_e"
        );
        assert_eq!(escape_xml_data("<&>"), "&lt;&amp;&gt;");
    }

    #[test]
    fn parses_external_urls_and_fragments() {
        let url = parsed("https://example.com/a b#anchor");
        assert_eq!(url.target, "https://example.com/a%20b");
        assert_eq!(url.location, "anchor");
        assert_eq!(url.text, "https://example.com/a b#anchor");
        assert!(!url.is_internal());
    }

    #[test]
    fn parses_mailto_internal_and_file_links() {
        let mail = parsed("mailto:user@example.com");
        assert_eq!(mail.target, "mailto:user@example.com");
        assert_eq!(mail.text, "user@example.com");

        let internal = parsed("internal:Sheet2!A1");
        assert!(internal.is_internal());
        assert_eq!(internal.location, "Sheet2!A1");
        assert_eq!(internal.text, "Sheet2!A1");
        assert_eq!(internal.target, "");

        let file = parsed("file://reports/summary.xlsx#Sheet1!A1");
        assert_eq!(file.target, "reports/summary.xlsx");
        assert_eq!(file.location, "Sheet1!A1");
        assert_eq!(file.text, "reports/summary.xlsx#Sheet1!A1");

        let absolute = parsed("file:///C:/My%20Files/report.xlsx");
        assert_eq!(absolute.target, "file:///C:/My%20Files/report.xlsx");
    }

    #[test]
    fn rejects_unknown_and_oversized_links() {
        let mut unknown = Url::new("not a url");
        assert!(matches!(
            unknown.initialize(),
            Err(Error::UnknownUrlType(_))
        ));

        let mut long = Url::new(format!("https://example.com/{}", "x".repeat(MAX_URL_UTF16)));
        assert!(matches!(
            long.initialize(),
            Err(Error::MaxUrlLengthExceeded)
        ));

        let mut tooltip = Url::new("https://example.com").set_tip("x".repeat(256));
        assert!(matches!(tooltip.initialize(), Err(Error::TooltipTooLong)));

        let mut empty_file = Url::new("file://");
        assert!(matches!(
            empty_file.initialize(),
            Err(Error::UnknownUrlType(_))
        ));

        let mut escaped_too_long =
            Url::new(format!("https://example.com/{}", " ".repeat(MAX_URL_UTF16)));
        assert!(matches!(
            escaped_too_long.initialize(),
            Err(Error::MaxUrlLengthExceeded)
        ));
    }
}
