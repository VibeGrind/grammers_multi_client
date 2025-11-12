use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Main application configuration containing all subsections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub session: SessionConfig,
    pub phoenix: PhoenixConfig,
    pub telegram: TelegramConfig,
    pub log: LogConfig,
}

/// Session and database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Path to the SQLite session file
    pub session_file: String,
    /// SQLite user_version to set for grammers compatibility
    pub sqlite_user_version: i32,
}

/// Phoenix Channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoenixConfig {
    /// Phoenix websocket URL
    pub url: String,
    /// Phoenix channel topic to join
    pub topic: String,
    /// Optional authentication token for Phoenix channel
    /// Set via PHOENIX_AUTH_TOKEN environment variable
    pub auth_token: Option<String>,
    /// Retry configuration for Phoenix operations
    pub retry: RetryConfig,
    /// Timeout for Phoenix connection (in seconds)
    pub connection_timeout_secs: u64,
    /// Timeout for Phoenix channel join (in seconds)
    pub join_timeout_secs: u64,
    /// Timeout for Phoenix send operations (in seconds)
    pub send_timeout_secs: u64,
}

/// Retry configuration with exponential backoff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Initial delay in seconds before first retry
    pub initial_delay_secs: u64,
    /// Maximum delay in seconds between retries
    pub max_delay_secs: u64,
    /// Maximum number of retry attempts
    pub max_attempts: u32,
}

/// Telegram client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Timeout for authorization check (in seconds)
    pub auth_timeout_secs: u64,
    /// Timeout for get_me() call (in seconds)
    pub get_me_timeout_secs: u64,
    /// Timeout for update stream polling (in seconds)
    pub update_stream_timeout_secs: u64,
    /// Maximum number of updates to queue (None for unlimited)
    pub update_queue_limit: Option<usize>,
    /// Whether to catch up on missed updates
    pub catch_up: bool,
    /// Enable automatic reconnection on connection loss
    pub enable_reconnection: bool,
    /// Maximum number of reconnection attempts (0 for unlimited)
    pub max_reconnection_attempts: u32,
    /// Initial delay before reconnection attempt (in seconds)
    pub reconnection_delay_secs: u64,
    /// Maximum delay between reconnection attempts (in seconds)
    pub max_reconnection_delay_secs: u64,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Default log level (trace, debug, info, warn, error)
    pub level: String,
    /// Log level for grammers module
    pub grammers_level: String,
}

// ============================================================================
// Default Implementations
// ============================================================================

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            session: SessionConfig::default(),
            phoenix: PhoenixConfig::default(),
            telegram: TelegramConfig::default(),
            log: LogConfig::default(),
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            session_file: "session/my.session".to_string(),
            sqlite_user_version: 1,
        }
    }
}

impl Default for PhoenixConfig {
    fn default() -> Self {
        Self {
            // SECURITY: Use wss:// (WebSocket Secure) by default instead of ws://
            // For local development, explicitly set PHOENIX_URL=ws://localhost:4000/socket
            url: "wss://localhost:4000/socket".to_string(),
            topic: "telegram:updates".to_string(),
            auth_token: None,  // No token by default - set via PHOENIX_AUTH_TOKEN
            retry: RetryConfig::default(),
            connection_timeout_secs: 30,
            join_timeout_secs: 30,
            send_timeout_secs: 10,
        }
    }
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

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            auth_timeout_secs: 30,
            get_me_timeout_secs: 30,
            update_stream_timeout_secs: 10,
            update_queue_limit: Some(100),  // Increased from 10 to 100 for production use
            catch_up: true,
            enable_reconnection: true,
            max_reconnection_attempts: 0,  // 0 means unlimited
            reconnection_delay_secs: 5,
            max_reconnection_delay_secs: 300,  // 5 minutes max
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            grammers_level: "debug".to_string(),
        }
    }
}

// ============================================================================
// Environment Variable Loading
// ============================================================================

impl AppConfig {
    /// Creates a new AppConfig by loading values from environment variables
    /// with fallback to defaults
    pub fn from_env() -> Self {
        Self {
            session: SessionConfig::from_env(),
            phoenix: PhoenixConfig::from_env(),
            telegram: TelegramConfig::from_env(),
            log: LogConfig::from_env(),
        }
    }

    /// Validates the entire configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.session.validate()?;
        self.phoenix.validate()?;
        self.telegram.validate()?;
        self.log.validate()?;
        Ok(())
    }
}

impl SessionConfig {
    /// Load session config from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(session_file) = std::env::var("SESSION_FILE") {
            config.session_file = session_file;
        }

        if let Ok(user_version) = std::env::var("SQLITE_USER_VERSION") {
            if let Ok(version) = user_version.parse() {
                config.sqlite_user_version = version;
            }
        }

        config
    }

    /// Validate session configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.session_file.is_empty() {
            return Err(ConfigError::Validation(
                "session_file cannot be empty".to_string()
            ));
        }

        if self.sqlite_user_version < 0 {
            return Err(ConfigError::Validation(
                "sqlite_user_version must be non-negative".to_string()
            ));
        }

        Ok(())
    }
}

