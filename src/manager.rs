use crate::config::ManagerConfig;
use crate::database::{SessionDatabase, SessionDbStatus};
use crate::errors::ManagerError;
use crate::phoenix_manager::PhoenixManager;
use crate::session::{SessionConfig, TelegramSession};
use crate::session_handle::{SessionHandle, SessionStatus};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};

/// Менеджер сессий - управляет жизненным циклом всех Telegram сессий
pub struct SessionManager {
    /// Активные сессии: session_id → SessionHandle
    sessions: Arc<RwLock<HashMap<String, SessionHandle>>>,

    /// Phoenix менеджер (единый клиент)
    phoenix_manager: Arc<PhoenixManager>,

    /// База данных для маппинга сессий
    database: Arc<SessionDatabase>,

    /// Конфигурация
    config: ManagerConfig,
}

impl SessionManager {
    /// Создать новый SessionManager
    pub async fn new(config: ManagerConfig) -> Result<Self, ManagerError> {
        log::info!("=== Initializing Session Manager ===");

        // Создать директории если не существуют
        Self::ensure_directories(&config)?;

        // Инициализировать базу данных
        log::info!("Initializing database: {:?}", config.database_path);
        let database = Arc::new(SessionDatabase::new(&config.database_path)?);

        // Подключиться к Phoenix
        log::info!("Connecting to Phoenix: {}", config.phoenix_url);
        let phoenix_manager = Arc::new(PhoenixManager::new(&config.phoenix_url).await?);

        log::info!("✓ Session Manager initialized");

        Ok(Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            phoenix_manager,
            database,
            config,
        })
    }

    /// Убедиться что директории существуют
    fn ensure_directories(config: &ManagerConfig) -> Result<(), ManagerError> {
        if !config.sessions_dir.exists() {
            log::info!("Creating sessions directory: {:?}", config.sessions_dir);
            fs::create_dir_all(&config.sessions_dir)?;
        }

        if !config.storage_dir.exists() {
            log::info!("Creating storage directory: {:?}", config.storage_dir);
            fs::create_dir_all(&config.storage_dir)?;
        }

        Ok(())
    }

    /// Зарегистрировать сессию: переместить из /sessions в /storage и записать в БД
    /// session_name - имя файла БЕЗ расширения .session
    pub async fn register_session(&self, session_name: &str) -> Result<String, ManagerError> {
        log::info!("[Manager] Registering session: {}", session_name);

        let session_id = session_name.to_string();

        // Проверить что файл существует в /sessions
        let source_path = self
            .config
            .sessions_dir
            .join(format!("{}.session", session_name));

        if !source_path.exists() {
            return Err(ManagerError::SessionFileNotFound(session_name.to_string()));
        }

        // Проверить что session_id не занят в БД
        if self.database.session_exists(&session_id)? {
            return Err(ManagerError::SessionAlreadyExists(session_id));
        }

        // Целевой путь в /storage
        let target_path = self
            .config
            .storage_dir
            .join(format!("{}.session", session_name));

        // Переместить файл
        log::info!(
            "[Manager] Moving session file: {:?} -> {:?}",
            source_path,
            target_path
        );
        fs::rename(&source_path, &target_path)?;

        // Зарегистрировать в БД
        let target_path_str = target_path.to_string_lossy().to_string();
        self.database
            .register_session(&session_id, &target_path_str)?;

        log::info!("[Manager] ✓ Session registered: {}", session_id);

        Ok(session_id)
    }

    /// Запустить сессию из /storage
    pub async fn start_session(&self, session_id: &str) -> Result<(), ManagerError> {
        log::info!("[Manager] Starting session: {}", session_id);

        // Получить запись из БД (до захвата lock)
        let record = self
            .database
            .get_session(session_id)?
            .ok_or_else(|| ManagerError::SessionNotRegistered(session_id.to_string()))?;

        // Проверить что статус = Registered
        if record.status != SessionDbStatus::Registered {
            return Err(ManagerError::SessionNotRegistered(session_id.to_string()));
        }

        // Проверить что файл существует
        let session_path = PathBuf::from(&record.file_path);
        if !session_path.exists() {
            return Err(ManagerError::SessionFileNotFound(session_id.to_string()));
        }

        // Захватить write lock на весь процесс (fix TOCTOU race condition)
        let mut sessions = self.sessions.write().await;

        // Проверить что сессия не запущена (атомарно с добавлением)
        if sessions.contains_key(session_id) {
            return Err(ManagerError::SessionAlreadyRunning(session_id.to_string()));
        }

        // Создать shutdown канал (один раз!)
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Создать status Arc
        let status_arc = Arc::new(RwLock::new(SessionStatus::Starting));

        // Создать конфигурацию сессии
        let session_config = SessionConfig {
            session_id: session_id.to_string(),
            session_path: record.file_path.clone(),
            phoenix_manager: self.phoenix_manager.as_ref().clone(),
            update_queue_limit: self.config.update_queue_limit,
            catch_up: self.config.catch_up,
        };

        // Запустить сессию в отдельной задаче (один раз!)
        let task_handle = tokio::spawn(TelegramSession::run(
            session_config,
            shutdown_rx,
            Arc::clone(&status_arc),
        ));

        // Создать SessionHandle (один раз!)
        let session_handle = SessionHandle::new(
            session_id.to_string(),
            task_handle,
            shutdown_tx,
        );

        // Сохранить в HashMap (атомарно с проверкой)
        sessions.insert(session_id.to_string(), session_handle);

        log::info!("[Manager] ✓ Session started: {}", session_id);

        Ok(())
    }

    /// Остановить сессию (graceful shutdown)
    pub async fn stop_session(&self, session_id: &str) -> Result<(), ManagerError> {
        log::info!("[Manager] Stopping session: {}", session_id);

        // Извлечь SessionHandle из HashMap
        let session_handle = {
            let mut sessions = self.sessions.write().await;
            sessions
                .remove(session_id)
                .ok_or_else(|| ManagerError::SessionNotRunning(session_id.to_string()))?
        };

        // Выполнить graceful shutdown
        if let Err(e) = session_handle.shutdown().await {
            log::error!("[Manager] Error during shutdown: {:?}", e);
            return Err(ManagerError::SessionError(format!(
                "Shutdown error: {}",
                e
            )));
        }

        // Удалить канал из Phoenix менеджера
        self.phoenix_manager.remove_channel(session_id);

        log::info!("[Manager] ✓ Session stopped: {}", session_id);

        Ok(())
    }

    /// Удалить сессию: остановить (если запущена) и пометить как deleted в БД
    /// delete_file - удалить ли файл из /storage
    pub async fn remove_session(
        &self,
        session_id: &str,
        delete_file: bool,
    ) -> Result<(), ManagerError> {
        log::info!("[Manager] Removing session: {}", session_id);

        // Попытаться остановить (fix race condition - не проверяем is_running отдельно)
        match self.stop_session(session_id).await {
            Ok(()) => {
                log::info!("[Manager] Session stopped before removal");
            }
            Err(ManagerError::SessionNotRunning(_)) => {
                // Сессия уже остановлена - это нормально
                log::debug!("[Manager] Session already stopped");
            }
            Err(e) => return Err(e),
        }

        // Получить запись из БД
        let record = self
            .database
            .get_session(session_id)?
            .ok_or_else(|| ManagerError::SessionNotRegistered(session_id.to_string()))?;

        // Удалить файл если нужно
        if delete_file {
            let file_path = PathBuf::from(&record.file_path);
            if file_path.exists() {
                log::info!("[Manager] Deleting session file: {:?}", file_path);
                fs::remove_file(&file_path)?;
            }
        }

        // Пометить как удалённую в БД
        self.database.mark_as_deleted(session_id)?;

        log::info!("[Manager] ✓ Session removed: {}", session_id);

        Ok(())
    }

    /// Получить статус сессии
    pub async fn get_session_status(&self, session_id: &str) -> Option<SessionStatus> {
        let sessions = self.sessions.read().await;
        if let Some(handle) = sessions.get(session_id) {
            Some(handle.get_status().await)
        } else {
            None
        }
    }

    /// Проверить запущена ли сессия
    pub async fn is_session_running(&self, session_id: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions.contains_key(session_id)
    }

    /// Список активных (запущенных) сессий
    pub async fn list_active_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions.keys().cloned().collect()
    }

    /// Список зарегистрированных сессий в БД
    pub fn list_registered_sessions(&self) -> Result<Vec<String>, ManagerError> {
        let records = self.database.get_registered_sessions()?;
        Ok(records.into_iter().map(|r| r.session_id).collect())
    }

    /// Список доступных сессий в /storage (файлы .session)
    pub fn list_available_sessions(&self) -> Result<Vec<String>, ManagerError> {
        let mut sessions = Vec::new();

        for entry in fs::read_dir(&self.config.storage_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "session" {
                        if let Some(stem) = path.file_stem() {
                            sessions.push(stem.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        Ok(sessions)
    }

    /// Список новых сессий в /sessions (для регистрации)
    pub fn list_pending_sessions(&self) -> Result<Vec<String>, ManagerError> {
        let mut sessions = Vec::new();

        for entry in fs::read_dir(&self.config.sessions_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "session" {
                        if let Some(stem) = path.file_stem() {
                            sessions.push(stem.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }

        Ok(sessions)
    }

    /// Получить конфигурацию
    pub fn config(&self) -> &ManagerConfig {
        &self.config
    }

    /// Получить Phoenix менеджер
    pub fn phoenix_manager(&self) -> &Arc<PhoenixManager> {
        &self.phoenix_manager
    }
}
