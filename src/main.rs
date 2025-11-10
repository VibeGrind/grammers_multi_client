mod phoenix_bridge;

use std::sync::Arc;
use grammers_client::{Client, Update};
use grammers_mtsender::SenderPool;
use grammers_session::storages::SqliteSession;
use sqlite::Connection;
use phoenix_bridge::{PhoenixBridge, TelegramUpdate, UserInfo};

const SESSION_FILE: &str = "session/my.session";

// Phoenix Channel configuration (from environment variables)
const PHOENIX_URL_ENV: &str = "PHOENIX_URL";
const PHOENIX_TOPIC_ENV: &str = "PHOENIX_TOPIC";
const DEFAULT_PHOENIX_URL: &str = "ws://localhost:4000/socket";
const DEFAULT_PHOENIX_TOPIC: &str = "telegram:updates";

// Структура для хранения данных из body таблицы
struct SessionData {
    app_id: i32,
    device: String,
    sdk: String,
    app_version: String,
    lang_code: String,
    system_lang_code: String,
    proxy: Option<String>,
}

fn load_session_data(session_path: &str) -> Result<SessionData, Box<dyn std::error::Error + Send + Sync>> {
    use sqlite::State;

    let conn = Connection::open(session_path)?;

    // ВАЖНО: Устанавливаем user_version = 1 для существующей grammers сессии
    // Это предотвращает повторное создание таблиц при SqliteSession::open()
    conn.execute("PRAGMA user_version = 1")?;

    // Читаем app_id
    let mut stmt = conn.prepare("SELECT value FROM body WHERE key = 'app_id'")?;
    let app_id: i32 = if let State::Row = stmt.next()? {
        stmt.read::<i64, _>(0)? as i32
    } else {
        return Err("app_id not found".into());
    };

    // Читаем остальные поля
    let device = read_string(&conn, "device")?;
    let sdk = read_string(&conn, "sdk")?;
    let app_version = read_string(&conn, "app_version")?;
    let lang_code = read_string(&conn, "lang_code")?;
    let system_lang_code = read_string(&conn, "system_lang_code")?;

    // Читаем proxy (опционально) - уже в формате socks5://login:password@ip:port
    let proxy = read_string(&conn, "proxy").ok().and_then(|proxy_str| {
        if proxy_str.is_empty() {
            None
        } else {
            Some(proxy_str)
        }
    });

    Ok(SessionData {
        app_id,
        device,
        sdk,
        app_version,
        lang_code,
        system_lang_code,
        proxy,
    })
}

