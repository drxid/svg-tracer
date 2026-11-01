//! Тот же прогон, что и в окне, но без окна: для сверки качества.

use crate::error::{AppError, AppResult};
use crate::loader;
use crate::pipeline;
use crate::settings::TraceSettings;
use crate::state::Document;
use std::path::Path;
use std::sync::Arc;
use vtracer::CancelToken;

pub struct Traced {
    pub svg: String,
    pub milliseconds: u128,
}

pub fn trace_file(input: &Path, settings: &str, potrace: Option<&Path>) -> AppResult<Traced> {
    let bytes = std::fs::read(input).map_err(|e| AppError::new("unreadable", e.to_string()))?;
    let settings: TraceSettings = serde_json::from_str(settings)
        .map_err(|e| AppError::new("settings", e.to_string()))?;
    let mut document = Document {
        id: 0,
        source: loader::decode_bytes(&bytes)?,
        preview: Arc::new(Vec::new()),
        work: None,
        mask: None,
        logo: None,
        session: None,
    };
    let started = std::time::Instant::now();
    let output = pipeline::run(&mut document, &settings.clamped(), potrace, &CancelToken::new())?;
    Ok(Traced { svg: output.svg, milliseconds: started.elapsed().as_millis() })
}
