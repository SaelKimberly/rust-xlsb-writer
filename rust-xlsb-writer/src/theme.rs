//! Workbook theme parts and theme-file readers.

use std::io::{Cursor, Read};
use std::path::Path;

use zip::ZipArchive;

use crate::{Error, Result};

const THEME_XML_2023: &str = include_str!("theme_2023.xml");

/// Workbook theme selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Theme {
    Excel2023,
    Custom(Box<str>),
}

impl Theme {
    pub(crate) fn xml(&self) -> &str {
        match self {
            Self::Excel2023 => THEME_XML_2023,
            Self::Custom(xml) => xml,
        }
    }

    pub(crate) const fn version(&self) -> u32 {
        match self {
            Self::Excel2023 => 202_300,
            Self::Custom(_) => 0,
        }
    }
}

/// Reads a raw theme XML file, a `.thmx` archive, or an `.xlsx` archive.
pub fn read_theme_from_path(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;

    if let Ok(mut archive) = ZipArchive::new(Cursor::new(&bytes)) {
        for theme_path in ["theme/theme/theme1.xml", "xl/theme/theme1.xml"] {
            let Ok(mut theme_file) = archive.by_name(theme_path) else {
                continue;
            };
            let mut theme_xml = String::new();
            theme_file.read_to_string(&mut theme_xml)?;
            return Ok(theme_xml);
        }

        return Err(theme_error(path, "No theme1.xml found in file"));
    }

    String::from_utf8(bytes).map_err(|_| theme_error(path, "Invalid XML theme file"))
}

pub fn theme_error(path: &Path, message: &str) -> Error {
    Error::ThemeError(format!("{message}: '{}'", path.display()))
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::{Cursor, Read, Write};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use zip::ZipArchive;
    use zip::write::SimpleFileOptions;

    use super::*;
    use crate::Workbook;

    const CUSTOM_THEME: &str = r#"<?xml version="1.0"?><a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"/>"#;
    const XLSX_THEME: &str = r#"<?xml version="1.0"?><a:theme name="xlsx"/>"#;
    const THMX_THEME: &str = r#"<?xml version="1.0"?><a:theme name="thmx"/>"#;

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    fn temp_path(name: &str) -> PathBuf {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "rust_xlsb_writer_{}_{}_{name}",
            std::process::id(),
            id
        ))
    }

    fn write_zip(path: &Path, members: &[(&str, &str)]) {
        let mut archive = zip::ZipWriter::new(File::create(path).expect("zip file"));
        for (name, contents) in members {
            archive
                .start_file(*name, SimpleFileOptions::default())
                .expect("zip member");
            archive
                .write_all(contents.as_bytes())
                .expect("zip member contents");
        }
        archive.finish().expect("zip finishes");
    }

    fn theme_part(bytes: &[u8]) -> String {
        let mut archive = ZipArchive::new(Cursor::new(bytes)).expect("package is a ZIP");
        let mut theme = String::new();
        archive
            .by_name("xl/theme/theme1.xml")
            .expect("theme part")
            .read_to_string(&mut theme)
            .expect("theme UTF-8");
        theme
    }

    #[test]
    fn custom_theme_reads_plain_xml() {
        let path = temp_path("custom.xml");
        fs::write(&path, CUSTOM_THEME).expect("theme file");

        let mut workbook = Workbook::new();
        workbook.use_custom_theme(&path).expect("custom theme");
        let bytes = workbook.save_to_buffer().expect("saves");
        assert_eq!(theme_part(&bytes), CUSTOM_THEME);
    }

    #[test]
    fn custom_theme_reads_xlsx_and_thmx_archives() {
        let xlsx = temp_path("theme.xlsx");
        write_zip(&xlsx, &[("xl/theme/theme1.xml", XLSX_THEME)]);
        let mut workbook = Workbook::new();
        workbook.use_custom_theme(&xlsx).expect("xlsx theme");
        assert_eq!(
            theme_part(&workbook.save_to_buffer().expect("saves")),
            XLSX_THEME
        );

        let thmx = temp_path("theme.thmx");
        write_zip(
            &thmx,
            &[
                ("theme/theme/theme1.xml", THMX_THEME),
                ("xl/theme/theme1.xml", XLSX_THEME),
            ],
        );
        let mut workbook = Workbook::new();
        workbook.use_custom_theme(&thmx).expect("thmx theme");
        assert_eq!(
            theme_part(&workbook.save_to_buffer().expect("saves")),
            THMX_THEME
        );
    }

    #[test]
    fn custom_theme_rejects_invalid_inputs() {
        let missing = temp_path("missing.xlsx");
        write_zip(&missing, &[("xl/workbook.bin", "x")]);
        let mut workbook = Workbook::new();
        assert!(matches!(
            workbook.use_custom_theme(&missing),
            Err(Error::ThemeError(_))
        ));

        let text = temp_path("not_theme.txt");
        fs::write(&text, "not xml").expect("text file");
        assert!(matches!(
            workbook.use_custom_theme(&text),
            Err(Error::ThemeError(_))
        ));

        let image_fill = temp_path("image_fill.xml");
        fs::write(
            &image_fill,
            r#"<?xml version="1.0"?><a:theme><a:blipFill/></a:theme>"#,
        )
        .expect("image-fill theme");
        assert!(matches!(
            workbook.use_custom_theme(&image_fill),
            Err(Error::ThemeError(_))
        ));
    }
}
