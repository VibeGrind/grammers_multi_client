# Configuration System Documentation

## Overview

The application uses a centralized configuration module (`src/config.rs`) that consolidates all configuration settings previously scattered throughout the codebase. The configuration system supports:

- Default values for all settings
- Loading from environment variables
- Comprehensive validation
- Builder pattern for easy construction
- Type-safe access to configuration values

## Configuration Structure

### AppConfig (Main Configuration)

The top-level configuration container with four subsections:

```rust
use config::AppConfig;

// Load configuration from environment variables with defaults
let config = AppConfig::from_env();

// Validate all configuration
config.validate()?;
```

### SessionConfig

Configuration for session management and SQLite database:

**Fields:**
- `session_file`: Path to SQLite session file (default: `"session/my.session"`)
- `sqlite_user_version`: SQLite user_version for grammers compatibility (default: `1`)

**Environment Variables:**
- `SESSION_FILE`: Override session file path
- `SQLITE_USER_VERSION`: Override SQLite user version

**Example:**
```rust
use config::SessionConfigBuilder;

let session = SessionConfigBuilder::new()
    .session_file("custom/path.session")
    .sqlite_user_version(1)
    .build();
```

### PhoenixConfig

Configuration for Phoenix Channel integration:

**Fields:**
- `url`: Phoenix websocket URL (default: `"ws://localhost:4000/socket"`)
- `topic`: Phoenix channel topic (default: `"telegram:updates"`)
- `retry`: Retry configuration for Phoenix operations
- `connection_timeout_secs`: Connection timeout in seconds (default: `30`)
- `join_timeout_secs`: Channel join timeout in seconds (default: `30`)
- `send_timeout_secs`: Send operation timeout in seconds (default: `10`)

**Environment Variables:**
- `PHOENIX_URL`: Phoenix websocket URL
- `PHOENIX_TOPIC`: Phoenix channel topic
- `PHOENIX_CONNECTION_TIMEOUT_SECS`: Connection timeout
- `PHOENIX_JOIN_TIMEOUT_SECS`: Join timeout
- `PHOENIX_SEND_TIMEOUT_SECS`: Send timeout

**Helper Methods:**
```rust
let timeout: Duration = config.phoenix.connection_timeout();
let join_timeout: Duration = config.phoenix.join_timeout();
let send_timeout: Duration = config.phoenix.send_timeout();
```

**Example:**
```rust
use config::PhoenixConfigBuilder;

let phoenix = PhoenixConfigBuilder::new()
    .url("wss://production.example.com/socket")
    .topic("production:updates")
    .connection_timeout_secs(60)
    .build();
```

### RetryConfig

Configuration for retry logic with exponential backoff:

**Fields:**
- `initial_delay_secs`: Initial delay before first retry (default: `1`)
- `max_delay_secs`: Maximum delay between retries (default: `30`)
- `max_attempts`: Maximum number of retry attempts (default: `5`)

**Environment Variables:**
- `RETRY_INITIAL_DELAY_SECS`: Initial delay
- `RETRY_MAX_DELAY_SECS`: Maximum delay
- `RETRY_MAX_ATTEMPTS`: Maximum attempts

**Example:**
```rust
use config::RetryConfig;

let retry = RetryConfig {
    initial_delay_secs: 2,
    max_delay_secs: 60,
    max_attempts: 10,
};
```

### TelegramConfig

Configuration for Telegram client operations:

**Fields:**
- `auth_timeout_secs`: Authorization check timeout (default: `30`)
- `get_me_timeout_secs`: get_me() call timeout (default: `30`)
- `update_stream_timeout_secs`: Update stream polling timeout (default: `10`)
- `update_queue_limit`: Max updates to queue, None for unlimited (default: `Some(10)`)
- `catch_up`: Whether to catch up on missed updates (default: `true`)

**Environment Variables:**
- `TELEGRAM_AUTH_TIMEOUT_SECS`: Authorization timeout
- `TELEGRAM_GET_ME_TIMEOUT_SECS`: get_me timeout
- `TELEGRAM_UPDATE_STREAM_TIMEOUT_SECS`: Update stream timeout
- `TELEGRAM_UPDATE_QUEUE_LIMIT`: Update queue limit
- `TELEGRAM_CATCH_UP`: Enable catch up (true/false)

