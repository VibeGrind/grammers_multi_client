# Integration Notes for Configuration Module

## Overview

The new `src/config.rs` module has been created successfully. This document provides specific notes on integrating it with the existing codebase.

## RetryConfig Duplication Issue

**Problem**: There are currently two `RetryConfig` definitions:
1. `src/config.rs` - The new centralized version (with Serialize/Deserialize)
2. `src/phoenix_bridge.rs` - The old version (lines 26-41)

**Resolution**: Update `phoenix_bridge.rs` to use the config module's `RetryConfig`.

### Changes Required in phoenix_bridge.rs

**Step 1: Add import at the top of phoenix_bridge.rs**

```rust
use crate::config::RetryConfig;
```

**Step 2: Remove the duplicate RetryConfig definition**

Remove these lines from `phoenix_bridge.rs` (lines 25-41):

```rust
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
```

**Step 3: No other changes needed**

The `RetryConfig` in `config.rs` has the same structure and fields, so all existing code in `phoenix_bridge.rs` will continue to work without modifications. The only difference is that the config version also implements `Serialize` and `Deserialize`.

## Module Dependency Graph

```
main.rs
├── config (new)
├── phoenix_bridge
│   └── uses config::RetryConfig (after update)
├── error
└── storage (if exists)
```

## Complete Integration Checklist

- [x] Create `src/config.rs` with all configuration structures
- [x] Add `mod config;` to `src/main.rs`
- [ ] Update `phoenix_bridge.rs` to import and use `config::RetryConfig`
- [ ] Remove duplicate `RetryConfig` from `phoenix_bridge.rs`
- [ ] Update `main.rs` to use `AppConfig::from_env()`
- [ ] Replace hardcoded constants in `main.rs` with config values
- [ ] Replace timeout durations with config helper methods
- [ ] Update UpdatesConfiguration to use config values
- [ ] Test configuration loading
- [ ] Test configuration validation
- [ ] Test environment variable overrides

## Quick Integration Example

Here's a minimal example of integrating the config into main.rs:

```rust
mod phoenix_bridge;
mod error;
mod config;  // ✓ Already added

use config::AppConfig;

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Load configuration
    let config = AppConfig::from_env();

    // Step 2: Validate early
    if let Err(e) = config.validate() {
        eprintln!("Configuration error: {}", e);
        return Err(e.into());
    }

    // Step 3: Use throughout the application
    simple_logger::SimpleLogger::new()
        .with_level(config.log.level_filter())
        .with_module_level("grammers", config.log.grammers_level_filter())
        .init()?;

    log::info!("Session file: {}", config.session.session_file);
    log::info!("Phoenix URL: {}", config.phoenix.url);

    // ... rest of application
    Ok(())
}
```

## Configuration File Support

While the system primarily uses environment variables, you can also support configuration files:

**config.json example:**
```json
{
  "session": {
    "session_file": "session/my.session",
    "sqlite_user_version": 1
  },
  "phoenix": {
    "url": "ws://localhost:4000/socket",
    "topic": "telegram:updates",
    "retry": {
      "initial_delay_secs": 1,
      "max_delay_secs": 30,
      "max_attempts": 5
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

**Loading from file:**
```rust
use std::fs;
use config::AppConfig;

fn load_config_from_file(path: &str) -> Result<AppConfig, Box<dyn std::error::Error>> {
    let json = fs::read_to_string(path)?;
    let config: AppConfig = serde_json::from_str(&json)?;
    config.validate()?;
    Ok(config)
}

// Usage
let config = if std::path::Path::new("config.json").exists() {
    load_config_from_file("config.json")?
} else {
    AppConfig::from_env()
};
```

## Environment Variable Priority

Recommended priority order:
1. Environment variables (highest priority)
2. Configuration file (if exists)
3. Default values (lowest priority)

**Implementation:**
```rust
fn load_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    // Load base config from file or defaults
    let mut config = if std::path::Path::new("config.json").exists() {
        let json = fs::read_to_string("config.json")?;
        serde_json::from_str(&json)?
    } else {
        AppConfig::default()
    };

    // Override with environment variables
    if let Ok(url) = std::env::var("PHOENIX_URL") {
        config.phoenix.url = url;
    }
    if let Ok(level) = std::env::var("LOG_LEVEL") {
        config.log.level = level;
    }

    // Or simpler: use from_env which already handles this
    let config = AppConfig::from_env();

    // Validate
    config.validate()?;

    Ok(config)
}
```

## Cargo.toml Dependencies

The config module requires these dependencies (already in Cargo.toml):

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
log = "0.4"
tokio = { version = "1", features = ["time"] }
```

