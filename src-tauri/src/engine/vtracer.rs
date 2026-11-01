//! Весь контакт с крейтом VTracer. Это alpha: при откате на 0.6 меняется только
//! этот файл.

use crate::error::{AppError, AppResult};
use crate::settings::ColorSettings;
use image::RgbaImage;
use vtracer::{CancelToken, ColorImage, Config, FitMode, Hierarchical, Session};

pub fn new_session(rgba: RgbaImage) -> Session {
    let (width, height) = (rgba.width() as usize, rgba.height() as usize);
    Session::new(ColorImage { pixels: rgba.into_raw(), width, height })
}

fn config(c: &ColorSettings, scale: u32) -> Config {
    Config {
        hierarchical: if c.hierarchical == "cutout" {
            Hierarchical::Cutout
        } else {
            Hierarchical::Stacked
        },
        mode: match c.lines.as_str() {
            "polygon" => FitMode::Polygon,
            "pixel" => FitMode::Pixel,
            _ => FitMode::Spline,
        },
        filter_speckle: (c.speckle * scale) as usize,
        color_precision: c.color_precision,
        layer_difference: c.gradient_step,
        simplify: (c.simplify > 0.0).then_some(c.simplify * scale as f64),
        max_colors: (c.max_colors >= 2).then_some(c.max_colors as usize),
        // Оптимизацией занимается SVGO, иначе статистика «до и после» нечестная.
        optimize: 0,
        ..Config::default()
    }
}

/// Корень заменяем своим: без `width`/`height`, а `viewBox` совпадает с рамкой.
fn with_view_box(svg: &str, width: u32, height: u32) -> String {
    let body = svg.find("<svg").and_then(|start| svg[start..].find('>').map(|end| start + end + 1));
    let inner = &svg[body.unwrap_or(0)..];
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}">{inner}"#)
}

pub fn render(
    session: &mut Session,
    c: &ColorSettings,
    scale: u32,
    cancel: &CancelToken,
) -> AppResult<String> {
    let cfg = config(c, scale);
    let (width, height) = (session.image().width as u32, session.image().height as u32);
    let doc = session.render_with_progress(&cfg, cancel, &mut |_| {}).map_err(|e| match e {
        vtracer::Error::Cancelled => AppError::cancelled(),
        other => AppError::new("vtracer", format!("VTracer: {other}")),
    })?;
    let pipeline = cfg.build().map_err(|e| AppError::new("vtracer", format!("VTracer: {e}")))?;
    Ok(with_view_box(&pipeline.writer.write(&doc), width, height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn settings() -> ColorSettings {
        ColorSettings {
            hierarchical: "stacked".into(),
            lines: "spline".into(),
            speckle: 4,
            color_precision: 6,
            gradient_step: 16,
            simplify: 1.0,
            max_colors: 0,
        }
    }

    #[test]
    fn traces_two_colours_with_our_root() {
        let image = RgbaImage::from_fn(64, 48, |x, _| {
            Rgba(if x < 32 { [220, 30, 30, 255] } else { [30, 30, 220, 255] })
        });
        let mut session = new_session(image);
        let svg = render(&mut session, &settings(), 1, &CancelToken::new()).unwrap();
        assert!(
            svg.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 48">"#),
            "{svg}"
        );
        assert!(svg.contains("<path"), "{svg}");
        assert_eq!(svg.matches("<svg").count(), 1);
    }

    #[test]
    fn cancelled_token_stops_the_run() {
        let image = RgbaImage::from_pixel(32, 32, Rgba([1, 2, 3, 255]));
        let mut session = new_session(image);
        let cancel = CancelToken::new();
        cancel.cancel();
        let error = render(&mut session, &settings(), 1, &cancel).unwrap_err();
        assert!(error.is_cancelled());
    }
}
