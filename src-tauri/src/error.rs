use serde::{Serialize, ser::Serializer};

#[derive(Debug)]
pub enum AppError {
    Internal(String),
    Io(std::io::Error),
    Tauri(String),
    System(String),
    Cancelled,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::Internal(s) => write!(f, "Internal error: {}", s),
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::Tauri(s) => write!(f, "Tauri error: {}", s),
            AppError::System(s) => write!(f, "System error: {}", s),
            AppError::Cancelled => write!(f, "Operation cancelled"),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let msg = self.to_string();

        let code = match self {
            AppError::Internal(_) => "INTERNAL",
            AppError::Io(_) => "IO_ERROR",
            AppError::Tauri(_) => "TAURI_ERROR",
            AppError::System(_) => "SYSTEM_ERROR",
            AppError::Cancelled => "CANCELLED",
        };

        #[derive(Serialize)]
        struct ErrorWrapper {
            code: &'static str,
            message: String,
        }

        ErrorWrapper { code, message: msg }.serialize(serializer)
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err)
    }
}

impl From<tauri::Error> for AppError {
    fn from(err: tauri::Error) -> Self {
        AppError::Tauri(err.to_string())
    }
}

impl From<String> for AppError {
    fn from(err: String) -> Self {
        AppError::Internal(err)
    }
}
