//! Предобработка: кадр → увеличение → маска фона.

pub mod lab;

use crate::error::{AppError, AppResult};
use crate::settings::Preprocess;
use image::{imageops, GrayImage, Luma, RgbImage, RgbaImage};
use imageproc::distance_transform::Norm;
use imageproc::morphology;
use lab::{distance_sq, to_lab};
use rayon::prelude::*;

/// Рабочее изображение: после кадра и увеличения.
pub struct Work {
    pub rgba: RgbaImage,
    /// Лёгкое размытие σ = 0,35 × увеличение: глушит шум JPEG.
    pub blurred: RgbImage,
    pub transparent: bool,
}

/// Прямоугольник результата в пикселях рабочего изображения. С полями может
/// выходить за его границы, поэтому координаты знаковые.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
}

pub fn build_work(source: &RgbaImage, p: &Preprocess) -> AppResult<Work> {
    let cropped = match p.crop {
        Some(crop) => {
            let x = crop.x.min(source.width().saturating_sub(1));
            let y = crop.y.min(source.height().saturating_sub(1));
            let width = crop.width.min(source.width() - x);
            let height = crop.height.min(source.height() - y);
            if width == 0 || height == 0 {
                return Err(AppError::new("empty_crop", "Рамка кадра пустая"));
            }
            imageops::crop_imm(source, x, y, width, height).to_image()
        }
        None => source.clone(),
    };
    let rgba = if p.scale > 1 { upscale(&cropped, p.scale) } else { cropped };

    let rgb = RgbImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, _] = rgba.get_pixel(x, y).0;
        image::Rgb([r, g, b])
    });
    let blurred = imageproc::filter::gaussian_blur_f32(&rgb, 0.35 * p.scale as f32);
    let transparent = rgba.pixels().any(|px| px.0[3] < 128);
    Ok(Work { rgba, blurred, transparent })
}

/// Веса кубического ядра с a = -0,75 для дробной части координаты.
fn cubic_weights(t: f32) -> [f32; 4] {
    const A: f32 = -0.75;
    let (t2, t3) = (t * t, t * t * t);
    [
        A * t3 - 2.0 * A * t2 + A * t,
        (A + 2.0) * t3 - (A + 3.0) * t2 + 1.0,
        -(A + 2.0) * t3 + (2.0 * A + 3.0) * t2 - A * t,
        -A * t3 + A * t2,
    ]
}

/// Для каждой выходной координаты — четыре исходных индекса и их веса.
fn cubic_samples(size: u32, scale: u32) -> Vec<([usize; 4], [f32; 4])> {
    (0..size * scale)
        .map(|output| {
            let position = (output as f32 + 0.5) / scale as f32 - 0.5;
            let base = position.floor();
            let clamp = |offset: i64| (base as i64 + offset).clamp(0, size as i64 - 1) as usize;
            ([clamp(-1), clamp(0), clamp(1), clamp(2)], cubic_weights(position - base))
        })
        .collect()
}

/// Увеличение тем же ядром, что в OpenCV (`INTER_CUBIC`). Catmull-Rom из `image`
/// с a = -0,5 даёт край с дрожанием в пиксель: на глаз незаметно, но оптимальный
/// полигон Potrace дробится, и файл выходит вдвое тяжелее.
fn upscale(source: &RgbaImage, scale: u32) -> RgbaImage {
    let (width, height) = source.dimensions();
    let (new_width, new_height) = (width * scale, height * scale);
    let columns = cubic_samples(width, scale);
    let rows = cubic_samples(height, scale);
    let raw = source.as_raw();
    let stride = width as usize * 4;

    // Сначала по горизонтали во float, чтобы не копить округление.
    let row_len = new_width as usize * 4;
    let mut horizontal = vec![0f32; row_len * height as usize];
    horizontal.par_chunks_exact_mut(row_len).zip(raw.par_chunks_exact(stride)).for_each(
        |(target, row)| {
            for (pixel, (indices, kernel)) in target.chunks_exact_mut(4).zip(&columns) {
                for (c, value) in pixel.iter_mut().enumerate() {
                    *value = (0..4).map(|i| row[indices[i] * 4 + c] as f32 * kernel[i]).sum();
                }
            }
        },
    );

    let mut out = vec![0u8; row_len * new_height as usize];
    out.par_chunks_exact_mut(row_len).enumerate().for_each(|(y, target)| {
        let (indices, kernel) = &rows[y];
        for (x, value) in target.iter_mut().enumerate() {
            let sum: f32 = (0..4).map(|i| horizontal[indices[i] * row_len + x] * kernel[i]).sum();
            *value = sum.round().clamp(0.0, 255.0) as u8;
        }
    });
    RgbaImage::from_raw(new_width, new_height, out).expect("размер посчитан выше")
}

