use deku::{DekuRead, DekuWrite};

use crate::prelude::{RichStr, UncheckedRfX, XlWideString};
use crate::{Checked, CheckedBiff, KnownID};

/// Starts one comment; MS-XLSB `2.4.61`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtBeginComment {
    pub iauthor: u32,
    pub rfx: UncheckedRfX,
    pub guid: [u8; 16],
}

impl Checked for BrtBeginComment {
    fn check(&self) -> Result<(), &'static str> {
        self.rfx.check()?;
        if self.rfx.rwFirst != self.rfx.rwLast {
            return Err("comment range rows must be equal");
        }
        if self.rfx.colFirst != self.rfx.colLast {
            return Err("comment range columns must be equal");
        }
        Ok(())
    }
}

impl CheckedBiff for BrtBeginComment {
    fn id(&self) -> KnownID {
        KnownID::BrtBeginComment
    }
}

/// Ends one comment; MS-XLSB `2.4.339`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtEndComment {}

impl crate::prelude::Unchecked for BrtEndComment {}

impl CheckedBiff for BrtEndComment {
    fn id(&self) -> KnownID {
        KnownID::BrtEndComment
    }
}

/// One comment author; MS-XLSB `2.4.340`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtCommentAuthor {
    pub author: XlWideString,
}

impl Checked for BrtCommentAuthor {
    fn check(&self) -> Result<(), &'static str> {
        if self.author.is_empty() || self.author.len() > 54 {
            return Err("comment author must contain 1 to 54 UTF-16 code units");
        }
        Ok(())
    }
}

impl CheckedBiff for BrtCommentAuthor {
    fn id(&self) -> KnownID {
        KnownID::BrtCommentAuthor
    }
}

/// Comment rich text; MS-XLSB `2.4.341`.
#[derive(DekuRead, DekuWrite, Debug, Hash, PartialEq, Eq, Default)]
pub struct BrtCommentText {
    pub text: RichStr,
}

impl Checked for BrtCommentText {
    fn check(&self) -> Result<(), &'static str> {
        if !self.text.has_rich_runs() {
            return Err("comment text must set fRichStr");
        }
        self.text.check()
    }
}

impl CheckedBiff for BrtCommentText {
    fn id(&self) -> KnownID {
        KnownID::BrtCommentText
    }
}

#[cfg(test)]
mod tests {
    use deku::{DekuContainerRead, DekuContainerWrite};

    use crate::Checked;
    use crate::prelude::{
        BrtBeginComment, BrtCommentAuthor, BrtCommentText, RichStr, StrRun, StrRuns, UncheckedRfX,
        XlStringExt, XlWideString,
    };

    #[test]
    fn begin_comment_roundtrips_cell_and_guid() {
        let record = BrtBeginComment {
            iauthor: 2,
            rfx: UncheckedRfX {
                rwFirst: 7,
                rwLast: 7,
                colFirst: 3,
                colLast: 3,
            },
            guid: [0xAB; 16],
        };
        let bytes = record.to_bytes().expect("serialize comment");
        assert_eq!(bytes.len(), 36);
        let (rest, decoded) = BrtBeginComment::from_bytes((&bytes, 0)).expect("deserialize");
        assert_eq!(rest.0.len(), 0);
        assert_eq!(decoded, record);
        record.check().expect("valid comment");
    }

    #[test]
    fn comment_author_and_text_roundtrip() {
        let author = BrtCommentAuthor {
            author: XlWideString::construct(Some("Alice")),
        };
        let text = BrtCommentText {
            text: RichStr {
                flags: RichStr::F_RICH_STR,
                str: XlWideString::construct(Some("hi")),
                rich: Some(StrRuns {
                    dwSizeStrRun: 1,
                    rgsStrRun: vec![StrRun { ich: 0, ifnt: 0 }],
                }),
                phonetic: None,
            },
        };

        let author_bytes = author.to_bytes().expect("serialize author");
        assert_eq!(
            author_bytes,
            [5, 0, 0, 0, b'A', 0, b'l', 0, b'i', 0, b'c', 0, b'e', 0]
        );
        let (_, decoded_author) =
            BrtCommentAuthor::from_bytes((&author_bytes, 0)).expect("deserialize author");
        assert_eq!(decoded_author, author);
        author.check().expect("valid author");

        let text_bytes = text.to_bytes().expect("serialize text");
        assert_eq!(text_bytes[0], RichStr::F_RICH_STR);
        let (_, decoded_text) =
            BrtCommentText::from_bytes((&text_bytes, 0)).expect("deserialize text");
        assert_eq!(decoded_text, text);
        text.check().expect("valid text");
    }
}
