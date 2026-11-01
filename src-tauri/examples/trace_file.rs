//! `trace_file trace <вход> <настройки.json> <выход.svg> <potrace>` — прогон без окна.
//! `trace_file render <вход.svg> <выход.png> <ширина>` — растеризация для сверки.

use resvg::{tiny_skia, usvg};
use std::path::Path;

fn render(input: &str, output: &str, width: u32) {
    let text = std::fs::read_to_string(input).expect("чтение SVG");
    let tree = usvg::Tree::from_str(&text, &usvg::Options::default()).expect("разбор SVG");
    let size = tree.size();
    let scale = width as f32 / size.width();
    let height = (size.height() * scale).round() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height).expect("холст");
    pixmap.fill(tiny_skia::Color::WHITE);
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    pixmap.save_png(output).expect("запись PNG");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("trace") => {
            let settings = std::fs::read_to_string(&args[2]).expect("чтение настроек");
            let result = svg_tracer_lib::cli::trace_file(
                Path::new(&args[1]),
                &settings,
                Some(Path::new(&args[4])),
            );
            match result {
                Ok(traced) => {
                    std::fs::write(&args[3], traced.svg).expect("запись SVG");
                    println!("{}", traced.milliseconds);
                }
                Err(error) => {
                    eprintln!("{}: {}", error.code, error.message);
                    std::process::exit(1);
                }
            }
        }
        Some("render") => render(&args[1], &args[2], args[3].parse().expect("ширина")),
        _ => eprintln!("trace | render"),
    }
}
