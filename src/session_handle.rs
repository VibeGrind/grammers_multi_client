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
        status: Arc<RwLock<SessionStatus>>,
    ) -> Self {
        Self {
            session_id,
            task_handle,
            shutdown_tx: Some(shutdown_tx),
            status,
        }
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

    /// Отправить shutdown сигнал и дождаться завершения (с timeout)
    pub async fn shutdown(mut self) -> Result<(), SessionError> {
        use tokio::time::{timeout, Duration};

        log::info!("[{}] Initiating graceful shutdown", self.session_id);

        // Клонировать status_arc ДО начала операций, чтобы использовать после await
        let status_arc = Arc::clone(&self.status);

        // Обновляем статус через клонированный Arc
        {
            let mut status = status_arc.write().await;
            *status = SessionStatus::Stopping;
        }

        // Отправляем сигнал остановки
        if let Some(tx) = self.shutdown_tx.take() {
            if tx.send(()).is_err() {
                log::warn!(
                    "[{}] Failed to send shutdown signal (receiver dropped)",
                    self.session_id
                );
            }
        }

        // Ждём завершения задачи с timeout (30 секунд)
        let shutdown_timeout = Duration::from_secs(30);

        // Получить abort handle перед timeout
        let abort_handle = self.task_handle.abort_handle();

        match timeout(shutdown_timeout, self.task_handle).await {
            Ok(Ok(Ok(()))) => {
                log::info!("[{}] Session stopped successfully", self.session_id);
                let mut status = status_arc.write().await;
                *status = SessionStatus::Stopped;
                Ok(())
            }
            Ok(Ok(Err(e))) => {
                log::error!("[{}] Session stopped with error: {:?}", self.session_id, e);
                let mut status = status_arc.write().await;
                *status = SessionStatus::Error {
                    error: e.to_string(),
                    occurred_at: Utc::now(),
                };
                Err(e)
            }
            Ok(Err(e)) => {
                log::error!("[{}] Task join error: {:?}", self.session_id, e);
                let err = SessionError::Other(format!("Task join error: {}", e));
                let mut status = status_arc.write().await;
                *status = SessionStatus::Error {
                    error: err.to_string(),
                    occurred_at: Utc::now(),
                };
                Err(err)
            }
            Err(_) => {
                log::error!(
                    "[{}] Shutdown timeout after {} seconds, aborting task",
                    self.session_id,
                    shutdown_timeout.as_secs()
                );

                // Force abort задачи
                abort_handle.abort();

                // Небольшая задержка для завершения abort
                tokio::time::sleep(Duration::from_millis(100)).await;

                let err = SessionError::Other(format!(
                    "Shutdown timeout after {} seconds",
                    shutdown_timeout.as_secs()
                ));
                let mut status = status_arc.write().await;
                *status = SessionStatus::Error {
                    error: err.to_string(),
                    occurred_at: Utc::now(),
                };
                Err(err)
            }
        }
    }

    /// Получить session_id
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

impl Drop for SessionHandle {
    fn drop(&mut self) {
        // Если shutdown_tx еще не использован (shutdown() не был вызван)
        if let Some(tx) = self.shutdown_tx.take() {
            // Попытка graceful shutdown
            let _ = tx.send(());

            // Force abort задачи на случай если shutdown signal не обработан
            self.task_handle.abort();

            // Safely log without risking panic in destructor
            let session_id = self.session_id.clone();
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                log::warn!(
                    "[{}] SessionHandle dropped without explicit shutdown, task aborted",
                    session_id
                );
            }));
        }
    }
}