fn parse_hex(value: &str) -> Option<[u8; 3]> {
    let value = value.strip_prefix('#')?;
    if value.len() != 6 || !value.is_ascii() {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&value[i..i + 2], 16).ok();
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// Цвет фона: медиана углов 8×8.
fn corner_color(rgb: &RgbImage) -> [u8; 3] {
    let (w, h) = rgb.dimensions();
    let side = 8.min(w).min(h);
    let mut channels: [Vec<u8>; 3] = Default::default();
    for (ox, oy) in [(0, 0), (w - side, 0), (0, h - side), (w - side, h - side)] {
        for y in oy..oy + side {
            for x in ox..ox + side {
                let px = rgb.get_pixel(x, y).0;
                (0..3).for_each(|i| channels[i].push(px[i]));
            }
        }
    }
    channels.map(|mut values| {
        values.sort_unstable();
        values[values.len() / 2]
    })
}

fn binary(width: u32, height: u32, data: Vec<bool>) -> GrayImage {
    let bytes = data.into_iter().map(|on| if on { 255 } else { 0 }).collect();
    GrayImage::from_raw(width, height, bytes).expect("размер маски совпадает с изображением")
}

/// Маска рисунка, 255 — рисунок. Порог фиксированный: Оцу из `trace.py` делит
/// пиксели на две группы и теряет светлые цвета.
pub fn foreground(work: &Work, p: &Preprocess) -> AppResult<GrayImage> {
    let (w, h) = work.rgba.dimensions();
    let opaque = || work.rgba.pixels().map(|px| px.0[3] >= 128).collect::<Vec<_>>();

    let data = if p.background == "keep" || work.transparent {
        opaque()
    } else {
        let reference = parse_hex(&p.background).unwrap_or_else(|| corner_color(&work.blurred));
        let target = to_lab(reference);
        let limit = p.background_distance.powi(2);
        work.blurred
            .as_raw()
            .par_chunks_exact(3)
            .map(|px| distance_sq(to_lab([px[0], px[1], px[2]]), target) > limit)
            .collect()
    };
    let mask = binary(w, h, data);
    let mask = if p.background == "keep" { mask } else { morphology::open(&mask, Norm::L1, 1) };
    if mask.pixels().all(|px| px.0[0] == 0) {
        return Err(AppError::empty_artwork());
    }
    Ok(mask)
}

pub fn frame(mask: &GrayImage, p: &Preprocess) -> Frame {
    let (w, h) = mask.dimensions();
    let (mut left, mut top, mut right, mut bottom) = (0, 0, w as i64 - 1, h as i64 - 1);
    if p.crop_to_artwork {
        (left, top, right, bottom) = (w as i64, h as i64, -1, -1);
        for (x, y, px) in mask.enumerate_pixels() {
            if px.0[0] > 0 {
                left = left.min(x as i64);
                top = top.min(y as i64);
                right = right.max(x as i64);
                bottom = bottom.max(y as i64);
            }
        }
    }
    let pad = (p.padding * p.scale) as i64;
    Frame {
        x: left - pad,
        y: top - pad,
        width: right - left + 1 + 2 * pad,
        height: bottom - top + 1 + 2 * pad,
    }
}

/// Канальная медиана цветов под маской: цвет заливки для одноцветных режимов.
pub fn median_color(rgba: &RgbaImage, mask: &GrayImage) -> [u8; 3] {
    let mut histograms = [[0u64; 256]; 3];
    let mut count = 0u64;
    for (px, on) in rgba.pixels().zip(mask.pixels()) {
        if on.0[0] > 0 {
            (0..3).for_each(|i| histograms[i][px.0[i] as usize] += 1);
            count += 1;
        }
    }
    histograms.map(|histogram| median_of(&histogram, count))
}

pub fn median_of(histogram: &[u64; 256], count: u64) -> u8 {
    let mut seen = 0;
    for (value, n) in histogram.iter().enumerate() {
        seen += n;
        if seen * 2 >= count {
            return value as u8;
        }
    }
    0
}

/// Бинаризация по порогу яркости: тёмное становится рисунком.
pub fn threshold_mask(rgba: &RgbaImage, threshold: u8) -> GrayImage {
    GrayImage::from_fn(rgba.width(), rgba.height(), |x, y| {
        let [r, g, b, a] = rgba.get_pixel(x, y).0.map(|v| v as u32);
        // Прозрачное считаем белым: это фон, а не чёрный рисунок.
        let over_white = |c: u32| (c * a + 255 * (255 - a)) / 255;
        let luma = (299 * over_white(r) + 587 * over_white(g) + 114 * over_white(b)) / 1000;
        Luma([if luma < threshold as u32 { 255 } else { 0 }])
    })
}

pub fn invert(mask: &GrayImage) -> GrayImage {
    let bytes = mask.as_raw().iter().map(|v| 255 - v).collect();
    GrayImage::from_raw(mask.width(), mask.height(), bytes).expect("тот же размер")
}

pub fn hex_color(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn settings() -> Preprocess {
        Preprocess {
            crop: None,
            scale: 1,
            background: "auto".into(),
            background_distance: 40.0,
            crop_to_artwork: true,
            padding: 0,
        }
    }

    fn logo(fill: [u8; 4]) -> RgbaImage {
        RgbaImage::from_fn(64, 64, |x, y| {
            let inside = (16..48).contains(&x) && (20..44).contains(&y);
            Rgba(if inside { fill } else { [255, 255, 255, 255] })
        })
    }

    #[test]
    fn cubic_upscale_keeps_flat_colour() {
        let source = RgbaImage::from_pixel(6, 4, Rgba([10, 200, 90, 255]));
        let out = upscale(&source, 3);
        assert_eq!(out.dimensions(), (18, 12));
        assert!(out.pixels().all(|px| px.0 == [10, 200, 90, 255]));
    }

    #[test]
    fn upscale_matches_opencv_inter_cubic() {
        // cv2.resize(np.array([[0, 255, 0, 255]], np.uint8), (w, 1), INTER_CUBIC)
        let expected: [(u32, &[i32]); 3] = [
            (2, &[0, 67, 224, 215, 40, 31, 188, 255]),
            (3, &[0, 0, 95, 203, 255, 189, 66, 0, 52, 160, 255, 255]),
            (4, &[0, 0, 29, 109, 191, 247, 244, 174, 81, 11, 8, 64, 146, 226, 255, 255]),
        ];
        let source = RgbaImage::from_fn(4, 1, |x, _| {
            let v = if x % 2 == 0 { 0 } else { 255 };
            Rgba([v, v, v, 255])
        });
        for (scale, values) in expected {
            let row: Vec<i32> = upscale(&source, scale).pixels().map(|px| px.0[0] as i32).collect();
            let close = row.iter().zip(values).all(|(a, b)| (a - b).abs() <= 1);
            assert!(close, "×{scale}: {row:?} ≠ {values:?}");
        }
    }

    #[test]
    fn light_colour_survives_background_removal() {
        // Жёлтый ближе к белому, чем любой другой цвет; именно его терял Оцу.
        let source = logo([250, 215, 60, 255]);
        let p = settings();
        let work = build_work(&source, &p).unwrap();
        let mask = foreground(&work, &p).unwrap();
        let on = mask.pixels().filter(|px| px.0[0] > 0).count();
        assert!((on as i64 - 32 * 24).abs() < 80, "{on}");
    }

    #[test]
    fn frame_follows_artwork_and_padding() {
        let source = logo([0, 0, 0, 255]);
        let mut p = settings();
        p.padding = 2;
        let work = build_work(&source, &p).unwrap();
        let mask = foreground(&work, &p).unwrap();
        let box_ = frame(&mask, &p);
        assert_eq!((box_.x, box_.y), (14, 18));
        assert_eq!((box_.width, box_.height), (36, 28));
    }

    #[test]
    fn solid_image_is_reported() {
        let source = RgbaImage::from_pixel(32, 32, Rgba([10, 20, 30, 255]));
        let p = settings();
        let work = build_work(&source, &p).unwrap();
        let error = foreground(&work, &p).unwrap_err();
        assert_eq!(error.code, "empty_artwork");
    }

    #[test]
    fn transparency_wins_over_colour() {
        let mut source = RgbaImage::from_pixel(32, 32, Rgba([255, 255, 255, 0]));
        for y in 8..24 {
            for x in 8..24 {
                source.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        let p = settings();
        let work = build_work(&source, &p).unwrap();
        let on = foreground(&work, &p).unwrap().pixels().filter(|px| px.0[0] > 0).count();
        assert!(on >= 16 * 16 - 8, "{on}");
    }
}
