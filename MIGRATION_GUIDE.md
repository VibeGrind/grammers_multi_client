# Migration Guide: Using the Centralized Configuration System

This guide shows how to refactor the existing code to use the new centralized configuration module.

## Overview

The configuration system replaces:
- Hardcoded constants in `main.rs`
- Scattered environment variable access
- Inline timeout durations
- Hardcoded retry settings in `phoenix_bridge.rs`

## Step-by-Step Migration

### Step 1: Load Configuration at Startup

**Before:**
```rust
const SESSION_FILE: &str = "session/my.session";
const PHOENIX_URL_ENV: &str = "PHOENIX_URL";
const PHOENIX_TOPIC_ENV: &str = "PHOENIX_TOPIC";
const DEFAULT_PHOENIX_URL: &str = "ws://localhost:4000/socket";
const DEFAULT_PHOENIX_TOPIC: &str = "telegram:updates";

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // ...
}
```

**After:**
```rust
use config::AppConfig;

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment variables with defaults
    let config = AppConfig::from_env();

    // Validate configuration early
    if let Err(e) = config.validate() {
        log::error!("Configuration error: {}", e);
        return Err(e.into());
    }

    // ... rest of the function
}
```

### Step 2: Replace Logger Initialization

**Before:**
```rust
simple_logger::SimpleLogger::new()
    .with_level(log::LevelFilter::Info)
    .with_module_level("grammers", log::LevelFilter::Debug)
    .init()
    .expect("Failed to initialize logger");
```

**After:**
```rust
simple_logger::SimpleLogger::new()
    .with_level(config.log.level_filter())
    .with_module_level("grammers", config.log.grammers_level_filter())
    .init()
    .expect("Failed to initialize logger");

log::info!("=== Telegram Session Client ===");
log::info!("Configuration loaded:");
log::info!("  Log level: {}", config.log.level);
log::info!("  Grammers log level: {}", config.log.grammers_level);
```

### Step 3: Replace Session File Path

**Before:**
```rust
log::info!("Loading session: {}", SESSION_FILE);

let session_file = SESSION_FILE.to_string();
let session_data = tokio::task::spawn_blocking(move || {
    load_session_data(&session_file)
})
.await??;

let session = Arc::new(SqliteSession::open(SESSION_FILE)?);
```

**After:**
```rust
log::info!("Loading session: {}", config.session.session_file);

let session_file = config.session.session_file.clone();
let session_data = tokio::task::spawn_blocking(move || {
    load_session_data(&session_file)
})
.await??;

let session = Arc::new(SqliteSession::open(&config.session.session_file)?);
```

### Step 4: Replace Authorization Timeout

**Before:**
```rust
log::info!("Checking authorization...");
let auth_result = tokio::time::timeout(
    std::time::Duration::from_secs(30),
    client.is_authorized()
).await;
```

**After:**
```rust
log::info!("Checking authorization...");
let auth_result = tokio::time::timeout(
    config.telegram.auth_timeout(),
    client.is_authorized()
).await;
```

### Step 5: Replace get_me() Timeout

**Before:**
```rust
let get_me_result = tokio::time::timeout(
    std::time::Duration::from_secs(30),
    client.get_me()
).await;
```

**After:**
```rust
let get_me_result = tokio::time::timeout(
    config.telegram.get_me_timeout(),
    client.get_me()
).await;
```

### Step 6: Replace Phoenix Configuration

**Before:**
```rust
let phoenix_url = std::env::var(PHOENIX_URL_ENV)
    .unwrap_or_else(|_| DEFAULT_PHOENIX_URL.to_string());
let phoenix_topic = std::env::var(PHOENIX_TOPIC_ENV)
    .unwrap_or_else(|_| DEFAULT_PHOENIX_TOPIC.to_string());

log::info!("=== Phoenix Channel Integration ===");
log::info!("  URL: {}", phoenix_url);
log::info!("  Topic: {}", phoenix_topic);

let phoenix_result = tokio::time::timeout(
    std::time::Duration::from_secs(30),
    PhoenixBridge::new(&phoenix_url, &phoenix_topic)
).await;
```