impl PhoenixConfig {
    /// Load Phoenix config from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(url) = std::env::var("PHOENIX_URL") {
            config.url = url;
        }

        if let Ok(topic) = std::env::var("PHOENIX_TOPIC") {
            config.topic = topic;
        }

        // Load authentication token if provided
        if let Ok(token) = std::env::var("PHOENIX_AUTH_TOKEN") {
            if !token.is_empty() {
                config.auth_token = Some(token);
            }
        }

        if let Ok(timeout) = std::env::var("PHOENIX_CONNECTION_TIMEOUT_SECS") {
            if let Ok(secs) = timeout.parse() {
                config.connection_timeout_secs = secs;
            }
        }

        if let Ok(timeout) = std::env::var("PHOENIX_JOIN_TIMEOUT_SECS") {
            if let Ok(secs) = timeout.parse() {
                config.join_timeout_secs = secs;
            }
        }

        if let Ok(timeout) = std::env::var("PHOENIX_SEND_TIMEOUT_SECS") {
            if let Ok(secs) = timeout.parse() {
                config.send_timeout_secs = secs;
            }
        }

        config.retry = RetryConfig::from_env();

        config
    }

    /// Validate Phoenix configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.url.is_empty() {
            return Err(ConfigError::Validation(
                "phoenix.url cannot be empty".to_string()
            ));
        }

        if !self.url.starts_with("ws://") && !self.url.starts_with("wss://") {
            return Err(ConfigError::Validation(
                "phoenix.url must start with ws:// or wss://".to_string()
            ));
        }

        if self.topic.is_empty() {
            return Err(ConfigError::Validation(
                "phoenix.topic cannot be empty".to_string()
            ));
        }

        if self.connection_timeout_secs == 0 {
            return Err(ConfigError::Validation(
                "phoenix.connection_timeout_secs must be greater than 0".to_string()
            ));
        }

        if self.join_timeout_secs == 0 {
            return Err(ConfigError::Validation(
                "phoenix.join_timeout_secs must be greater than 0".to_string()
            ));
        }

        if self.send_timeout_secs == 0 {
            return Err(ConfigError::Validation(
                "phoenix.send_timeout_secs must be greater than 0".to_string()
            ));
        }

        self.retry.validate()?;

        Ok(())
    }

    /// Get connection timeout as Duration
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.connection_timeout_secs)
    }

    /// Get join timeout as Duration
    pub fn join_timeout(&self) -> Duration {
        Duration::from_secs(self.join_timeout_secs)
    }

    /// Get send timeout as Duration
    pub fn send_timeout(&self) -> Duration {
        Duration::from_secs(self.send_timeout_secs)
    }
}

impl RetryConfig {
    /// Load retry config from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(delay) = std::env::var("RETRY_INITIAL_DELAY_SECS") {
            if let Ok(secs) = delay.parse() {
                config.initial_delay_secs = secs;
            }
        }

        if let Ok(delay) = std::env::var("RETRY_MAX_DELAY_SECS") {
            if let Ok(secs) = delay.parse() {
                config.max_delay_secs = secs;
            }
        }

        if let Ok(attempts) = std::env::var("RETRY_MAX_ATTEMPTS") {
            if let Ok(max) = attempts.parse() {
                config.max_attempts = max;
            }
        }

        config
    }

    /// Validate retry configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.initial_delay_secs == 0 {
            return Err(ConfigError::Validation(
                "retry.initial_delay_secs must be greater than 0".to_string()
            ));
        }

        if self.max_delay_secs == 0 {
            return Err(ConfigError::Validation(
                "retry.max_delay_secs must be greater than 0".to_string()
            ));
        }

        if self.max_delay_secs < self.initial_delay_secs {
            return Err(ConfigError::Validation(
                "retry.max_delay_secs must be >= initial_delay_secs".to_string()
            ));
        }

        if self.max_attempts == 0 {
            return Err(ConfigError::Validation(
                "retry.max_attempts must be greater than 0".to_string()
            ));
        }

        Ok(())
    }
}

impl TelegramConfig {
    /// Load Telegram config from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(timeout) = std::env::var("TELEGRAM_AUTH_TIMEOUT_SECS") {
            if let Ok(secs) = timeout.parse() {
                config.auth_timeout_secs = secs;
            }
        }

        if let Ok(timeout) = std::env::var("TELEGRAM_GET_ME_TIMEOUT_SECS") {
            if let Ok(secs) = timeout.parse() {
                config.get_me_timeout_secs = secs;
            }
        }

        if let Ok(timeout) = std::env::var("TELEGRAM_UPDATE_STREAM_TIMEOUT_SECS") {
            if let Ok(secs) = timeout.parse() {
                config.update_stream_timeout_secs = secs;
            }
        }

        if let Ok(limit) = std::env::var("TELEGRAM_UPDATE_QUEUE_LIMIT") {
            if let Ok(queue_limit) = limit.parse() {
                config.update_queue_limit = Some(queue_limit);
            }
        }

        if let Ok(catch_up) = std::env::var("TELEGRAM_CATCH_UP") {
            if let Ok(enabled) = catch_up.parse() {
                config.catch_up = enabled;
            }
        }

        if let Ok(enable) = std::env::var("TELEGRAM_ENABLE_RECONNECTION") {
            if let Ok(enabled) = enable.parse() {
                config.enable_reconnection = enabled;
            }
        }

        if let Ok(attempts) = std::env::var("TELEGRAM_MAX_RECONNECTION_ATTEMPTS") {
            if let Ok(max) = attempts.parse() {
                config.max_reconnection_attempts = max;
            }
        }

        if let Ok(delay) = std::env::var("TELEGRAM_RECONNECTION_DELAY_SECS") {
            if let Ok(secs) = delay.parse() {
                config.reconnection_delay_secs = secs;
            }
        }

        if let Ok(max_delay) = std::env::var("TELEGRAM_MAX_RECONNECTION_DELAY_SECS") {
            if let Ok(secs) = max_delay.parse() {
                config.max_reconnection_delay_secs = secs;
            }
        }

        config
    }

    /// Validate Telegram configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.auth_timeout_secs == 0 {
            return Err(ConfigError::Validation(
                "telegram.auth_timeout_secs must be greater than 0".to_string()
            ));
        }

        if self.get_me_timeout_secs == 0 {
            return Err(ConfigError::Validation(
                "telegram.get_me_timeout_secs must be greater than 0".to_string()
            ));
        }

        if self.update_stream_timeout_secs == 0 {
            return Err(ConfigError::Validation(
                "telegram.update_stream_timeout_secs must be greater than 0".to_string()
            ));
        }

        if let Some(limit) = self.update_queue_limit {
            if limit == 0 {
                return Err(ConfigError::Validation(
                    "telegram.update_queue_limit must be greater than 0 or None".to_string()
                ));
            }
        }

        if self.reconnection_delay_secs == 0 {
            return Err(ConfigError::Validation(
                "telegram.reconnection_delay_secs must be greater than 0".to_string()
            ));
        }

        if self.max_reconnection_delay_secs == 0 {
            return Err(ConfigError::Validation(
                "telegram.max_reconnection_delay_secs must be greater than 0".to_string()
            ));
        }

        if self.max_reconnection_delay_secs < self.reconnection_delay_secs {
            return Err(ConfigError::Validation(
                "telegram.max_reconnection_delay_secs must be >= reconnection_delay_secs".to_string()
            ));
        }

        Ok(())
    }

    /// Get authorization timeout as Duration
    pub fn auth_timeout(&self) -> Duration {
        Duration::from_secs(self.auth_timeout_secs)
    }

    /// Get get_me timeout as Duration
    pub fn get_me_timeout(&self) -> Duration {
        Duration::from_secs(self.get_me_timeout_secs)
    }

    /// Get update stream timeout as Duration
    pub fn update_stream_timeout(&self) -> Duration {
        Duration::from_secs(self.update_stream_timeout_secs)
    }
}

