//! Worksheet background images.

use std::sync::Arc;

use crate::{Error, Result};

/// An image that can be used as a worksheet background.
///
/// The supported formats are PNG, JPEG, GIF, and 24-bit BMP. The image is read
/// when the object is created, so later file changes do not affect the saved
/// workbook.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    data: Arc<[u8]>,
    kind: ImageType,
}

impl Image {
    /// Creates an image from an image file path.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when the file cannot be read, or
    /// [`Error::ImageError`] when the bytes are not a supported image.
    pub fn new(path: impl AsRef<std::path::Path>) -> Result<Self> {
        Self::from_vec(std::fs::read(path)?)
    }

    /// Creates an image from an in-memory buffer.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ImageError`] when the bytes are not a supported image.
    pub fn new_from_buffer(buffer: &[u8]) -> Result<Self> {
        Self::from_vec(buffer.to_vec())
    }

    fn from_vec(data: Vec<u8>) -> Result<Self> {
        let Some(kind) = ImageType::from_bytes(&data) else {
            return Err(Error::ImageError("unsupported image format".to_owned()));
        };
        Ok(Self {
            data: Arc::from(data),
            kind,
        })
    }

    pub(crate) fn data(&self) -> &[u8] {
        &self.data
    }

    pub(crate) const fn extension(&self) -> &'static str {
        self.kind.extension()
    }

    pub(crate) const fn content_type(&self) -> &'static str {
        self.kind.content_type()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageType {
    Png,
    Jpeg,
    Gif,
    Bmp,
}

impl ImageType {
    fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(Self::Png)
        } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(Self::Jpeg)
        } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
            Some(Self::Gif)
        } else if data.starts_with(b"BM")
            && data.len() >= 30
            && u16::from_le_bytes([data[28], data[29]]) == 24
        {
            Some(Self::Bmp)
        } else {
            None
        }
    }

    const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Gif => "gif",
            Self::Bmp => "bmp",
        }
    }

    const fn content_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Bmp => "image/bmp",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nnot-a-real-image";
    const JPEG: &[u8] = b"\xFF\xD8\xFF\xE0rest";
    const GIF: &[u8] = b"GIF89a-rest";

    #[test]
    fn detects_supported_image_magic_bytes() {
        let mut bmp = [0u8; 30];
        bmp[..2].copy_from_slice(b"BM");
        bmp[28] = 24;
        let cases = [
            (PNG, "png", "image/png"),
            (JPEG, "jpg", "image/jpeg"),
            (GIF, "gif", "image/gif"),
            (&bmp, "bmp", "image/bmp"),
        ];
        for (bytes, extension, content_type) in cases {
            let image = Image::new_from_buffer(bytes).expect("supported image");
            assert_eq!(image.extension(), extension);
            assert_eq!(image.content_type(), content_type);
            assert_eq!(image.data(), bytes);
        }
    }

    #[test]
    fn rejects_non_24_bit_bmp() {
        let mut bmp = [0u8; 30];
        bmp[..2].copy_from_slice(b"BM");
        bmp[28] = 32;
        assert!(Image::new_from_buffer(&bmp).is_err());
    }

    #[test]
    fn rejects_unknown_bytes() {
        let error = Image::new_from_buffer(b"not an image").expect_err("unknown image");
        assert!(matches!(error, Error::ImageError(_)));
    }

    #[test]
    fn clones_share_image_bytes() {
        let image = Image::new_from_buffer(PNG).expect("png");
        let clone = image.clone();
        assert!(Arc::ptr_eq(&image.data, &clone.data));
    }
}
