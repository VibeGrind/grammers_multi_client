//! Integration tests for telegram-minimal-client
//!
//! These tests verify the interaction between modules and test the complete
//! application flow. They require real external dependencies (Telegram API,
//! Phoenix server, SQLite database) and are marked with #[ignore] by default.
//!
//! To run these tests:
//! ```bash
//! cargo test --test integration_test -- --ignored
//! ```

use telegram_minimal_client::{config::*, domain::*, error::*, storage::*};

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_full_config_creation_and_validation() {
    // Create a complete configuration using builders
    let config = AppConfigBuilder::new()
        .session(
            SessionConfigBuilder::new()
                .session_file("test.session")
                .sqlite_user_version(1)
                .build(),
        )
        .phoenix(
            PhoenixConfigBuilder::new()
                .url("wss://example.com/socket")
                .topic("telegram:updates")
                .connection_timeout_secs(30)
                .join_timeout_secs(30)
                .send_timeout_secs(10)
                .build(),
        )
        .telegram(
            TelegramConfigBuilder::new()
                .auth_timeout_secs(30)
                .get_me_timeout_secs(30)
                .update_stream_timeout_secs(10)
                .update_queue_limit(Some(100))
                .catch_up(true)
                .enable_reconnection(true)
                .build(),
        )
        .log(
            LogConfigBuilder::new()
                .level("info")
                .grammers_level("debug")
                .build(),
        )
        .build();

    // Validate the entire configuration
    assert!(config.validate().is_ok());

    // Verify individual components
    assert_eq!(config.session.session_file, "test.session");
    assert_eq!(config.phoenix.url, "wss://example.com/socket");
    assert_eq!(config.telegram.update_queue_limit, Some(100));
    assert_eq!(config.log.level, "info");
}

#[test]
fn test_config_validation_catches_errors() {
    // Create an invalid configuration
    let config = AppConfigBuilder::new()
        .phoenix(
            PhoenixConfigBuilder::new()
                .url("http://invalid.com") // Invalid protocol
                .build(),
        )
        .build();

    // Validation should fail
    assert!(config.validate().is_err());
}

// ============================================================================
// Domain Model Integration Tests
// ============================================================================

#[test]
fn test_telegram_update_full_workflow() {
    // Create domain entities
    let chat_id = ChatId::new(-1001234567890).unwrap();
    let message_id = MessageId::new(12345).unwrap();
    let user_id = UserId::new(987654321).unwrap();

    let user_info = UserInfo::new(
        user_id,
        "Test User".to_string(),
        Some("testuser".to_string()),
    );

    // Create a new message update
    let update = TelegramUpdate::new_message(
        chat_id,
        message_id,
        "Hello, World!".to_string(),
        Some(user_info.clone()),
        1234567890,
    );

    // Verify the update
    assert!(update.is_new_message());
    assert_eq!(update.chat_id, Some(chat_id));
    assert_eq!(update.message_id, Some(message_id));
    assert_eq!(update.text, Some("Hello, World!".to_string()));
    assert!(update.from_user.is_some());

    // Serialize to JSON (as would be sent to Phoenix)
    let json = serde_json::to_string(&update).unwrap();
    assert!(json.contains("new_message"));
    assert!(json.contains("Hello, World!"));

    // Deserialize back
    let deserialized: TelegramUpdate = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.update_type, update.update_type);
    assert_eq!(deserialized.text, update.text);
}

#[test]
fn test_session_data_domain_conversion() {
    // Create session data with domain types
    let api_id = ApiId::new(12345).unwrap();
    let proxy = ProxyUrl::new("socks5://proxy.example.com:1080".to_string()).unwrap();

    let session_data = crate::domain::SessionData::new(
        api_id,
        "Android 13".to_string(),
        "SDK 33".to_string(),
        "1.0.0".to_string(),
        "en".to_string(),
        "en-US".to_string(),
        Some(proxy.clone()),
    );

    // Verify session data
    assert_eq!(session_data.app_id.as_i32(), 12345);
    assert_eq!(session_data.device, "Android 13");
    assert!(session_data.has_proxy());

    // Serialize session data
    let json = serde_json::to_string(&session_data).unwrap();
    assert!(json.contains("12345"));
    assert!(json.contains("Android 13"));
}

// ============================================================================
// Error Handling Integration Tests
// ============================================================================

#[test]
fn test_error_chain_propagation() {
    // Test error conversion chain: Domain -> Session -> App
    let domain_error = DomainError::InvalidApiId(-1);
    let session_error: SessionError = domain_error.into();
    let app_error: AppError = session_error.into();

    // Verify error chain
    match app_error {
        AppError::Session(SessionError::Domain(e)) => {
            assert!(e.to_string().contains("Invalid API ID"));
        }
        _ => panic!("Unexpected error type"),
    }
}

