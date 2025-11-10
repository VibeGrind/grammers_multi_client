use crate::errors::SessionError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};
use tokio::task::JoinHandle;

/// Статус сессии (runtime состояние)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    /// Запускается
    Starting,

    /// Работает нормально
    Running {
        started_at: DateTime<Utc>,
        user_id: Option<i64>,
        username: Option<String>,
        phoenix_connected: bool,
    },

    /// Останавливается
    Stopping,

    /// Остановлена
    Stopped,

    /// Ошибка
    Error {
        error: String,
        occurred_at: DateTime<Utc>,
    },
}

/// Handle для управления запущенной сессией
pub struct SessionHandle {
    /// ID сессии
    pub session_id: String,

    /// Tokio task handle
    task_handle: JoinHandle<Result<(), SessionError>>,

    /// Канал для отправки shutdown сигнала
    shutdown_tx: Option<oneshot::Sender<()>>,

    /// Текущий статус сессии
    status: Arc<RwLock<SessionStatus>>,
}

impl SessionHandle {
    /// Создать новый SessionHandle
    pub fn new(
        session_id: String,
        task_handle: JoinHandle<Result<(), SessionError>>,
        shutdown_tx: oneshot::Sender<()>,
    ) -> Self {
        Self {
            session_id,
            task_handle,
            shutdown_tx: Some(shutdown_tx),
            status: Arc::new(RwLock::new(SessionStatus::Starting)),
        }
    }

    /// Получить клон статуса Arc для обновления из сессии
    pub fn status_arc(&self) -> Arc<RwLock<SessionStatus>> {
        Arc::clone(&self.status)
    }

    /// Получить текущий статус
    pub async fn get_status(&self) -> SessionStatus {
        self.status.read().await.clone()
    }

    /// Обновить статус (используется из SessionManager)
    pub async fn update_status(&self, new_status: SessionStatus) {
        let mut status = self.status.write().await;
        *status = new_status;
    }

    /// Проверить запущена ли сессия
    pub async fn is_running(&self) -> bool {
        matches!(
            *self.status.read().await,
            SessionStatus::Running { .. } | SessionStatus::Starting
        )
    }

    /// Отправить shutdown сигнал и дождаться завершения
    pub async fn shutdown(mut self) -> Result<(), SessionError> {
        log::info!("[{}] Initiating graceful shutdown", self.session_id);

        // Обновляем статус
        self.update_status(SessionStatus::Stopping).await;

        // Отправляем сигнал остановки
        if let Some(tx) = self.shutdown_tx.take() {
            if tx.send(()).is_err() {
                log::warn!(
                    "[{}] Failed to send shutdown signal (receiver dropped)",
                    self.session_id
                );
            }
        }

        // Ждём завершения задачи
        match self.task_handle.await {
            Ok(Ok(())) => {
                log::info!("[{}] Session stopped successfully", self.session_id);
                self.update_status(SessionStatus::Stopped).await;
                Ok(())
            }
            Ok(Err(e)) => {
                log::error!("[{}] Session stopped with error: {:?}", self.session_id, e);
                self.update_status(SessionStatus::Error {
                    error: e.to_string(),
                    occurred_at: Utc::now(),
                })
                .await;
                Err(e)
            }
            Err(e) => {
                log::error!("[{}] Task join error: {:?}", self.session_id, e);
                let err = SessionError::Other(format!("Task join error: {}", e));
                self.update_status(SessionStatus::Error {
                    error: err.to_string(),
                    occurred_at: Utc::now(),
                })
                .await;
                Err(err)
            }
        }
    }

    /// Получить session_id
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}
