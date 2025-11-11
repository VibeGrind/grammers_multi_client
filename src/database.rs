use crate::errors::ManagerError;
use serde::{Deserialize, Serialize};
use sqlite::{Connection, State};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Статус сессии в базе данных
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionDbStatus {
    /// Зарегистрирована, но не запущена
    Registered,
    /// Удалена из системы
    Deleted,
}

impl SessionDbStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "registered" => Some(Self::Registered),
            "deleted" => Some(Self::Deleted),
            _ => None,
        }
    }
}

/// Запись о сессии в базе данных
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub session_id: String,
    pub file_path: String,
    pub status: SessionDbStatus,
    pub registered_at: i64,
    pub updated_at: i64,
}

/// Менеджер базы данных для хранения маппинга сессий
pub struct SessionDatabase {
    db_path: String,
    conn: Arc<Mutex<Connection>>,
}

impl SessionDatabase {
    /// Создать новый менеджер БД и инициализировать схему
    pub fn new(db_path: impl AsRef<Path>) -> Result<Self, ManagerError> {
        let db_path = db_path.as_ref().to_string_lossy().to_string();
        let conn = Connection::open(&db_path)?;

        // Инициализировать схему
        conn.execute(
            "
            CREATE TABLE IF NOT EXISTS sessions (
                session_id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                status TEXT NOT NULL,
                registered_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            ",
        )?;

        // Создать индекс по статусу для быстрого поиска
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status)",
        )?;

