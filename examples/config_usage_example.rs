/// Example demonstrating how to use the centralized configuration module
///
/// This example shows how to refactor the main application to use the config module
/// instead of hardcoded constants and scattered configuration values.
///
/// Run with: cargo run --example config_usage_example

use std::time::Duration;

// Mock types for the example (these would come from the actual dependencies)
mod mock {
    pub struct Client;
    pub struct UpdatesConfiguration {
        pub catch_up: bool,
        pub update_queue_limit: Option<usize>,
    }
}

// Import the config module (in real code: `use crate::config::*;`)
use telegram_minimal_client::config::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Configuration System Example ===\n");

    // ========================================================================
    // Example 1: Load configuration from environment variables
    // ========================================================================
    println!("1. Loading configuration from environment variables:");
    let config = AppConfig::from_env();

    // Validate the configuration
    match config.validate() {
        Ok(_) => println!("   ✓ Configuration is valid"),
        Err(e) => {
            eprintln!("   ✗ Configuration validation failed: {}", e);
            return Err(e.into());
        }
    }

    println!("   Session file: {}", config.session.session_file);
    println!("   Phoenix URL: {}", config.phoenix.url);
    println!("   Phoenix topic: {}", config.phoenix.topic);
    println!("   Log level: {}", config.log.level);
    println!();

    // ========================================================================
    // Example 2: Using builder pattern for custom configuration
    // ========================================================================
    println!("2. Creating custom configuration with builder pattern:");

    let custom_config = AppConfigBuilder::new()
        .session(SessionConfigBuilder::new()
            .session_file("custom_session.db")
            .sqlite_user_version(1)
            .build())
        .phoenix(PhoenixConfigBuilder::new()
            .url("wss://production.example.com/socket")
            .topic("production:updates")
            .connection_timeout_secs(60)
            .send_timeout_secs(15)
            .build())
        .telegram(TelegramConfigBuilder::new()
            .auth_timeout_secs(45)
            .update_queue_limit(Some(50))
            .catch_up(true)
            .build())
        .log(LogConfigBuilder::new()
            .level("debug")
            .grammers_level("info")
            .build())
        .build_validated()?;

    println!("   ✓ Custom configuration created and validated");
    println!("   Custom session file: {}", custom_config.session.session_file);
    println!("   Custom Phoenix URL: {}", custom_config.phoenix.url);
    println!();

    // ========================================================================
    // Example 3: Using configuration values (replacing hardcoded constants)
    // ========================================================================
    println!("3. Using configuration in application code:");

    // OLD WAY (hardcoded):
    // const SESSION_FILE: &str = "session/my.session";
    // let timeout = Duration::from_secs(30);

    // NEW WAY (from config):
    let session_file = &config.session.session_file;
    let auth_timeout = config.telegram.auth_timeout();
    let get_me_timeout = config.telegram.get_me_timeout();
    let update_stream_timeout = config.telegram.update_stream_timeout();

    println!("   Session file: {}", session_file);
    println!("   Auth timeout: {:?}", auth_timeout);
    println!("   Get me timeout: {:?}", get_me_timeout);
    println!("   Update stream timeout: {:?}", update_stream_timeout);
    println!();

    // ========================================================================
    // Example 4: Logger initialization with config
    // ========================================================================
    println!("4. Initializing logger with config:");

    // OLD WAY:
    // simple_logger::SimpleLogger::new()
    //     .with_level(log::LevelFilter::Info)
    //     .with_module_level("grammers", log::LevelFilter::Debug)
    //     .init()?;

    // NEW WAY:
    println!("   Log level: {} -> {:?}", config.log.level, config.log.level_filter());
    println!("   Grammers level: {} -> {:?}", config.log.grammers_level, config.log.grammers_level_filter());

    // In real code:
    // simple_logger::SimpleLogger::new()
    //     .with_level(config.log.level_filter())
    //     .with_module_level("grammers", config.log.grammers_level_filter())
    //     .init()?;
    println!();

    // ========================================================================
    // Example 5: Phoenix configuration
    // ========================================================================
    println!("5. Phoenix Channel configuration:");

    // OLD WAY:
    // let phoenix_url = std::env::var("PHOENIX_URL")
    //     .unwrap_or_else(|_| "ws://localhost:4000/socket".to_string());
    // let phoenix_topic = std::env::var("PHOENIX_TOPIC")
    //     .unwrap_or_else(|_| "telegram:updates".to_string());

    // NEW WAY:
    let phoenix_url = &config.phoenix.url;
    let phoenix_topic = &config.phoenix.topic;
    let connection_timeout = config.phoenix.connection_timeout();
    let send_timeout = config.phoenix.send_timeout();

    println!("   URL: {}", phoenix_url);
    println!("   Topic: {}", phoenix_topic);
    println!("   Connection timeout: {:?}", connection_timeout);
    println!("   Send timeout: {:?}", send_timeout);
    println!("   Retry config:");
    println!("     - Initial delay: {}s", config.phoenix.retry.initial_delay_secs);
    println!("     - Max delay: {}s", config.phoenix.retry.max_delay_secs);
    println!("     - Max attempts: {}", config.phoenix.retry.max_attempts);
    println!();

    // ========================================================================
    // Example 6: Telegram updates configuration
    // ========================================================================
    println!("6. Telegram updates configuration:");

    // OLD WAY:
    // use grammers_client::UpdatesConfiguration;
    // let updates_config = UpdatesConfiguration {
    //     catch_up: true,
    //     update_queue_limit: Some(10),
    // };

    // NEW WAY:
    let updates_config = mock::UpdatesConfiguration {
        catch_up: config.telegram.catch_up,
        update_queue_limit: config.telegram.update_queue_limit,
    };

    println!("   Catch up: {}", updates_config.catch_up);
    println!("   Queue limit: {:?}", updates_config.update_queue_limit);
    println!();

    // ========================================================================
    // Example 7: Serialization/Deserialization
    // ========================================================================
    println!("7. Configuration serialization:");

    let json = serde_json::to_string_pretty(&config)?;
    println!("   Serialized to JSON:");
    println!("{}", json);
    println!();

    // Deserialize
    let deserialized: AppConfig = serde_json::from_str(&json)?;
    println!("   ✓ Successfully deserialized configuration");
    println!();

    // ========================================================================
    // Example 8: Validation examples
    // ========================================================================
    println!("8. Configuration validation examples:");

    // Valid configuration
    let valid_config = PhoenixConfigBuilder::new()
        .url("wss://valid.example.com/socket")
        .topic("valid:topic")
        .build();

    match valid_config.validate() {
        Ok(_) => println!("   ✓ Valid Phoenix config accepted"),
        Err(e) => println!("   ✗ Unexpected error: {}", e),
    }

    // Invalid configuration (bad URL)
    let invalid_config = PhoenixConfig {
        url: "http://invalid.com".to_string(), // Should be ws:// or wss://
        topic: "topic".to_string(),
        auth_token: None,
        retry: RetryConfig::default(),
        connection_timeout_secs: 30,
        join_timeout_secs: 30,
        send_timeout_secs: 10,
        enable_circuit_breaker: false,
        circuit_breaker_failure_threshold: 5,
        circuit_breaker_success_threshold: 2,
        circuit_breaker_timeout_secs: 60,
    };

    match invalid_config.validate() {
        Ok(_) => println!("   ✗ Invalid config was accepted (should fail)"),
        Err(e) => println!("   ✓ Invalid Phoenix URL rejected: {}", e),
    }

    // Invalid log level
    let invalid_log = LogConfig {
        level: "invalid_level".to_string(),
        grammers_level: "debug".to_string(),
    };

    match invalid_log.validate() {
        Ok(_) => println!("   ✗ Invalid log level was accepted (should fail)"),
        Err(e) => println!("   ✓ Invalid log level rejected: {}", e),
    }
    println!();

    // ========================================================================
    // Example 9: Environment variable override demonstration
    // ========================================================================
    println!("9. Environment variable override example:");
    println!("   Set environment variables to override defaults:");
    println!("   export PHOENIX_URL=\"wss://production.example.com/socket\"");
    println!("   export PHOENIX_TOPIC=\"production:updates\"");
    println!("   export LOG_LEVEL=\"warn\"");
    println!("   export TELEGRAM_UPDATE_QUEUE_LIMIT=\"100\"");
    println!();

    // ========================================================================
    // Example 10: Practical usage pattern
    // ========================================================================
    println!("10. Practical usage pattern in main application:");
    println!();
    println!("```rust");
    println!("async fn run() -> Result<(), Box<dyn std::error::Error>> {{");
    println!("    // Load and validate configuration");
    println!("    let config = AppConfig::from_env();");
    println!("    config.validate()?;");
    println!();
    println!("    // Initialize logger");
    println!("    simple_logger::SimpleLogger::new()");
    println!("        .with_level(config.log.level_filter())");
    println!("        .with_module_level(\"grammers\", config.log.grammers_level_filter())");
    println!("        .init()?;");
    println!();
    println!("    // Load session");
    println!("    let session = SqliteSession::open(&config.session.session_file)?;");
    println!();
    println!("    // Check authorization with configured timeout");
    println!("    let auth_result = tokio::time::timeout(");
    println!("        config.telegram.auth_timeout(),");
    println!("        client.is_authorized()");
    println!("    ).await?;");
    println!();
    println!("    // Connect to Phoenix");
    println!("    let phoenix = PhoenixBridge::new_with_config(");
    println!("        &config.phoenix.url,");
    println!("        &config.phoenix.topic,");
    println!("        config.phoenix.retry.clone()");
    println!("    ).await?;");
    println!();
    println!("    // Configure updates");
    println!("    let updates_config = UpdatesConfiguration {{");
    println!("        catch_up: config.telegram.catch_up,");
    println!("        update_queue_limit: config.telegram.update_queue_limit,");
    println!("    }};");
    println!();
    println!("    Ok(())");
    println!("}}");
    println!("```");
    println!();

    println!("=== Example Complete ===");

    Ok(())
}