**After:**
```rust
log::info!("=== Phoenix Channel Integration ===");
log::info!("  URL: {}", config.phoenix.url);
log::info!("  Topic: {}", config.phoenix.topic);
log::info!("  Connection timeout: {:?}", config.phoenix.connection_timeout());
log::info!("  Retry attempts: {}", config.phoenix.retry.max_attempts);

let phoenix_result = tokio::time::timeout(
    config.phoenix.connection_timeout(),
    PhoenixBridge::new_with_config(
        &config.phoenix.url,
        &config.phoenix.topic,
        config.phoenix.retry.clone()
    )
).await;
```

### Step 7: Replace Updates Configuration

**Before:**
```rust
use grammers_client::UpdatesConfiguration;
let updates_config = UpdatesConfiguration {
    catch_up: true,
    update_queue_limit: Some(10),  // Hardcoded value
};
let mut updates_stream = client.stream_updates(updates, updates_config);
```

**After:**
```rust
use grammers_client::UpdatesConfiguration;
let updates_config = UpdatesConfiguration {
    catch_up: config.telegram.catch_up,
    update_queue_limit: config.telegram.update_queue_limit,
};
let mut updates_stream = client.stream_updates(updates, updates_config);

log::info!("Update stream configuration:");
log::info!("  Catch up: {}", updates_config.catch_up);
log::info!("  Queue limit: {:?}", updates_config.update_queue_limit);
```

### Step 8: Replace Update Stream Timeout

**Before:**
```rust
update_result = tokio::time::timeout(
    std::time::Duration::from_secs(10),
    updates_stream.next()
) => {
    // ...
}
```

**After:**
```rust
update_result = tokio::time::timeout(
    config.telegram.update_stream_timeout(),
    updates_stream.next()
) => {
    // ...
}
```

### Step 9: Replace Phoenix Send Timeout

**Before:**
```rust
let send_result = tokio::time::timeout(
    std::time::Duration::from_secs(10),
    phoenix.send_update(telegram_update)
).await;
```

**After:**
```rust
let send_result = tokio::time::timeout(
    config.phoenix.send_timeout(),
    phoenix.send_update(telegram_update)
).await;
```

## Complete Refactored Function

Here's how the `run()` function looks after migration:

