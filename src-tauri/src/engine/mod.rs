pub mod logo;
pub mod potrace;
pub mod vtracer;

use crate::preprocess::Frame;

/// Число для SVG: два знака, без хвостовых нулей.
pub fn number(value: f64) -> String {
    let text = format!("{value:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" { "0".into() } else { text.into() }
}

/// Открывающий тег: размер в пикселях исходника.
pub fn svg_root(frame: Frame, scale: u32) -> String {
    let width = number(frame.width as f64 / scale as f64);
    let height = number(frame.height as f64 / scale as f64);
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}">"#)
}
