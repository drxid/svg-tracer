//! Загрузка изображений и автоопределение режима.

use crate::error::{AppError, AppResult};
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, ImageReader, RgbaImage};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{BufRead, Cursor, Seek};

const MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_PIXELS: u64 = 64_000_000;
/// Автоопределению не нужны все пиксели, а мелкие детали ему только мешают.
const SUGGEST_SIDE: u32 = 256;
const BACKGROUND_SHARE: f64 = 0.005;
/// Плоская графика: столько площади рисунка занимают её основные цвета.
const FLAT_SHARE: f64 = 0.7;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub mode: &'static str,
    pub colors: u32,
}

fn unsupported() -> AppError {
    AppError::new("unsupported", "Не удалось прочитать файл: нужен PNG, JPEG, WebP, BMP или GIF")
}

pub fn check_size(bytes: u64) -> AppResult<()> {
    if bytes > MAX_FILE_BYTES {
        return Err(AppError::new("too_large", "Файл больше 100 МБ"));
    }
    Ok(())
}

fn decode<R: BufRead + Seek>(reader: ImageReader<R>) -> AppResult<RgbaImage> {
    let mut decoder = reader
        .with_guessed_format()
        .map_err(|_| unsupported())?
        .into_decoder()
        .map_err(|_| unsupported())?;
    let (width, height) = decoder.dimensions();
    if width as u64 * height as u64 > MAX_PIXELS {
        return Err(AppError::new("too_large", "Изображение больше 64 Мп"));
    }
    let orientation = decoder.orientation().map_err(|_| unsupported())?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(|_| unsupported())?;
    image.apply_orientation(orientation);
    Ok(image.into_rgba8())
}

pub fn decode_bytes(bytes: &[u8]) -> AppResult<RgbaImage> {
    check_size(bytes.len() as u64)?;
    decode(ImageReader::new(Cursor::new(bytes)))
}

pub fn encode_preview(image: &RgbaImage) -> AppResult<Vec<u8>> {
    let mut out = Vec::new();
    PngEncoder::new_with_quality(&mut out, CompressionType::Fast, FilterType::Adaptive)
        .write_image(image.as_raw(), image.width(), image.height(), ExtendedColorType::Rgba8)
        .map_err(|e| AppError::new("preview", format!("Не удалось показать изображение: {e}")))?;
    Ok(out)
}

/// 1–2 цвета → «Контур», 3–12 → «Логотип», больше → «Цвет».
pub fn suggest(image: &RgbaImage) -> Suggestion {
    let step_x = (image.width() / SUGGEST_SIDE).max(1) as usize;
    let step_y = (image.height() / SUGGEST_SIDE).max(1) as usize;
    let mut counts: HashMap<[u8; 3], u32> = HashMap::new();
    let mut total = 0u32;
    for y in (0..image.height()).step_by(step_y) {
        for x in (0..image.width()).step_by(step_x) {
            let [r, g, b, a] = image.get_pixel(x, y).0;
            if a >= 128 {
                *counts.entry([r >> 3, g >> 3, b >> 3]).or_default() += 1;
                total += 1;
            }
        }
    }
    let corner = [image.get_pixel(0, 0).0, image.get_pixel(image.width() - 1, 0).0];
    let background: Vec<[u8; 3]> = corner
        .iter()
        .filter(|px| px[3] >= 128)
        .map(|px| [px[0] >> 3, px[1] >> 3, px[2] >> 3])
        .collect();
    let floor = ((total as f64 * BACKGROUND_SHARE) as u32).max(1);
    let foreground: Vec<u32> = counts
        .iter()
        .filter(|(color, _)| !background.contains(color))
        .map(|(_, count)| *count)
        .collect();
    let artwork: u32 = foreground.iter().sum();
    let significant: Vec<u32> = foreground.into_iter().filter(|count| *count >= floor).collect();

    // В градиенте и на фото каждый оттенок редок: значимых цветов «мало», но
    // это не плоская графика, а площадь размазана по тысячам оттенков.
    let covered: u32 = significant.iter().sum();
    if (covered as f64) < artwork as f64 * FLAT_SHARE {
        return Suggestion { mode: "color", colors: 4 };
    }

    match significant.len() as u32 {
        colors @ 0..=2 => Suggestion { mode: "outline", colors: colors.max(1) },
        colors @ 3..=12 => Suggestion { mode: "logo", colors },
        _ => Suggestion { mode: "color", colors: 4 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn flat(colors: &[[u8; 3]]) -> RgbaImage {
        RgbaImage::from_fn(300, 300, |x, y| {
            let [r, g, b] = if y < 30 { [255, 255, 255] } else { colors[x as usize * colors.len() / 300] };
            Rgba([r, g, b, 255])
        })
    }

    #[test]
    fn suggests_by_colour_count() {
        assert_eq!(suggest(&flat(&[[0, 0, 0]])).mode, "outline");
        let logo = suggest(&flat(&[[200, 0, 0], [0, 200, 0], [0, 0, 200], [200, 200, 0]]));
        assert_eq!((logo.mode, logo.colors), ("logo", 4));
        let gradient = RgbaImage::from_fn(300, 300, |x, y| Rgba([x as u8, y as u8, 128, 255]));
        assert_eq!(suggest(&gradient).mode, "color");
    }

    #[test]
    fn rejects_garbage_and_huge_files() {
        assert_eq!(decode_bytes(b"not an image").unwrap_err().code, "unsupported");
        assert_eq!(check_size(MAX_FILE_BYTES + 1).unwrap_err().code, "too_large");
    }
}
