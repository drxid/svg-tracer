//! Метки цветов: квантование, голосование соседей, перенос цвета на край.

use super::kmeans::{kmeans, nearest};
use crate::preprocess::lab::to_lab;
use crate::preprocess::{median_of, Work};
use image::GrayImage;
use imageproc::distance_transform::Norm;
use imageproc::morphology;
use imageproc::region_labelling::{connected_components, Connectivity};
use rayon::prelude::*;

pub struct Quantized {
    pub labels: Vec<u8>,
    /// Индекс — метка; `None` у цвета без единого пикселя.
    pub palette: Vec<Option<[u8; 3]>>,
}

/// Сводит рисунок к `colors` плоским цветам.
pub fn quantize(
    work: &Work,
    mask: &GrayImage,
    colors: usize,
    scale: u32,
    turdsize: u32,
) -> Quantized {
    let (width, height) = mask.dimensions();
    let rgb = work.blurred.as_raw();

    // Выборка только внутри рисунка: пиксели края — смесь с фоном и дали бы
    // ложный оттенок.
    let mut core = morphology::erode(mask, Norm::L2, 4);
    if core.pixels().filter(|px| px.0[0] > 0).count() < colors * 16 {
        core = mask.clone();
    }

    let lab: Vec<[f32; 3]> =
        rgb.par_chunks_exact(3).map(|px| to_lab([px[0], px[1], px[2]])).collect();
    let samples: Vec<[f32; 3]> = lab
        .iter()
        .zip(core.as_raw())
        .filter(|(_, inside)| **inside > 0)
        .map(|(point, _)| *point)
        .collect();
    let centers = kmeans(&samples, colors);
    let labels: Vec<u8> = lab.par_iter().map(|point| nearest(*point, &centers).0 as u8).collect();
    let palette = median_palette(rgb, &labels, core.as_raw(), colors);

    let radius = (scale * 2) as usize;
    let labels = smooth_labels(&labels, colors, width as usize, height as usize, radius);
    let labels = extend_from_core(&labels, &core, turdsize.saturating_mul(8));
    Quantized { labels, palette }
}

fn median_palette(rgb: &[u8], labels: &[u8], core: &[u8], colors: usize) -> Vec<Option<[u8; 3]>> {
    let collect = |only_core: bool| {
        let mut histograms = vec![[[0u64; 256]; 3]; colors];
        let mut counts = vec![0u64; colors];
        for (index, label) in labels.iter().enumerate() {
            if !only_core || core[index] > 0 {
                let label = *label as usize;
                (0..3).for_each(|c| histograms[label][c][rgb[index * 3 + c] as usize] += 1);
                counts[label] += 1;
            }
        }
        (histograms, counts)
    };
    let (inner, inner_counts) = collect(true);
    let (all, all_counts) = collect(false);
    (0..colors)
        .map(|label| {
            let (histogram, count) = if inner_counts[label] > 0 {
                (&inner[label], inner_counts[label])
            } else {
                (&all[label], all_counts[label])
            };
            (count > 0).then(|| std::array::from_fn(|c| median_of(&histogram[c], count)))
        })
        .collect()
}

/// Каждый пиксель голосует с соседями в окне: убирает крошку на стыках цветов.
pub fn smooth_labels(
    labels: &[u8],
    colors: usize,
    width: usize,
    height: usize,
    radius: usize,
) -> Vec<u8> {
    let stride = width + 1;
    let mut table = vec![0u32; stride * (height + 1)];
    let mut best_votes = vec![0u32; labels.len()];
    let mut best = vec![0u8; labels.len()];

    for color in 0..colors as u8 {
        for y in 0..height {
            let mut row = 0;
            for x in 0..width {
                row += (labels[y * width + x] == color) as u32;
                table[(y + 1) * stride + x + 1] = table[y * stride + x + 1] + row;
            }
        }
        let table = &table;
        best_votes
            .par_chunks_mut(width)
            .zip(best.par_chunks_mut(width))
            .enumerate()
            .for_each(|(y, (votes, chosen))| {
                let top = y.saturating_sub(radius);
                let bottom = (y + radius + 1).min(height);
                for x in 0..width {
                    let left = x.saturating_sub(radius);
                    let right = (x + radius + 1).min(width);
                    let count = table[bottom * stride + right] + table[top * stride + left]
                        - table[top * stride + right]
                        - table[bottom * stride + left];
                    if count > votes[x] {
                        votes[x] = count;
                        chosen[x] = color;
                    }
                }
            });
    }
    best
}

