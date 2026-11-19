//! Worksheet background and foreground images.

use std::sync::Arc;

use crate::{Error, Result};

/// An image that can be used as a worksheet background or foreground picture.
///
/// The supported formats are PNG, JPEG, GIF, and 24-bit BMP. The image is read
/// when the object is created, so later file changes do not affect the saved
/// workbook. Native dimensions and DPI are parsed once; foreground insertion
/// uses them to compute the displayed size without changing the media bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    data: Arc<[u8]>,
    kind: ImageType,
    info: Option<ImageInfo>,
    scale_width: f64,
    scale_height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ImageInfo {
    width: u32,
    height: u32,
    width_dpi: f64,
    height_dpi: f64,
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
        let info = match kind {
            ImageType::Png => png_info(&data),
            ImageType::Jpeg => jpeg_info(&data),
            ImageType::Gif => gif_info(&data),
            ImageType::Bmp => bmp_info(&data),
        };
        Ok(Self {
            data: Arc::from(data),
            kind,
            info,
            scale_width: 1.0,
            scale_height: 1.0,
        })
    }

    pub(crate) fn data(&self) -> &[u8] {
        &self.data
    }

    pub(crate) fn same_media(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.data, &other.data) || self.data == other.data
    }

    pub(crate) const fn extension(&self) -> &'static str {
        self.kind.extension()
    }

    pub(crate) const fn content_type(&self) -> &'static str {
        self.kind.content_type()
    }

    /// Returns the image width in pixels.
    ///
    /// Returns `0.0` when the header does not contain readable dimensions.
    #[must_use]
    pub fn width(&self) -> f64 {
        self.info.map_or(0.0, |info| f64::from(info.width))
    }

    /// Returns the image height in pixels.
    ///
    /// Returns `0.0` when the header does not contain readable dimensions.
    #[must_use]
    pub fn height(&self) -> f64 {
        self.info.map_or(0.0, |info| f64::from(info.height))
    }

    /// Returns the horizontal image DPI.
    ///
    /// Images without DPI metadata use Excel's default of 96 DPI.
    #[must_use]
    pub fn width_dpi(&self) -> f64 {
        self.info.map_or(96.0, |info| info.width_dpi)
    }

    /// Returns the vertical image DPI.
    ///
    /// Images without DPI metadata use Excel's default of 96 DPI.
    #[must_use]
    pub fn height_dpi(&self) -> f64 {
        self.info.map_or(96.0, |info| info.height_dpi)
    }

    /// Sets the logical image width in pixels before DPI adjustment.
    ///
    /// The displayed size is this width multiplied by `96 / width_dpi`. This
    /// changes the logical size only; the media bytes are not rescaled. A width
    /// of zero is ignored.
    #[must_use]
    pub fn set_width(mut self, width: u32) -> Self {
        if width == 0 {
            return self;
        }
        if let Some(info) = self.info.filter(|info| info.width != 0) {
            self.scale_width = f64::from(width) / f64::from(info.width);
        }
        self
    }

    /// Sets the logical image height in pixels before DPI adjustment.
    ///
    /// The displayed size is this height multiplied by `96 / height_dpi`. This
    /// changes the logical size only; the media bytes are not rescaled. A
    /// height of zero is ignored.
    #[must_use]
    pub fn set_height(mut self, height: u32) -> Self {
        if height == 0 {
            return self;
        }
        if let Some(info) = self.info.filter(|info| info.height != 0) {
            self.scale_height = f64::from(height) / f64::from(info.height);
        }
        self
    }

    /// Sets the horizontal scale relative to `1.0` (100%).
    ///
    /// Non-finite and non-positive scales are ignored.
    #[must_use]
    pub fn set_scale_width(mut self, scale: f64) -> Self {
        if scale.is_finite() && scale > 0.0 {
            self.scale_width = scale;
        }
        self
    }

    /// Sets the vertical scale relative to `1.0` (100%).
    ///
    /// Non-finite and non-positive scales are ignored.
    #[must_use]
    pub fn set_scale_height(mut self, scale: f64) -> Self {
        if scale.is_finite() && scale > 0.0 {
            self.scale_height = scale;
        }
        self
    }

    /// Scales the image to a target pixel width and height.
    ///
    /// When `keep_aspect_ratio` is `true`, the lesser of the horizontal and
    /// vertical scales is used for both axes. Non-finite and zero targets are
    /// ignored.
    #[must_use]
    pub fn set_scale_to_size<T>(mut self, width: T, height: T, keep_aspect_ratio: bool) -> Self
    where
        T: Into<f64> + Copy,
    {
        let width = width.into();
        let height = height.into();
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return self;
        }
        let Some(info) = self.info.filter(|info| {
            info.width != 0 && info.height != 0 && info.width_dpi > 0.0 && info.height_dpi > 0.0
        }) else {
            return self;
        };

        let mut scale_width = (width / f64::from(info.width)) * (info.width_dpi / 96.0);
        let mut scale_height = (height / f64::from(info.height)) * (info.height_dpi / 96.0);
        if keep_aspect_ratio {
            if scale_width < scale_height {
                scale_height = scale_width;
            } else {
                scale_width = scale_height;
            }
        }
        if scale_width.is_finite() && scale_height.is_finite() {
            self.scale_width = scale_width;
            self.scale_height = scale_height;
        }
        self
    }

    fn dimensions(&self) -> Result<ImageInfo> {
        let Some(info) = self.info else {
            return Err(Error::ImageError("image dimensions unavailable".to_owned()));
        };
        if info.width == 0 || info.height == 0 {
            return Err(Error::ImageError("image dimensions are zero".to_owned()));
        }
        if info.width_dpi <= 0.0 || info.height_dpi <= 0.0 {
            return Err(Error::ImageError("image DPI is invalid".to_owned()));
        }
        Ok(info)
    }

    #[expect(
        clippy::float_cmp,
        reason = "the default scale is exactly 1.0; non-default scales come from setters"
    )]
    pub(crate) fn is_scaled(&self) -> bool {
        self.scale_width != 1.0 || self.scale_height != 1.0
    }

    pub(crate) fn scaled_size(&self) -> Result<(f64, f64)> {
        let info = self.dimensions()?;
        let width = f64::from(info.width) * self.scale_width * 96.0 / info.width_dpi;
        let height = f64::from(info.height) * self.scale_height * 96.0 / info.height_dpi;
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(Error::ImageError(
                "scaled image dimensions are invalid".to_owned(),
            ));
        }
        Ok((width, height))
    }
}