impl LogConfig {
    /// Load log config from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(level) = std::env::var("LOG_LEVEL") {
            config.level = level.to_lowercase();
        }

        if let Ok(level) = std::env::var("LOG_GRAMMERS_LEVEL") {
            config.grammers_level = level.to_lowercase();
        }

        config
    }

    /// Validate log configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        let valid_levels = ["trace", "debug", "info", "warn", "error"];

        if !valid_levels.contains(&self.level.as_str()) {
            return Err(ConfigError::Validation(
                format!("log.level must be one of: {:?}", valid_levels)
            ));
        }

        if !valid_levels.contains(&self.grammers_level.as_str()) {
            return Err(ConfigError::Validation(
                format!("log.grammers_level must be one of: {:?}", valid_levels)
            ));
        }

        Ok(())
    }

    /// Convert log level string to log::LevelFilter
    pub fn level_filter(&self) -> log::LevelFilter {
        match self.level.as_str() {
            "trace" => log::LevelFilter::Trace,
            "debug" => log::LevelFilter::Debug,
            "info" => log::LevelFilter::Info,
            "warn" => log::LevelFilter::Warn,
            "error" => log::LevelFilter::Error,
            _ => log::LevelFilter::Info, // Default fallback
        }
    }

    /// Convert grammers log level string to log::LevelFilter
    pub fn grammers_level_filter(&self) -> log::LevelFilter {
        match self.grammers_level.as_str() {
            "trace" => log::LevelFilter::Trace,
            "debug" => log::LevelFilter::Debug,
            "info" => log::LevelFilter::Info,
            "warn" => log::LevelFilter::Warn,
            "error" => log::LevelFilter::Error,
            _ => log::LevelFilter::Debug, // Default fallback
        }
    }
}

// ============================================================================
// Builder Pattern
// ============================================================================

/// Builder for constructing AppConfig with a fluent API
#[derive(Debug, Default)]
pub struct AppConfigBuilder {
    session: Option<SessionConfig>,
    phoenix: Option<PhoenixConfig>,
    telegram: Option<TelegramConfig>,
    log: Option<LogConfig>,
}

impl AppConfigBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set session configuration
    pub fn session(mut self, session: SessionConfig) -> Self {
        self.session = Some(session);
        self
    }

    /// Set Phoenix configuration
    pub fn phoenix(mut self, phoenix: PhoenixConfig) -> Self {
        self.phoenix = Some(phoenix);
        self
    }

    /// Set Telegram configuration
    pub fn telegram(mut self, telegram: TelegramConfig) -> Self {
        self.telegram = Some(telegram);
        self
    }

    /// Set log configuration
    pub fn log(mut self, log: LogConfig) -> Self {
        self.log = Some(log);
        self
    }

    /// Build the AppConfig, using defaults for unset fields
    pub fn build(self) -> AppConfig {
        AppConfig {
            session: self.session.unwrap_or_default(),
            phoenix: self.phoenix.unwrap_or_default(),
            telegram: self.telegram.unwrap_or_default(),
            log: self.log.unwrap_or_default(),
        }
    }

    /// Build and validate the AppConfig
    pub fn build_validated(self) -> Result<AppConfig, ConfigError> {
        let config = self.build();
        config.validate()?;
        Ok(config)
    }
}

/// Builder for SessionConfig
#[derive(Debug, Default)]
pub struct SessionConfigBuilder {
    session_file: Option<String>,
    sqlite_user_version: Option<i32>,
}

impl SessionConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn session_file(mut self, path: impl Into<String>) -> Self {
        self.session_file = Some(path.into());
        self
    }

    pub fn sqlite_user_version(mut self, version: i32) -> Self {
        self.sqlite_user_version = Some(version);
        self
    }

    pub fn build(self) -> SessionConfig {
        let default = SessionConfig::default();
        SessionConfig {
            session_file: self.session_file.unwrap_or(default.session_file),
            sqlite_user_version: self.sqlite_user_version.unwrap_or(default.sqlite_user_version),
        }
    }
}

/// Builder for PhoenixConfig
#[derive(Debug, Default)]
pub struct PhoenixConfigBuilder {
    url: Option<String>,
    topic: Option<String>,
    auth_token: Option<Option<String>>,
    retry: Option<RetryConfig>,
    connection_timeout_secs: Option<u64>,
    join_timeout_secs: Option<u64>,
    send_timeout_secs: Option<u64>,
}

