//! Внешние модули: докачиваются сами, проверяются по SHA-256.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

struct Module {
    name: &'static str,
    version: &'static str,
    url: &'static str,
    archive_sha256: &'static str,
    entry: &'static str,
    file: &'static str,
    file_sha256: &'static str,
}

const POTRACE: Module = Module {
    name: "potrace",
    version: "1.16",
    url: "https://potrace.sourceforge.net/download/1.16/potrace-1.16.win64.zip",
    archive_sha256: "63699f9885b5ed053977fd94452a7bdbb77ee62b77bf28a3f17b2f14b9b9c65d",
    entry: "potrace-1.16.win64/potrace.exe",
    file: "potrace.exe",
    file_sha256: "f1bcde040ae6576dc9c5868be7a72064957c5aea08e1dece240428319fc672a4",
};

const MAX_ARCHIVE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ModuleState {
    Checking,
    Downloading { percent: u32 },
    Ready,
    Failed { message: String },
}

pub struct Modules {
    state: Mutex<ModuleState>,
    busy: AtomicBool,
    potrace: Mutex<Option<PathBuf>>,
}

impl Default for Modules {
    fn default() -> Self {
        Self {
            state: Mutex::new(ModuleState::Checking),
            busy: AtomicBool::new(false),
            potrace: Mutex::new(None),
        }
    }
}

impl Modules {
    pub fn state(&self) -> ModuleState {
        self.state.lock().unwrap().clone()
    }

    pub fn potrace(&self) -> Option<PathBuf> {
        self.potrace.lock().unwrap().clone()
    }
}

fn publish(app: &AppHandle, state: ModuleState) {
    let modules = app.state::<Modules>();
    let mut current = modules.state.lock().unwrap();
    if *current != state {
        *current = state.clone();
        let _ = app.emit("modules://state", state);
    }
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn module_dir(app: &AppHandle, module: &Module) -> Result<PathBuf, String> {
    let base = app.path().local_data_dir().map_err(|e| e.to_string())?;
    Ok(base.join("svg-tracer").join("modules").join(module.name).join(module.version))
}

fn is_valid(path: &Path, module: &Module) -> bool {
    std::fs::read(path).map(|bytes| sha256(&bytes) == module.file_sha256).unwrap_or(false)
}

fn download(app: &AppHandle, module: &Module) -> Result<Vec<u8>, String> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(module.url)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|_| "Нет соединения с potrace.sourceforge.net".to_string())?;

    let total = response.content_length().unwrap_or(0).min(MAX_ARCHIVE_BYTES);
    let mut bytes = Vec::with_capacity(total as usize);
    let mut chunk = [0u8; 16 * 1024];
    loop {
        let read = response.read(&mut chunk).map_err(|_| "Загрузка оборвалась".to_string())?;
        if read == 0 {
            return Ok(bytes);
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.len() as u64 > MAX_ARCHIVE_BYTES {
            return Err("Файл модуля больше ожидаемого".into());
        }
        if total > 0 {
            let percent = (bytes.len() as u64 * 100 / total).min(99) as u32;
            publish(app, ModuleState::Downloading { percent });
        }
    }
}

fn unpack(archive: &[u8], module: &Module) -> Result<Vec<u8>, String> {
    let broken = |_| "Архив модуля повреждён".to_string();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive)).map_err(broken)?;
    let mut entry = zip.by_name(module.entry).map_err(|_| "В архиве нет программы".to_string())?;
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut bytes).map_err(|_| "Архив модуля повреждён".to_string())?;
    Ok(bytes)
}

fn install(app: &AppHandle, module: &Module, target: &Path) -> Result<(), String> {
    publish(app, ModuleState::Downloading { percent: 0 });
    let archive = download(app, module)?;
    if sha256(&archive) != module.archive_sha256 {
        return Err("Контрольная сумма архива не совпала".into());
    }
    let binary = unpack(&archive, module)?;
    if sha256(&binary) != module.file_sha256 {
        return Err("Контрольная сумма программы не совпала".into());
    }

    let io_error = |e: std::io::Error| format!("Не удалось записать модуль: {e}");
    let dir = target.parent().ok_or("Неверный путь модуля")?;
    std::fs::create_dir_all(dir).map_err(io_error)?;
    let part = target.with_extension("part");
    std::fs::write(&part, &binary).map_err(io_error)?;
    std::fs::rename(&part, target).map_err(io_error)
}

fn ensure(app: &AppHandle, module: &Module) -> Result<PathBuf, String> {
    let target = module_dir(app, module)?.join(module.file);
    if !is_valid(&target, module) {
        // Файл с неверным хэшем запускать нельзя.
        let _ = std::fs::remove_file(&target);
        install(app, module, &target)?;
    }
    Ok(target)
}

/// Запускает проверку в фоне. Повторный вызов во время работы игнорируется.
pub fn start(app: AppHandle) {
    let modules = app.state::<Modules>();
    if modules.busy.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        publish(&app, ModuleState::Checking);
        let state = match ensure(&app, &POTRACE) {
            Ok(path) => {
                *app.state::<Modules>().potrace.lock().unwrap() = Some(path);
                ModuleState::Ready
            }
            Err(message) => ModuleState::Failed { message },
        };
        publish(&app, state);
        app.state::<Modules>().busy.store(false, Ordering::SeqCst);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_matches_frontend_contract() {
        let json = serde_json::to_string(&ModuleState::Downloading { percent: 42 }).unwrap();
        assert_eq!(json, r#"{"status":"downloading","percent":42}"#);
        let json = serde_json::to_string(&ModuleState::Ready).unwrap();
        assert_eq!(json, r#"{"status":"ready"}"#);
    }
}
