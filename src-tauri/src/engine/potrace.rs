//! Запуск `potrace.exe` через stdin/stdout и разбор его вывода.

use super::number;
use crate::error::{AppError, AppResult};
use crate::preprocess::Frame;
use image::GrayImage;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use vtracer::CancelToken;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// Potrace пишет координаты в единицах 1/UNIT пикселя.
const UNIT: f64 = 10.0;

#[derive(Debug, Clone, Copy)]
pub struct Params {
    pub turdsize: u32,
    pub alphamax: f64,
    pub tolerance: f64,
}

/// PBM P4: 1 — чёрный, строки выровнены по байту.
pub fn encode_pbm(mask: &GrayImage) -> Vec<u8> {
    let (w, h) = mask.dimensions();
    let mut out = format!("P4\n{w} {h}\n").into_bytes();
    let row_bytes = w.div_ceil(8) as usize;
    out.reserve(row_bytes * h as usize);
    for row in mask.as_raw().chunks_exact(w as usize) {
        let mut bytes = vec![0u8; row_bytes];
        for (x, value) in row.iter().enumerate() {
            if *value > 0 {
                bytes[x / 8] |= 0x80 >> (x % 8);
            }
        }
        out.extend_from_slice(&bytes);
    }
    out
}

/// Обводит маску и возвращает `d` в координатах результата.
pub fn trace_mask(
    exe: &Path,
    mask: &GrayImage,
    params: Params,
    scale: u32,
    frame: Frame,
    cancel: &CancelToken,
) -> AppResult<String> {
    if cancel.is_cancelled() {
        return Err(AppError::cancelled());
    }
    let pbm = encode_pbm(mask);
    let mut command = Command::new(exe);
    command
        .args(["--backend", "svg", "--flat", "--output", "-"])
        .args(["--unit", &UNIT.to_string()])
        .args(["--turdsize", &params.turdsize.to_string()])
        .args(["--alphamax", &params.alphamax.to_string()])
        .args(["--opttolerance", &params.tolerance.to_string()])
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let spawn_error =
        |e: std::io::Error| AppError::new("potrace", format!("Potrace не запустился: {e}"));
    let mut child = command.spawn().map_err(spawn_error)?;
    let mut stdin = child.stdin.take().expect("stdin задан");
    let mut stdout = child.stdout.take().expect("stdout задан");
    let mut stderr = child.stderr.take().expect("stderr задан");

    // Запись и чтение идут параллельно, иначе буферы канала заблокируют обоих.
    let output = std::thread::scope(|scope| {
        scope.spawn(move || {
            let _ = stdin.write_all(&pbm);
        });
        let reader = scope.spawn(move || {
            let mut text = String::new();
            stdout.read_to_string(&mut text).map(|_| text)
        });
        let errors = scope.spawn(move || {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            text
        });
        let status = loop {
            if cancel.is_cancelled() {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AppError::cancelled());
            }
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => std::thread::sleep(Duration::from_millis(2)),
                Err(e) => return Err(spawn_error(e)),
            }
        };
        let text = reader.join().expect("поток чтения").map_err(spawn_error)?;
        let errors = errors.join().expect("поток ошибок");
        if !status.success() {
            let message = format!("Potrace завершился с ошибкой: {}", errors.trim());
            return Err(AppError::new("potrace", message));
        }
        Ok(text)
    })?;

    Ok(convert_svg(&output, mask.height() as f64, scale as f64, frame))
}

enum Token {
    Command(char),
    Number(f64),
}

fn tokenize(d: &str) -> Vec<Token> {
    let bytes = d.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_alphabetic() {
            tokens.push(Token::Command(c));
            i += 1;
        } else if c.is_ascii_digit() || c == '-' || c == '.' {
            let start = i;
            i += 1;
            while i < bytes.len() && matches!(bytes[i] as char, '0'..='9' | '.') {
                i += 1;
            }
            tokens.push(Token::Number(d[start..i].parse().unwrap_or(0.0)));
        } else {
            i += 1;
        }
    }
    tokens
}

