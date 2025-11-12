mod phoenix_bridge;
mod error;
mod config;
mod storage;
mod domain;
mod circuit_breaker;

use std::sync::Arc;
use grammers_client::{Client, Update};
use grammers_mtsender::SenderPool;
use grammers_session::storages::SqliteSession;
use phoenix_bridge::{PhoenixBridge, TelegramUpdate, UserInfo};
use error::*;
use config::AppConfig;
use storage::{SessionRepository, SqliteSessionRepository};
use domain::{ChatId, MessageId, UserId};

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Load and validate configuration from environment variables
    let app_config = AppConfig::from_env();
    app_config.validate()?;

    // Initialize logger with config values
    simple_logger::SimpleLogger::new()
        .with_level(app_config.log.level_filter())
        .with_module_level("grammers", app_config.log.grammers_level_filter())
        .init()
        .expect("Failed to initialize logger - another logger may already be initialized");

    log::info!("=== Telegram Session Client ===");
    if log::log_enabled!(log::Level::Info) {
        log::info!("Loading session: {}", app_config.session.session_file);
    }

    // Load session data using the storage repository (blocking operation in spawn_blocking)
    let session_file = app_config.session.session_file.clone();
    let session_data = tokio::task::spawn_blocking(move || {
        let repository = SqliteSessionRepository::new(session_file);
        repository.load_session_data()
    })
    .await
    .map_err(|e| SessionError::TaskJoinError(e.to_string()))?
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    if log::log_enabled!(log::Level::Info) {
        log::info!("Device fingerprint loaded:");
        log::info!("  Device: {}", session_data.device_info.device_model);
        log::info!("  SDK: {}", session_data.device_info.sdk);
        log::info!("  App version: {}", session_data.device_info.app_version);
        log::info!("  API ID: {}", session_data.app_id);
        log::info!("  Lang code: {}", session_data.device_info.lang_code);
        log::info!("  System lang code: {}", session_data.device_info.system_lang_code);
        if session_data.proxy.is_some() {
            log::info!("  Proxy: REDACTED (SOCKS5)");
        } else {
            log::info!("  Proxy: None (direct connection)");
        }
    }

    // Открываем сессию
    let session = Arc::new(SqliteSession::open(&app_config.session.session_file)?);

    // Create ConnectionParams with all parameters from session
    use grammers_mtsender::ConnectionParams;

    let connection_params = ConnectionParams {
        device_model: session_data.device_info.device_model.clone(),
        system_version: session_data.device_info.sdk.clone(),
        app_version: session_data.device_info.app_version.clone(),
        system_lang_code: session_data.device_info.system_lang_code.clone(),
        lang_code: session_data.device_info.lang_code.clone(),
        #[cfg(feature = "proxy")]
        proxy_url: session_data.proxy.as_ref().map(|p| p.as_str().to_string()),
        __non_exhaustive: (),
    };

    if log::log_enabled!(log::Level::Info) {
        log::info!("=== Connection Parameters ===");
        log::info!("  Device Model: {}", connection_params.device_model);
        log::info!("  System Version: {}", connection_params.system_version);
        log::info!("  App Version: {}", connection_params.app_version);
        log::info!("  Lang Code: {}", connection_params.lang_code);
        log::info!("  System Lang Code: {}", connection_params.system_lang_code);
        #[cfg(feature = "proxy")]
        if connection_params.proxy_url.is_some() {
            log::info!("  Proxy URL: REDACTED");
        }
    }

    // Создаём pool с конфигурацией
    let pool = SenderPool::with_configuration(
        Arc::clone(&session),
        session_data.app_id.as_i32(),
        connection_params,
    );
    let client = Client::new(&pool);

    let SenderPool { runner, handle, updates } = pool;

    // Запускаем pool runner
    log::info!("Starting MTProto connection...");
    let pool_task = tokio::spawn(runner.run());

    // Проверяем авторизацию
    log::info!("Checking authorization...");
    let auth_result = tokio::time::timeout(
        app_config.telegram.auth_timeout(),
        client.is_authorized()
    ).await;

    match auth_result {
        Ok(Ok(true)) => {
            log::info!("✓ Session is authorized");
            println!("✓ Connected to Telegram");
        }
        Ok(Ok(false)) => {
            log::error!("✗ Session is NOT authorized");
            eprintln!("✗ Session not authorized!");
            handle.quit();
            if let Err(e) = pool_task.await {
                log::error!("Pool task panicked during shutdown: {:?}", e);
            }
            return Err("Session not authorized".into());
        }
        Ok(Err(e)) => {
            log::error!("Authorization check failed: {:?}", e);
            handle.quit();
            if let Err(e) = pool_task.await {
                log::error!("Pool task panicked during shutdown: {:?}", e);
            }
            return Err(format!("Authorization check failed: {}", e).into());
        }
        Err(_) => {
            if log::log_enabled!(log::Level::Error) {
                log::error!("Authorization check timed out after {} seconds", app_config.telegram.auth_timeout_secs);
            }
            handle.quit();
            if let Err(e) = pool_task.await {
                log::error!("Pool task panicked during shutdown: {:?}", e);
            }
            return Err("Authorization check timed out".into());
        }
    }

    // Получаем информацию о себе
    let get_me_result = tokio::time::timeout(
        app_config.telegram.get_me_timeout(),
        client.get_me()
    ).await;

    match get_me_result {
        Ok(Ok(user)) => {
            let name = user.first_name().unwrap_or("Unknown");
            let id = user.raw.id();
            if log::log_enabled!(log::Level::Info) {
                log::info!("✓ Logged in as: {} (ID: {})", name, id);
            }
            println!("✓ Logged in as: {} (ID: {})", name, id);
        }
        Ok(Err(e)) => {
            if log::log_enabled!(log::Level::Warn) {
                log::warn!("Could not get user info: {:?}", e);
            }
        }
        Err(_) => {
            if log::log_enabled!(log::Level::Warn) {
                log::warn!("get_me() timed out after {} seconds", app_config.telegram.get_me_timeout_secs);
            }
        }
    }

    // Инициализация Phoenix Channel
    if log::log_enabled!(log::Level::Info) {
        log::info!("=== Phoenix Channel Integration ===");
        log::info!("  URL: {}", app_config.phoenix.url);
        log::info!("  Topic: {}", app_config.phoenix.topic);
    }

    // Prepare circuit breaker configuration if enabled
    let circuit_breaker_config = if app_config.phoenix.enable_circuit_breaker {
        Some(app_config.phoenix.circuit_breaker_config())
    } else {
        None
    };

    let phoenix_result = tokio::time::timeout(
        app_config.phoenix.connection_timeout(),
        phoenix_bridge::PhoenixBridge::new_with_config(
            &app_config.phoenix.url,
            &app_config.phoenix.topic,
            app_config.phoenix.auth_token.as_deref(),
            phoenix_bridge::RetryConfig::default(),
            circuit_breaker_config
        )
    ).await;

    let phoenix = match phoenix_result {
        Ok(Ok(bridge)) => {
            log::info!("✓ Connected to Phoenix Channel");
            println!("✓ Connected to Phoenix Channel");
            Some(bridge)
        }
        Ok(Err(e)) => {
            if log::log_enabled!(log::Level::Warn) {
                log::warn!("⚠ Failed to connect to Phoenix: {:?}", e);
                log::warn!("  Continuing without Phoenix integration");
            }
            println!("⚠ Phoenix connection failed - continuing in fallback mode");
            None
        }
        Err(_) => {
            if log::log_enabled!(log::Level::Warn) {
                log::warn!("⚠ Phoenix connection timed out after {} seconds", app_config.phoenix.connection_timeout_secs);
                log::warn!("  Continuing without Phoenix integration");
            }
            println!("⚠ Phoenix connection timed out - continuing in fallback mode");
            None
        }
    };

    // Настройка для получения обновлений - use config values
    log::info!("Starting update stream...");
    use grammers_client::UpdatesConfiguration;
    let updates_config = UpdatesConfiguration {
        catch_up: app_config.telegram.catch_up,
        update_queue_limit: app_config.telegram.update_queue_limit,
    };
    let mut updates_stream = client.stream_updates(updates, updates_config);
    log::info!("✓ Now online, listening for updates...");
    println!("✓ Online - Press Ctrl+C to stop\n");

    // Buffer for batching Phoenix updates
    let mut update_batch: Vec<TelegramUpdate> = Vec::with_capacity(10);
    let batch_interval = tokio::time::interval(std::time::Duration::from_millis(100));
    tokio::pin!(batch_interval);

    // Основной цикл - batch processing for Phoenix
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                log::info!("Received Ctrl+C, shutting down...");
                println!("\n⚠ Shutting down...");
                break;
            }

            _ = batch_interval.tick() => {
                // Send batched updates to Phoenix
                if !update_batch.is_empty() && phoenix.is_some() {
                    let batch_to_send = std::mem::replace(&mut update_batch, Vec::with_capacity(10));
                    let phoenix_ref = phoenix.as_ref().unwrap();

                    // Spawn task to send batch without blocking
                    let phoenix_clone = phoenix_ref.clone();
                    let send_timeout = app_config.phoenix.send_timeout();
                    tokio::spawn(async move {
                        for telegram_update in batch_to_send {
                            let send_result = tokio::time::timeout(
                                send_timeout,
                                phoenix_clone.send_update(telegram_update)
                            ).await;

                            if send_result.is_err() && log::log_enabled!(log::Level::Warn) {
                                log::warn!("Phoenix send_update timed out");
                            }
                        }
                    });
                }
            }

            update_result = tokio::time::timeout(
                app_config.telegram.update_stream_timeout(),
                updates_stream.next()
            ) => {
                match update_result {
                    Err(_) => {
                        if log::log_enabled!(log::Level::Warn) {
                            log::warn!("Update stream timed out after {} seconds", app_config.telegram.update_stream_timeout_secs);
                        }
                        continue;
                    }
                    Ok(Ok(update)) => {
                        // Process update with minimal allocations
                        // MessageDeleted is special - it can generate multiple updates
                        match &update {
                            Update::NewMessage(msg) => {
                                let text = msg.text();
                                let chat_id_raw = msg.peer_id().bot_api_dialog_id();
                                let message_id_raw = msg.id();

                                if log::log_enabled!(log::Level::Info) {
                                    let peer_name = msg.peer().ok().and_then(|p| p.name()).unwrap_or("");
                                    log::info!("New message from {}: {}", peer_name, text);
                                }

                                // Validate and wrap IDs using domain types
                                let chat_id = ChatId::new(chat_id_raw).ok();
                                let message_id = MessageId::new(message_id_raw).ok();

                                let from_user = msg.sender().and_then(|sender_peer| {
                                    let user_id_raw = sender_peer.id().bot_api_dialog_id();
                                    UserId::new(user_id_raw).ok().map(|user_id| {
                                        // Minimize allocations - only allocate when necessary
                                        let name = sender_peer.name().unwrap_or("");
                                        let username = sender_peer.username();
                                        domain::UserInfo::new(
                                            user_id,
                                            name.to_string(),
                                            username.map(str::to_string),
                                        )
                                    })
                                });

                                // Create update using domain constructor
                                if let Some(telegram_update) = chat_id.and_then(|cid| message_id.map(|mid| {
                                    TelegramUpdate::new_message(
                                        cid,
                                        mid,
                                        text.to_string(),  // Only allocation needed for serialization
                                        from_user,
                                        msg.date().timestamp(),
                                    )
                                })) {
                                    update_batch.push(telegram_update);
                                }
                            }
                            Update::MessageEdited(msg) => {
                                if log::log_enabled!(log::Level::Debug) {
                                    log::debug!("Message edited: {}", msg.text());
                                }

                                let chat_id_raw = msg.peer_id().bot_api_dialog_id();
                                let message_id_raw = msg.id();

                                // Validate and wrap IDs using domain types
                                let chat_id = ChatId::new(chat_id_raw).ok();
                                let message_id = MessageId::new(message_id_raw).ok();

                                // Create update using domain constructor
                                if let Some(telegram_update) = chat_id.and_then(|cid| message_id.map(|mid| {
                                    TelegramUpdate::message_edited(
                                        cid,
                                        mid,
                                        msg.text().to_string(),
                                        msg.date().timestamp(),
                                    )
                                })) {
                                    update_batch.push(telegram_update);
                                }
                            }
                            Update::MessageDeleted(deleted) => {
                                // CRITICAL FIX: Process ALL deleted messages, not just the first one
                                // Previously this was causing data loss by ignoring all but the first message
                                let chat_id_raw = deleted.channel_id();
                                let messages = deleted.messages();

                                if log::log_enabled!(log::Level::Debug) {
                                    log::debug!("Message deleted: channel_id={:?}, count={}, messages={:?}",
                                        chat_id_raw, messages.len(), messages);
                                }

                                // Validate chat_id once
                                let chat_id = chat_id_raw.and_then(|id| ChatId::new(id).ok());

                                // Process EACH deleted message
                                for &msg_id in messages {
                                    if let Some(message_id) = MessageId::new(msg_id).ok() {
                                        // Only format when necessary
                                        let raw_data = if log::log_enabled!(log::Level::Debug) {
                                            format!("deleted message_id={}", msg_id)
                                        } else {
                                            String::from("deleted")
                                        };

                                        // Create update using domain constructor
                                        let telegram_update = TelegramUpdate::message_deleted(
                                            chat_id,
                                            Some(message_id),
                                            raw_data,
                                        );
                                        update_batch.push(telegram_update);
                                    }
                                }
                            }
                            _ => {
                                if log::log_enabled!(log::Level::Debug) {
                                    log::debug!("Other update: {:?}", update);
                                }
                                // Only format debug info when debug logging is enabled
                                let raw_data = if log::log_enabled!(log::Level::Debug) {
                                    format!("{:?}", update)
                                } else {
                                    String::from("other")
                                };
                                update_batch.push(TelegramUpdate::other(raw_data));
                            }
                        }

                        // Check if batch is large enough to send immediately
                        if update_batch.len() >= 10 && phoenix.is_some() {
                            let batch_to_send = std::mem::replace(&mut update_batch, Vec::with_capacity(10));
                            let phoenix_ref = phoenix.as_ref().unwrap();
                            let phoenix_clone = phoenix_ref.clone();
                            let send_timeout = app_config.phoenix.send_timeout();

                            tokio::spawn(async move {
                                for telegram_update in batch_to_send {
                                    let send_result = tokio::time::timeout(
                                        send_timeout,
                                        phoenix_clone.send_update(telegram_update)
                                    ).await;

                                    if send_result.is_err() && log::log_enabled!(log::Level::Warn) {
                                        log::warn!("Phoenix send_update timed out");
                                    }
                                }
                            });
                        }
                    }
                    Ok(Err(e)) => {
                        if log::log_enabled!(log::Level::Error) {
                            log::error!("Update error: {:?}", e);
                        }

                        // Check if reconnection is enabled
                        if app_config.telegram.enable_reconnection {
                            log::warn!("Connection lost, triggering reconnection...");
                            // Return error to trigger reconnection
                            return Err(format!("Update stream error: {:?}", e).into());
                        } else {
                            log::warn!("Reconnection disabled, continuing...");
                        }
                    }
                }
            }
        }
    }

    // Send any remaining batched updates before shutdown
    if !update_batch.is_empty() && phoenix.is_some() {
        let phoenix_ref = phoenix.as_ref().unwrap();
        for telegram_update in update_batch {
            let send_result = tokio::time::timeout(
                app_config.phoenix.send_timeout(),
                phoenix_ref.send_update(telegram_update)
            ).await;

            if send_result.is_err() && log::log_enabled!(log::Level::Warn) {
                log::warn!("Phoenix send_update timed out during shutdown");
            }
        }
    }

    // Graceful shutdown
    log::info!("Syncing state...");
    updates_stream.sync_update_state();
    log::info!("State synced");

    log::info!("Stopping connections...");
    handle.quit();

    log::info!("Waiting for pool to stop...");
    if let Err(e) = pool_task.await {
        log::error!("Pool task panicked during shutdown: {:?}", e);
    }

    log::info!("✓ Shutdown complete");
    println!("✓ Disconnected");

    Ok(())
}

