//! Режим «Логотип»: порт `trace.py`.

use super::potrace::{self, Params};
use super::svg_root;
use crate::error::AppResult;
use crate::logo::labels::Quantized;
use crate::logo::polygon::polygon_path;
use crate::preprocess::{hex_color, median_color, Frame, Work};
use crate::settings::LogoSettings;
use image::GrayImage;
use imageproc::distance_transform::Norm;
use imageproc::morphology;
use rayon::prelude::*;
use std::path::Path;
use vtracer::CancelToken;

pub struct LogoInput<'a> {
    pub work: &'a Work,
    pub mask: &'a GrayImage,
    pub quantized: Option<&'a Quantized>,
    pub potrace: Option<&'a Path>,
    pub scale: u32,
    pub frame: Frame,
}

pub struct LogoOutput {
    pub svg: String,
    pub palette: Vec<String>,
}

pub fn turdsize(speckle: u32, scale: u32) -> u32 {
    speckle * scale * scale
}

fn outline(
    input: &LogoInput,
    s: &LogoSettings,
    mask: &GrayImage,
    cancel: &CancelToken,
) -> AppResult<String> {
    let turdsize = turdsize(s.speckle, input.scale);
    if s.polygon > 0.0 {
        return Ok(polygon_path(mask, s.polygon, turdsize as f64, input.scale, input.frame));
    }
    let exe = input.potrace.expect("вызывающий код проверил Potrace");
    let params = Params { turdsize, alphamax: s.alphamax, tolerance: s.tolerance };
    potrace::trace_mask(exe, mask, params, input.scale, input.frame, cancel)
}

pub fn render(input: &LogoInput, s: &LogoSettings, cancel: &CancelToken) -> AppResult<LogoOutput> {
    let open = svg_root(input.frame, input.scale);

    let Some(quantized) = input.quantized else {
        let path = outline(input, s, input.mask, cancel)?;
        let fill = if s.fill == "source" {
            hex_color(median_color(&input.work.rgba, input.mask))
        } else {
            s.fill.clone()
        };
        let svg = format!(r#"{open}<path fill="{fill}" fill-rule="evenodd" d="{path}"/></svg>"#);
        let palette = if fill.starts_with('#') { vec![fill] } else { vec![] };
        return Ok(LogoOutput { svg, palette });
    };

    let mask = input.mask;
    let mut areas = vec![0u64; quantized.palette.len()];
    for (label, on) in quantized.labels.iter().zip(mask.as_raw()) {
        if *on > 0 {
            areas[*label as usize] += 1;
        }
    }
    // Слои от большего к меньшему: мелкие лежат поверх перекрытия.
    let mut order: Vec<usize> =
        (0..areas.len()).filter(|i| quantized.palette[*i].is_some()).collect();
    order.sort_by_key(|i| std::cmp::Reverse(areas[*i]));

    let radius = input.scale as u8;
    let grown_mask = morphology::dilate(mask, Norm::L2, radius);
    let silhouette = outline(input, s, mask, cancel)?;

    let (width, height) = mask.dimensions();
    let layers: Vec<(usize, String)> = order
        .par_iter()
        .map(|index| {
            let bytes = quantized
                .labels
                .iter()
                .map(|label| if *label as usize == *index { 255 } else { 0 })
                .collect();
            let layer = GrayImage::from_raw(width, height, bytes).expect("тот же размер");
            let mut layer = morphology::dilate(&layer, Norm::L2, radius);
            layer
                .iter_mut()
                .zip(grown_mask.as_raw())
                .for_each(|(value, grown)| *value = (*value).min(*grown));
            Ok((*index, outline(input, s, &layer, cancel)?))
        })
        .collect::<AppResult<_>>()?;

    let mut svg = format!(
        r#"{open}<clipPath id="shape"><path clip-rule="evenodd" d="{silhouette}"/></clipPath><g clip-path="url(#shape)">"#
    );
    let mut palette = Vec::new();
    for (index, path) in layers {
        if path.is_empty() {
            continue;
        }
        let color = hex_color(quantized.palette[index].expect("пустые цвета отброшены"));
        svg.push_str(&format!(r#"<path fill="{color}" d="{path}"/>"#));
        palette.push(color);
    }
    svg.push_str("</g></svg>");
    Ok(LogoOutput { svg, palette })
}
