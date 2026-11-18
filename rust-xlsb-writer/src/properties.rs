//! Workbook document metadata.

/// Core and extended document properties written to `docProps/core.xml` and
/// `docProps/app.xml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
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
}