fn read_string(conn: &Connection, key: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use sqlite::State;

    let query = format!("SELECT value FROM body WHERE key = '{}'", key);
    let mut stmt = conn.prepare(&query)?;

    let value = if let State::Row = stmt.next()? {
        stmt.read::<String, _>(0)?
    } else {
        return Err(format!("{} not found", key).into());
    };

    // Убираем кавычки из JSON строки
    let cleaned = value.trim_matches('"').to_string();
    Ok(cleaned)
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Включаем логи библиотеки
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .with_module_level("grammers", log::LevelFilter::Debug)
        .init()
        .unwrap();

    log::info!("=== Telegram Session Client ===");
    log::info!("Loading session: {}", SESSION_FILE);

    // Читаем данные из body таблицы (блокирующая операция в spawn_blocking)
    let session_file = SESSION_FILE.to_string();
    let session_data = tokio::task::spawn_blocking(move || {
        load_session_data(&session_file)
    })
    .await
    .map_err(|e| format!("Join error: {}", e))?
    .map_err(|e| format!("Session load error: {}", e))?;
    log::info!("Device fingerprint loaded:");
    log::info!("  Device: {}", session_data.device);
    log::info!("  SDK: {}", session_data.sdk);
    log::info!("  App version: {}", session_data.app_version);
    log::info!("  API ID: {}", session_data.app_id);
    log::info!("  Lang code: {}", session_data.lang_code);
    log::info!("  System lang code: {}", session_data.system_lang_code);
    if let Some(ref proxy) = session_data.proxy {
        log::info!("  Proxy: {} (SOCKS5)", proxy.split('@').nth(1).unwrap_or("***"));
    } else {
        log::info!("  Proxy: None (direct connection)");
    }

    // Открываем сессию
    let session = Arc::new(SqliteSession::open(SESSION_FILE)?);

    // Создаём ConnectionParams со всеми параметрами из сессии
    use grammers_mtsender::ConnectionParams;

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

    log::info!("=== Connection Parameters ===");
    log::info!("  Device Model: {}", connection_params.device_model);
    log::info!("  System Version: {}", connection_params.system_version);
    log::info!("  App Version: {}", connection_params.app_version);
    log::info!("  Lang Code: {}", connection_params.lang_code);
    log::info!("  System Lang Code: {}", connection_params.system_lang_code);
    #[cfg(feature = "proxy")]
    if let Some(ref proxy) = connection_params.proxy_url {
        log::info!("  Proxy URL: {}", proxy.split('@').nth(1).unwrap_or("***"));
    }

    // Создаём pool с конфигурацией
    let pool = SenderPool::with_configuration(
        Arc::clone(&session),
        session_data.app_id,
        connection_params,
    );
    let client = Client::new(&pool);

    let SenderPool { runner, handle, updates } = pool;

    // Запускаем pool runner
    log::info!("Starting MTProto connection...");
    let pool_task = tokio::spawn(runner.run());

    // Проверяем авторизацию
    log::info!("Checking authorization...");
    match client.is_authorized().await {
        Ok(true) => {
            log::info!("✓ Session is authorized");
            println!("✓ Connected to Telegram");
        }
        Ok(false) => {
            log::error!("✗ Session is NOT authorized");
            eprintln!("✗ Session not authorized!");
            handle.quit();
            let _ = pool_task.await;
            return Err("Session not authorized".into());
        }
        Err(e) => {
            log::error!("Authorization check failed: {:?}", e);
            handle.quit();
            let _ = pool_task.await;
            return Err(format!("Authorization check failed: {}", e).into());
        }
    }

    // Получаем информацию о себе
    match client.get_me().await {
        Ok(user) => {
            let name = user.first_name().unwrap_or("Unknown");
            let id = user.raw.id();
            log::info!("✓ Logged in as: {} (ID: {})", name, id);
            println!("✓ Logged in as: {} (ID: {})", name, id);
        }
        Err(e) => {
            log::warn!("Could not get user info: {:?}", e);
        }
    }

    // Инициализация Phoenix Channel
    let phoenix_url = std::env::var(PHOENIX_URL_ENV)
        .unwrap_or_else(|_| DEFAULT_PHOENIX_URL.to_string());
    let phoenix_topic = std::env::var(PHOENIX_TOPIC_ENV)
        .unwrap_or_else(|_| DEFAULT_PHOENIX_TOPIC.to_string());

    log::info!("=== Phoenix Channel Integration ===");
    log::info!("  URL: {}", phoenix_url);
    log::info!("  Topic: {}", phoenix_topic);

    let phoenix = match PhoenixBridge::new(&phoenix_url, &phoenix_topic).await {
        Ok(bridge) => {
            log::info!("✓ Connected to Phoenix Channel");
            println!("✓ Connected to Phoenix Channel");
            Some(bridge)
        }
        Err(e) => {
            log::warn!("⚠ Failed to connect to Phoenix: {:?}", e);
            log::warn!("  Continuing without Phoenix integration");
            println!("⚠ Phoenix connection failed - continuing in fallback mode");
            None
        }
    };

    // Настройка для получения обновлений (минимальный буфер для экономии памяти)
    log::info!("Starting update stream...");
    use grammers_client::UpdatesConfiguration;
    let config = UpdatesConfiguration {
        catch_up: true,
        update_queue_limit: Some(10),  // Уменьшено с 100 до 10 для экономии памяти
    };
    let mut updates_stream = client.stream_updates(updates, config);
    log::info!("✓ Now online, listening for updates...");
    println!("✓ Online - Press Ctrl+C to stop\n");

    // Основной цикл - строго асинхронная обработка с прямой отправкой в Phoenix
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                log::info!("Received Ctrl+C, shutting down...");
                println!("\n⚠ Shutting down...");
                break;
            }

            update = updates_stream.next() => {
                match update {
                    Ok(update) => {
                        // Создаём TelegramUpdate для отправки в Phoenix
                        let telegram_update = match &update {
                            Update::NewMessage(msg) => {
                                let text = msg.text();
                                let chat_id = msg.peer_id().bot_api_dialog_id();
                                let message_id = msg.id();
                                let peer = msg.peer().ok().and_then(|p| p.name()).unwrap_or_default();

                                log::info!("New message from {}: {}", peer, text);

                                Some(TelegramUpdate {
                                    update_type: "new_message".to_string(),
                                    chat_id: Some(chat_id),
                                    message_id: Some(message_id),
                                    text: Some(text.to_string()),
                                    from_user: msg.sender().map(|sender_peer| UserInfo {
                                        id: sender_peer.id().bot_api_dialog_id(),
                                        first_name: sender_peer.name().unwrap_or("").to_string(),
                                        username: sender_peer.username().map(String::from),
                                    }),
                                    timestamp: Some(msg.date().timestamp()),
                                    raw_data: None,
                                })
                            }
                            Update::MessageEdited(msg) => {
                                log::debug!("Message edited: {}", msg.text());
                                Some(TelegramUpdate {
                                    update_type: "message_edited".to_string(),
                                    chat_id: Some(msg.peer_id().bot_api_dialog_id()),
                                    message_id: Some(msg.id()),
                                    text: Some(msg.text().to_string()),
                                    from_user: None,
                                    timestamp: Some(msg.date().timestamp()),
                                    raw_data: None,
                                })
                            }
                            Update::MessageDeleted(deleted) => {
                                let chat_id = deleted.channel_id();
                                log::debug!("Message deleted: channel_id={:?}, messages={:?}", chat_id, deleted.messages());
                                Some(TelegramUpdate {
                                    update_type: "message_deleted".to_string(),
                                    chat_id,
                                    message_id: deleted.messages().first().copied(),
                                    text: None,
                                    from_user: None,
                                    timestamp: None,
                                    raw_data: Some(format!("deleted_messages: {:?}", deleted.messages())),
                                })
                            }
                            _ => {
                                log::debug!("Other update: {:?}", update);
                                Some(TelegramUpdate {
                                    update_type: "other".to_string(),
                                    chat_id: None,
                                    message_id: None,
                                    text: None,
                                    from_user: None,
                                    timestamp: None,
                                    raw_data: Some(format!("{:?}", update)),
                                })
                            }
                        };

                        // Прямая отправка в Phoenix без буферизации (fire-and-forget)
                        if let (Some(phoenix), Some(telegram_update)) = (&phoenix, telegram_update) {
                            phoenix.send_update(telegram_update).await;
                        }
                    }
                    Err(e) => {
                        log::error!("Update error: {:?}", e);
                    }
                }
            }
        }
    }

    // Graceful shutdown
    log::info!("Syncing state...");
    updates_stream.sync_update_state();

    log::info!("Stopping connections...");
    handle.quit();

    log::info!("Waiting for pool to stop...");
    let _ = pool_task.await;

    log::info!("✓ Shutdown complete");
    println!("✓ Disconnected");

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Fatal error: {}", e);
        std::process::exit(1);
    }
}