```rust
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Load and validate configuration
    let config = AppConfig::from_env();
    config.validate()?;

    // Initialize logger with config
    simple_logger::SimpleLogger::new()
        .with_level(config.log.level_filter())
        .with_module_level("grammers", config.log.grammers_level_filter())
        .init()
        .expect("Failed to initialize logger");

    log::info!("=== Telegram Session Client ===");
    log::info!("Configuration loaded successfully");
    log::info!("Loading session: {}", config.session.session_file);

    // Load session data
    let session_file = config.session.session_file.clone();
    let session_data = tokio::task::spawn_blocking(move || {
        load_session_data(&session_file)
    })
    .await??;

    log::info!("Device fingerprint loaded:");
    log::info!("  Device: {}", session_data.device);
    log::info!("  API ID: {}", session_data.app_id);

    // Open session
    let session = Arc::new(SqliteSession::open(&config.session.session_file)?);

    // Create connection params
    let connection_params = ConnectionParams {
        device_model: session_data.device.clone(),
        system_version: session_data.sdk.clone(),
        app_version: session_data.app_version.clone(),
        system_lang_code: session_data.system_lang_code.clone(),
        lang_code: session_data.lang_code.clone(),
        #[cfg(feature = "proxy")]
        proxy_url: session_data.proxy.clone(),
        __non_exhaustive: (),
    };

    // Create pool and client
    let pool = SenderPool::with_configuration(
        Arc::clone(&session),
        session_data.app_id,
        connection_params,
    );
    let client = Client::new(&pool);
    let SenderPool { runner, handle, updates } = pool;

    // Start pool runner
    log::info!("Starting MTProto connection...");
    let pool_task = tokio::spawn(runner.run());

    // Check authorization with configured timeout
    log::info!("Checking authorization (timeout: {:?})...", config.telegram.auth_timeout());
    let auth_result = tokio::time::timeout(
        config.telegram.auth_timeout(),
        client.is_authorized()
    ).await;

    match auth_result {
        Ok(Ok(true)) => {
            log::info!("✓ Session is authorized");
            println!("✓ Connected to Telegram");
        }
        Ok(Ok(false)) => {
            log::error!("✗ Session is NOT authorized");
            handle.quit();
            pool_task.await?;
            return Err("Session not authorized".into());
        }
        Ok(Err(e)) => {
            log::error!("Authorization check failed: {:?}", e);
            handle.quit();
            pool_task.await?;
            return Err(format!("Authorization check failed: {}", e).into());
        }
        Err(_) => {
            log::error!("Authorization check timed out after {:?}", config.telegram.auth_timeout());
            handle.quit();
            pool_task.await?;
            return Err("Authorization check timed out".into());
        }
    }

    // Get user info with configured timeout
    let get_me_result = tokio::time::timeout(
        config.telegram.get_me_timeout(),
        client.get_me()
    ).await;

    match get_me_result {
        Ok(Ok(user)) => {
            let name = user.first_name().unwrap_or("Unknown");
            let id = user.raw.id();
            log::info!("✓ Logged in as: {} (ID: {})", name, id);
            println!("✓ Logged in as: {} (ID: {})", name, id);
        }
        Ok(Err(e)) => {
            log::warn!("Could not get user info: {:?}", e);
        }
        Err(_) => {
            log::warn!("get_me() timed out after {:?}", config.telegram.get_me_timeout());
        }
    }

    // Initialize Phoenix with retry config
    log::info!("=== Phoenix Channel Integration ===");
    log::info!("  URL: {}", config.phoenix.url);
    log::info!("  Topic: {}", config.phoenix.topic);
    log::info!("  Max retry attempts: {}", config.phoenix.retry.max_attempts);

    let phoenix_result = tokio::time::timeout(
        config.phoenix.connection_timeout(),
        PhoenixBridge::new_with_config(
            &config.phoenix.url,
            &config.phoenix.topic,
            config.phoenix.retry.clone()
        )
    ).await;

    let phoenix = match phoenix_result {
        Ok(Ok(bridge)) => {
            log::info!("✓ Connected to Phoenix Channel");
            println!("✓ Connected to Phoenix Channel");
            Some(bridge)
        }
        Ok(Err(e)) => {
            log::warn!("⚠ Failed to connect to Phoenix: {:?}", e);
            println!("⚠ Phoenix connection failed - continuing in fallback mode");
            None
        }
        Err(_) => {
            log::warn!("⚠ Phoenix connection timed out after {:?}", config.phoenix.connection_timeout());
            println!("⚠ Phoenix connection timed out - continuing in fallback mode");
            None
        }
    };

    // Configure updates stream
    log::info!("Starting update stream...");
    let updates_config = UpdatesConfiguration {
        catch_up: config.telegram.catch_up,
        update_queue_limit: config.telegram.update_queue_limit,
    };
    let mut updates_stream = client.stream_updates(updates, updates_config);
    log::info!("✓ Now online, listening for updates...");
    println!("✓ Online - Press Ctrl+C to stop\n");

    // Main event loop
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                log::info!("Received Ctrl+C, shutting down...");
                break;
            }

            update_result = tokio::time::timeout(
                config.telegram.update_stream_timeout(),
                updates_stream.next()
            ) => {
                match update_result {
                    Err(_) => {
                        log::warn!("Update stream timed out after {:?}", config.telegram.update_stream_timeout());
                        continue;
                    }
                    Ok(Ok(update)) => {
                        // Process update...
                        let telegram_update = process_update(update);

                        // Send to Phoenix with configured timeout
                        if let (Some(phoenix), Some(telegram_update)) = (&phoenix, telegram_update) {
                            let send_result = tokio::time::timeout(
                                config.phoenix.send_timeout(),
                                phoenix.send_update(telegram_update)
                            ).await;

                            if let Err(_) = send_result {
                                log::warn!("Phoenix send timed out after {:?}", config.phoenix.send_timeout());
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        log::error!("Update error: {:?}", e);
                    }
                }
            }
        }
    }

    // Graceful shutdown
    log::info!("Syncing state...");
    if let Err(e) = updates_stream.sync_update_state() {
        log::error!("Failed to sync update state: {:?}", e);
    }

    log::info!("Stopping connections...");
    handle.quit();

    log::info!("Waiting for pool to stop...");
    pool_task.await?;

    log::info!("✓ Shutdown complete");
    println!("✓ Disconnected");

    Ok(())
}
```

