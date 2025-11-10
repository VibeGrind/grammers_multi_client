use crate::errors::ManagerError;
use dashmap::DashMap;
use phoenix_channels_client::{Channel, Client as PhxClient, Config};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

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
        })
    }

    /// Получить или создать канал для сессии
    /// Topic формат: "telegram:updates:{session_id}"
    pub async fn get_or_create_channel(
        &self,
        session_id: &str,
    ) -> Result<Arc<Channel>, ManagerError> {
        use dashmap::mapref::entry::Entry;

        // Быстрая проверка существующего канала
        if let Some(channel) = self.channels.get(session_id) {
            log::debug!("[Phoenix] Using cached channel for session: {}", session_id);
            return Ok(Arc::clone(channel.value()));
        }

        // Создаём новый канал (вне лока)
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

        // Атомарная вставка: insert-if-absent (fix race condition)
        match self.channels.entry(session_id.to_string()) {
            Entry::Occupied(entry) => {
                // Другая задача уже создала канал - используем его
                log::debug!("[Phoenix] Channel created by another task, using existing");
                Ok(Arc::clone(entry.get()))
            }
            Entry::Vacant(entry) => {
                // Мы первые - вставляем наш канал
                entry.insert(Arc::clone(&channel_arc));
                log::info!("[Phoenix] Channel created and cached for session: {}", session_id);
                Ok(channel_arc)
            }
        }
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

    /// Удалить канал из кеша (при остановке сессии)
    pub fn remove_channel(&self, session_id: &str) {
        if self.channels.remove(session_id).is_some() {
            log::info!("[Phoenix] Channel removed from cache: {}", session_id);
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
        }
    }
}
