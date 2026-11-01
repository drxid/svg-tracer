//! Контуры из прямых линий: для графики без кривых.

use crate::engine::number;
use crate::preprocess::Frame;
use image::GrayImage;
use imageproc::contours::find_contours;
use imageproc::geometry::approximate_polygon_dp;
use imageproc::point::Point;

fn area(points: &[Point<i32>]) -> f64 {
    let twice: i64 = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| a.x as i64 * b.y as i64 - b.x as i64 * a.y as i64)
        .sum();
    (twice as f64 / 2.0).abs()
}

pub fn polygon_path(
    mask: &GrayImage,
    epsilon: f64,
    min_area: f64,
    scale: u32,
    frame: Frame,
) -> String {
    let scale = scale as f64;
    let mut out = String::new();
    for contour in find_contours::<i32>(mask) {
        if area(&contour.points) < min_area {
            continue;
        }
        let points = approximate_polygon_dp(&contour.points, epsilon * scale, true);
        for (index, point) in points.iter().enumerate() {
            // Контур идёт по центрам пикселей, отсюда сдвиг на половину.
            let x = (point.x as f64 + 0.5 - frame.x as f64) / scale;
            let y = (point.y as f64 + 0.5 - frame.y as f64) / scale;
            out.push(if index == 0 { 'M' } else { 'L' });
            out.push_str(&format!("{} {}", number(x), number(y)));
        }
        out.push('Z');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_collapses_to_four_points() {
        let mask = GrayImage::from_fn(32, 32, |x, y| {
            image::Luma([if (8..24).contains(&x) && (8..24).contains(&y) { 255 } else { 0 }])
        });
        let frame = Frame { x: 0, y: 0, width: 32, height: 32 };
        let d = polygon_path(&mask, 1.0, 16.0, 1, frame);
        assert_eq!(d.matches('L').count(), 3, "{d}");
        assert!(d.ends_with('Z'));
    }

    #[test]
    fn small_contours_are_dropped() {
        let mut mask = GrayImage::new(32, 32);
        mask.put_pixel(4, 4, image::Luma([255]));
        let frame = Frame { x: 0, y: 0, width: 32, height: 32 };
        assert_eq!(polygon_path(&mask, 1.0, 16.0, 1, frame), "");
    }
}
