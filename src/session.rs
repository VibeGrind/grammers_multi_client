use crate::errors::SessionError;
use crate::phoenix_manager::{PhoenixManager, TelegramUpdate, UserInfo};
use crate::session_handle::{SessionHandle, SessionStatus};
use chrono::Utc;
use grammers_client::{Client, Update, UpdatesConfiguration};
use grammers_mtsender::{ConnectionParams, SenderPool};
use grammers_session::storages::SqliteSession;
use phoenix_channels_client::Channel;
use sqlite::Connection;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};

/// Данные из session SQLite файла
struct SessionData {
    app_id: i32,
    device: String,
    sdk: String,
    app_version: String,
    lang_code: String,
    system_lang_code: String,
    proxy: Option<String>,
}

/// Конфигурация для запуска сессии
pub struct SessionConfig {
    pub session_id: String,
    pub session_path: String,
    pub phoenix_manager: PhoenixManager,
    pub update_queue_limit: usize,
    pub catch_up: bool,
}

/// Telegram сессия - полностью изолированная в отдельной tokio задаче
pub struct TelegramSession;

impl TelegramSession {
    /// Запустить сессию (основной метод, вызывается в tokio::spawn)
    pub async fn run(
        config: SessionConfig,
        mut shutdown_rx: oneshot::Receiver<()>,
        status_arc: Arc<RwLock<SessionStatus>>,
    ) -> Result<(), SessionError> {
        let session_id = config.session_id.clone();

        log::info!("[{}] Starting Telegram session", session_id);
        log::info!("[{}] Loading session: {}", session_id, config.session_path);

        // Загрузить SessionData из SQLite (блокирующая операция)
        let session_path = config.session_path.clone();
        let session_data = tokio::task::spawn_blocking(move || {
            Self::load_session_data(&session_path)
        })
        .await
        .map_err(|e| SessionError::LoadError(format!("Join error: {}", e)))?
        .map_err(|e| SessionError::LoadError(format!("Session load error: {}", e)))?;

        log::info!("[{}] Device fingerprint loaded:", session_id);
        log::info!("[{}]   Device: {}", session_id, session_data.device);
        log::info!("[{}]   SDK: {}", session_id, session_data.sdk);
        log::info!("[{}]   App version: {}", session_id, session_data.app_version);
        log::info!("[{}]   API ID: {}", session_id, session_data.app_id);
        log::info!("[{}]   Lang code: {}", session_id, session_data.lang_code);
        log::info!(
            "[{}]   System lang code: {}",
            session_id,
            session_data.system_lang_code
        );

        if let Some(ref proxy) = session_data.proxy {
            // Извлечь только host:port часть (после последнего @) для безопасного логирования
            let safe_part = proxy.rsplit('@').next().unwrap_or("***");
            log::info!(
                "[{}]   Proxy: {} (SOCKS5)",
                session_id,
                safe_part
            );
        } else {
            log::info!("[{}]   Proxy: None (direct connection)", session_id);
        }

        // Открыть сессию
        let session = Arc::new(
            SqliteSession::open(&config.session_path)
                .map_err(|e| SessionError::LoadError(format!("Failed to open session: {}", e)))?,
        );

        // Создать ConnectionParams
        let connection_params = ConnectionParams {
            device_model: session_data.device.clone(),
            system_version: session_data.sdk.clone(),
            app_version: session_data.app_version.clone(),
            system_lang_code: session_data.system_lang_code.clone(),
            lang_code: session_data.lang_code.clone(),
            #[cfg(feature = "proxy")]
            proxy_url: session_data.proxy.clone(),
            __non_exhaustive: (),
        };

        log::info!("[{}] Creating MTProto connection...", session_id);

        // Создать SenderPool
        let pool = SenderPool::with_configuration(
            Arc::clone(&session),
            session_data.app_id,
            connection_params,
        );

        let client = Client::new(&pool);
        let SenderPool {
            runner,
            handle,
            updates,
        } = pool;

        // Запустить pool runner в фоновой задаче
        let pool_task = tokio::spawn(runner.run());

        log::info!("[{}] Checking authorization...", session_id);

        // Проверить авторизацию
        match client.is_authorized().await {
            Ok(true) => {
                log::info!("[{}] ✓ Session is authorized", session_id);
            }
            Ok(false) => {
                log::error!("[{}] ✗ Session is NOT authorized", session_id);
                handle.quit();
                if let Err(join_err) = pool_task.await {
                    log::warn!(
                        "[{}] Pool task error during cleanup: {:?}",
                        session_id,
                        join_err
                    );
                }
                return Err(SessionError::AuthorizationError(
                    "Session not authorized".to_string(),
                ));
            }
            Err(e) => {
                log::error!("[{}] Authorization check failed: {:?}", session_id, e);
                handle.quit();
                if let Err(join_err) = pool_task.await {
                    log::warn!(
                        "[{}] Pool task error during cleanup: {:?}",
                        session_id,
                        join_err
                    );
                }
                return Err(SessionError::AuthorizationError(format!(
                    "Authorization check failed: {}",
                    e
                )));
            }
        }

        // Получить информацию о пользователе
        let (user_id, username) = match client.get_me().await {
            Ok(user) => {
                let name = user.first_name().unwrap_or("Unknown");
                let id = user.raw.id();
                log::info!("[{}] ✓ Logged in as: {} (ID: {})", session_id, name, id);
                (Some(id), user.username().map(String::from))
            }
            Err(e) => {
                log::warn!("[{}] Could not get user info: {:?}", session_id, e);
                (None, None)
            }
        };

        // Получить Phoenix канал
        log::info!("[{}] Getting Phoenix channel...", session_id);
        let phoenix_channel = match config
            .phoenix_manager
            .get_or_create_channel(&session_id)
            .await
        {
            Ok(channel) => channel,
            Err(e) => {
                log::error!(
                    "[{}] Failed to get Phoenix channel, cleaning up: {:?}",
                    session_id,
                    e
                );
                handle.quit();
                if let Err(join_err) = pool_task.await {
                    log::warn!(
                        "[{}] Pool task error during cleanup: {:?}",
                        session_id,
                        join_err
                    );
                }
                return Err(SessionError::PhoenixError(format!(
                    "Failed to get channel: {}",
                    e
                )));
            }
        };

        log::info!("[{}] ✓ Phoenix channel ready", session_id);

        // Обновить статус на Running
        {
            let mut status = status_arc.write().await;
            *status = SessionStatus::Running {
                started_at: Utc::now(),
                user_id,
                username: username.clone(),
                phoenix_connected: true,
            };
        }

        log::info!("[{}] Creating update stream...", session_id);

        // Создать update stream
        let updates_config = UpdatesConfiguration {
            catch_up: config.catch_up,
            update_queue_limit: Some(config.update_queue_limit),
        };

        let mut updates_stream = client.stream_updates(updates, updates_config);

        log::info!("[{}] ✓ Now online, listening for updates...", session_id);

        // Основной цикл обработки обновлений
        loop {
            tokio::select! {
                // Получен сигнал остановки
                _ = &mut shutdown_rx => {
                    log::info!("[{}] Shutdown signal received", session_id);
                    break;
                }

                // Получено обновление
                update = updates_stream.next() => {
                    match update {
                        Ok(update) => {
                            Self::handle_update(
                                &session_id,
                                update,
                                &phoenix_channel,
                            ).await;
                        }
                        Err(e) => {
                            log::error!("[{}] Update error: {:?}", session_id, e);
                        }
                    }
                }
            }
        }

        // Graceful shutdown последовательность
        log::info!("[{}] Syncing update state...", session_id);
        updates_stream.sync_update_state();

        // Явно drop stream перед остановкой pool
        log::debug!("[{}] Dropping update stream", session_id);
        drop(updates_stream);

        log::info!("[{}] Stopping MTProto connection...", session_id);
        handle.quit();

        log::info!("[{}] Waiting for pool to stop...", session_id);
        // Обрабатываем ошибки pool task
        match pool_task.await {
            Ok(_) => {
                log::debug!("[{}] Pool task completed successfully", session_id);
            }
            Err(e) => {
                log::warn!("[{}] Pool task join error: {:?}", session_id, e);
            }
        }

        log::info!("[{}] ✓ Shutdown complete", session_id);

        Ok(())
    }

