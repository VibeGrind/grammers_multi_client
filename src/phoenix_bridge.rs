use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub first_name: String,
    pub username: Option<String>,
}

pub struct PhoenixBridge {
    channel: Arc<phoenix_channels_client::Channel>,
    topic: String,
}

impl PhoenixBridge {
    pub async fn new(url: &str, topic: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        use phoenix_channels_client::{Client as PhxClient, Config};
        use std::time::Duration;

        log::info!("Connecting to Phoenix Channel: {}", url);
        let config = Config::new(url)?;
        let mut client = PhxClient::new(config)?;
        client.connect().await?;

        log::info!("Joining channel: {}", topic);
        let channel = client.join(topic, Some(Duration::from_secs(10))).await?;
        log::info!("Successfully joined Phoenix channel");

        Ok(Self {
            channel,
            topic: topic.to_string(),
        })
    }

    /// Отправляет update в Phoenix Channel (fire-and-forget, без ожидания ответа)
    pub async fn send_update(&self, update: TelegramUpdate) {
        match serde_json::to_value(&update) {
            Ok(json_value) => {
                // send_noreply = fire-and-forget (не ждем ответа)
                // Это самый быстрый способ отправки без буферизации
                if let Err(e) = self.channel.send_noreply("telegram_update", json_value).await {
                    log::error!("Failed to send update to Phoenix: {:?}", e);
                }
            }
            Err(e) => {
                log::error!("Failed to serialize update: {:?}", e);
            }
        }
    }

    /// Отправляет update с ожиданием подтверждения (если нужна надежность)
    #[allow(dead_code)]
    pub async fn send_update_with_confirmation(
        &self,
        update: TelegramUpdate
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::time::Duration;

        let json_value = serde_json::to_value(&update)?;

        // send_with_timeout = ждем ответ от сервера (более медленно, но надежно)
        self.channel.send_with_timeout("telegram_update", json_value, Some(Duration::from_secs(5))).await?;

        Ok(())
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }
}

impl Clone for PhoenixBridge {
    fn clone(&self) -> Self {
        Self {
            channel: Arc::clone(&self.channel),
            topic: self.topic.clone(),
        }
    }
}
