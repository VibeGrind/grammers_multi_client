# Configuration Module Quick Reference

## Quick Start

```rust
use config::AppConfig;

// Load configuration
let config = AppConfig::from_env();
config.validate()?;

// Use configuration
log::info!("Session: {}", config.session.session_file);
log::info!("Phoenix: {}", config.phoenix.url);
```

## Environment Variables Cheat Sheet

```bash
# Session
export SESSION_FILE="session/my.session"
export SQLITE_USER_VERSION="1"

# Phoenix
export PHOENIX_URL="ws://localhost:4000/socket"
export PHOENIX_TOPIC="telegram:updates"
export PHOENIX_CONNECTION_TIMEOUT_SECS="30"
export PHOENIX_JOIN_TIMEOUT_SECS="30"
export PHOENIX_SEND_TIMEOUT_SECS="10"

# Retry
export RETRY_INITIAL_DELAY_SECS="1"
export RETRY_MAX_DELAY_SECS="30"
export RETRY_MAX_ATTEMPTS="5"

# Telegram
export TELEGRAM_AUTH_TIMEOUT_SECS="30"
export TELEGRAM_GET_ME_TIMEOUT_SECS="30"
export TELEGRAM_UPDATE_STREAM_TIMEOUT_SECS="10"
export TELEGRAM_UPDATE_QUEUE_LIMIT="10"
export TELEGRAM_CATCH_UP="true"

# Logging
export LOG_LEVEL="info"
export LOG_GRAMMERS_LEVEL="debug"
```

## Configuration Structure

```
AppConfig
├── SessionConfig
│   ├── session_file: String
│   └── sqlite_user_version: i32
├── PhoenixConfig
│   ├── url: String
│   ├── topic: String
│   ├── retry: RetryConfig
│   │   ├── initial_delay_secs: u64
│   │   ├── max_delay_secs: u64
│   │   └── max_attempts: u32
│   ├── connection_timeout_secs: u64
│   ├── join_timeout_secs: u64
│   └── send_timeout_secs: u64
├── TelegramConfig
│   ├── auth_timeout_secs: u64
│   ├── get_me_timeout_secs: u64
│   ├── update_stream_timeout_secs: u64
│   ├── update_queue_limit: Option<usize>
│   └── catch_up: bool
└── LogConfig
    ├── level: String
    └── grammers_level: String
```

## Common Patterns

### Load and Validate

```rust
let config = AppConfig::from_env();
config.validate()?;
```

### Use Duration Helpers

```rust
// Good
let timeout = config.telegram.auth_timeout();

// Avoid
let timeout = Duration::from_secs(config.telegram.auth_timeout_secs);
```

### Builder Pattern

```rust
let config = AppConfigBuilder::new()
    .session(SessionConfigBuilder::new()
        .session_file("custom.session")
        .build())
    .phoenix(PhoenixConfigBuilder::new()
        .url("wss://example.com/socket")
        .build())
    .build_validated()?;
```

### Logger Setup

```rust
simple_logger::SimpleLogger::new()
    .with_level(config.log.level_filter())
    .with_module_level("grammers", config.log.grammers_level_filter())
    .init()?;
```

## Replacements Map

| Old (Hardcoded) | New (Config) |
|-----------------|--------------|
| `SESSION_FILE` | `config.session.session_file` |
| `DEFAULT_PHOENIX_URL` | `config.phoenix.url` |
| `DEFAULT_PHOENIX_TOPIC` | `config.phoenix.topic` |
| `Duration::from_secs(30)` (auth) | `config.telegram.auth_timeout()` |
| `Duration::from_secs(30)` (get_me) | `config.telegram.get_me_timeout()` |
| `Duration::from_secs(10)` (updates) | `config.telegram.update_stream_timeout()` |
| `Duration::from_secs(10)` (phoenix) | `config.phoenix.send_timeout()` |
| `update_queue_limit: Some(10)` | `config.telegram.update_queue_limit` |
| `LevelFilter::Info` | `config.log.level_filter()` |
| `LevelFilter::Debug` | `config.log.grammers_level_filter()` |

