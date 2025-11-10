use std::path::PathBuf;

/// Конфигурация менеджера сессий
#[derive(Debug, Clone)]
pub struct ManagerConfig {
    /// URL Phoenix сервера
    pub phoenix_url: String,

    /// Путь к папке /sessions (staging для новых сессий)
    pub sessions_dir: PathBuf,

    /// Путь к папке /storage (рабочая папка с активными сессиями)
    pub storage_dir: PathBuf,

    /// Путь к файлу базы данных менеджера
    pub database_path: PathBuf,

    /// Лимит очереди обновлений для каждой сессии
    pub update_queue_limit: usize,

    /// Catch up обновлений при старте
    pub catch_up: bool,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            phoenix_url: "ws://localhost:4000/socket".to_string(),
            sessions_dir: PathBuf::from("./sessions"),
            storage_dir: PathBuf::from("./storage"),
            database_path: PathBuf::from("./manager.db"),
            update_queue_limit: 10,
            catch_up: true,
        }
    }
}

impl ManagerConfig {
    /// Создать конфигурацию из переменных окружения
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(url) = std::env::var("PHOENIX_URL") {
            config.phoenix_url = url;
        }

        if let Ok(path) = std::env::var("SESSIONS_DIR") {
            config.sessions_dir = PathBuf::from(path);
        }

        if let Ok(path) = std::env::var("STORAGE_DIR") {
            config.storage_dir = PathBuf::from(path);
        }

        if let Ok(path) = std::env::var("DATABASE_PATH") {
            config.database_path = PathBuf::from(path);
        }

        if let Ok(limit) = std::env::var("UPDATE_QUEUE_LIMIT") {
            if let Ok(val) = limit.parse() {
                config.update_queue_limit = val;
            }
        }

        if let Ok(catch_up) = std::env::var("CATCH_UP") {
            config.catch_up = catch_up.to_lowercase() == "true";
        }

        config
    }

    /// Создать конфигурацию с кастомными параметрами
    pub fn new(phoenix_url: impl Into<String>) -> Self {
        Self {
            phoenix_url: phoenix_url.into(),
            ..Default::default()
        }
    }

    /// Установить путь к папке sessions
    pub fn with_sessions_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.sessions_dir = path.into();
        self
    }

    /// Установить путь к папке storage
    pub fn with_storage_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.storage_dir = path.into();
        self
    }

    /// Установить путь к базе данных
    pub fn with_database_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.database_path = path.into();
        self
    }

    /// Установить лимит очереди обновлений
    pub fn with_update_queue_limit(mut self, limit: usize) -> Self {
        self.update_queue_limit = limit;
        self
    }

    /// Установить catch up
    pub fn with_catch_up(mut self, catch_up: bool) -> Self {
        self.catch_up = catch_up;
        self
    }
}