All dependencies are already present, so no changes needed.

## Testing Strategy

### Unit Tests

The config module includes comprehensive unit tests in `src/config.rs`:

```bash
# Run config module tests
cargo test --lib config

# Run all tests
cargo test
```

### Integration Tests

Create an integration test to verify configuration loading:

**tests/config_integration_test.rs:**
```rust
use telegram_minimal_client::config::*;

#[test]
fn test_default_config_is_valid() {
    let config = AppConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_env_override() {
    std::env::set_var("PHOENIX_URL", "wss://test.com/socket");
    let config = AppConfig::from_env();
    assert_eq!(config.phoenix.url, "wss://test.com/socket");
    std::env::remove_var("PHOENIX_URL");
}

#[test]
fn test_builder_pattern() {
    let config = AppConfigBuilder::new()
        .phoenix(PhoenixConfigBuilder::new()
            .url("wss://example.com/socket")
            .build())
        .build();

    assert_eq!(config.phoenix.url, "wss://example.com/socket");
}
```

### Manual Testing Checklist

```bash
# Test 1: Default configuration
cargo run

# Test 2: Environment overrides
export PHOENIX_URL="wss://test.example.com/socket"
export LOG_LEVEL="debug"
export TELEGRAM_UPDATE_QUEUE_LIMIT="50"
cargo run

# Test 3: Invalid configuration (should fail validation)
export PHOENIX_URL="http://invalid.com"
cargo run  # Should exit with validation error

# Test 4: All environment variables
export SESSION_FILE="custom.session"
export PHOENIX_TOPIC="custom:topic"
export RETRY_MAX_ATTEMPTS="10"
export TELEGRAM_CATCH_UP="false"
export LOG_GRAMMERS_LEVEL="info"
cargo run

# Cleanup
unset SESSION_FILE PHOENIX_URL PHOENIX_TOPIC RETRY_MAX_ATTEMPTS
unset TELEGRAM_CATCH_UP LOG_LEVEL LOG_GRAMMERS_LEVEL TELEGRAM_UPDATE_QUEUE_LIMIT
```

## Common Pitfalls

### 1. Forgetting to Validate

**Problem:**
```rust
let config = AppConfig::from_env();
// Using config without validation
```

**Solution:**
```rust
let config = AppConfig::from_env();
config.validate()?;  // Always validate!
```

### 2. Using Raw Seconds Instead of Duration Helpers

**Problem:**
```rust
tokio::time::timeout(
    Duration::from_secs(config.telegram.auth_timeout_secs),
    client.is_authorized()
)
```

**Solution:**
```rust
tokio::time::timeout(
    config.telegram.auth_timeout(),  // Use helper method
    client.is_authorized()
)
```

### 3. Duplicating RetryConfig

**Problem:**
Both `config.rs` and `phoenix_bridge.rs` define `RetryConfig`.

**Solution:**
Import from config module in `phoenix_bridge.rs`:
```rust
use crate::config::RetryConfig;
```

### 4. Not Using Builder Pattern in Tests

**Problem:**
Creating configs manually in tests is verbose.

**Solution:**
```rust
#[test]
fn test_something() {
    let config = TelegramConfigBuilder::new()
        .auth_timeout_secs(5)  // Short timeout for tests
        .build();
}
```

## Documentation

The following documentation files have been created:

1. **CONFIG.md** - Complete configuration system documentation
2. **MIGRATION_GUIDE.md** - Step-by-step migration guide
3. **INTEGRATION_NOTES.md** - This file, with integration details
4. **examples/config_usage_example.rs** - Runnable example

## Next Steps

1. **Update phoenix_bridge.rs**:
   ```bash
   # Edit src/phoenix_bridge.rs
   # Add: use crate::config::RetryConfig;
   # Remove: lines 25-41 (duplicate RetryConfig)
   ```

2. **Update main.rs**:
   ```rust
   // Add at the beginning of run()
   let config = AppConfig::from_env();
   config.validate()?;

   // Replace all hardcoded values with config references
   ```

3. **Test the integration**:
   ```bash
   cargo build
   cargo test
   cargo run
   ```

4. **Clean up old constants**:
   ```rust
   // Remove these from main.rs:
   // const SESSION_FILE: &str = ...
   // const PHOENIX_URL_ENV: &str = ...
   // const DEFAULT_PHOENIX_URL: &str = ...
   // etc.
   ```

## Support

For questions or issues:
1. Check CONFIG.md for usage examples
2. Check MIGRATION_GUIDE.md for refactoring steps
3. Run the example: `cargo run --example config_usage_example`
4. Review the unit tests in `src/config.rs`
