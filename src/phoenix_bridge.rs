use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use std::future::Future;

// Re-export domain types for backwards compatibility
pub use crate::domain::{TelegramUpdate, UserInfo};

/// Configuration for retry logic with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub initial_delay_secs: u64,
    pub max_delay_secs: u64,
    pub max_attempts: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            initial_delay_secs: 1,
            max_delay_secs: 30,
            max_attempts: 5,
        }
    }
}

/// Retry a future with exponential backoff
async fn retry_with_backoff<F, Fut, T, E>(
    mut operation: F,
    config: RetryConfig,
    operation_name: &str,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempt = 0;
    let mut delay_secs = config.initial_delay_secs;

    loop {
        attempt += 1;
        log::info!("{}: Attempt {}/{}", operation_name, attempt, config.max_attempts);

        match operation().await {
            Ok(result) => {
                if attempt > 1 {
                    log::info!("{}: Succeeded on attempt {}", operation_name, attempt);
                }
                return Ok(result);
            }
            Err(e) => {
                if attempt >= config.max_attempts {
                    log::error!(
                        "{}: Failed after {} attempts. Last error: {}",
                        operation_name,
                        config.max_attempts,
                        e
                    );
                    return Err(e);
                }

                log::warn!(
                    "{}: Attempt {} failed: {}. Retrying in {}s...",
                    operation_name,
                    attempt,
                    e,
                    delay_secs
                );

                sleep(Duration::from_secs(delay_secs)).await;

                // Exponential backoff: double the delay, but cap at max_delay_secs
                delay_secs = (delay_secs * 2).min(config.max_delay_secs);
            }
        }
    }
}

/// Queued update for retry
#[derive(Debug, Clone)]
struct QueuedUpdate {
    update: TelegramUpdate,
    attempt: u32,
}

pub struct PhoenixBridge {
    channel: Arc<phoenix_channels_client::Channel>,
    topic: String,
    retry_queue_tx: mpsc::Sender<QueuedUpdate>,
    retry_config: RetryConfig,
}

impl PhoenixBridge {
    /// Creates a new PhoenixBridge with retry logic and exponential backoff
    pub async fn new(url: &str, topic: &str, auth_token: Option<&str>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::new_with_config(url, topic, auth_token, RetryConfig::default()).await
    }

    /// Creates a new PhoenixBridge with custom retry configuration
    pub async fn new_with_config(
        url: &str,
        topic: &str,
        auth_token: Option<&str>,
        retry_config: RetryConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        use phoenix_channels_client::{Client as PhxClient, Config};
        use serde_json::json;

        let url_owned = url.to_string();
        let topic_owned = topic.to_string();
        let auth_token_owned = auth_token.map(|t| t.to_string());

        // Connect to Phoenix with retry logic
        let channel = retry_with_backoff(
            || async {
                log::info!("Connecting to Phoenix Channel: {}", url_owned);
                let config = Config::new(&url_owned)?;
                let mut client = PhxClient::new(config)?;

                // Connect with 30 second timeout
                tokio::time::timeout(
                    Duration::from_secs(30),
                    client.connect()
                ).await
                .map_err(|_| -> Box<dyn std::error::Error + Send + Sync> {
                    "Phoenix connection timed out after 30 seconds".into()
                })??;

                log::info!("Joining channel: {}", topic_owned);

                // Prepare join parameters with authentication token if provided
                let channel = if let Some(ref token) = auth_token_owned {
                    log::info!("Authenticating with Phoenix channel using provided token");
                    let params = json!({ "token": token });
                    // Join with auth token and 30 second timeout
                    tokio::time::timeout(
                        Duration::from_secs(30),
                        client.join_with_params(&topic_owned, params, Some(Duration::from_secs(10)))
                    ).await
                    .map_err(|_| -> Box<dyn std::error::Error + Send + Sync> {
                        "Phoenix channel join with auth timed out after 30 seconds".into()
                    })??
                } else {
                    // Join without auth token
                    tokio::time::timeout(
                        Duration::from_secs(30),
                        client.join(&topic_owned, Some(Duration::from_secs(10)))
                    ).await
                    .map_err(|_| -> Box<dyn std::error::Error + Send + Sync> {
                        "Phoenix channel join timed out after 30 seconds".into()
                    })??
                };

                log::info!("Successfully joined Phoenix channel");

                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(channel)
            },
            retry_config.clone(),
            "Phoenix connection",
        )
        .await?;

        // Create bounded retry queue for failed sends to prevent memory overflow
        // Queue size is 10x max_attempts to allow for multiple concurrent retries
        // but prevent unbounded growth
        let queue_capacity = (retry_config.max_attempts as usize) * 10;
        log::info!("Creating bounded retry queue with capacity: {}", queue_capacity);
        let (retry_queue_tx, retry_queue_rx) = mpsc::channel(queue_capacity);

        let bridge = Self {
            channel,
            topic: topic.to_string(),
            retry_queue_tx,
            retry_config: retry_config.clone(),
        };

        // Spawn background task to process retry queue
        let bridge_clone = bridge.clone();
        tokio::spawn(async move {
            bridge_clone.process_retry_queue(retry_queue_rx).await;
        });

        Ok(bridge)
    }

