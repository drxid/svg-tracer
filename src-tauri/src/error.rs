use serde::Serialize;

/// Ошибка для фронтенда: сообщение уже на русском и показывается как есть.
#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
}

impl AppError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }

    pub fn cancelled() -> Self {
        Self::new("cancelled", "Прогон отменён")
    }

    pub fn is_cancelled(&self) -> bool {
        self.code == "cancelled"
    }

    pub fn empty_artwork() -> Self {
        Self::new("empty_artwork", "Не нашёл рисунка: всё изображение совпало с фоном")
    }
}

pub type AppResult<T> = Result<T, AppError>;