**Helper Methods:**
```rust
let auth_timeout: Duration = config.telegram.auth_timeout();
let get_me_timeout: Duration = config.telegram.get_me_timeout();
let stream_timeout: Duration = config.telegram.update_stream_timeout();
```

**Example:**
```rust
use config::TelegramConfigBuilder;

let telegram = TelegramConfigBuilder::new()
    .auth_timeout_secs(60)
    .update_queue_limit(Some(100))
    .catch_up(true)
    .build();
```

### LogConfig

Configuration for logging:

**Fields:**
- `level`: Default log level (default: `"info"`)
- `grammers_level`: Grammers module log level (default: `"debug"`)

**Valid log levels:** `trace`, `debug`, `info`, `warn`, `error`

**Environment Variables:**
- `LOG_LEVEL`: Default log level
- `LOG_GRAMMERS_LEVEL`: Grammers log level

**Helper Methods:**
```rust
let level_filter: log::LevelFilter = config.log.level_filter();
let grammers_filter: log::LevelFilter = config.log.grammers_level_filter();

// Use with logger
simple_logger::SimpleLogger::new()
    .with_level(config.log.level_filter())
    .with_module_level("grammers", config.log.grammers_level_filter())
    .init()?;
```

**Example:**
```rust
use config::LogConfigBuilder;

let log = LogConfigBuilder::new()
    .level("debug")
    .grammers_level("trace")
    .build();
```

## Usage Examples

### Basic Usage (Environment Variables)

```rust
use config::AppConfig;

// Load configuration from environment variables with sensible defaults
let config = AppConfig::from_env();

// Validate the configuration
if let Err(e) = config.validate() {
    eprintln!("Invalid configuration: {}", e);
    std::process::exit(1);
}

// Use configuration values
println!("Session file: {}", config.session.session_file);
println!("Phoenix URL: {}", config.phoenix.url);
```

### Builder Pattern

```rust
use config::{
    AppConfigBuilder, SessionConfigBuilder, PhoenixConfigBuilder,
    TelegramConfigBuilder, LogConfigBuilder
};

let config = AppConfigBuilder::new()
    .session(SessionConfigBuilder::new()
        .session_file("my_custom.session")
        .build())
    .phoenix(PhoenixConfigBuilder::new()
        .url("wss://example.com/socket")
        .topic("custom:topic")
        .connection_timeout_secs(60)
        .build())
    .telegram(TelegramConfigBuilder::new()
        .auth_timeout_secs(45)
        .update_queue_limit(Some(50))
        .build())
    .log(LogConfigBuilder::new()
        .level("debug")
        .grammers_level("info")
        .build())
    .build_validated()?; // Build and validate in one step
```

### Mixed Approach (Defaults + Overrides)

```rust
use config::AppConfig;

// Start with environment-based config
let mut config = AppConfig::from_env();

// Override specific values programmatically
config.phoenix.url = "wss://production.example.com/socket".to_string();
config.telegram.update_queue_limit = Some(100);

// Validate after modifications
config.validate()?;
```

### Serialization/Deserialization

The configuration supports JSON serialization:

```rust
use config::AppConfig;

// Serialize to JSON
let config = AppConfig::default();
let json = serde_json::to_string_pretty(&config)?;
println!("{}", json);

// Deserialize from JSON
let config: AppConfig = serde_json::from_str(&json)?;
```

## Configuration File Example

While the system primarily uses environment variables, you can also load from a JSON file:

```json
{
  "session": {
    "session_file": "session/production.session",
    "sqlite_user_version": 1
  },
  "phoenix": {
    "url": "wss://example.com/socket",
    "topic": "production:updates",
    "retry": {
      "initial_delay_secs": 2,
      "max_delay_secs": 60,
      "max_attempts": 10
    },
    "connection_timeout_secs": 30,
    "join_timeout_secs": 30,
    "send_timeout_secs": 10
  },
  "telegram": {
    "auth_timeout_secs": 30,
    "get_me_timeout_secs": 30,
    "update_stream_timeout_secs": 10,
    "update_queue_limit": 10,
    "catch_up": true
  },
  "log": {
    "level": "info",
    "grammers_level": "debug"
  }
}
```

Load from file:

```rust
use std::fs;
use config::AppConfig;

let json = fs::read_to_string("config.json")?;
let config: AppConfig = serde_json::from_str(&json)?;
config.validate()?;
```

## Validation

All configuration sections implement validation:

