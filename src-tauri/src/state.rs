//! Текущее изображение и кэши стадий: подбор параметров движка не пересчитывает
//! предобработку.

use crate::error::{AppError, AppResult};
use crate::logo::labels::Quantized;
use crate::preprocess::{Frame, Work};
use crate::queue::Queue;
use crate::settings::{CropRect, Preprocess};
use image::{GrayImage, RgbaImage};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Настройки, от которых зависит рабочее изображение.
#[derive(Clone, PartialEq)]
pub struct WorkKey {
    pub crop: Option<CropRect>,
    pub scale: u32,
}

/// Настройки, от которых зависит маска.
#[derive(Clone, PartialEq)]
pub struct MaskKey {
    pub work: WorkKey,
    pub background: String,
    pub distance: f32,
}

#[derive(Clone, PartialEq)]
pub struct LogoKey {
    pub mask: MaskKey,
    pub colors: u32,
    pub speckle: u32,
}

#[derive(Clone, PartialEq)]
pub struct SessionKey {
    pub mask: MaskKey,
    pub frame: Frame,
}

impl From<&Preprocess> for WorkKey {
    fn from(p: &Preprocess) -> Self {
        Self { crop: p.crop, scale: p.scale }
    }
}

impl From<&Preprocess> for MaskKey {
    fn from(p: &Preprocess) -> Self {
        Self { work: p.into(), background: p.background.clone(), distance: p.background_distance }
    }
}

pub struct Document {
    pub id: u64,
    pub source: RgbaImage,
    pub preview: Arc<Vec<u8>>,
    pub work: Option<(WorkKey, Arc<Work>)>,
    pub mask: Option<(MaskKey, Arc<GrayImage>)>,
    pub logo: Option<(LogoKey, Arc<Quantized>)>,
    pub session: Option<(SessionKey, vtracer::Session)>,
}

#[derive(Default)]
pub struct AppState {
    pub document: Mutex<Option<Document>>,
    pub queue: Queue,
    next_id: AtomicU64,
}

impl AppState {
    pub fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst) + 1
    }
}

pub fn stale_image() -> AppError {
    AppError::new("no_image", "Изображение уже закрыто")
}

pub fn with_document<T>(
    state: &AppState,
    id: u64,
    action: impl FnOnce(&mut Document) -> AppResult<T>,
) -> AppResult<T> {
    let mut guard = state.document.lock().unwrap();
    match guard.as_mut() {
        Some(document) if document.id == id => action(document),
        _ => Err(stale_image()),
    }
}