#[test]
fn test_error_display_messages() {
    // Test that error messages are helpful
    let errors = vec![
        AppError::Session(SessionError::FileNotFound {
            path: "/test/session.db".to_string(),
        }),
        AppError::Phoenix(PhoenixError::Timeout),
        AppError::Telegram(TelegramError::NotAuthorized),
        AppError::Config("Invalid configuration".to_string()),
    ];

    for error in errors {
        let message = error.to_string();
        // Verify error messages are not empty and contain relevant info
        assert!(message.len() > 10);
        println!("Error message: {}", message); // For manual inspection during test runs
    }
}

// ============================================================================
// Storage Layer Integration Tests
// ============================================================================

#[test]
fn test_proxy_url_validation_comprehensive() {
    // Test various proxy URL formats
    let valid_urls = vec![
        "socks5://proxy.example.com:1080",
        "socks5://user:pass@proxy.example.com:1080",
        "socks5://127.0.0.1:9050",
        "socks5://[::1]:1080",
        "socks5://[2001:db8::1]:1080",
    ];

    for url in valid_urls {
        let result = ProxyUrl::new(url.to_string());
        assert!(
            result.is_ok(),
            "Expected {} to be valid, but got error: {:?}",
            url,
            result.err()
        );
    }

    let invalid_urls = vec![
        "http://proxy.example.com:1080",  // Wrong protocol
        "socks5://proxy.example.com",     // Missing port
        "socks5://:1080",                 // Missing host
        "socks5://proxy.example.com:0",   // Invalid port
        "socks5://proxy.example.com:99999", // Port too large
    ];

    for url in invalid_urls {
        let result = ProxyUrl::new(url.to_string());
        assert!(result.is_err(), "Expected {} to be invalid", url);
    }
}

// ============================================================================
// Mock-based Integration Tests
// ============================================================================

/// Mock SessionRepository for integration testing
#[derive(Clone)]
struct MockSessionRepo {
    api_id: ApiId,
    has_proxy: bool,
}

impl MockSessionRepo {
    fn new(api_id: i32, has_proxy: bool) -> Self {
        Self {
            api_id: ApiId::new(api_id).unwrap(),
            has_proxy,
        }
    }
}

impl SessionRepository for MockSessionRepo {
    fn load_session_data(&self) -> Result<telegram_minimal_client::storage::SessionData, SessionError> {
        Ok(telegram_minimal_client::storage::SessionData {
            app_id: self.api_id,
            device_info: telegram_minimal_client::storage::DeviceInfo {
                device_model: "Test Device".to_string(),
                sdk: "Test SDK".to_string(),
                app_version: "1.0.0".to_string(),
                lang_code: "en".to_string(),
                system_lang_code: "en-US".to_string(),
            },
            proxy: if self.has_proxy {
                Some(ProxyUrl::new("socks5://test.com:1080".to_string()).unwrap())
            } else {
                None
            },
        })
    }

    fn get_api_id(&self) -> Result<ApiId, SessionError> {
        Ok(self.api_id)
    }

    fn get_device_info(&self) -> Result<telegram_minimal_client::storage::DeviceInfo, SessionError> {
        Ok(telegram_minimal_client::storage::DeviceInfo {
            device_model: "Test Device".to_string(),
            sdk: "Test SDK".to_string(),
            app_version: "1.0.0".to_string(),
            lang_code: "en".to_string(),
            system_lang_code: "en-US".to_string(),
        })
    }

    fn get_proxy_url(&self) -> Result<Option<ProxyUrl>, SessionError> {
        if self.has_proxy {
            Ok(Some(
                ProxyUrl::new("socks5://test.com:1080".to_string()).unwrap(),
            ))
        } else {
            Ok(None)
        }
    }
}

#[test]
fn test_session_repository_integration() {
    // Test with proxy
    let repo_with_proxy = MockSessionRepo::new(12345, true);
    let session_data = repo_with_proxy.load_session_data().unwrap();
    assert_eq!(session_data.app_id.as_i32(), 12345);
    assert!(session_data.proxy.is_some());

    // Test without proxy
    let repo_without_proxy = MockSessionRepo::new(54321, false);
    let session_data = repo_without_proxy.load_session_data().unwrap();
    assert_eq!(session_data.app_id.as_i32(), 54321);
    assert!(session_data.proxy.is_none());
}

// ============================================================================
// End-to-End Integration Tests (require real services - marked #[ignore])
// ============================================================================

