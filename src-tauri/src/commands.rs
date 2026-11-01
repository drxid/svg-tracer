use crate::error::{AppError, AppResult};
use crate::loader::{self, Suggestion};
use crate::modules::{self, ModuleState, Modules};
use crate::pipeline::{self, Bounds};
use crate::settings::TraceSettings;
use crate::state::{with_document, AppState, Document};
use image::RgbaImage;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tauri::ipc::{InvokeBody, Request, Response};
use tauri::{AppHandle, Manager, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInfo {
    image_id: u64,
    name: String,
    width: u32,
    height: u32,
    has_alpha: bool,
    bytes: u64,
    suggested: Suggestion,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceOutcome {
    run_id: u64,
    cancelled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    svg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bounds: Option<Bounds>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    palette: Option<Vec<String>>,
}

impl TraceOutcome {
    fn cancelled(run_id: u64) -> Self {
        Self { run_id, cancelled: true, svg: None, bounds: None, ms: None, palette: None }
    }
}

async fn blocking<T: Send + 'static>(
    action: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(action)
        .await
        .map_err(|e| AppError::new("internal", format!("Внутренняя ошибка: {e}")))?
}

fn install(state: &AppState, image: RgbaImage, name: String, bytes: u64) -> AppResult<ImageInfo> {
    let suggested = loader::suggest(&image);
    let preview = Arc::new(loader::encode_preview(&image)?);
    let info = ImageInfo {
        image_id: state.next_id(),
        name,
        width: image.width(),
        height: image.height(),
        has_alpha: image.pixels().any(|px| px.0[3] < 255),
        bytes,
        suggested,
    };
    state.queue.cancel_all();
    *state.document.lock().unwrap() = Some(Document {
        id: info.image_id,
        source: image,
        preview,
        work: None,
        mask: None,
        logo: None,
        session: None,
    });
    Ok(info)
}

#[tauri::command]
pub async fn load_image_path(app: AppHandle, path: PathBuf) -> AppResult<ImageInfo> {
    blocking(move || {
        let unreadable = |_| AppError::new("unreadable", "Не удалось открыть файл");
        let metadata = std::fs::metadata(&path).map_err(unreadable)?;
        loader::check_size(metadata.len())?;
        let bytes = std::fs::read(&path).map_err(unreadable)?;
        let image = loader::decode_bytes(&bytes)?;
        let name = path.file_name().map_or_else(
            || "image".to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        install(&app.state::<AppState>(), image, name, metadata.len())
    })
    .await
}

/// Тело запроса — сырые байты картинки из буфера обмена.
#[tauri::command]
pub async fn load_image_bytes(app: AppHandle, request: Request<'_>) -> AppResult<ImageInfo> {
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err(AppError::new("unsupported", "Ожидались байты изображения"));
    };
    let bytes = bytes.clone();
    blocking(move || {
        let image = loader::decode_bytes(&bytes)?;
        let size = bytes.len() as u64;
        install(&app.state::<AppState>(), image, "clipboard.png".into(), size)
    })
    .await
}

#[tauri::command]
pub async fn get_preview(state: State<'_, AppState>, image_id: u64) -> AppResult<Response> {
    let preview = with_document(&state, image_id, |document| Ok(document.preview.clone()))?;
    Ok(Response::new(preview.as_ref().clone()))
}

#[tauri::command]
pub async fn trace(
    app: AppHandle,
    image_id: u64,
    run_id: u64,
    settings: TraceSettings,
) -> AppResult<TraceOutcome> {
    blocking(move || {
        let state = app.state::<AppState>();
        let Some(cancel) = state.queue.enter(run_id) else {
            return Ok(TraceOutcome::cancelled(run_id));
        };
        let started = Instant::now();
        let settings = settings.clamped();
        let potrace = app.state::<Modules>().potrace();
        let result = with_document(&state, image_id, |document| {
            pipeline::run(document, &settings, potrace.as_deref(), &cancel)
        });
        state.queue.leave();

        match result {
            Ok(output) => Ok(TraceOutcome {
                run_id,
                cancelled: false,
                svg: Some(output.svg),
                bounds: Some(output.bounds),
                ms: Some(started.elapsed().as_millis() as u64),
                palette: Some(output.palette),
            }),
            Err(error) if error.is_cancelled() => Ok(TraceOutcome::cancelled(run_id)),
            Err(error) => Err(error),
        }
    })
    .await
}

#[tauri::command]
pub fn cancel_trace(state: State<'_, AppState>) {
    state.queue.cancel_all();
}

#[tauri::command]
pub async fn save_file(path: PathBuf, contents: String) -> AppResult<()> {
    blocking(move || {
        std::fs::write(&path, contents)
            .map_err(|e| AppError::new("save", format!("Не удалось сохранить файл: {e}")))
    })
    .await
}

#[tauri::command]
pub fn modules_state(modules: State<'_, Modules>) -> ModuleState {
    modules.state()
}

#[tauri::command]
pub fn modules_retry(app: AppHandle) {
    modules::start(app);
}
