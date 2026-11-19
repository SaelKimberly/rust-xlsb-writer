//! Worksheet background images.

use std::sync::Arc;

use crate::{Error, Result};

/// An image that can be used as a worksheet background or foreground picture.
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

    /// Returns the image width and height in pixels.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ImageError`] when the header does not contain readable
    /// non-zero dimensions.
    pub(crate) fn pixel_size(&self) -> Result<(u32, u32)> {
        let size = match self.kind {
            ImageType::Png => png_size(&self.data),
            ImageType::Jpeg => jpeg_size(&self.data),
            ImageType::Gif => gif_size(&self.data),
            ImageType::Bmp => bmp_size(&self.data),
        };
        match size {
            Some((width, height)) if width != 0 && height != 0 => Ok((width, height)),
            Some(_) => Err(Error::ImageError("image dimensions are zero".to_owned())),
            None => Err(Error::ImageError("image dimensions unavailable".to_owned())),
        }
    }
}

fn png_size(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 24 || &data[12..16] != b"IHDR" {
        return None;
    }
    Some((
        u32::from_be_bytes([data[16], data[17], data[18], data[19]]),
        u32::from_be_bytes([data[20], data[21], data[22], data[23]]),
    ))
}

fn gif_size(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 10 {
        return None;
    }
    Some((
        u32::from(u16::from_le_bytes([data[6], data[7]])),
        u32::from(u16::from_le_bytes([data[8], data[9]])),
    ))
}

fn bmp_size(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 26 {
        return None;
    }
    let width = i32::from_le_bytes(data[18..22].try_into().ok()?);
    let height = i32::from_le_bytes(data[22..26].try_into().ok()?);
    Some((width.unsigned_abs(), height.unsigned_abs()))
}

fn jpeg_size(data: &[u8]) -> Option<(u32, u32)> {
    let mut pos = 2;
    while pos + 4 <= data.len() {
        if data[pos] != 0xFF {
            return None;
        }
        let marker = data[pos + 1];
        pos += 2;
        if marker == 0xFF {
            pos -= 1;
            continue;
        }
        if marker == 0xD8 || marker == 0xD9 || marker == 0x01 || (0xD0..=0xD7).contains(&marker) {
            continue;
        }
        let length = usize::from(u16::from_be_bytes([data[pos], data[pos + 1]]));
        if length < 2 || pos + length > data.len() {
            return None;
        }
        let is_frame = (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC);
        if is_frame {
            if length < 7 || pos + 7 > data.len() {
                return None;
            }
            let height = u32::from(u16::from_be_bytes([data[pos + 3], data[pos + 4]]));
            let width = u32::from(u16::from_be_bytes([data[pos + 5], data[pos + 6]]));
            return Some((width, height));
        }
        pos += length;
    }
    None
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
    fn reads_native_dimensions() {
        let mut png = [0u8; 24];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[12..16].copy_from_slice(b"IHDR");
        png[16..20].copy_from_slice(&3u32.to_be_bytes());
        png[20..24].copy_from_slice(&4u32.to_be_bytes());
        assert_eq!(
            Image::new_from_buffer(&png)
                .expect("png")
                .pixel_size()
                .expect("size"),
            (3, 4)
        );

        let mut jpeg = vec![0u8; 21];
        jpeg[..11].copy_from_slice(&[
            0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x02, 0x00, 0x03,
        ]);
        assert_eq!(
            Image::new_from_buffer(&jpeg)
                .expect("jpeg")
                .pixel_size()
                .expect("size"),
            (3, 2)
        );

        let mut gif = b"GIF89a".to_vec();
        gif.extend_from_slice(&[5, 0, 6, 0]);
        assert_eq!(
            Image::new_from_buffer(&gif)
                .expect("gif")
                .pixel_size()
                .expect("size"),
            (5, 6)
        );

        let mut bmp = [0u8; 30];
        bmp[..2].copy_from_slice(b"BM");
        bmp[18..22].copy_from_slice(&7i32.to_le_bytes());
        bmp[22..26].copy_from_slice(&(-8i32).to_le_bytes());
        bmp[28] = 24;
        assert_eq!(
            Image::new_from_buffer(&bmp)
                .expect("bmp")
                .pixel_size()
                .expect("size"),
            (7, 8)
        );
    }

    #[test]
    fn rejects_missing_or_zero_dimensions() {
        let png = b"\x89PNG\r\n\x1a\nnot-a-real-image";
        assert!(
            Image::new_from_buffer(png)
                .expect("png")
                .pixel_size()
                .is_err()
        );

        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x02];
        assert!(
            Image::new_from_buffer(&jpeg)
                .expect("jpeg")
                .pixel_size()
                .is_err()
        );

        let mut gif = b"GIF89a".to_vec();
        gif.extend_from_slice(&[0, 0, 6, 0]);
        assert!(
            Image::new_from_buffer(&gif)
                .expect("gif")
                .pixel_size()
                .is_err()
        );
    }

    #[test]
    fn clones_share_image_bytes() {
        let image = Image::new_from_buffer(PNG).expect("png");
        let clone = image.clone();
        assert!(Arc::ptr_eq(&image.data, &clone.data));
    }
}
