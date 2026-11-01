mod commands;
mod engine;
mod error;
mod loader;
mod logo;
mod modules;
mod pipeline;
mod preprocess;
mod queue;
mod settings;
mod state;

pub mod cli;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(state::AppState::default())
        .manage(modules::Modules::default())
        .invoke_handler(tauri::generate_handler![
            commands::load_image_path,
            commands::load_image_bytes,
            commands::get_preview,
            commands::trace,
            commands::cancel_trace,
            commands::save_file,
            commands::modules_state,
            commands::modules_retry,
        ])
        .setup(|app| {
            modules::start(app.app_handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("не удалось запустить приложение");
}
