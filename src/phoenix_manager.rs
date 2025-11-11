use crate::errors::ManagerError;
use dashmap::DashMap;
use phoenix_channels_client::{Channel, Client as PhxClient, Config};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Структура для Telegram обновления
#[derive(Debug, Serialize, Deserialize)]
pub struct TelegramUpdate {
    pub update_type: String,
    pub chat_id: Option<i64>,
    pub message_id: Option<i32>,
    pub text: Option<String>,
    pub from_user: Option<UserInfo>,
    pub timestamp: Option<i64>,
    pub raw_data: Option<String>,
}

/// Информация о пользователе
#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub first_name: String,
    pub username: Option<String>,
}

/// Менеджер Phoenix - управление единым клиентом и множественными каналами
pub struct PhoenixManager {
    /// Единый Phoenix клиент (shared)
    client: Arc<PhxClient>,

    /// Кеш каналов: session_id → Arc<Channel>
    channels: Arc<DashMap<String, Arc<Channel>>>,

    /// Блокировки для предотвращения гонки при создании каналов
    creation_locks: Arc<DashMap<String, Arc<Mutex<()>>>>,
}

impl PhoenixManager {
    /// Создать новый PhoenixManager и подключиться к Phoenix серверу
    pub async fn new(url: &str) -> Result<Self, ManagerError> {
        log::info!("[Phoenix] Connecting to: {}", url);

        let config = Config::new(url)
            .map_err(|e| ManagerError::PhoenixError(format!("Config error: {:?}", e)))?;

        let mut client = PhxClient::new(config)
            .map_err(|e| ManagerError::PhoenixError(format!("Client creation error: {:?}", e)))?;

        client
            .connect()
            .await
            .map_err(|e| ManagerError::PhoenixError(format!("Connection error: {:?}", e)))?;

        log::info!("[Phoenix] Connected successfully");

        Ok(Self {
            client: Arc::new(client),
            channels: Arc::new(DashMap::new()),
            creation_locks: Arc::new(DashMap::new()),
        })
    }

    /// Получить или создать канал для сессии
    /// Topic формат: "telegram:updates:{session_id}"
    pub async fn get_or_create_channel(
        &self,
        session_id: &str,
    ) -> Result<Arc<Channel>, ManagerError> {
        // Быстрая проверка существующего канала (без блокировки)
        if let Some(channel) = self.channels.get(session_id) {
            log::debug!("[Phoenix] Using cached channel for session: {}", session_id);
            return Ok(Arc::clone(channel.value()));
        }

        // Получить или создать блокировку для этого session_id
        // Это предотвращает создание нескольких каналов одновременно
        let lock = self
            .creation_locks
            .entry(session_id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone();

        // Захватить блокировку (только один поток создаст канал)
        let _guard = lock.lock().await;

        // Double-check: другой поток мог создать канал пока мы ждали блокировку
        if let Some(channel) = self.channels.get(session_id) {
            log::debug!("[Phoenix] Channel created by another task, using existing");
            return Ok(Arc::clone(channel.value()));
        }

        // Теперь создаём новый канал (защищено блокировкой)
        let topic = format!("telegram:updates:{}", session_id);
        log::info!("[Phoenix] Creating channel for topic: {}", topic);

        let channel = self
            .client
            .join(&topic, Some(Duration::from_secs(10)))
            .await
            .map_err(|e| {
                ManagerError::PhoenixError(format!("Failed to join topic '{}': {:?}", topic, e))
            })?;

        let channel_arc = Arc::new(channel);

        // Вставляем в кеш
        self.channels
            .insert(session_id.to_string(), Arc::clone(&channel_arc));

        log::info!(
            "[Phoenix] Channel created and cached for session: {}",
            session_id
        );

        // Очистка: удаляем блокировку для экономии памяти
        self.creation_locks.remove(session_id);

        Ok(channel_arc)
    }

    /// Отправить обновление в канал сессии (fire-and-forget)
    pub async fn send_update(
        &self,
        session_id: &str,
        update: TelegramUpdate,
    ) -> Result<(), ManagerError> {
        let channel = self.get_or_create_channel(session_id).await?;

        let json_value = serde_json::to_value(&update).map_err(|e| {
            ManagerError::PhoenixError(format!("Serialization error: {:?}", e))
        })?;

        channel
            .send_noreply("telegram_update", json_value)
            .await
            .map_err(|e| {
                ManagerError::PhoenixError(format!(
                    "Failed to send update for session {}: {:?}",
                    session_id, e
                ))
            })?;

        log::debug!("[Phoenix] Update sent for session: {}", session_id);

        Ok(())
    }

    /// Удалить канал и закрыть соединение (при остановке сессии)
    pub async fn remove_channel(&self, session_id: &str) -> Result<(), ManagerError> {
        if let Some((_, channel)) = self.channels.remove(session_id) {
            log::info!("[Phoenix] Removing channel for session: {}", session_id);

            // Явно покинуть канал на Phoenix сервере
            if let Err(e) = channel.leave().await {
                log::warn!(
                    "[Phoenix] Failed to leave channel for {}: {:?}",
                    session_id,
                    e
                );
                // Возвращаем ошибку чтобы caller мог retry
                return Err(ManagerError::PhoenixError(format!(
                    "Failed to leave channel: {:?}",
                    e
                )));
            } else {
                log::info!("[Phoenix] Channel left successfully: {}", session_id);
            }
        } else {
            log::debug!("[Phoenix] No channel to remove for: {}", session_id);
        }
        Ok(())
    }

    /// Force удалить канал из DashMap без graceful leave
    /// Используется когда leave() постоянно fails
    pub fn force_remove_channel(&self, session_id: &str) {
        if let Some((_, channel)) = self.channels.remove(session_id) {
            log::warn!(
                "[Phoenix] Force removed channel without graceful leave: {}",
                session_id
            );
            drop(channel); // Явно drop, connection закроется
        }
    }

    /// Получить количество активных каналов
    pub fn active_channels_count(&self) -> usize {
        self.channels.len()
    }

    /// Получить список всех активных session_id с каналами
    pub fn list_active_channels(&self) -> Vec<String> {
        self.channels
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }
}

impl Clone for PhoenixManager {
    fn clone(&self) -> Self {
        Self {
            client: Arc::clone(&self.client),
            channels: Arc::clone(&self.channels),
            creation_locks: Arc::clone(&self.creation_locks),
        }
    }
}