impl PhoenixConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn topic(mut self, topic: impl Into<String>) -> Self {
        self.topic = Some(topic.into());
        self
    }

    pub fn auth_token(mut self, token: Option<String>) -> Self {
        self.auth_token = Some(token);
        self
    }

    pub fn retry(mut self, retry: RetryConfig) -> Self {
        self.retry = Some(retry);
        self
    }

    pub fn connection_timeout_secs(mut self, secs: u64) -> Self {
        self.connection_timeout_secs = Some(secs);
        self
    }

    pub fn join_timeout_secs(mut self, secs: u64) -> Self {
        self.join_timeout_secs = Some(secs);
        self
    }

    pub fn send_timeout_secs(mut self, secs: u64) -> Self {
        self.send_timeout_secs = Some(secs);
        self
    }

    pub fn build(self) -> PhoenixConfig {
        let default = PhoenixConfig::default();
        PhoenixConfig {
            url: self.url.unwrap_or(default.url),
            topic: self.topic.unwrap_or(default.topic),
            auth_token: self.auth_token.unwrap_or(default.auth_token),
            retry: self.retry.unwrap_or(default.retry),
            connection_timeout_secs: self.connection_timeout_secs.unwrap_or(default.connection_timeout_secs),
            join_timeout_secs: self.join_timeout_secs.unwrap_or(default.join_timeout_secs),
            send_timeout_secs: self.send_timeout_secs.unwrap_or(default.send_timeout_secs),
        }
    }
}

/// Builder for TelegramConfig
#[derive(Debug, Default)]
pub struct TelegramConfigBuilder {
    auth_timeout_secs: Option<u64>,
    get_me_timeout_secs: Option<u64>,
    update_stream_timeout_secs: Option<u64>,
    update_queue_limit: Option<Option<usize>>,
    catch_up: Option<bool>,
    enable_reconnection: Option<bool>,
    max_reconnection_attempts: Option<u32>,
    reconnection_delay_secs: Option<u64>,
    max_reconnection_delay_secs: Option<u64>,
}

impl TelegramConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn auth_timeout_secs(mut self, secs: u64) -> Self {
        self.auth_timeout_secs = Some(secs);
        self
    }

    pub fn get_me_timeout_secs(mut self, secs: u64) -> Self {
        self.get_me_timeout_secs = Some(secs);
        self
    }

    pub fn update_stream_timeout_secs(mut self, secs: u64) -> Self {
        self.update_stream_timeout_secs = Some(secs);
        self
    }

    pub fn update_queue_limit(mut self, limit: Option<usize>) -> Self {
        self.update_queue_limit = Some(limit);
        self
    }

    pub fn catch_up(mut self, enabled: bool) -> Self {
        self.catch_up = Some(enabled);
        self
    }

    pub fn enable_reconnection(mut self, enabled: bool) -> Self {
        self.enable_reconnection = Some(enabled);
        self
    }

    pub fn max_reconnection_attempts(mut self, attempts: u32) -> Self {
        self.max_reconnection_attempts = Some(attempts);
        self
    }

    pub fn reconnection_delay_secs(mut self, secs: u64) -> Self {
        self.reconnection_delay_secs = Some(secs);
        self
    }

    pub fn max_reconnection_delay_secs(mut self, secs: u64) -> Self {
        self.max_reconnection_delay_secs = Some(secs);
        self
    }

    pub fn build(self) -> TelegramConfig {
        let default = TelegramConfig::default();
        TelegramConfig {
            auth_timeout_secs: self.auth_timeout_secs.unwrap_or(default.auth_timeout_secs),
            get_me_timeout_secs: self.get_me_timeout_secs.unwrap_or(default.get_me_timeout_secs),
            update_stream_timeout_secs: self.update_stream_timeout_secs.unwrap_or(default.update_stream_timeout_secs),
            update_queue_limit: self.update_queue_limit.unwrap_or(default.update_queue_limit),
            catch_up: self.catch_up.unwrap_or(default.catch_up),
            enable_reconnection: self.enable_reconnection.unwrap_or(default.enable_reconnection),
            max_reconnection_attempts: self.max_reconnection_attempts.unwrap_or(default.max_reconnection_attempts),
            reconnection_delay_secs: self.reconnection_delay_secs.unwrap_or(default.reconnection_delay_secs),
            max_reconnection_delay_secs: self.max_reconnection_delay_secs.unwrap_or(default.max_reconnection_delay_secs),
        }
    }
}

/// Builder for LogConfig
#[derive(Debug, Default)]
pub struct LogConfigBuilder {
    level: Option<String>,
    grammers_level: Option<String>,
}

impl LogConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn level(mut self, level: impl Into<String>) -> Self {
        self.level = Some(level.into());
        self
    }

    pub fn grammers_level(mut self, level: impl Into<String>) -> Self {
        self.grammers_level = Some(level.into());
        self
    }

    pub fn build(self) -> LogConfig {
        let default = LogConfig::default();
        LogConfig {
            level: self.level.unwrap_or(default.level),
            grammers_level: self.grammers_level.unwrap_or(default.grammers_level),
        }
    }
}

// ============================================================================
// Error Types
// ============================================================================