/// Full integration test that requires:
/// - A valid SQLite session file
/// - A running Phoenix server
/// - Valid Telegram credentials
///
/// This test is marked #[ignore] and must be run explicitly:
/// ```bash
/// cargo test --test integration_test test_full_integration -- --ignored
/// ```
#[tokio::test]
#[ignore]
async fn test_full_integration_with_real_services() {
    // Load configuration from environment
    let config = AppConfig::from_env();

    // Validate configuration
    assert!(
        config.validate().is_ok(),
        "Configuration validation failed: {:?}",
        config.validate().err()
    );

    // Test that session file exists (if specified)
    let session_path = std::path::Path::new(&config.session.session_file);
    println!(
        "Checking session file: {}",
        config.session.session_file
    );

    // Note: In a real test, we would:
    // 1. Load session data from SQLite
    // 2. Connect to Phoenix
    // 3. Connect to Telegram
    // 4. Send a test update
    // 5. Verify everything works

    // This is just a placeholder that verifies the config can be loaded
    println!("Configuration loaded successfully");
    println!("Phoenix URL: {}", config.phoenix.url);
    println!("Phoenix Topic: {}", config.phoenix.topic);
}

/// Test that verifies the application can handle a simulated update flow
#[tokio::test]
async fn test_simulated_update_flow() {
    // Create test data
    let chat_id = ChatId::new(123).unwrap();
    let message_id = MessageId::new(456).unwrap();
    let user_id = UserId::new(789).unwrap();

    let user_info = UserInfo::new(
        user_id,
        "Test User".to_string(),
        Some("testuser".to_string()),
    );

    // Create update
    let update = TelegramUpdate::new_message(
        chat_id,
        message_id,
        "Integration test message".to_string(),
        Some(user_info),
        1234567890, // Fixed timestamp for testing
    );

    // Serialize (as would happen before sending to Phoenix)
    let serialized = serde_json::to_string(&update).unwrap();

    // Deserialize (as would happen on Phoenix side)
    let deserialized: TelegramUpdate = serde_json::from_str(&serialized).unwrap();

    // Verify roundtrip
    assert_eq!(deserialized.update_type, "new_message");
    assert_eq!(deserialized.chat_id, Some(chat_id));
    assert_eq!(deserialized.message_id, Some(message_id));
    assert_eq!(
        deserialized.text,
        Some("Integration test message".to_string())
    );

    println!("Update serialization roundtrip successful");
}

/// Test configuration from environment variables
#[test]
fn test_environment_variable_integration() {
    // Save current environment
    let original_log_level = std::env::var("LOG_LEVEL").ok();

    // Set test environment variables
    std::env::set_var("LOG_LEVEL", "debug");
    std::env::set_var("LOG_GRAMMERS_LEVEL", "trace");

    // Load config from environment
    let log_config = LogConfig::from_env();

    // Verify
    assert_eq!(log_config.level, "debug");
    assert_eq!(log_config.grammers_level, "trace");

    // Validate
    assert!(log_config.validate().is_ok());

    // Restore environment
    std::env::remove_var("LOG_LEVEL");
    std::env::remove_var("LOG_GRAMMERS_LEVEL");
    if let Some(val) = original_log_level {
        std::env::set_var("LOG_LEVEL", val);
    }
}

// ============================================================================
// Performance and Load Tests
// ============================================================================

#[test]
fn test_large_batch_of_updates() {
    // Create a large batch of updates to test serialization performance
    let chat_id = ChatId::new(123).unwrap();
    let mut updates = Vec::new();

    for i in 1..=1000 {
        let message_id = MessageId::new(i).unwrap();
        let update = TelegramUpdate::new_message(
            chat_id,
            message_id,
            format!("Message {}", i),
            None,
            1234567890 + i as i64,
        );
        updates.push(update);
    }

    // Serialize all updates
    let start = std::time::Instant::now();
    let serialized: Vec<String> = updates
        .iter()
        .map(|u| serde_json::to_string(u).unwrap())
        .collect();
    let duration = start.elapsed();

    println!(
        "Serialized {} updates in {:?}",
        updates.len(),
        duration
    );
    assert_eq!(serialized.len(), 1000);
    assert!(duration.as_secs() < 1, "Serialization too slow");
}

#[test]
fn test_domain_type_edge_cases() {
    // Test boundary values for all domain types
    assert!(ApiId::new(1).is_ok());
    assert!(ApiId::new(i32::MAX).is_ok());
    assert!(ApiId::new(0).is_err());
    assert!(ApiId::new(-1).is_err());

    assert!(ChatId::new(i64::MAX).is_ok());
    assert!(ChatId::new(i64::MIN).is_ok());
    assert!(ChatId::new(0).is_err());

    assert!(MessageId::new(1).is_ok());
    assert!(MessageId::new(i32::MAX).is_ok());
    assert!(MessageId::new(0).is_err());

    assert!(UserId::new(i64::MAX).is_ok());
    assert!(UserId::new(i64::MIN).is_ok());
    assert!(UserId::new(0).is_err());
}