## Files Created

1. **src/config.rs** - Main configuration module (775 lines)
2. **CONFIG.md** - Complete documentation
3. **MIGRATION_GUIDE.md** - Migration guide from old constants
4. **INTEGRATION_NOTES.md** - Integration details and checklist
5. **examples/config_usage_example.rs** - Runnable example
6. **CONFIG_QUICK_REFERENCE.md** - This file

## Module Status

- [x] src/config.rs created
- [x] mod config added to main.rs
- [ ] phoenix_bridge.rs updated (remove duplicate RetryConfig)
- [ ] main.rs refactored (replace hardcoded values)
- [ ] Integration tested

## Default Values

| Config Path | Default Value |
|-------------|---------------|
| `session.session_file` | `"session/my.session"` |
| `session.sqlite_user_version` | `1` |
| `phoenix.url` | `"ws://localhost:4000/socket"` |
| `phoenix.topic` | `"telegram:updates"` |
| `phoenix.connection_timeout_secs` | `30` |
| `phoenix.join_timeout_secs` | `30` |
| `phoenix.send_timeout_secs` | `10` |
| `phoenix.retry.initial_delay_secs` | `1` |
| `phoenix.retry.max_delay_secs` | `30` |
| `phoenix.retry.max_attempts` | `5` |
| `telegram.auth_timeout_secs` | `30` |
| `telegram.get_me_timeout_secs` | `30` |
| `telegram.update_stream_timeout_secs` | `10` |
| `telegram.update_queue_limit` | `Some(10)` |
| `telegram.catch_up` | `true` |
| `log.level` | `"info"` |
| `log.grammers_level` | `"debug"` |

## Validation Rules

- Phoenix URL must start with `ws://` or `wss://`
- All timeout values must be > 0
- Log levels must be: trace, debug, info, warn, error
- Retry max_delay must be >= initial_delay
- Max attempts must be > 0
- Session file cannot be empty

## Helper Methods

```rust
// Duration helpers
config.phoenix.connection_timeout() -> Duration
config.phoenix.join_timeout() -> Duration
config.phoenix.send_timeout() -> Duration
config.telegram.auth_timeout() -> Duration
config.telegram.get_me_timeout() -> Duration
config.telegram.update_stream_timeout() -> Duration

// Log level helpers
config.log.level_filter() -> log::LevelFilter
config.log.grammers_level_filter() -> log::LevelFilter
```

## Testing

```bash
# Run unit tests
cargo test --lib config

# Run example
cargo run --example config_usage_example

# Test with env vars
export PHOENIX_URL="wss://test.com/socket"
export LOG_LEVEL="debug"
cargo run

# Test validation (should fail)
export PHOENIX_URL="http://invalid"
cargo run
```

## Error Handling

```rust
use config::ConfigError;

match config.validate() {
    Ok(_) => { /* proceed */ }
    Err(ConfigError::Validation(msg)) => { /* handle validation error */ }
    Err(ConfigError::Parse(msg)) => { /* handle parse error */ }
    Err(e) => { /* handle other errors */ }
}
```

## JSON Serialization

```rust
// Serialize
let json = serde_json::to_string_pretty(&config)?;

// Deserialize
let config: AppConfig = serde_json::from_str(&json)?;

// Load from file
let json = std::fs::read_to_string("config.json")?;
let config: AppConfig = serde_json::from_str(&json)?;
config.validate()?;
```

## Best Practices

1. ✅ Load config once at startup
2. ✅ Validate immediately after loading
3. ✅ Use Duration helper methods
4. ✅ Use builder pattern for tests
5. ✅ Use environment variables in production
6. ❌ Don't modify config after validation
7. ❌ Don't hardcode values anymore
8. ❌ Don't skip validation

## Integration Priority

1. **High Priority**: Update phoenix_bridge.rs (remove duplicate RetryConfig)
2. **High Priority**: Update main.rs (replace constants with config)
3. **Medium Priority**: Add config file support
4. **Low Priority**: Add dynamic config reloading