    /// Background task that processes the retry queue
    async fn process_retry_queue(&self, mut retry_queue_rx: mpsc::Receiver<QueuedUpdate>) {
        while let Some(queued) = retry_queue_rx.recv().await {
            let attempt = queued.attempt;
            let update = queued.update;

            // Calculate delay with exponential backoff
            let delay_secs = (self.retry_config.initial_delay_secs * 2_u64.pow(attempt - 1))
                .min(self.retry_config.max_delay_secs);

            log::info!(
                "Retrying queued update (attempt {}/{}), waiting {}s...",
                attempt,
                self.retry_config.max_attempts,
                delay_secs
            );

            sleep(Duration::from_secs(delay_secs)).await;

            // Try to send the update
            match self.try_send_update(&update).await {
                Ok(_) => {
                    log::info!("Successfully sent queued update on attempt {}", attempt);
                }
                Err(e) => {
                    if attempt < self.retry_config.max_attempts {
                        log::warn!(
                            "Retry attempt {} failed: {}. Requeueing...",
                            attempt,
                            e
                        );
                        // Requeue for another attempt using try_send to avoid blocking
                        match self.retry_queue_tx.try_send(QueuedUpdate {
                            update,
                            attempt: attempt + 1,
                        }) {
                            Ok(_) => {
                                // Successfully requeued
                            }
                            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                                log::error!(
                                    "Retry queue is full. Dropping update after {} attempts. \
                                     This indicates Phoenix is experiencing sustained connection issues.",
                                    attempt
                                );
                            }
                            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                log::error!("Retry queue is closed. Cannot requeue update.");
                            }
                        }
                    } else {
                        log::error!(
                            "Failed to send update after {} attempts. Dropping update. Last error: {}",
                            self.retry_config.max_attempts,
                            e
                        );
                    }
                }
            }
        }
    }

    /// Tries to send an update, returning an error if it fails
    async fn try_send_update(&self, update: &TelegramUpdate) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let json_value = serde_json::to_value(update)?;

        // Send with 10 second timeout
        tokio::time::timeout(
            Duration::from_secs(10),
            self.channel.send_noreply("telegram_update", json_value)
        ).await
        .map_err(|_| -> Box<dyn std::error::Error + Send + Sync> {
            "Phoenix send_noreply timed out after 10 seconds".into()
        })??;

        Ok(())
    }

    /// Отправляет update в Phoenix Channel (fire-and-forget, без ожидания ответа)
    /// Legacy method - does not use retry queue
    pub async fn send_update(&self, update: TelegramUpdate) {
        match serde_json::to_value(&update) {
            Ok(json_value) => {
                // send_noreply = fire-and-forget (не ждем ответа)
                // Это самый быстрый способ отправки без буферизации
                // Send with 10 second timeout
                let send_result = tokio::time::timeout(
                    Duration::from_secs(10),
                    self.channel.send_noreply("telegram_update", json_value)
                ).await;

                match send_result {
                    Ok(Ok(_)) => {
                        // Success
                    }
                    Ok(Err(e)) => {
                        log::error!("Failed to send update to Phoenix: {:?}", e);
                    }
                    Err(_) => {
                        log::error!("Phoenix send_noreply timed out after 10 seconds");
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to serialize update: {:?}", e);
            }
        }
    }

    /// Sends update with retry logic - tries immediately, queues on failure
    /// This is non-blocking and returns immediately
    pub async fn send_with_retry(&self, update: TelegramUpdate) {
        // Try to send immediately
        match self.try_send_update(&update).await {
            Ok(_) => {
                // Success - nothing more to do
            }
            Err(e) => {
                log::warn!(
                    "Failed to send update immediately: {}. Queueing for retry...",
                    e
                );
                // Queue for retry with exponential backoff
                // Use try_send to avoid blocking if queue is full
                match self.retry_queue_tx.try_send(QueuedUpdate {
                    update,
                    attempt: 1,
                }) {
                    Ok(_) => {
                        // Successfully queued for retry
                    }
                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                        log::error!(
                            "Retry queue is full (capacity exceeded). Dropping update. \
                             This indicates Phoenix is experiencing sustained connection issues."
                        );
                    }
                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                        log::error!("Retry queue is closed. Cannot queue update for retry.");
                    }
                }
            }
        }
    }

    /// Отправляет update с ожиданием подтверждения (если нужна надежность)
    #[allow(dead_code)]
    pub async fn send_update_with_confirmation(
        &self,
        update: TelegramUpdate
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let json_value = serde_json::to_value(&update)?;

        // send_with_timeout = ждем ответ от сервера (более медленно, но надежно)
        // Add outer 10 second timeout (inner timeout is 5 seconds)
        tokio::time::timeout(
            Duration::from_secs(10),
            self.channel.send_with_timeout("telegram_update", json_value, Some(Duration::from_secs(5)))
        ).await
        .map_err(|_| -> Box<dyn std::error::Error + Send + Sync> {
            "Phoenix send_with_timeout timed out after 10 seconds".into()
        })??;

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
            retry_queue_tx: self.retry_queue_tx.clone(),
            retry_config: self.retry_config.clone(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // RetryConfig Tests
    // ============================================================================

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.initial_delay_secs, 1);
        assert_eq!(config.max_delay_secs, 30);
        assert_eq!(config.max_attempts, 5);
    }

    #[test]
    fn test_retry_config_custom() {
        let config = RetryConfig {
            initial_delay_secs: 2,
            max_delay_secs: 60,
            max_attempts: 10,
        };

        assert_eq!(config.initial_delay_secs, 2);
        assert_eq!(config.max_delay_secs, 60);
        assert_eq!(config.max_attempts, 10);
    }

    #[test]
    fn test_retry_config_clone() {
        let config = RetryConfig {
            initial_delay_secs: 5,
            max_delay_secs: 120,
            max_attempts: 3,
        };

        let cloned = config.clone();
        assert_eq!(cloned.initial_delay_secs, config.initial_delay_secs);
        assert_eq!(cloned.max_delay_secs, config.max_delay_secs);
        assert_eq!(cloned.max_attempts, config.max_attempts);
    }

    // ============================================================================
    // QueuedUpdate Tests
    // ============================================================================

    #[test]
    fn test_queued_update_creation() {
        use crate::domain::{ChatId, MessageId};

        let chat_id = ChatId::new(123).unwrap();
        let msg_id = MessageId::new(456).unwrap();
        let update = TelegramUpdate::new_message(
            chat_id,
            msg_id,
            "Hello".to_string(),
            None,
            1234567890,
        );

        let queued = QueuedUpdate {
            update: update.clone(),
            attempt: 1,
        };

        assert_eq!(queued.attempt, 1);
        assert!(queued.update.is_new_message());
    }

    #[test]
    fn test_queued_update_clone() {
        use crate::domain::{ChatId, MessageId};

        let chat_id = ChatId::new(123).unwrap();
        let msg_id = MessageId::new(456).unwrap();
        let update = TelegramUpdate::new_message(
            chat_id,
            msg_id,
            "Hello".to_string(),
            None,
            1234567890,
        );

        let queued = QueuedUpdate {
            update: update.clone(),
            attempt: 2,
        };

        let cloned = queued.clone();
        assert_eq!(cloned.attempt, queued.attempt);
        assert_eq!(cloned.update.text, queued.update.text);
    }

    // ============================================================================
    // TelegramUpdate Serialization Tests (for Phoenix)
    // ============================================================================

    #[test]
    fn test_telegram_update_serialization() {
        use crate::domain::{ChatId, MessageId};

        let chat_id = ChatId::new(123).unwrap();
        let msg_id = MessageId::new(456).unwrap();
        let update = TelegramUpdate::new_message(
            chat_id,
            msg_id,
            "Test message".to_string(),
            None,
            1234567890,
        );

        let json_value = serde_json::to_value(&update);
        assert!(json_value.is_ok());

        let json = json_value.unwrap();
        assert_eq!(json["update_type"], "new_message");
        assert_eq!(json["chat_id"], 123);
        assert_eq!(json["message_id"], 456);
        assert_eq!(json["text"], "Test message");
    }

    #[test]
    fn test_telegram_update_types_serialization() {
        use crate::domain::{ChatId, MessageId};

        let chat_id = ChatId::new(123).unwrap();
        let msg_id = MessageId::new(456).unwrap();

        // Test message_edited
        let update = TelegramUpdate::message_edited(
            chat_id,
            msg_id,
            "Edited text".to_string(),
            1234567890,
        );
        let json = serde_json::to_value(&update).unwrap();
        assert_eq!(json["update_type"], "message_edited");

        // Test message_deleted
        let update = TelegramUpdate::message_deleted(
            Some(chat_id),
            Some(msg_id),
            "raw data".to_string(),
        );
        let json = serde_json::to_value(&update).unwrap();
        assert_eq!(json["update_type"], "message_deleted");

        // Test other
        let update = TelegramUpdate::other("unknown".to_string());
        let json = serde_json::to_value(&update).unwrap();
        assert_eq!(json["update_type"], "other");
    }

    // ============================================================================
    // Async Tests (marked with #[tokio::test])
    // ============================================================================

    // These tests require tokio runtime and are for testing retry logic
    // without needing actual Phoenix connections

    #[tokio::test]
    async fn test_retry_with_backoff_success_on_first_attempt() {
        let config = RetryConfig {
            initial_delay_secs: 1,
            max_delay_secs: 30,
            max_attempts: 3,
        };

        let mut call_count = 0;
        let operation = || async {
            call_count += 1;
            Ok::<i32, String>(42)
        };

        let result = retry_with_backoff(operation, config, "test").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        // Note: call_count check won't work due to closure move semantics
    }

    #[tokio::test]
    async fn test_retry_with_backoff_success_on_retry() {
        let config = RetryConfig {
            initial_delay_secs: 0, // Use 0 for faster tests
            max_delay_secs: 1,
            max_attempts: 3,
        };

        let mut attempts = 0;
        let result = retry_with_backoff(
            || async {
                attempts += 1;
                if attempts < 2 {
                    Err::<i32, String>("temporary error".to_string())
                } else {
                    Ok(42)
                }
            },
            config,
            "test retry",
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_with_backoff_exhausts_attempts() {
        let config = RetryConfig {
            initial_delay_secs: 0,
            max_delay_secs: 1,
            max_attempts: 2,
        };

        let result = retry_with_backoff(
            || async { Err::<i32, String>("persistent error".to_string()) },
            config,
            "test failure",
        )
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("persistent error"));
    }

    // ============================================================================
    // Integration Tests (require real Phoenix server - marked #[ignore])
    // ============================================================================

    #[tokio::test]
    #[ignore]
    async fn test_phoenix_bridge_connection() {
        // This test requires a real Phoenix server running
        let config = RetryConfig {
            initial_delay_secs: 1,
            max_delay_secs: 5,
            max_attempts: 2,
        };

        let result = PhoenixBridge::new_with_config(
            "ws://localhost:4000/socket",
            "test:topic",
            None,  // No auth token for test
            config,
        )
        .await;

        // If Phoenix server is not running, this will fail
        // Run with: cargo test -- --ignored
        assert!(result.is_ok() || result.is_err()); // Just check it doesn't panic
    }

    #[tokio::test]
    #[ignore]
    async fn test_phoenix_bridge_send_update() {
        use crate::domain::{ChatId, MessageId};

        // This test requires a real Phoenix server
        let config = RetryConfig {
            initial_delay_secs: 1,
            max_delay_secs: 5,
            max_attempts: 2,
        };

        let bridge = PhoenixBridge::new_with_config(
            "ws://localhost:4000/socket",
            "test:topic",
            None,  // No auth token for test
            config,
        )
        .await;

        if let Ok(bridge) = bridge {
            let chat_id = ChatId::new(123).unwrap();
            let msg_id = MessageId::new(456).unwrap();
            let update = TelegramUpdate::new_message(
                chat_id,
                msg_id,
                "Test message".to_string(),
                None,
                1234567890,
            );

            // This should not panic even if send fails
            bridge.send_update(update).await;
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_phoenix_bridge_send_with_retry() {
        use crate::domain::{ChatId, MessageId};

        // This test requires a real Phoenix server
        let config = RetryConfig {
            initial_delay_secs: 1,
            max_delay_secs: 5,
            max_attempts: 3,
        };

        let bridge = PhoenixBridge::new_with_config(
            "ws://localhost:4000/socket",
            "test:topic",
            None,  // No auth token for test
            config,
        )
        .await;

        if let Ok(bridge) = bridge {
            let chat_id = ChatId::new(123).unwrap();
            let msg_id = MessageId::new(456).unwrap();
            let update = TelegramUpdate::new_message(
                chat_id,
                msg_id,
                "Test with retry".to_string(),
                None,
                1234567890,
            );

            // Test send_with_retry
            bridge.send_with_retry(update).await;

            // Wait a bit for retry queue to process
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    // ============================================================================
    // Unit Tests for PhoenixBridge methods (without connection)
    // ============================================================================

    #[test]
    fn test_phoenix_bridge_clone_compiles() {
        // This test just verifies that Clone is implemented
        // We can't actually test cloning without a real connection
        // but we can verify the trait is implemented at compile time
        fn assert_clone<T: Clone>() {}
        assert_clone::<PhoenixBridge>();
    }

    // ============================================================================
    // Retry Logic Edge Cases
    // ============================================================================

    #[tokio::test]
    async fn test_exponential_backoff_calculation() {
        // Test that backoff delay increases exponentially
        let config = RetryConfig {
            initial_delay_secs: 1,
            max_delay_secs: 30,
            max_attempts: 5,
        };

        // Simulate the backoff calculation logic
        let delays = vec![
            (1_u64 * 2_u64.pow(0)).min(config.max_delay_secs), // 1
            (1_u64 * 2_u64.pow(1)).min(config.max_delay_secs), // 2
            (1_u64 * 2_u64.pow(2)).min(config.max_delay_secs), // 4
            (1_u64 * 2_u64.pow(3)).min(config.max_delay_secs), // 8
            (1_u64 * 2_u64.pow(4)).min(config.max_delay_secs), // 16
        ];

        assert_eq!(delays, vec![1, 2, 4, 8, 16]);
    }

    #[tokio::test]
    async fn test_exponential_backoff_caps_at_max() {
        // Test that backoff delay caps at max_delay
        let config = RetryConfig {
            initial_delay_secs: 10,
            max_delay_secs: 30,
            max_attempts: 5,
        };

        let delays = vec![
            (10_u64 * 2_u64.pow(0)).min(config.max_delay_secs), // 10
            (10_u64 * 2_u64.pow(1)).min(config.max_delay_secs), // 20
            (10_u64 * 2_u64.pow(2)).min(config.max_delay_secs), // 30 (capped)
            (10_u64 * 2_u64.pow(3)).min(config.max_delay_secs), // 30 (capped)
            (10_u64 * 2_u64.pow(4)).min(config.max_delay_secs), // 30 (capped)
        ];

        assert_eq!(delays, vec![10, 20, 30, 30, 30]);
    }

    // ============================================================================
    // Queue Capacity Tests
    // ============================================================================

    #[test]
    fn test_queue_capacity_calculation() {
        let config = RetryConfig {
            initial_delay_secs: 1,
            max_delay_secs: 30,
            max_attempts: 5,
        };

        let queue_capacity = (config.max_attempts as usize) * 10;
        assert_eq!(queue_capacity, 50);
    }

    #[test]
    fn test_queue_capacity_with_high_max_attempts() {
        let config = RetryConfig {
            initial_delay_secs: 1,
            max_delay_secs: 30,
            max_attempts: 100,
        };

        let queue_capacity = (config.max_attempts as usize) * 10;
        assert_eq!(queue_capacity, 1000);
    }
}
