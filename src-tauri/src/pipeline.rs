//! Один прогон: настройки → сырой SVG.

use crate::engine::logo::{self, LogoInput};
use crate::engine::potrace::{self, Params};
use crate::engine::{svg_root, vtracer as vt};
use crate::error::{AppError, AppResult};
use crate::logo::labels::{quantize, Quantized};
use crate::preprocess::{self, hex_color, median_color, Frame, Work};
use crate::settings::{Mode, OutlineSettings, TraceSettings};
use crate::state::{Document, LogoKey, MaskKey, SessionKey, WorkKey};
use image::{GrayImage, Rgba, RgbaImage};
use serde::Serialize;
use std::path::Path;
use std::sync::Arc;
use vtracer::CancelToken;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub struct TraceOutput {
    pub svg: String,
    pub bounds: Bounds,
    pub palette: Vec<String>,
}

fn cached_work(document: &mut Document, s: &TraceSettings) -> AppResult<Arc<Work>> {
    let key = WorkKey::from(&s.preprocess);
    if let Some((cached, work)) = &document.work {
        if *cached == key {
            return Ok(work.clone());
        }
    }
    let work = Arc::new(preprocess::build_work(&document.source, &s.preprocess)?);
    document.work = Some((key, work.clone()));
    Ok(work)
}

fn cached_mask(document: &mut Document, s: &TraceSettings, work: &Work) -> AppResult<Arc<GrayImage>> {
    let key = MaskKey::from(&s.preprocess);
    if let Some((cached, mask)) = &document.mask {
        if *cached == key {
            return Ok(mask.clone());
        }
    }
    let mask = Arc::new(preprocess::foreground(work, &s.preprocess)?);
    document.mask = Some((key, mask.clone()));
    Ok(mask)
}

fn cached_labels(
    document: &mut Document,
    s: &TraceSettings,
    work: &Work,
    mask: &GrayImage,
) -> Arc<Quantized> {
    let key = LogoKey {
        mask: MaskKey::from(&s.preprocess),
        colors: s.logo.colors,
        speckle: s.logo.speckle,
    };
    if let Some((cached, quantized)) = &document.logo {
        if *cached == key {
            return quantized.clone();
        }
    }
    let scale = s.preprocess.scale;
    let turdsize = logo::turdsize(s.logo.speckle, scale);
    let quantized = Arc::new(quantize(work, mask, s.logo.colors as usize, scale, turdsize));
    document.logo = Some((key, quantized.clone()));
    quantized
}

/// Вырезает рамку; всё, что не рисунок и что вне изображения, становится
/// прозрачным.
fn frame_image(rgba: &RgbaImage, mask: &GrayImage, frame: Frame, clear: bool) -> RgbaImage {
    let (width, height) = (rgba.width() as i64, rgba.height() as i64);
    RgbaImage::from_fn(frame.width as u32, frame.height as u32, |x, y| {
        let (sx, sy) = (x as i64 + frame.x, y as i64 + frame.y);
        if sx < 0 || sy < 0 || sx >= width || sy >= height {
            return Rgba([0, 0, 0, 0]);
        }
        if clear && mask.get_pixel(sx as u32, sy as u32).0[0] == 0 {
            return Rgba([0, 0, 0, 0]);
        }
        *rgba.get_pixel(sx as u32, sy as u32)
    })
}

fn render_color(
    document: &mut Document,
    s: &TraceSettings,
    work: &Work,
    mask: &GrayImage,
    frame: Frame,
    cancel: &CancelToken,
) -> AppResult<String> {
    let key = SessionKey { mask: MaskKey::from(&s.preprocess), frame };
    let matches = document.session.as_ref().is_some_and(|(cached, _)| *cached == key);
    if !matches {
        let clear = s.preprocess.background != "keep";
        let image = frame_image(&work.rgba, mask, frame, clear);
        document.session = Some((key, vt::new_session(image)));
    }
    let (_, session) = document.session.as_mut().expect("сессия только что создана");
    vt::render(session, &s.color, s.preprocess.scale, cancel)
}

fn render_outline(
    s: &OutlineSettings,
    scale: u32,
    work: &Work,
    mask: &GrayImage,
    frame: Frame,
    exe: &Path,
    cancel: &CancelToken,
) -> AppResult<(String, Vec<String>)> {
    let params = Params {
        turdsize: logo::turdsize(s.speckle, scale),
        alphamax: s.alphamax,
        tolerance: s.tolerance,
    };
    let d = potrace::trace_mask(exe, mask, params, scale, frame, cancel)?;
    let fill = if s.fill == "source" {
        hex_color(median_color(&work.rgba, mask))
    } else {
        s.fill.clone()
    };
    let open = svg_root(frame, scale);
    let svg = format!(r#"{open}<path fill="{fill}" fill-rule="evenodd" d="{d}"/></svg>"#);
    let palette = if fill.starts_with('#') { vec![fill] } else { vec![] };
    Ok((svg, palette))
}

fn bounds(s: &TraceSettings, frame: Frame) -> Bounds {
    let scale = s.preprocess.scale as f64;
    let (ox, oy) = s.preprocess.crop.map_or((0.0, 0.0), |c| (c.x as f64, c.y as f64));
    Bounds {
        x: ox + frame.x as f64 / scale,
        y: oy + frame.y as f64 / scale,
        width: frame.width as f64 / scale,
        height: frame.height as f64 / scale,
    }
}

pub fn run(
    document: &mut Document,
    s: &TraceSettings,
    potrace_exe: Option<&Path>,
    cancel: &CancelToken,
) -> AppResult<TraceOutput> {
    let needs_potrace = match s.mode {
        Mode::Color => false,
        Mode::Outline => true,
        Mode::Logo => s.logo.polygon <= 0.0,
    };
    if needs_potrace && potrace_exe.is_none() {
        return Err(AppError::new("module_missing", "Potrace ещё не загружен"));
    }
    let scale = s.preprocess.scale;
    let check = || if cancel.is_cancelled() { Err(AppError::cancelled()) } else { Ok(()) };

    let work = cached_work(document, s)?;
    check()?;

    // В «Контуре» с «Оставить» рисунок задаёт порог, а не удаление фона.
    let outline_threshold = s.mode == Mode::Outline && s.preprocess.background == "keep";
    let mut mask = if outline_threshold {
        Arc::new(preprocess::threshold_mask(&work.rgba, s.outline.threshold))
    } else {
        cached_mask(document, s, &work)?
    };
    if s.mode == Mode::Outline && s.outline.invert {
        mask = Arc::new(preprocess::invert(&mask));
    }
    if mask.pixels().all(|px| px.0[0] == 0) {
        return Err(AppError::empty_artwork());
    }
    check()?;
    let frame = preprocess::frame(&mask, &s.preprocess);

    let (svg, palette) = match s.mode {
        Mode::Color => (render_color(document, s, &work, &mask, frame, cancel)?, vec![]),
        Mode::Outline => {
            let exe = potrace_exe.expect("проверено выше");
            render_outline(&s.outline, scale, &work, &mask, frame, exe, cancel)?
        }
        Mode::Logo => {
            let quantized =
                (s.logo.colors > 1).then(|| cached_labels(document, s, &work, &mask));
            check()?;
            let input = LogoInput {
                work: &work,
                mask: &mask,
                quantized: quantized.as_deref(),
                potrace: potrace_exe,
                scale,
                frame,
            };
            let output = logo::render(&input, &s.logo, cancel)?;
            (output.svg, output.palette)
        }
    };
    Ok(TraceOutput { svg, bounds: bounds(s, frame), palette })
}
