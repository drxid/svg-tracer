//! Настройки прогона. Зеркало `src/lib/settings.ts`; диапазоны проверяются здесь.

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Color,
    Outline,
    Logo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct CropRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preprocess {
    pub crop: Option<CropRect>,
    pub scale: u32,
    /// `auto`, `keep` или `#rrggbb`.
    pub background: String,
    pub background_distance: f32,
    pub crop_to_artwork: bool,
    pub padding: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorSettings {
    pub hierarchical: String,
    pub lines: String,
    pub speckle: u32,
    pub color_precision: i32,
    pub gradient_step: i32,
    pub simplify: f64,
    pub max_colors: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutlineSettings {
    pub threshold: u8,
    pub invert: bool,
    pub speckle: u32,
    pub alphamax: f64,
    pub tolerance: f64,
    pub fill: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogoSettings {
    pub colors: u32,
    pub speckle: u32,
    pub alphamax: f64,
    pub tolerance: f64,
    /// Допуск прямых линий в пикселях исходника, 0 — кривые.
    pub polygon: f64,
    pub fill: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceSettings {
    pub mode: Mode,
    pub preprocess: Preprocess,
    pub color: ColorSettings,
    pub outline: OutlineSettings,
    pub logo: LogoSettings,
}

/// Цвет заливки попадает в SVG, поэтому пропускаем только известные формы.
fn safe_fill(fill: &mut String) {
    let hex = fill.len() == 7
        && fill.starts_with('#')
        && fill[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !hex && fill != "currentColor" {
        *fill = "source".into();
    }
}

impl TraceSettings {
    pub fn clamped(mut self) -> Self {
        let p = &mut self.preprocess;
        p.scale = p.scale.clamp(1, 4);
        p.background_distance = p.background_distance.clamp(5.0, 120.0);
        p.padding = p.padding.min(64);

        let c = &mut self.color;
        c.speckle = c.speckle.min(128);
        c.color_precision = c.color_precision.clamp(1, 8);
        c.gradient_step = c.gradient_step.clamp(0, 255);
        c.simplify = c.simplify.clamp(0.0, 5.0);
        c.max_colors = if c.max_colors < 2 { 0 } else { c.max_colors.min(64) };

        let o = &mut self.outline;
        o.speckle = o.speckle.min(100);
        o.alphamax = o.alphamax.clamp(0.0, 1.34);
        o.tolerance = o.tolerance.clamp(0.0, 1.0);
        safe_fill(&mut o.fill);

        let l = &mut self.logo;
        l.colors = l.colors.clamp(1, 16);
        l.speckle = l.speckle.min(100);
        l.alphamax = l.alphamax.clamp(0.0, 1.34);
        l.tolerance = l.tolerance.clamp(0.0, 1.0);
        l.polygon = if l.polygon <= 0.0 { 0.0 } else { l.polygon.clamp(0.5, 5.0) };
        safe_fill(&mut l.fill);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_fill_never_reaches_svg() {
        for bad in ["\"/><script>", "red", "#12345", "#12345g"] {
            let mut fill = bad.to_string();
            safe_fill(&mut fill);
            assert_eq!(fill, "source");
        }
        let mut fill = "#a1B2c3".to_string();
        safe_fill(&mut fill);
        assert_eq!(fill, "#a1B2c3");
    }
}
