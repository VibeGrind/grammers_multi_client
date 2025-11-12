use thiserror::Error;
use crate::domain::DomainError;

/// Errors that can occur during session loading and validation
#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Failed to open session database: {0}")]
    DatabaseOpen(#[from] sqlite::Error),

    #[error("Session file not found at path: {path}")]
    FileNotFound { path: String },

    #[error("Required field '{field}' not found in session database")]
    FieldNotFound { field: String },

    #[error("Invalid proxy URL: {reason}")]
    InvalidProxyUrl { reason: String },

    #[error("Session is not authorized - please re-authenticate")]
    NotAuthorized,

    #[error("Session validation failed: {0}")]
    ValidationFailed(String),

    #[error("Failed to parse session data: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Invalid port number '{port}' in proxy URL: must be between 1 and 65535")]
    InvalidProxyPort { port: String },

    #[error("Proxy URL must use SOCKS5 protocol (socks5://...)")]
    UnsupportedProxyProtocol,

    #[error("Task join error during session loading: {0}")]
    TaskJoinError(String),

    #[error("Failed to open SQLite session storage: {0}")]
    StorageError(String),

    #[error("Domain validation error: {0}")]
    Domain(#[from] DomainError),

    #[error("Failed to acquire lock on session file '{path}': {reason}")]
    FileLockError { path: String, reason: String },
}

/// Errors that can occur during Phoenix Channel operations
#[derive(Error, Debug)]
pub enum PhoenixError {
    #[error("Failed to connect to Phoenix server at {url}: {source}")]
    ConnectionFailed {
        url: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to join Phoenix channel '{topic}': {source}")]
    ChannelJoinFailed {
        topic: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Failed to send update to Phoenix channel: {0}")]
    SendFailed(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Failed to serialize update data: {0}")]
    SerializationFailed(#[from] serde_json::Error),

    #[error("Invalid Phoenix configuration: {reason}")]
    InvalidConfig { reason: String },

    #[error("Phoenix channel disconnected unexpectedly")]
    ChannelDisconnected,

    #[error("Timeout waiting for Phoenix channel response")]
    Timeout,
}

/// Errors that can occur during Telegram operations
#[derive(Error, Debug)]
pub enum TelegramError {
    #[error("Failed to check authorization status: {0}")]
    AuthorizationCheckFailed(String),

    #[error("User is not authorized - session may be invalid")]
    NotAuthorized,

    #[error("Failed to retrieve user information: {0}")]
    GetUserFailed(String),

    #[error("Failed to process update: {0}")]
    UpdateProcessingFailed(String),

    #[error("Client operation failed: {0}")]
    ClientError(String),

    #[error("Failed to send message: {0}")]
    SendMessageFailed(String),

    #[error("Failed to fetch messages: {0}")]
    FetchMessagesFailed(String),

    #[error("Invalid peer or chat ID: {0}")]
    InvalidPeer(String),

    #[error("Network error during Telegram operation: {0}")]
    NetworkError(String),

    #[error("RPC error from Telegram: {0}")]
    RpcError(String),

    #[error("Domain validation error: {0}")]
    Domain(#[from] DomainError),
}

/// Unified error type for the entire application
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Session error: {0}")]
    Session(#[from] SessionError),

    #[error("Phoenix error: {0}")]
    Phoenix(#[from] PhoenixError),

    #[error("Telegram error: {0}")]
    Telegram(#[from] TelegramError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Fatal error: {0}")]
    Fatal(String),
}

// Convenience conversion from Box<dyn Error> for backwards compatibility
impl From<Box<dyn std::error::Error + Send + Sync>> for SessionError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        SessionError::ValidationFailed(err.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for PhoenixError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        PhoenixError::SendFailed(err)
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for TelegramError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        TelegramError::ClientError(err.to_string())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::DomainError;

    // ============================================================================
    // SessionError Tests
    // ============================================================================

    #[test]
    fn test_session_error_display() {
        // Test DatabaseOpen error
        let db_error = sqlite::Error {
            code: Some(1),
            message: Some("database locked".to_string()),
        };
        let err = SessionError::DatabaseOpen(db_error);
        assert!(err.to_string().contains("Failed to open session database"));

        // Test FileNotFound error
        let err = SessionError::FileNotFound {
            path: "/path/to/session.db".to_string(),
        };
        assert!(err.to_string().contains("Session file not found"));
        assert!(err.to_string().contains("/path/to/session.db"));

        // Test FieldNotFound error
        let err = SessionError::FieldNotFound {
            field: "app_id".to_string(),
        };
        assert!(err.to_string().contains("Required field 'app_id' not found"));

        // Test InvalidProxyUrl error
        let err = SessionError::InvalidProxyUrl {
            reason: "Invalid port".to_string(),
        };
        assert!(err.to_string().contains("Invalid proxy URL"));
        assert!(err.to_string().contains("Invalid port"));

        // Test NotAuthorized error
        let err = SessionError::NotAuthorized;
        assert!(err.to_string().contains("not authorized"));

        // Test ValidationFailed error
        let err = SessionError::ValidationFailed("Invalid session data".to_string());
        assert!(err.to_string().contains("Session validation failed"));
        assert!(err.to_string().contains("Invalid session data"));

        // Test InvalidProxyPort error
        let err = SessionError::InvalidProxyPort {
            port: "99999".to_string(),
        };
        assert!(err.to_string().contains("Invalid port number"));
        assert!(err.to_string().contains("99999"));

        // Test UnsupportedProxyProtocol error
        let err = SessionError::UnsupportedProxyProtocol;
        assert!(err.to_string().contains("SOCKS5 protocol"));

        // Test TaskJoinError
        let err = SessionError::TaskJoinError("Task panicked".to_string());
        assert!(err.to_string().contains("Task join error"));

        // Test StorageError
        let err = SessionError::StorageError("Failed to open storage".to_string());
        assert!(err.to_string().contains("Failed to open SQLite"));

        // Test FileLockError
        let err = SessionError::FileLockError {
            path: "/path/to/session.db".to_string(),
            reason: "Already locked".to_string(),
        };
        assert!(err.to_string().contains("Failed to acquire lock"));
        assert!(err.to_string().contains("/path/to/session.db"));
        assert!(err.to_string().contains("Already locked"));
    }

    #[test]
    fn test_session_error_from_sqlite() {
        let sqlite_err = sqlite::Error {
            code: Some(1),
            message: Some("test error".to_string()),
        };
        let session_err: SessionError = sqlite_err.into();
        assert!(matches!(session_err, SessionError::DatabaseOpen(_)));
    }

    #[test]
    fn test_session_error_from_json() {
        let json_str = r#"{"invalid": json"#;
        let json_err = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let session_err: SessionError = json_err.into();
        assert!(matches!(session_err, SessionError::ParseError(_)));
    }

    #[test]
    fn test_session_error_from_domain() {
        let domain_err = DomainError::InvalidApiId(-1);
        let session_err: SessionError = domain_err.into();
        assert!(matches!(session_err, SessionError::Domain(_)));
    }

    #[test]
    fn test_session_error_from_boxed() {
        let boxed_err: Box<dyn std::error::Error + Send + Sync> =
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test"));
        let session_err: SessionError = boxed_err.into();
        assert!(matches!(session_err, SessionError::ValidationFailed(_)));
    }

    // ============================================================================
    // PhoenixError Tests
    // ============================================================================

    #[test]
    fn test_phoenix_error_display() {
        // Test ConnectionFailed error
        let source: Box<dyn std::error::Error + Send + Sync> =
            Box::new(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused"));
        let err = PhoenixError::ConnectionFailed {
            url: "wss://example.com/socket".to_string(),
            source,
        };
        assert!(err.to_string().contains("Failed to connect"));
        assert!(err.to_string().contains("wss://example.com/socket"));

        // Test ChannelJoinFailed error
        let source: Box<dyn std::error::Error + Send + Sync> =
            Box::new(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout"));
        let err = PhoenixError::ChannelJoinFailed {
            topic: "telegram:updates".to_string(),
            source,
        };
        assert!(err.to_string().contains("Failed to join Phoenix channel"));
        assert!(err.to_string().contains("telegram:updates"));

        // Test SendFailed error
        let source: Box<dyn std::error::Error + Send + Sync> =
            Box::new(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "broken"));
        let err = PhoenixError::SendFailed(source);
        assert!(err.to_string().contains("Failed to send update"));

        // Test InvalidConfig error
        let err = PhoenixError::InvalidConfig {
            reason: "Invalid URL".to_string(),
        };
        assert!(err.to_string().contains("Invalid Phoenix configuration"));
        assert!(err.to_string().contains("Invalid URL"));

        // Test ChannelDisconnected error
        let err = PhoenixError::ChannelDisconnected;
        assert!(err.to_string().contains("disconnected unexpectedly"));

        // Test Timeout error
        let err = PhoenixError::Timeout;
        assert!(err.to_string().contains("Timeout"));
    }

    #[test]
    fn test_phoenix_error_from_json() {
        let json_str = r#"{"invalid": json"#;
        let json_err = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let phoenix_err: PhoenixError = json_err.into();
        assert!(matches!(phoenix_err, PhoenixError::SerializationFailed(_)));
    }

    #[test]
    fn test_phoenix_error_from_boxed() {
        let boxed_err: Box<dyn std::error::Error + Send + Sync> =
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test"));
        let phoenix_err: PhoenixError = boxed_err.into();
        assert!(matches!(phoenix_err, PhoenixError::SendFailed(_)));
    }

    // ============================================================================
    // TelegramError Tests
    // ============================================================================

    #[test]
    fn test_telegram_error_display() {
        // Test AuthorizationCheckFailed error
        let err = TelegramError::AuthorizationCheckFailed("Connection timeout".to_string());
        assert!(err.to_string().contains("Failed to check authorization"));
        assert!(err.to_string().contains("Connection timeout"));

        // Test NotAuthorized error
        let err = TelegramError::NotAuthorized;
        assert!(err.to_string().contains("not authorized"));

        // Test GetUserFailed error
        let err = TelegramError::GetUserFailed("User not found".to_string());
        assert!(err.to_string().contains("Failed to retrieve user information"));
        assert!(err.to_string().contains("User not found"));

        // Test UpdateProcessingFailed error
        let err = TelegramError::UpdateProcessingFailed("Invalid update format".to_string());
        assert!(err.to_string().contains("Failed to process update"));
        assert!(err.to_string().contains("Invalid update format"));

        // Test ClientError error
        let err = TelegramError::ClientError("Connection lost".to_string());
        assert!(err.to_string().contains("Client operation failed"));
        assert!(err.to_string().contains("Connection lost"));

        // Test SendMessageFailed error
        let err = TelegramError::SendMessageFailed("Flood wait".to_string());
        assert!(err.to_string().contains("Failed to send message"));
        assert!(err.to_string().contains("Flood wait"));

        // Test FetchMessagesFailed error
        let err = TelegramError::FetchMessagesFailed("Network error".to_string());
        assert!(err.to_string().contains("Failed to fetch messages"));
        assert!(err.to_string().contains("Network error"));

        // Test InvalidPeer error
        let err = TelegramError::InvalidPeer("Chat not found".to_string());
        assert!(err.to_string().contains("Invalid peer or chat ID"));
        assert!(err.to_string().contains("Chat not found"));

        // Test NetworkError error
        let err = TelegramError::NetworkError("DNS resolution failed".to_string());
        assert!(err.to_string().contains("Network error"));
        assert!(err.to_string().contains("DNS resolution failed"));

        // Test RpcError error
        let err = TelegramError::RpcError("PEER_ID_INVALID".to_string());
        assert!(err.to_string().contains("RPC error from Telegram"));
        assert!(err.to_string().contains("PEER_ID_INVALID"));
    }

    #[test]
    fn test_telegram_error_from_domain() {
        let domain_err = DomainError::InvalidChatId(0);
        let telegram_err: TelegramError = domain_err.into();
        assert!(matches!(telegram_err, TelegramError::Domain(_)));
    }

    #[test]
    fn test_telegram_error_from_boxed() {
        let boxed_err: Box<dyn std::error::Error + Send + Sync> =
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, "test"));
        let telegram_err: TelegramError = boxed_err.into();
        assert!(matches!(telegram_err, TelegramError::ClientError(_)));
    }

    // ============================================================================
    // AppError Tests
    // ============================================================================

    #[test]
    fn test_app_error_display() {
        // Test Session variant
        let err = AppError::Session(SessionError::NotAuthorized);
        assert!(err.to_string().contains("Session error"));
        assert!(err.to_string().contains("not authorized"));

        // Test Phoenix variant
        let err = AppError::Phoenix(PhoenixError::Timeout);
        assert!(err.to_string().contains("Phoenix error"));
        assert!(err.to_string().contains("Timeout"));

        // Test Telegram variant
        let err = AppError::Telegram(TelegramError::NotAuthorized);
        assert!(err.to_string().contains("Telegram error"));
        assert!(err.to_string().contains("not authorized"));

        // Test Io variant
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = AppError::Io(io_err);
        assert!(err.to_string().contains("IO error"));

        // Test Config variant
        let err = AppError::Config("Invalid configuration".to_string());
        assert!(err.to_string().contains("Configuration error"));
        assert!(err.to_string().contains("Invalid configuration"));

        // Test Fatal variant
        let err = AppError::Fatal("Critical system error".to_string());
        assert!(err.to_string().contains("Fatal error"));
        assert!(err.to_string().contains("Critical system error"));
    }

    #[test]
    fn test_app_error_from_session() {
        let session_err = SessionError::NotAuthorized;
        let app_err: AppError = session_err.into();
        assert!(matches!(app_err, AppError::Session(_)));
    }

    #[test]
    fn test_app_error_from_phoenix() {
        let phoenix_err = PhoenixError::Timeout;
        let app_err: AppError = phoenix_err.into();
        assert!(matches!(app_err, AppError::Phoenix(_)));
    }

    #[test]
    fn test_app_error_from_telegram() {
        let telegram_err = TelegramError::NotAuthorized;
        let app_err: AppError = telegram_err.into();
        assert!(matches!(app_err, AppError::Telegram(_)));
    }

    #[test]
    fn test_app_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let app_err: AppError = io_err.into();
        assert!(matches!(app_err, AppError::Io(_)));
    }

    // ============================================================================
    // Error Chaining Tests
    // ============================================================================

    #[test]
    fn test_error_chain_session_to_app() {
        // Domain -> Session -> App
        let domain_err = DomainError::InvalidApiId(-1);
        let session_err: SessionError = domain_err.into();
        let app_err: AppError = session_err.into();

        assert!(matches!(app_err, AppError::Session(SessionError::Domain(_))));
    }

    #[test]
    fn test_error_chain_domain_to_telegram_to_app() {
        // Domain -> Telegram -> App
        let domain_err = DomainError::InvalidChatId(0);
        let telegram_err: TelegramError = domain_err.into();
        let app_err: AppError = telegram_err.into();

        assert!(matches!(app_err, AppError::Telegram(TelegramError::Domain(_))));
    }

    #[test]
    fn test_error_source_chain() {
        // Create a nested error with source
        let inner_err: Box<dyn std::error::Error + Send + Sync> =
            Box::new(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout"));
        let phoenix_err = PhoenixError::SendFailed(inner_err);

        // Verify we can access the source
        use std::error::Error;
        assert!(phoenix_err.source().is_some());
    }

    // ============================================================================
    // Error Conversion Tests
    // ============================================================================

    #[test]
    fn test_multiple_conversion_paths() {
        // Test that we can convert through multiple paths

        // Path 1: sqlite -> Session -> App
        let sqlite_err = sqlite::Error {
            code: Some(1),
            message: Some("test".to_string()),
        };
        let app_err: AppError = SessionError::from(sqlite_err).into();
        assert!(matches!(app_err, AppError::Session(SessionError::DatabaseOpen(_))));

        // Path 2: serde_json -> Phoenix -> App
        let json_err = serde_json::from_str::<serde_json::Value>(r#"{"invalid"#).unwrap_err();
        let app_err: AppError = PhoenixError::from(json_err).into();
        assert!(matches!(app_err, AppError::Phoenix(PhoenixError::SerializationFailed(_))));

        // Path 3: io -> App (direct)
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let app_err: AppError = io_err.into();
        assert!(matches!(app_err, AppError::Io(_)));
    }

    #[test]
    fn test_error_downcasting() {
        // Test that we can pattern match on specific error variants
        let err = AppError::Session(SessionError::FileNotFound {
            path: "/test/path".to_string(),
        });

        match err {
            AppError::Session(SessionError::FileNotFound { path }) => {
                assert_eq!(path, "/test/path");
            }
            _ => panic!("Wrong error variant"),
        }
    }

    #[test]
    fn test_all_error_variants_are_error_trait() {
        // Verify all error types implement std::error::Error
        use std::error::Error;

        let session_err: Box<dyn Error> = Box::new(SessionError::NotAuthorized);
        assert!(session_err.to_string().len() > 0);

        let phoenix_err: Box<dyn Error> = Box::new(PhoenixError::Timeout);
        assert!(phoenix_err.to_string().len() > 0);

        let telegram_err: Box<dyn Error> = Box::new(TelegramError::NotAuthorized);
        assert!(telegram_err.to_string().len() > 0);

        let app_err: Box<dyn Error> = Box::new(AppError::Fatal("test".to_string()));
        assert!(app_err.to_string().len() > 0);
    }
}