/// Даёт пикселям края цвет ближайшего внутреннего пикселя: край — смесь с фоном,
/// он попадает на посторонний полутон и выглядит как бахрома.
pub fn extend_from_core(labels: &[u8], core: &GrayImage, min_area: u32) -> Vec<u8> {
    let (width, height) = core.dimensions();
    let mut keep: Vec<bool> = core.as_raw().iter().map(|v| *v > 0).collect();

    // Мелкие островки — остатки того же смешивания, чаще всего на острых концах.
    if min_area > 0 {
        // Области разделяются по значению, поэтому хватает одного прохода на все цвета.
        let bytes = labels
            .iter()
            .zip(&keep)
            .map(|(label, kept)| if *kept { label + 1 } else { 0 })
            .collect();
        let layer = GrayImage::from_raw(width, height, bytes).expect("тот же размер");
        let regions = connected_components(&layer, Connectivity::Eight, image::Luma([0u8]));
        let raw = regions.as_raw();
        let mut areas = vec![0u32; raw.iter().copied().max().unwrap_or(0) as usize + 1];
        raw.iter().for_each(|region| areas[*region as usize] += 1);
        for (index, region) in raw.iter().enumerate() {
            if *region > 0 && areas[*region as usize] < min_area {
                keep[index] = false;
            }
        }
    }
    if !keep.iter().any(|on| *on) {
        return labels.to_vec();
    }

    let nearest = nearest_seed(&keep, width as usize, height as usize);
    nearest.iter().map(|seed| labels[*seed as usize]).collect()
}

/// Для каждого пикселя — индекс ближайшего затравочного (двупроходный обход,
/// точности хватает для полосы края в несколько пикселей).
fn nearest_seed(seeds: &[bool], width: usize, height: usize) -> Vec<u32> {
    const NONE: u32 = u32::MAX;
    let mut nearest: Vec<u32> = seeds
        .iter()
        .enumerate()
        .map(|(index, on)| if *on { index as u32 } else { NONE })
        .collect();

    let distance = |index: usize, seed: u32| {
        let (x, y) = ((index % width) as i64, (index / width) as i64);
        let (sx, sy) = ((seed as usize % width) as i64, (seed as usize / width) as i64);
        (x - sx).pow(2) + (y - sy).pow(2)
    };
    let relax = |nearest: &mut [u32], index: usize, neighbour: usize| {
        let seed = nearest[neighbour];
        // Затравочные пиксели (а их большинство) уже на месте.
        if nearest[index] == index as u32 {
            return;
        }
        if seed != NONE
            && (nearest[index] == NONE || distance(index, seed) < distance(index, nearest[index]))
        {
            nearest[index] = seed;
        }
    };

    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            if x > 0 {
                relax(&mut nearest, index, index - 1);
            }
            if y > 0 {
                relax(&mut nearest, index, index - width);
                if x > 0 {
                    relax(&mut nearest, index, index - width - 1);
                }
                if x + 1 < width {
                    relax(&mut nearest, index, index - width + 1);
                }
            }
        }
        for x in (0..width.saturating_sub(1)).rev() {
            relax(&mut nearest, y * width + x, y * width + x + 1);
        }
    }
    for y in (0..height).rev() {
        for x in (0..width).rev() {
            let index = y * width + x;
            if x + 1 < width {
                relax(&mut nearest, index, index + 1);
            }
            if y + 1 < height {
                relax(&mut nearest, index, index + width);
                if x + 1 < width {
                    relax(&mut nearest, index, index + width + 1);
                }
                if x > 0 {
                    relax(&mut nearest, index, index + width - 1);
                }
            }
        }
        for x in 1..width {
            relax(&mut nearest, y * width + x, y * width + x - 1);
        }
    }
    nearest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn votes_remove_a_lone_pixel() {
        let mut labels = vec![0u8; 25];
        labels[12] = 1;
        assert_eq!(smooth_labels(&labels, 2, 5, 5, 1), vec![0u8; 25]);
    }

    #[test]
    fn edge_takes_colour_of_nearest_core_pixel() {
        // Левая половина ядра цвета 1, правая цвета 2, край — посторонний 3.
        let width = 8;
        let labels: Vec<u8> = (0..64)
            .map(|i| {
                let (x, y) = (i % width, i / width);
                if x == 0 || x == 7 || y == 0 || y == 7 {
                    3
                } else if x < 4 {
                    1
                } else {
                    2
                }
            })
            .collect();
        let core = GrayImage::from_fn(8, 8, |x, y| {
            image::Luma([if (1..7).contains(&x) && (1..7).contains(&y) { 255 } else { 0 }])
        });
        let out = extend_from_core(&labels, &core, 0);
        assert!(!out.contains(&3));
        assert_eq!((out[0], out[7], out[56], out[63]), (1, 2, 1, 2));
    }

    #[test]
    fn tiny_island_is_absorbed() {
        let mut labels = vec![1u8; 100];
        labels[55] = 2;
        let core = GrayImage::from_pixel(10, 10, image::Luma([255]));
        let out = extend_from_core(&labels, &core, 4);
        assert!(out.iter().all(|label| *label == 1));
    }
}