    /// Обработать обновление и отправить в Phoenix
    async fn handle_update(session_id: &str, update: Update, phoenix_channel: &Arc<Channel>) {
        let telegram_update = match &update {
            Update::NewMessage(msg) => {
                let text = msg.text();
                let chat_id = msg.peer_id().bot_api_dialog_id();
                let message_id = msg.id();
                let peer = msg.peer().ok().and_then(|p| p.name()).unwrap_or_default();

                log::info!("[{}] New message from {}: {}", session_id, peer, text);

                Some(TelegramUpdate {
                    update_type: "new_message".to_string(),
                    chat_id: Some(chat_id),
                    message_id: Some(message_id),
                    text: Some(text.to_string()),
                    from_user: msg.sender().map(|sender_peer| UserInfo {
                        id: sender_peer.id().bot_api_dialog_id(),
                        first_name: sender_peer.name().unwrap_or("").to_string(),
                        username: sender_peer.username().map(String::from),
                    }),
                    timestamp: Some(msg.date().timestamp()),
                    raw_data: None,
                })
            }
            Update::MessageEdited(msg) => {
                log::debug!("[{}] Message edited: {}", session_id, msg.text());
                Some(TelegramUpdate {
                    update_type: "message_edited".to_string(),
                    chat_id: Some(msg.peer_id().bot_api_dialog_id()),
                    message_id: Some(msg.id()),
                    text: Some(msg.text().to_string()),
                    from_user: None,
                    timestamp: Some(msg.date().timestamp()),
                    raw_data: None,
                })
            }
            Update::MessageDeleted(deleted) => {
                let chat_id = deleted.channel_id();
                log::debug!(
                    "[{}] Message deleted: channel_id={:?}, messages={:?}",
                    session_id,
                    chat_id,
                    deleted.messages()
                );
                Some(TelegramUpdate {
                    update_type: "message_deleted".to_string(),
                    chat_id,
                    message_id: deleted.messages().first().copied(),
                    text: None,
                    from_user: None,
                    timestamp: None,
                    raw_data: Some(format!("deleted_messages: {:?}", deleted.messages())),
                })
            }
            _ => {
                log::debug!("[{}] Other update: {:?}", session_id, update);
                Some(TelegramUpdate {
                    update_type: "other".to_string(),
                    chat_id: None,
                    message_id: None,
                    text: None,
                    from_user: None,
                    timestamp: None,
                    raw_data: Some(format!("{:?}", update)),
                })
            }
        };

        // Отправить в Phoenix с retry логикой
        if let Some(telegram_update) = telegram_update {
            if let Ok(json_value) = serde_json::to_value(&telegram_update) {
                // Retry до 3 раз с exponential backoff
                let max_retries = 3;
                let mut last_error = None;

                for retry in 0..max_retries {
                    match phoenix_channel
                        .send_noreply("telegram_update", json_value.clone())
                        .await
                    {
                        Ok(()) => {
                            // Успешно отправлено
                            if retry > 0 {
                                log::info!(
                                    "[{}] Update sent successfully after {} retries",
                                    session_id,
                                    retry
                                );
                            }
                            break;
                        }
                        Err(e) => {
                            last_error = Some(e);
                            if retry < max_retries - 1 {
                                let delay_ms = 100 * 2_u64.pow(retry as u32);
                                log::warn!(
                                    "[{}] Send failed, retry {}/{} after {}ms: {:?}",
                                    session_id,
                                    retry + 1,
                                    max_retries,
                                    delay_ms,
                                    last_error
                                );
                                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms))
                                    .await;
                            }
                        }
                    }
                }

                // Если все retry провалились
                if let Some(e) = last_error {
                    log::error!(
                        "[{}] Failed to send update to Phoenix after {} retries: {:?}",
                        session_id,
                        max_retries,
                        e
                    );
                }
            } else {
                log::error!("[{}] Failed to serialize update", session_id);
            }
        }
    }

    /// Загрузить данные сессии из SQLite файла (блокирующая операция)
    fn load_session_data(session_path: &str) -> Result<SessionData, Box<dyn std::error::Error + Send + Sync>> {
        use sqlite::State;

        let conn = Connection::open(session_path)?;

        // Установить user_version = 1 для существующей grammers сессии
        conn.execute("PRAGMA user_version = 1")?;

        // Читаем app_id
        let mut stmt = conn.prepare("SELECT value FROM body WHERE key = 'app_id'")?;
        let app_id: i32 = if let State::Row = stmt.next()? {
            let app_id_i64: i64 = stmt.read::<i64, _>(0)?;
            app_id_i64
                .try_into()
                .map_err(|_| format!("app_id overflow: {} exceeds i32::MAX", app_id_i64))?
        } else {
            return Err("app_id not found".into());
        };

        // Читаем остальные поля
        let device = Self::read_string(&conn, "device")?;
        let sdk = Self::read_string(&conn, "sdk")?;
        let app_version = Self::read_string(&conn, "app_version")?;
        let lang_code = Self::read_string(&conn, "lang_code")?;
        let system_lang_code = Self::read_string(&conn, "system_lang_code")?;

        // Читаем proxy (опционально)
        let proxy = Self::read_string(&conn, "proxy")
            .ok()
            .and_then(|proxy_str| {
                if proxy_str.is_empty() {
                    None
                } else {
                    Some(proxy_str)
                }
            });

        Ok(SessionData {
            app_id,
            device,
            sdk,
            app_version,
            lang_code,
            system_lang_code,
            proxy,
        })
    }

    /// Прочитать строку из body таблицы
    fn read_string(
        conn: &Connection,
        key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        use sqlite::State;

        // Используем параметризованный запрос для защиты от SQL injection
        let query = "SELECT value FROM body WHERE key = ?";
        let mut stmt = conn.prepare(query)?;
        stmt.bind((1, key))?;

        let value = if let State::Row = stmt.next()? {
            stmt.read::<String, _>(0)?
        } else {
            return Err(format!("{} not found", key).into());
        };

        // Убираем кавычки из JSON строки
        let cleaned = value.trim_matches('"').to_string();
        Ok(cleaned)
    }
}