fn png_info(data: &[u8]) -> Option<ImageInfo> {
    if data.len() < 24 || &data[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let height = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    let mut width_dpi = 96.0;
    let mut height_dpi = 96.0;
    let mut offset = 8usize;
    while offset.checked_add(8)? <= data.len() {
        let length = usize::try_from(u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]))
        .ok()?;
        let end = offset.checked_add(length)?.checked_add(12)?;
        if end > data.len() {
            break;
        }
        let marker = &data[offset + 4..offset + 8];
        if marker == b"pHYs" && length >= 9 {
            let units = data[offset + 16];
            let x_density = u32::from_be_bytes([
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
            ]);
            let y_density = u32::from_be_bytes([
                data[offset + 12],
                data[offset + 13],
                data[offset + 14],
                data[offset + 15],
            ]);
            if units == 1 && x_density > 0 && y_density > 0 {
                width_dpi = f64::from(x_density) * 0.0254;
                height_dpi = f64::from(y_density) * 0.0254;
            }
        }
        if marker == b"IEND" {
            break;
        }
        offset = end;
    }
    Some(ImageInfo {
        width,
        height,
        width_dpi,
        height_dpi,
    })
}

fn gif_info(data: &[u8]) -> Option<ImageInfo> {
    if data.len() < 10 {
        return None;
    }
    Some(ImageInfo {
        width: u32::from(u16::from_le_bytes([data[6], data[7]])),
        height: u32::from(u16::from_le_bytes([data[8], data[9]])),
        width_dpi: 96.0,
        height_dpi: 96.0,
    })
}