```rust
use config::{AppConfig, ConfigError};

let config = AppConfig::from_env();

match config.validate() {
    Ok(_) => println!("Configuration is valid"),
    Err(ConfigError::Validation(msg)) => {
        eprintln!("Validation error: {}", msg);
    }
    Err(e) => eprintln!("Configuration error: {}", e),
}
```

Common validation rules:
- Paths and URLs cannot be empty
- Phoenix URL must start with `ws://` or `wss://`
- Timeout values must be greater than 0
- Log levels must be valid (`trace`, `debug`, `info`, `warn`, `error`)
- Retry max_delay must be >= initial_delay
- Max attempts must be greater than 0

## Environment Variables Summary

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `SESSION_FILE` | String | `session/my.session` | SQLite session file path |
| `SQLITE_USER_VERSION` | i32 | `1` | SQLite user version |
| `PHOENIX_URL` | String | `ws://localhost:4000/socket` | Phoenix websocket URL |
| `PHOENIX_TOPIC` | String | `telegram:updates` | Phoenix channel topic |
| `PHOENIX_CONNECTION_TIMEOUT_SECS` | u64 | `30` | Connection timeout |
| `PHOENIX_JOIN_TIMEOUT_SECS` | u64 | `30` | Join timeout |
| `PHOENIX_SEND_TIMEOUT_SECS` | u64 | `10` | Send timeout |
| `RETRY_INITIAL_DELAY_SECS` | u64 | `1` | Initial retry delay |
| `RETRY_MAX_DELAY_SECS` | u64 | `30` | Maximum retry delay |
| `RETRY_MAX_ATTEMPTS` | u32 | `5` | Maximum retry attempts |
| `TELEGRAM_AUTH_TIMEOUT_SECS` | u64 | `30` | Auth check timeout |
| `TELEGRAM_GET_ME_TIMEOUT_SECS` | u64 | `30` | get_me timeout |
| `TELEGRAM_UPDATE_STREAM_TIMEOUT_SECS` | u64 | `10` | Update stream timeout |
| `TELEGRAM_UPDATE_QUEUE_LIMIT` | usize | `10` | Update queue limit |
| `TELEGRAM_CATCH_UP` | bool | `true` | Catch up on updates |
| `LOG_LEVEL` | String | `info` | Default log level |
| `LOG_GRAMMERS_LEVEL` | String | `debug` | Grammers log level |

## Migration Guide

### Before (Scattered Constants)

```rust
const SESSION_FILE: &str = "session/my.session";
const DEFAULT_PHOENIX_URL: &str = "ws://localhost:4000/socket";
let timeout = Duration::from_secs(30);
```

### After (Centralized Config)

```rust
let config = AppConfig::from_env();
let session_file = &config.session.session_file;
let phoenix_url = &config.phoenix.url;
let timeout = config.telegram.auth_timeout();
```

## Best Practices

1. **Load Once, Use Everywhere**: Load configuration once at startup and pass references where needed.

2. **Validate Early**: Always validate configuration immediately after loading:
   ```rust
   let config = AppConfig::from_env();
   config.validate()?;
   ```

3. **Use Builder for Tests**: Use the builder pattern in tests for explicit configuration:
   ```rust
   #[test]
   fn test_with_custom_config() {
       let config = AppConfigBuilder::new()
           .telegram(TelegramConfigBuilder::new()
               .auth_timeout_secs(5)
               .build())
           .build();
   }
   ```

4. **Environment Variables for Production**: Use environment variables for production deployments:
   ```bash
   export PHOENIX_URL="wss://production.example.com/socket"
   export LOG_LEVEL="warn"
   ./telegram-minimal-client
   ```

5. **Type-Safe Durations**: Use the helper methods to get `Duration` types:
   ```rust
   // Good
   let timeout = config.telegram.auth_timeout();

   // Avoid
   let timeout = Duration::from_secs(config.telegram.auth_timeout_secs);
   ```

## Error Handling

The configuration module provides a custom error type:

```rust
use config::ConfigError;

match config.validate() {
    Ok(_) => { /* proceed */ }
    Err(ConfigError::Validation(msg)) => {
        eprintln!("Invalid config: {}", msg);
    }
    Err(ConfigError::Parse(msg)) => {
        eprintln!("Parse error: {}", msg);
    }
    Err(ConfigError::Io(e)) => {
        eprintln!("IO error: {}", e);
    }
    Err(e) => {
        eprintln!("Configuration error: {}", e);
    }
}
```