        Ok(Self {
            db_path,
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Получить guard для работы с соединением
    fn get_connection(&self) -> std::sync::MutexGuard<Connection> {
        self.conn
            .lock()
            .unwrap_or_else(|poisoned| {
                log::error!("Database connection mutex was poisoned, recovering guard");
                log::error!("This indicates a panic occurred while holding the database lock");
                poisoned.into_inner()
            })
    }

    /// Зарегистрировать новую сессию (атомарно с проверкой уникальности)
    pub fn register_session(
        &self,
        session_id: &str,
        file_path: &str,
    ) -> Result<(), ManagerError> {
        let conn = self.get_connection();

        let now = chrono::Utc::now().timestamp();

        let query = "
            INSERT INTO sessions (session_id, file_path, status, registered_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
        ";

        let mut stmt = conn.prepare(query)?;
        stmt.bind((1, session_id))?;
        stmt.bind((2, file_path))?;
        stmt.bind((3, SessionDbStatus::Registered.as_str()))?;
        stmt.bind((4, now))?;
        stmt.bind((5, now))?;

        // Попытка вставки - PRIMARY KEY constraint обеспечит уникальность
        match stmt.next() {
            Ok(_) => {
                log::info!("[DB] Registered session: {}", session_id);
                Ok(())
            }
            Err(e) => {
                let err_msg = e.to_string();
                // Проверяем нарушение UNIQUE/PRIMARY KEY constraint
                if err_msg.contains("UNIQUE constraint failed")
                    || err_msg.contains("PRIMARY KEY")
                {
                    Err(ManagerError::SessionAlreadyExists(session_id.to_string()))
                } else {
                    Err(ManagerError::DatabaseError(err_msg))
                }
            }
        }
    }

    /// Проверить существует ли сессия с таким ID
    pub fn session_exists(&self, session_id: &str) -> Result<bool, ManagerError> {
        let conn = self.get_connection();

        let query = "SELECT COUNT(*) FROM sessions WHERE session_id = ?";
        let mut stmt = conn.prepare(query)?;
        stmt.bind((1, session_id))?;

        if let State::Row = stmt.next()? {
            let count: i64 = stmt.read(0)?;
            Ok(count > 0)
        } else {
            Ok(false)
        }
    }

    /// Получить запись о сессии
    pub fn get_session(&self, session_id: &str) -> Result<Option<SessionRecord>, ManagerError> {
        let conn = self.get_connection();

        let query = "
            SELECT session_id, file_path, status, registered_at, updated_at
            FROM sessions
            WHERE session_id = ?
        ";

        let mut stmt = conn.prepare(query)?;
        stmt.bind((1, session_id))?;

        if let State::Row = stmt.next()? {
            let status_str: String = stmt.read(2)?;
            let status = SessionDbStatus::from_str(&status_str)
                .ok_or_else(|| ManagerError::DatabaseError("Invalid status".to_string()))?;

            Ok(Some(SessionRecord {
                session_id: stmt.read(0)?,
                file_path: stmt.read(1)?,
                status,
                registered_at: stmt.read(3)?,
                updated_at: stmt.read(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Пометить сессию как удалённую
    pub fn mark_as_deleted(&self, session_id: &str) -> Result<(), ManagerError> {
        // Сначала проверить существование сессии
        if !self.session_exists(session_id)? {
            return Err(ManagerError::SessionNotRegistered(
                session_id.to_string(),
            ));
        }

        let conn = self.get_connection();

        let now = chrono::Utc::now().timestamp();

        let query = "
            UPDATE sessions
            SET status = ?, updated_at = ?
            WHERE session_id = ?
        ";

        let mut stmt = conn.prepare(query)?;
        stmt.bind((1, SessionDbStatus::Deleted.as_str()))?;
        stmt.bind((2, now))?;
        stmt.bind((3, session_id))?;
        stmt.next()?;

        log::info!("[DB] Marked session as deleted: {}", session_id);

        Ok(())
    }

    /// Получить все зарегистрированные сессии
    pub fn get_registered_sessions(&self) -> Result<Vec<SessionRecord>, ManagerError> {
        let conn = self.get_connection();

        let query = "
            SELECT session_id, file_path, status, registered_at, updated_at
            FROM sessions
            WHERE status = ?
            ORDER BY registered_at ASC
        ";

        let mut stmt = conn.prepare(query)?;
        stmt.bind((1, SessionDbStatus::Registered.as_str()))?;

        let mut sessions = Vec::new();

        while let State::Row = stmt.next()? {
            let status_str: String = stmt.read(2)?;
            let status = SessionDbStatus::from_str(&status_str)
                .ok_or_else(|| ManagerError::DatabaseError("Invalid status".to_string()))?;

            sessions.push(SessionRecord {
                session_id: stmt.read(0)?,
                file_path: stmt.read(1)?,
                status,
                registered_at: stmt.read(3)?,
                updated_at: stmt.read(4)?,
            });
        }

        Ok(sessions)
    }

    /// Получить все сессии (включая удалённые)
    pub fn get_all_sessions(&self) -> Result<Vec<SessionRecord>, ManagerError> {
        let conn = self.get_connection();

        let query = "
            SELECT session_id, file_path, status, registered_at, updated_at
            FROM sessions
            ORDER BY registered_at ASC
        ";

        let mut stmt = conn.prepare(query)?;

        let mut sessions = Vec::new();

        while let State::Row = stmt.next()? {
            let status_str: String = stmt.read(2)?;
            let status = SessionDbStatus::from_str(&status_str)
                .ok_or_else(|| ManagerError::DatabaseError("Invalid status".to_string()))?;

            sessions.push(SessionRecord {
                session_id: stmt.read(0)?,
                file_path: stmt.read(1)?,
                status,
                registered_at: stmt.read(3)?,
                updated_at: stmt.read(4)?,
            });
        }

        Ok(sessions)
    }

    /// Удалить запись о сессии из базы данных (физическое удаление)
    pub fn delete_session(&self, session_id: &str) -> Result<(), ManagerError> {
        // Сначала проверить существование сессии
        if !self.session_exists(session_id)? {
            return Err(ManagerError::SessionNotRegistered(
                session_id.to_string(),
            ));
        }

        let conn = self.get_connection();

        let query = "DELETE FROM sessions WHERE session_id = ?";

        let mut stmt = conn.prepare(query)?;
        stmt.bind((1, session_id))?;
        stmt.next()?;

        log::info!("[DB] Deleted session record: {}", session_id);

        Ok(())
    }
}
