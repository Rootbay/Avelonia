use serde::{Serialize, ser::Serializer};

#[derive(Debug)]
pub enum AppError {
    Internal(String),
    Io(std::io::Error),
    Tauri(String),
    System(String),
    Cancelled,
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let msg = match self {
            AppError::Internal(s) => s.clone(),
            AppError::Io(e) => e.to_string(),
            AppError::Tauri(s) => s.clone(),
            AppError::System(s) => s.clone(),
            AppError::Cancelled => "Operation cancelled".to_string(),
        };
        
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

impl From<String> for AppError {
    fn from(err: String) -> Self {
        AppError::Internal(err)
    }
}