fn bmp_info(data: &[u8]) -> Option<ImageInfo> {
    if data.len() < 26 {
        return None;
    }
    let width = i32::from_le_bytes(data[18..22].try_into().ok()?);
    let height = i32::from_le_bytes(data[22..26].try_into().ok()?);
    Some(ImageInfo {
        width: width.unsigned_abs(),
        height: height.unsigned_abs(),
        width_dpi: 96.0,
        height_dpi: 96.0,
    })
}

fn jpeg_info(data: &[u8]) -> Option<ImageInfo> {
    let mut pos = 2;
    let mut width = 0;
    let mut height = 0;
    let mut width_dpi = 96.0;
    let mut height_dpi = 96.0;
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
            height = u32::from(u16::from_be_bytes([data[pos + 3], data[pos + 4]]));
            width = u32::from(u16::from_be_bytes([data[pos + 5], data[pos + 6]]));
        }
        if marker == 0xE0
            && length >= 16
            && pos + 16 <= data.len()
            && &data[pos + 2..pos + 7] == b"JFIF\0"
        {
            let units = data[pos + 9];
            let x_density = u16::from_be_bytes([data[pos + 10], data[pos + 11]]);
            let y_density = u16::from_be_bytes([data[pos + 12], data[pos + 13]]);
            if units == 1 {
                if x_density > 1 {
                    width_dpi = f64::from(x_density);
                }
                if y_density > 1 {
                    height_dpi = f64::from(y_density);
                }
            } else if units == 2 {
                if x_density > 1 {
                    width_dpi = f64::from(x_density) * 2.54;
                }
                if y_density > 1 {
                    height_dpi = f64::from(y_density) * 2.54;
                }
            }
        }
        if marker == 0xDA {
            break;
        }
        pos += length;
    }
    if width == 0 || height == 0 {
        return None;
    }
    Some(ImageInfo {
        width,
        height,
        width_dpi,
        height_dpi,
    })
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

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut png = [0u8; 24];
        png[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        png[12..16].copy_from_slice(b"IHDR");
        png[16..20].copy_from_slice(&width.to_be_bytes());
        png[20..24].copy_from_slice(&height.to_be_bytes());
        png.to_vec()
    }

    fn png_with_dpi(width: u32, height: u32, x_density: u32, y_density: u32) -> Vec<u8> {
        let mut png = Vec::with_capacity(8 + 25 + 21);
        png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        png.extend_from_slice(&13u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&width.to_be_bytes());
        png.extend_from_slice(&height.to_be_bytes());
        png.extend_from_slice(&[8, 2, 0, 0, 0]);
        png.extend_from_slice(&[0; 4]);
        png.extend_from_slice(&9u32.to_be_bytes());
        png.extend_from_slice(b"pHYs");
        png.extend_from_slice(&x_density.to_be_bytes());
        png.extend_from_slice(&y_density.to_be_bytes());
        png.extend_from_slice(&[1, 0, 0, 0, 0]);
        png
    }

    fn jpeg_with_dpi(width: u16, height: u16, x_density: u16, y_density: u16) -> Vec<u8> {
        let mut jpeg = Vec::new();
        jpeg.extend_from_slice(&[0xFF, 0xD8]);
        jpeg.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x10]);
        jpeg.extend_from_slice(b"JFIF\0");
        jpeg.extend_from_slice(&[1, 1, 1]);
        jpeg.extend_from_slice(&x_density.to_be_bytes());
        jpeg.extend_from_slice(&y_density.to_be_bytes());
        jpeg.extend_from_slice(&[0, 0]);
        jpeg.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 8]);
        jpeg.extend_from_slice(&height.to_be_bytes());
        jpeg.extend_from_slice(&width.to_be_bytes());
        jpeg.resize(jpeg.len() + 12, 0);
        jpeg
    }

    #[test]
    fn reads_native_dimensions() {
        assert_eq!(
            Image::new_from_buffer(&png(3, 4)).expect("png").width(),
            3.0
        );

        let jpeg = jpeg_with_dpi(3, 2, 96, 96);
        assert_eq!(Image::new_from_buffer(&jpeg).expect("jpeg").height(), 2.0);

        let mut gif = b"GIF89a".to_vec();
        gif.extend_from_slice(&[5, 0, 6, 0]);
        assert_eq!(Image::new_from_buffer(&gif).expect("gif").height(), 6.0);

        let mut bmp = [0u8; 30];
        bmp[..2].copy_from_slice(b"BM");
        bmp[18..22].copy_from_slice(&7i32.to_le_bytes());
        bmp[22..26].copy_from_slice(&(-8i32).to_le_bytes());
        bmp[28] = 24;
        assert_eq!(Image::new_from_buffer(&bmp).expect("bmp").width(), 7.0);
    }

    #[test]
    fn reads_image_dpi() {
        let png_with_phys = png_with_dpi(100, 50, 11_811, 11_811);
        let image = Image::new_from_buffer(&png_with_phys).expect("png");
        assert!((image.width_dpi() - 299.999_4).abs() < 0.001);
        assert!((image.height_dpi() - 299.999_4).abs() < 0.001);

        let jpeg = jpeg_with_dpi(100, 50, 300, 200);
        let image = Image::new_from_buffer(&jpeg).expect("jpeg");
        assert_eq!(image.width_dpi(), 300.0);
        assert_eq!(image.height_dpi(), 200.0);

        let jpeg = jpeg_with_dpi(100, 50, 1, 300);
        let image = Image::new_from_buffer(&jpeg).expect("jpeg");
        assert_eq!(image.width_dpi(), 96.0);
        assert_eq!(image.height_dpi(), 300.0);

        let image = Image::new_from_buffer(&png(10, 10)).expect("png");
        assert_eq!(image.width_dpi(), 96.0);
        assert_eq!(image.height_dpi(), 96.0);
    }

    #[test]
    fn scale_changes_the_logical_size() {
        let image = Image::new_from_buffer(&png(100, 50)).expect("png");
        assert_eq!(image.width(), 100.0);
        assert_eq!(image.height(), 50.0);

        let image = image.set_width(80);
        assert_eq!(image.scaled_size().expect("scaled"), (80.0, 50.0));

        let image = image.set_height(25);
        assert_eq!(image.scaled_size().expect("scaled"), (80.0, 25.0));

        let image = image.set_scale_width(0.5).set_scale_height(2.0);
        assert_eq!(image.scaled_size().expect("scaled"), (50.0, 100.0));

        let image = image.set_scale_to_size(200, 140, false);
        assert_eq!(image.scaled_size().expect("scaled"), (200.0, 140.0));

        let image = image.set_scale_to_size(200, 140, true);
        assert_eq!(image.scaled_size().expect("scaled"), (200.0, 100.0));

        let image = image.set_scale_width(0.0).set_scale_height(f64::NAN);
        assert_eq!(image.scaled_size().expect("scaled"), (200.0, 100.0));

        let image = image.set_width(0).set_height(0);
        assert_eq!(image.scaled_size().expect("scaled"), (200.0, 100.0));
    }

    #[test]
    fn dpi_scales_the_displayed_size() {
        let jpeg = jpeg_with_dpi(100, 50, 300, 300);
        let image = Image::new_from_buffer(&jpeg).expect("jpeg");
        let (width, height) = image.scaled_size().expect("scaled");
        assert!((width - 32.0).abs() < 0.001);
        assert!((height - 16.0).abs() < 0.001);

        let image = image.set_scale_to_size(100, 100, false);
        let (width, height) = image.scaled_size().expect("scaled");
        assert!((width - 100.0).abs() < 0.001);
        assert!((height - 100.0).abs() < 0.001);
    }

    #[test]
    fn rejects_missing_or_zero_dimensions() {
        let png = b"\x89PNG\r\n\x1a\nnot-a-real-image";
        assert!(
            Image::new_from_buffer(png)
                .expect("png")
                .scaled_size()
                .is_err()
        );

        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x02];
        assert!(
            Image::new_from_buffer(&jpeg)
                .expect("jpeg")
                .scaled_size()
                .is_err()
        );

        let mut gif = b"GIF89a".to_vec();
        gif.extend_from_slice(&[0, 0, 6, 0]);
        assert!(
            Image::new_from_buffer(&gif)
                .expect("gif")
                .scaled_size()
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