/// Wrapper function that handles reconnection logic
async fn run_with_reconnection() -> Result<(), Box<dyn std::error::Error>> {
    // Load config first to get reconnection settings
    let app_config = AppConfig::from_env();
    app_config.validate()?;

    if !app_config.telegram.enable_reconnection {
        // No reconnection - just run once
        return run().await;
    }

    let max_attempts = app_config.telegram.max_reconnection_attempts;
    let initial_delay = app_config.telegram.reconnection_delay_secs;
    let max_delay = app_config.telegram.max_reconnection_delay_secs;

    let mut attempt = 0;
    let mut delay_secs = initial_delay;

    loop {
        attempt += 1;

        if max_attempts > 0 {
            log::info!("Connection attempt {}/{}", attempt, max_attempts);
        } else {
            log::info!("Connection attempt {} (unlimited retries)", attempt);
        }

        match run().await {
            Ok(_) => {
                log::info!("Program completed successfully");
                return Ok(());
            }
            Err(e) => {
                log::error!("Connection failed: {}", e);

                // Check if we've exceeded max attempts
                if max_attempts > 0 && attempt >= max_attempts {
                    log::error!("Maximum reconnection attempts ({}) reached", max_attempts);
                    return Err(format!("Failed after {} reconnection attempts", max_attempts).into());
                }

                // Log reconnection delay
                log::warn!("Reconnecting in {} seconds...", delay_secs);
                println!("⚠ Connection lost. Reconnecting in {}s... (attempt {}/{})",
                    delay_secs,
                    attempt,
                    if max_attempts == 0 { "∞".to_string() } else { max_attempts.to_string() }
                );

                // Wait before reconnecting
                tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;

                // Exponential backoff with cap
                delay_secs = (delay_secs * 2).min(max_delay);
            }
        }
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    if let Err(e) = run_with_reconnection().await {
        eprintln!("Fatal error: {}", e);
        std::process::exit(1);
    }
}