/// Configuration error types
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration validation failed: {0}")]
    Validation(String),

    #[error("Failed to parse configuration: {0}")]
    Parse(String),

    #[error("Environment variable error: {0}")]
    Env(#[from] std::env::VarError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_session_config_validation() {
        let mut config = SessionConfig::default();
        assert!(config.validate().is_ok());

        config.session_file = "".to_string();
        assert!(config.validate().is_err());

        config.session_file = "session.db".to_string();
        config.sqlite_user_version = -1;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_phoenix_config_validation() {
        let mut config = PhoenixConfig::default();
        assert!(config.validate().is_ok());

        config.url = "http://localhost:4000".to_string();
        assert!(config.validate().is_err());

        config.url = "wss://example.com/socket".to_string();
        assert!(config.validate().is_ok());

        config.connection_timeout_secs = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_retry_config_validation() {
        let mut config = RetryConfig::default();
        assert!(config.validate().is_ok());

        config.max_delay_secs = config.initial_delay_secs - 1;
        assert!(config.validate().is_err());

        config.max_delay_secs = 30;
        config.max_attempts = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_telegram_config_validation() {
        let config = TelegramConfig::default();
        assert!(config.validate().is_ok());

        let mut config = TelegramConfig {
            auth_timeout_secs: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());

        config.auth_timeout_secs = 30;
        config.update_queue_limit = Some(0);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_log_config_validation() {
        let config = LogConfig::default();
        assert!(config.validate().is_ok());

        let config = LogConfig {
            level: "invalid".to_string(),
            grammers_level: "debug".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_builder_pattern() {
        let config = AppConfigBuilder::new()
            .session(SessionConfigBuilder::new()
                .session_file("test.session")
                .build())
            .phoenix(PhoenixConfigBuilder::new()
                .url("wss://example.com/socket")
                .topic("test:topic")
                .build())
            .build();

        assert_eq!(config.session.session_file, "test.session");
        assert_eq!(config.phoenix.url, "wss://example.com/socket");
        assert_eq!(config.phoenix.topic, "test:topic");
    }

    #[test]
    fn test_duration_helpers() {
        let config = TelegramConfig::default();
        assert_eq!(config.auth_timeout(), Duration::from_secs(30));
        assert_eq!(config.get_me_timeout(), Duration::from_secs(30));
        assert_eq!(config.update_stream_timeout(), Duration::from_secs(10));
    }

    #[test]
    fn test_log_level_filters() {
        let config = LogConfig {
            level: "warn".to_string(),
            grammers_level: "trace".to_string(),
        };

        assert_eq!(config.level_filter(), log::LevelFilter::Warn);
        assert_eq!(config.grammers_level_filter(), log::LevelFilter::Trace);
    }

    // ============================================================================
    // Comprehensive Validation Tests
    // ============================================================================

    #[test]
    fn test_session_config_validation_comprehensive() {
        // Valid config
        let valid = SessionConfig {
            session_file: "test.session".to_string(),
            sqlite_user_version: 1,
        };
        assert!(valid.validate().is_ok());

        // Empty session file
        let empty_file = SessionConfig {
            session_file: "".to_string(),
            sqlite_user_version: 1,
        };
        assert!(empty_file.validate().is_err());

        // Negative user version
        let negative_version = SessionConfig {
            session_file: "test.session".to_string(),
            sqlite_user_version: -1,
        };
        assert!(negative_version.validate().is_err());

        // Zero user version (should be valid)
        let zero_version = SessionConfig {
            session_file: "test.session".to_string(),
            sqlite_user_version: 0,
        };
        assert!(zero_version.validate().is_ok());
    }

    #[test]
    fn test_phoenix_config_validation_comprehensive() {
        // Valid ws:// URL
        let valid_ws = PhoenixConfig {
            url: "ws://localhost:4000".to_string(),
            topic: "test:topic".to_string(),
            auth_token: None,
            retry: RetryConfig::default(),
            connection_timeout_secs: 30,
            join_timeout_secs: 30,
            send_timeout_secs: 10,
        };
        assert!(valid_ws.validate().is_ok());

        // Valid wss:// URL
        let valid_wss = PhoenixConfig {
            url: "wss://example.com/socket".to_string(),
            ..valid_ws.clone()
        };
        assert!(valid_wss.validate().is_ok());

        // Invalid protocol
        let invalid_protocol = PhoenixConfig {
            url: "http://localhost:4000".to_string(),
            ..valid_ws.clone()
        };
        assert!(invalid_protocol.validate().is_err());

        // Empty URL
        let empty_url = PhoenixConfig {
            url: "".to_string(),
            ..valid_ws.clone()
        };
        assert!(empty_url.validate().is_err());

        // Empty topic
        let empty_topic = PhoenixConfig {
            topic: "".to_string(),
            ..valid_ws.clone()
        };
        assert!(empty_topic.validate().is_err());

        // Zero connection timeout
        let zero_conn_timeout = PhoenixConfig {
            connection_timeout_secs: 0,
            ..valid_ws.clone()
        };
        assert!(zero_conn_timeout.validate().is_err());

        // Zero join timeout
        let zero_join_timeout = PhoenixConfig {
            join_timeout_secs: 0,
            ..valid_ws.clone()
        };
        assert!(zero_join_timeout.validate().is_err());

        // Zero send timeout
        let zero_send_timeout = PhoenixConfig {
            send_timeout_secs: 0,
            ..valid_ws.clone()
        };
        assert!(zero_send_timeout.validate().is_err());
    }

    #[test]
    fn test_retry_config_validation_comprehensive() {
        // Valid config
        let valid = RetryConfig {
            initial_delay_secs: 1,
            max_delay_secs: 30,
            max_attempts: 5,
        };
        assert!(valid.validate().is_ok());

        // Zero initial delay
        let zero_initial = RetryConfig {
            initial_delay_secs: 0,
            max_delay_secs: 30,
            max_attempts: 5,
        };
        assert!(zero_initial.validate().is_err());

        // Zero max delay
        let zero_max = RetryConfig {
            initial_delay_secs: 1,
            max_delay_secs: 0,
            max_attempts: 5,
        };
        assert!(zero_max.validate().is_err());

        // Max delay less than initial delay
        let invalid_delays = RetryConfig {
            initial_delay_secs: 30,
            max_delay_secs: 10,
            max_attempts: 5,
        };
        assert!(invalid_delays.validate().is_err());

        // Equal delays (should be valid)
        let equal_delays = RetryConfig {
            initial_delay_secs: 10,
            max_delay_secs: 10,
            max_attempts: 5,
        };
        assert!(equal_delays.validate().is_ok());

        // Zero max attempts
        let zero_attempts = RetryConfig {
            initial_delay_secs: 1,
            max_delay_secs: 30,
            max_attempts: 0,
        };
        assert!(zero_attempts.validate().is_err());
    }

    #[test]
    fn test_telegram_config_validation_comprehensive() {
        // Valid config
        let valid = TelegramConfig::default();
        assert!(valid.validate().is_ok());

        // Zero auth timeout
        let zero_auth = TelegramConfig {
            auth_timeout_secs: 0,
            ..Default::default()
        };
        assert!(zero_auth.validate().is_err());

        // Zero get_me timeout
        let zero_get_me = TelegramConfig {
            get_me_timeout_secs: 0,
            ..Default::default()
        };
        assert!(zero_get_me.validate().is_err());

        // Zero update stream timeout
        let zero_update = TelegramConfig {
            update_stream_timeout_secs: 0,
            ..Default::default()
        };
        assert!(zero_update.validate().is_err());

        // Zero queue limit (should be invalid)
        let zero_queue = TelegramConfig {
            update_queue_limit: Some(0),
            ..Default::default()
        };
        assert!(zero_queue.validate().is_err());

        // None queue limit (should be valid - unlimited)
        let none_queue = TelegramConfig {
            update_queue_limit: None,
            ..Default::default()
        };
        assert!(none_queue.validate().is_ok());

        // Zero reconnection delay
        let zero_recon_delay = TelegramConfig {
            reconnection_delay_secs: 0,
            ..Default::default()
        };
        assert!(zero_recon_delay.validate().is_err());

        // Zero max reconnection delay
        let zero_max_recon = TelegramConfig {
            max_reconnection_delay_secs: 0,
            ..Default::default()
        };
        assert!(zero_max_recon.validate().is_err());

        // Max reconnection delay less than reconnection delay
        let invalid_recon_delays = TelegramConfig {
            reconnection_delay_secs: 30,
            max_reconnection_delay_secs: 10,
            ..Default::default()
        };
        assert!(invalid_recon_delays.validate().is_err());
    }

    #[test]
    fn test_log_config_validation_comprehensive() {
        // Valid levels
        for level in &["trace", "debug", "info", "warn", "error"] {
            let config = LogConfig {
                level: level.to_string(),
                grammers_level: "info".to_string(),
            };
            assert!(config.validate().is_ok());
        }

        // Invalid log level
        let invalid = LogConfig {
            level: "invalid".to_string(),
            grammers_level: "info".to_string(),
        };
        assert!(invalid.validate().is_err());

        // Invalid grammers level
        let invalid_grammers = LogConfig {
            level: "info".to_string(),
            grammers_level: "verbose".to_string(),
        };
        assert!(invalid_grammers.validate().is_err());

        // Case sensitive (should fail if not lowercase)
        let uppercase = LogConfig {
            level: "INFO".to_string(),
            grammers_level: "debug".to_string(),
        };
        assert!(uppercase.validate().is_err());
    }

    // ============================================================================
    // Environment Variable Parsing Tests
    // ============================================================================

    #[test]
    fn test_session_config_from_env() {
        // Save original environment
        let original_session = std::env::var("SESSION_FILE").ok();
        let original_version = std::env::var("SQLITE_USER_VERSION").ok();

        // Test with environment variables set
        std::env::set_var("SESSION_FILE", "custom.session");
        std::env::set_var("SQLITE_USER_VERSION", "2");

        let config = SessionConfig::from_env();
        assert_eq!(config.session_file, "custom.session");
        assert_eq!(config.sqlite_user_version, 2);

        // Test with invalid user version (should fall back to default)
        std::env::set_var("SQLITE_USER_VERSION", "not_a_number");
        let config = SessionConfig::from_env();
        assert_eq!(config.sqlite_user_version, 1); // Default value

        // Restore original environment
        std::env::remove_var("SESSION_FILE");
        std::env::remove_var("SQLITE_USER_VERSION");
        if let Some(val) = original_session {
            std::env::set_var("SESSION_FILE", val);
        }
        if let Some(val) = original_version {
            std::env::set_var("SQLITE_USER_VERSION", val);
        }
    }

    #[test]
    fn test_phoenix_config_from_env() {
        // Save original environment
        let vars = [
            ("PHOENIX_URL", std::env::var("PHOENIX_URL").ok()),
            ("PHOENIX_TOPIC", std::env::var("PHOENIX_TOPIC").ok()),
            ("PHOENIX_CONNECTION_TIMEOUT_SECS", std::env::var("PHOENIX_CONNECTION_TIMEOUT_SECS").ok()),
            ("PHOENIX_JOIN_TIMEOUT_SECS", std::env::var("PHOENIX_JOIN_TIMEOUT_SECS").ok()),
            ("PHOENIX_SEND_TIMEOUT_SECS", std::env::var("PHOENIX_SEND_TIMEOUT_SECS").ok()),
        ];

        // Set custom values
        std::env::set_var("PHOENIX_URL", "wss://custom.com/socket");
        std::env::set_var("PHOENIX_TOPIC", "custom:topic");
        std::env::set_var("PHOENIX_CONNECTION_TIMEOUT_SECS", "60");
        std::env::set_var("PHOENIX_JOIN_TIMEOUT_SECS", "45");
        std::env::set_var("PHOENIX_SEND_TIMEOUT_SECS", "20");

        let config = PhoenixConfig::from_env();
        assert_eq!(config.url, "wss://custom.com/socket");
        assert_eq!(config.topic, "custom:topic");
        assert_eq!(config.connection_timeout_secs, 60);
        assert_eq!(config.join_timeout_secs, 45);
        assert_eq!(config.send_timeout_secs, 20);

        // Test with invalid timeout (should fall back to default)
        std::env::set_var("PHOENIX_CONNECTION_TIMEOUT_SECS", "not_a_number");
        let config = PhoenixConfig::from_env();
        assert_eq!(config.connection_timeout_secs, 30); // Default value

        // Restore environment
        for (key, value) in vars {
            std::env::remove_var(key);
            if let Some(val) = value {
                std::env::set_var(key, val);
            }
        }
    }

    #[test]
    fn test_retry_config_from_env() {
        // Save original environment
        let vars = [
            ("RETRY_INITIAL_DELAY_SECS", std::env::var("RETRY_INITIAL_DELAY_SECS").ok()),
            ("RETRY_MAX_DELAY_SECS", std::env::var("RETRY_MAX_DELAY_SECS").ok()),
            ("RETRY_MAX_ATTEMPTS", std::env::var("RETRY_MAX_ATTEMPTS").ok()),
        ];

        // Set custom values
        std::env::set_var("RETRY_INITIAL_DELAY_SECS", "2");
        std::env::set_var("RETRY_MAX_DELAY_SECS", "60");
        std::env::set_var("RETRY_MAX_ATTEMPTS", "10");

        let config = RetryConfig::from_env();
        assert_eq!(config.initial_delay_secs, 2);
        assert_eq!(config.max_delay_secs, 60);
        assert_eq!(config.max_attempts, 10);

        // Restore environment
        for (key, value) in vars {
            std::env::remove_var(key);
            if let Some(val) = value {
                std::env::set_var(key, val);
            }
        }
    }

    #[test]
    fn test_telegram_config_from_env() {
        // Save original environment
        let vars = [
            ("TELEGRAM_AUTH_TIMEOUT_SECS", std::env::var("TELEGRAM_AUTH_TIMEOUT_SECS").ok()),
            ("TELEGRAM_UPDATE_QUEUE_LIMIT", std::env::var("TELEGRAM_UPDATE_QUEUE_LIMIT").ok()),
            ("TELEGRAM_CATCH_UP", std::env::var("TELEGRAM_CATCH_UP").ok()),
            ("TELEGRAM_ENABLE_RECONNECTION", std::env::var("TELEGRAM_ENABLE_RECONNECTION").ok()),
        ];

        // Test boolean parsing
        std::env::set_var("TELEGRAM_CATCH_UP", "false");
        std::env::set_var("TELEGRAM_ENABLE_RECONNECTION", "false");
        let config = TelegramConfig::from_env();
        assert_eq!(config.catch_up, false);
        assert_eq!(config.enable_reconnection, false);

        // Test with valid queue limit
        std::env::set_var("TELEGRAM_UPDATE_QUEUE_LIMIT", "50");
        let config = TelegramConfig::from_env();
        assert_eq!(config.update_queue_limit, Some(50));

        // Restore environment
        for (key, value) in vars {
            std::env::remove_var(key);
            if let Some(val) = value {
                std::env::set_var(key, val);
            }
        }
    }

    #[test]
    fn test_log_config_from_env() {
        // Save original environment
        let vars = [
            ("LOG_LEVEL", std::env::var("LOG_LEVEL").ok()),
            ("LOG_GRAMMERS_LEVEL", std::env::var("LOG_GRAMMERS_LEVEL").ok()),
        ];

        // Test level parsing and lowercase conversion
        std::env::set_var("LOG_LEVEL", "ERROR");
        std::env::set_var("LOG_GRAMMERS_LEVEL", "WARN");
        let config = LogConfig::from_env();
        assert_eq!(config.level, "error");
        assert_eq!(config.grammers_level, "warn");

        // Restore environment
        for (key, value) in vars {
            std::env::remove_var(key);
            if let Some(val) = value {
                std::env::set_var(key, val);
            }
        }
    }

    #[test]
    fn test_app_config_from_env() {
        let config = AppConfig::from_env();
        // Should not panic and should create a valid config
        // Validation might fail depending on environment, but construction should succeed
        assert!(config.session.session_file.len() > 0);
        assert!(config.phoenix.url.len() > 0);
    }

    // ============================================================================
    // Builder Pattern Tests
    // ============================================================================

    #[test]
    fn test_session_config_builder() {
        let config = SessionConfigBuilder::new()
            .session_file("test.session")
            .sqlite_user_version(2)
            .build();

        assert_eq!(config.session_file, "test.session");
        assert_eq!(config.sqlite_user_version, 2);
    }

    #[test]
    fn test_session_config_builder_defaults() {
        let config = SessionConfigBuilder::new().build();
        let default = SessionConfig::default();

        assert_eq!(config.session_file, default.session_file);
        assert_eq!(config.sqlite_user_version, default.sqlite_user_version);
    }

    #[test]
    fn test_phoenix_config_builder() {
        let retry = RetryConfig {
            initial_delay_secs: 2,
            max_delay_secs: 60,
            max_attempts: 10,
        };

        let config = PhoenixConfigBuilder::new()
            .url("wss://test.com/socket")
            .topic("test:topic")
            .retry(retry.clone())
            .connection_timeout_secs(60)
            .join_timeout_secs(45)
            .send_timeout_secs(20)
            .build();

        assert_eq!(config.url, "wss://test.com/socket");
        assert_eq!(config.topic, "test:topic");
        assert_eq!(config.retry.max_attempts, 10);
        assert_eq!(config.connection_timeout_secs, 60);
        assert_eq!(config.join_timeout_secs, 45);
        assert_eq!(config.send_timeout_secs, 20);
    }

    #[test]
    fn test_telegram_config_builder() {
        let config = TelegramConfigBuilder::new()
            .auth_timeout_secs(60)
            .get_me_timeout_secs(45)
            .update_stream_timeout_secs(15)
            .update_queue_limit(Some(200))
            .catch_up(false)
            .enable_reconnection(false)
            .max_reconnection_attempts(5)
            .reconnection_delay_secs(10)
            .max_reconnection_delay_secs(600)
            .build();

        assert_eq!(config.auth_timeout_secs, 60);
        assert_eq!(config.get_me_timeout_secs, 45);
        assert_eq!(config.update_stream_timeout_secs, 15);
        assert_eq!(config.update_queue_limit, Some(200));
        assert_eq!(config.catch_up, false);
        assert_eq!(config.enable_reconnection, false);
        assert_eq!(config.max_reconnection_attempts, 5);
        assert_eq!(config.reconnection_delay_secs, 10);
        assert_eq!(config.max_reconnection_delay_secs, 600);
    }

    #[test]
    fn test_telegram_config_builder_unlimited_queue() {
        let config = TelegramConfigBuilder::new()
            .update_queue_limit(None)
            .build();

        assert_eq!(config.update_queue_limit, None);
    }

    #[test]
    fn test_log_config_builder() {
        let config = LogConfigBuilder::new()
            .level("error")
            .grammers_level("warn")
            .build();

        assert_eq!(config.level, "error");
        assert_eq!(config.grammers_level, "warn");
    }

    #[test]
    fn test_app_config_builder() {
        let session = SessionConfigBuilder::new()
            .session_file("test.session")
            .build();

        let phoenix = PhoenixConfigBuilder::new()
            .url("wss://test.com/socket")
            .topic("test:topic")
            .build();

        let telegram = TelegramConfigBuilder::new()
            .auth_timeout_secs(60)
            .build();

        let log = LogConfigBuilder::new()
            .level("debug")
            .build();

        let config = AppConfigBuilder::new()
            .session(session.clone())
            .phoenix(phoenix.clone())
            .telegram(telegram.clone())
            .log(log.clone())
            .build();

        assert_eq!(config.session.session_file, "test.session");
        assert_eq!(config.phoenix.url, "wss://test.com/socket");
        assert_eq!(config.telegram.auth_timeout_secs, 60);
        assert_eq!(config.log.level, "debug");
    }

    #[test]
    fn test_app_config_builder_validated() {
        // Valid config
        let valid_config = AppConfigBuilder::new()
            .session(SessionConfigBuilder::new()
                .session_file("test.session")
                .build())
            .phoenix(PhoenixConfigBuilder::new()
                .url("wss://test.com/socket")
                .build())
            .build_validated();

        assert!(valid_config.is_ok());

        // Invalid config (invalid phoenix URL)
        let invalid_config = AppConfigBuilder::new()
            .phoenix(PhoenixConfigBuilder::new()
                .url("http://test.com")
                .build())
            .build_validated();

        assert!(invalid_config.is_err());
    }

    // ============================================================================
    // Duration Helper Tests
    // ============================================================================

    #[test]
    fn test_phoenix_duration_helpers() {
        let config = PhoenixConfig {
            connection_timeout_secs: 60,
            join_timeout_secs: 45,
            send_timeout_secs: 20,
            ..Default::default()
        };

        assert_eq!(config.connection_timeout(), Duration::from_secs(60));
        assert_eq!(config.join_timeout(), Duration::from_secs(45));
        assert_eq!(config.send_timeout(), Duration::from_secs(20));
    }

    #[test]
    fn test_telegram_duration_helpers() {
        let config = TelegramConfig {
            auth_timeout_secs: 60,
            get_me_timeout_secs: 45,
            update_stream_timeout_secs: 20,
            ..Default::default()
        };

        assert_eq!(config.auth_timeout(), Duration::from_secs(60));
        assert_eq!(config.get_me_timeout(), Duration::from_secs(45));
        assert_eq!(config.update_stream_timeout(), Duration::from_secs(20));
    }

    // ============================================================================
    // Default Value Tests
    // ============================================================================

    #[test]
    fn test_session_config_defaults() {
        let config = SessionConfig::default();
        assert_eq!(config.session_file, "session/my.session");
        assert_eq!(config.sqlite_user_version, 1);
    }

    #[test]
    fn test_phoenix_config_defaults() {
        let config = PhoenixConfig::default();
        // After security fix: default changed from ws:// to wss://
        assert_eq!(config.url, "wss://localhost:4000/socket");
        assert_eq!(config.topic, "telegram:updates");
        assert_eq!(config.connection_timeout_secs, 30);
        assert_eq!(config.join_timeout_secs, 30);
        assert_eq!(config.send_timeout_secs, 10);
    }

    #[test]
    fn test_retry_config_defaults() {
        let config = RetryConfig::default();
        assert_eq!(config.initial_delay_secs, 1);
        assert_eq!(config.max_delay_secs, 30);
        assert_eq!(config.max_attempts, 5);
    }

    #[test]
    fn test_telegram_config_defaults() {
        let config = TelegramConfig::default();
        assert_eq!(config.auth_timeout_secs, 30);
        assert_eq!(config.get_me_timeout_secs, 30);
        assert_eq!(config.update_stream_timeout_secs, 10);
        assert_eq!(config.update_queue_limit, Some(100));
        assert_eq!(config.catch_up, true);
        assert_eq!(config.enable_reconnection, true);
        assert_eq!(config.max_reconnection_attempts, 0);
        assert_eq!(config.reconnection_delay_secs, 5);
        assert_eq!(config.max_reconnection_delay_secs, 300);
    }

    #[test]
    fn test_log_config_defaults() {
        let config = LogConfig::default();
        assert_eq!(config.level, "info");
        assert_eq!(config.grammers_level, "debug");
    }

    #[test]
    fn test_app_config_defaults() {
        let config = AppConfig::default();
        // Verify all sub-configs are initialized
        assert_eq!(config.session.session_file, "session/my.session");
        // After security fix: default changed from ws:// to wss://
        assert_eq!(config.phoenix.url, "wss://localhost:4000/socket");
        assert_eq!(config.telegram.auth_timeout_secs, 30);
        assert_eq!(config.log.level, "info");
    }

    // ============================================================================
    // Log Level Filter Tests
    // ============================================================================

    #[test]
    fn test_all_log_levels() {
        let test_cases = vec![
            ("trace", log::LevelFilter::Trace),
            ("debug", log::LevelFilter::Debug),
            ("info", log::LevelFilter::Info),
            ("warn", log::LevelFilter::Warn),
            ("error", log::LevelFilter::Error),
        ];

        for (level_str, expected_filter) in test_cases {
            let config = LogConfig {
                level: level_str.to_string(),
                grammers_level: "info".to_string(),
            };
            assert_eq!(config.level_filter(), expected_filter);
        }
    }

    #[test]
    fn test_invalid_log_level_fallback() {
        let config = LogConfig {
            level: "invalid_level".to_string(),
            grammers_level: "also_invalid".to_string(),
        };

        // Should fall back to Info and Debug respectively
        assert_eq!(config.level_filter(), log::LevelFilter::Info);
        assert_eq!(config.grammers_level_filter(), log::LevelFilter::Debug);
    }
}
