use std::fmt;

/// Ошибки менеджера сессий
#[derive(Debug)]
pub enum ManagerError {
    /// Сессия не найдена в /sessions
    SessionFileNotFound(String),

    /// Сессия не зарегистрирована в системе
    SessionNotRegistered(String),

    /// Сессия с таким ID уже существует
    SessionAlreadyExists(String),

    /// Сессия уже запущена
    SessionAlreadyRunning(String),

    /// Сессия не запущена
    SessionNotRunning(String),

    /// Ошибка файловой системы
    IoError(std::io::Error),

    /// Ошибка базы данных
    DatabaseError(String),

    /// Ошибка Phoenix подключения
    PhoenixError(String),

    /// Сессия не авторизована в Telegram
    NotAuthorized(String),

    /// Общая ошибка сессии
    SessionError(String),
}

impl fmt::Display for ManagerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SessionFileNotFound(name) => {
                write!(f, "Session file not found: {}", name)
            }
            Self::SessionNotRegistered(id) => {
                write!(f, "Session not registered: {}", id)
            }
            Self::SessionAlreadyExists(id) => {
                write!(f, "Session already exists: {}", id)
            }
            Self::SessionAlreadyRunning(id) => {
                write!(f, "Session already running: {}", id)
            }
            Self::SessionNotRunning(id) => {
                write!(f, "Session not running: {}", id)
            }
            Self::IoError(e) => {
                write!(f, "IO error: {}", e)
            }
            Self::DatabaseError(msg) => {
                write!(f, "Database error: {}", msg)
            }
            Self::PhoenixError(msg) => {
                write!(f, "Phoenix connection error: {}", msg)
            }
            Self::NotAuthorized(id) => {
                write!(f, "Session not authorized: {}", id)
            }
            Self::SessionError(msg) => {
                write!(f, "Session error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ManagerError {}

impl From<std::io::Error> for ManagerError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

impl From<sqlite::Error> for ManagerError {
    fn from(e: sqlite::Error) -> Self {
        Self::DatabaseError(e.to_string())
    }
}

/// Ошибки Telegram сессии
#[derive(Debug)]
pub enum SessionError {
    /// Ошибка загрузки session data из SQLite
    LoadError(String),

    /// Ошибка подключения
    ConnectionError(String),

    /// Ошибка авторизации
    AuthorizationError(String),

    /// Ошибка получения обновлений
    UpdateError(String),

    /// Ошибка Phoenix
    PhoenixError(String),

    /// Общая ошибка
    Other(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoadError(msg) => write!(f, "Load error: {}", msg),
            Self::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            Self::AuthorizationError(msg) => write!(f, "Authorization error: {}", msg),
            Self::UpdateError(msg) => write!(f, "Update error: {}", msg),
            Self::PhoenixError(msg) => write!(f, "Phoenix error: {}", msg),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for SessionError {}