/// Переводит вывод Potrace в абсолютные координаты результата: единицы 1/10 с
/// отражением по Y становятся пикселями исходника, начало — угол рамки.
pub fn convert_svg(svg: &str, height: f64, scale: f64, frame: Frame) -> String {
    let map = |x: f64, y: f64| {
        let px = (x / UNIT - frame.x as f64) / scale;
        let py = (height - y / UNIT - frame.y as f64) / scale;
        format!("{} {}", number(px), number(py))
    };

    let mut out = String::new();
    let mut rest = svg;
    while let Some(start) = rest.find("d=\"") {
        rest = &rest[start + 3..];
        let end = rest.find('"').unwrap_or(rest.len());
        let tokens = tokenize(&rest[..end]);
        rest = &rest[end..];

        let (mut cx, mut cy, mut sx, mut sy) = (0.0, 0.0, 0.0, 0.0);
        let mut command = ' ';
        let mut i = 0;
        while i < tokens.len() {
            if let Token::Command(c) = tokens[i] {
                command = c;
                i += 1;
                if matches!(c, 'z' | 'Z') {
                    out.push('Z');
                    (cx, cy) = (sx, sy);
                }
                continue;
            }
            let arity = match command {
                'm' | 'M' | 'l' | 'L' => 2,
                'c' | 'C' => 6,
                _ => {
                    i += 1;
                    continue;
                }
            };
            let args: Vec<f64> = tokens[i..]
                .iter()
                .take(arity)
                .map_while(|t| if let Token::Number(v) = t { Some(*v) } else { None })
                .collect();
            if args.len() < arity {
                break;
            }
            i += arity;

            let relative = command.is_ascii_lowercase();
            let (bx, by) = if relative { (cx, cy) } else { (0.0, 0.0) };
            if arity == 6 {
                let (x, y) = (bx + args[4], by + args[5]);
                out.push_str(&format!(
                    "C{} {} {}",
                    map(bx + args[0], by + args[1]),
                    map(bx + args[2], by + args[3]),
                    map(x, y)
                ));
                (cx, cy) = (x, y);
            } else {
                let (x, y) = (bx + args[0], by + args[1]);
                let moving = matches!(command, 'm' | 'M');
                out.push(if moving { 'M' } else { 'L' });
                out.push_str(&map(x, y));
                (cx, cy) = (x, y);
                if moving {
                    (sx, sy) = (x, y);
                    // Продолжение после moveto по стандарту означает lineto.
                    command = if relative { 'l' } else { 'L' };
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: Frame = Frame { x: 0, y: 0, width: 40, height: 40 };

    #[test]
    fn pbm_bits_are_msb_first() {
        let mask = GrayImage::from_fn(9, 1, |x, _| {
            image::Luma([if x == 0 || x == 8 { 255 } else { 0 }])
        });
        assert_eq!(encode_pbm(&mask), b"P4\n9 1\n\x80\x80");
    }

    #[test]
    fn relative_path_becomes_absolute_and_flips_y() {
        let svg = r#"<g transform="translate(0,40) scale(0.1,-0.1)"><path d="M100 300 l100 0 0 -100 c0 -10 -10 -20 -20 -20 z m10 -10 l5 5 z"/></g>"#;
        let d = convert_svg(svg, 40.0, 1.0, FRAME);
        assert_eq!(d, "M10 10L20 10L20 20C20 21 19 22 18 22ZM11 11L11.5 10.5Z");
    }

    #[test]
    fn scale_and_frame_move_coordinates_to_source_pixels() {
        let frame = Frame { x: 8, y: 4, width: 32, height: 32 };
        let d = convert_svg(r#"<path d="M200 300 l40 0 z"/>"#, 40.0, 4.0, frame);
        assert_eq!(d, "M3 1.5L4 1.5Z");
    }
}
