//! sRGB → Lab в шкале OpenCV: L умножен на 2,55. На этой шкале подобран порог
//! чувствительности фона, поэтому она важнее строгого CIE.

use std::sync::OnceLock;

fn linear_table() -> &'static [f32; 256] {
    static TABLE: OnceLock<[f32; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        std::array::from_fn(|i| {
            let v = i as f32 / 255.0;
            if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        })
    })
}

fn f(t: f32) -> f32 {
    if t > 0.008856 { t.cbrt() } else { 7.787 * t + 16.0 / 116.0 }
}

pub fn to_lab(rgb: [u8; 3]) -> [f32; 3] {
    let table = linear_table();
    let [r, g, b] = rgb.map(|v| table[v as usize]);
    let x = f((0.412453 * r + 0.357580 * g + 0.180423 * b) / 0.950456);
    let y = f(0.212671 * r + 0.715160 * g + 0.072169 * b);
    let z = f((0.019334 * r + 0.119193 * g + 0.950227 * b) / 1.088754);
    [(116.0 * y - 16.0) * 2.55, 500.0 * (x - y), 200.0 * (y - z)]
}

pub fn distance_sq(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_opencv_scale() {
        let [l, a, b] = to_lab([255, 255, 255]);
        assert!((l - 255.0).abs() < 0.5 && a.abs() < 0.5 && b.abs() < 0.5);
        // cv2: красный (255,0,0) → L 136, a 208-128, b 195-128.
        let [l, a, b] = to_lab([255, 0, 0]);
        assert!((l - 136.0).abs() < 1.5, "{l}");
        assert!((a - 80.0).abs() < 1.5, "{a}");
        assert!((b - 67.0).abs() < 1.5, "{b}");
    }
}