## Phoenix Bridge Integration

The `PhoenixBridge` should be updated to accept `RetryConfig`:

**phoenix_bridge.rs changes:**

```rust
use crate::config::RetryConfig;

// Remove the old RetryConfig definition from phoenix_bridge.rs
// and use the one from config module instead

impl PhoenixBridge {
    pub async fn new_with_config(
        url: &str,
        topic: &str,
        retry_config: RetryConfig,  // Now uses config::RetryConfig
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // ... existing implementation
    }
}
```

## Environment Variables Reference

After migration, these environment variables can be used:

```bash
# Session configuration
export SESSION_FILE="session/production.session"

# Phoenix configuration
export PHOENIX_URL="wss://production.example.com/socket"
export PHOENIX_TOPIC="production:updates"
export PHOENIX_CONNECTION_TIMEOUT_SECS="60"
export PHOENIX_SEND_TIMEOUT_SECS="15"

# Retry configuration
export RETRY_INITIAL_DELAY_SECS="2"
export RETRY_MAX_DELAY_SECS="60"
export RETRY_MAX_ATTEMPTS="10"

# Telegram configuration
export TELEGRAM_AUTH_TIMEOUT_SECS="45"
export TELEGRAM_UPDATE_QUEUE_LIMIT="100"
export TELEGRAM_CATCH_UP="true"

# Logging
export LOG_LEVEL="warn"
export LOG_GRAMMERS_LEVEL="info"
```

## Benefits After Migration

1. **Centralized Configuration**: All config in one place
2. **Type Safety**: Durations are properly typed
3. **Validation**: Config is validated at startup
4. **Environment Override**: Easy to override via env vars
5. **Documentation**: Clear documentation of all settings
6. **Testing**: Easy to create test configurations
7. **Maintainability**: Changes to config structure are centralized

## Testing the Migration

After migrating, test with:

```bash
# Test with defaults
cargo run

# Test with custom environment variables
export LOG_LEVEL="debug"
export PHOENIX_URL="wss://test.example.com/socket"
export TELEGRAM_UPDATE_QUEUE_LIMIT="50"
cargo run

# Test configuration validation
export PHOENIX_URL="http://invalid.com"  # Should fail validation
cargo run
```

## Rollback Plan

If issues arise, the old constants can be temporarily restored while debugging:

```rust
// Temporary fallback
const SESSION_FILE: &str = "session/my.session";

// Use old value if config fails
let session_file = config.session.session_file.clone()
    .or_else(|_| SESSION_FILE.to_string());
```

## Next Steps

1. Remove old constants from `main.rs`
2. Update `phoenix_bridge.rs` to use `config::RetryConfig`
3. Add config validation to startup sequence
4. Update documentation
5. Add integration tests for configuration
