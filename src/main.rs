mod config;
mod database;
mod errors;
mod manager;
mod phoenix_manager;
mod session;
mod session_handle;

use config::ManagerConfig;
use manager::SessionManager;
use std::io::{self, Write};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Инициализация логирования
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .with_module_level("grammers", log::LevelFilter::Debug)
        .init()
        .unwrap();

    println!("=== Telegram Multi-Client Manager ===\n");

    // Загрузить конфигурацию
    let config = ManagerConfig::from_env();

    // Создать SessionManager
    let manager = match SessionManager::new(config).await {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to initialize SessionManager: {}", e);
            std::process::exit(1);
        }
    };

    println!("✓ Manager initialized");
    println!("  Phoenix URL: {}", manager.config().phoenix_url);
    println!("  Sessions dir: {:?}", manager.config().sessions_dir);
    println!("  Storage dir: {:?}", manager.config().storage_dir);
    println!("  Database: {:?}\n", manager.config().database_path);

    // CLI цикл
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            continue;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        let command = parts[0];

        match command {
            "help" => {
                print_help();
            }

            "register" => {
                if parts.len() < 2 {
                    println!("Usage: register <session_name>");
                    continue;
                }
                let session_name = parts[1];
                match manager.register_session(session_name).await {
                    Ok(session_id) => {
                        println!("✓ Session registered: {}", session_id);
                    }
                    Err(e) => {
                        println!("✗ Error: {}", e);
                    }
                }
            }

            "start" => {
                if parts.len() < 2 {
                    println!("Usage: start <session_id>");
                    continue;
                }
                let session_id = parts[1];
                match manager.start_session(session_id).await {
                    Ok(()) => {
                        println!("✓ Session started: {}", session_id);
                    }
                    Err(e) => {
                        println!("✗ Error: {}", e);
                    }
                }
            }

            "stop" => {
                if parts.len() < 2 {
                    println!("Usage: stop <session_id>");
                    continue;
                }
                let session_id = parts[1];
                match manager.stop_session(session_id).await {
                    Ok(()) => {
                        println!("✓ Session stopped: {}", session_id);
                    }
                    Err(e) => {
                        println!("✗ Error: {}", e);
                    }
                }
            }

            "remove" => {
                if parts.len() < 2 {
                    println!("Usage: remove <session_id> [delete_file]");
                    continue;
                }
                let session_id = parts[1];
                let delete_file = parts.get(2).map(|s| *s == "true").unwrap_or(false);
                match manager.remove_session(session_id, delete_file).await {
                    Ok(()) => {
                        println!("✓ Session removed: {}", session_id);
                    }
                    Err(e) => {
                        println!("✗ Error: {}", e);
                    }
                }
            }

            "status" => {
                if parts.len() < 2 {
                    println!("Usage: status <session_id>");
                    continue;
                }
                let session_id = parts[1];
                match manager.get_session_status(session_id).await {
                    Some(status) => {
                        println!("Status for {}: {:?}", session_id, status);
                    }
                    None => {
                        println!("Session not running: {}", session_id);
                    }
                }
            }

            "list" => {
                let active = manager.list_active_sessions().await;
                println!("Active sessions ({}): {:?}", active.len(), active);
            }

            "list-registered" => match manager.list_registered_sessions() {
                Ok(sessions) => {
                    println!("Registered sessions ({}): {:?}", sessions.len(), sessions);
                }
                Err(e) => {
                    println!("✗ Error: {}", e);
                }
            },

            "list-available" => match manager.list_available_sessions() {
                Ok(sessions) => {
                    println!("Available sessions in storage ({}): {:?}", sessions.len(), sessions);
                }
                Err(e) => {
                    println!("✗ Error: {}", e);
                }
            },

            "list-pending" => match manager.list_pending_sessions() {
                Ok(sessions) => {
                    println!(
                        "Pending sessions for registration ({}): {:?}",
                        sessions.len(),
                        sessions
                    );
                }
                Err(e) => {
                    println!("✗ Error: {}", e);
                }
            },

            "exit" | "quit" => {
                println!("Shutting down...");
                // Остановить все активные сессии
                let active = manager.list_active_sessions().await;
                for session_id in active {
                    println!("Stopping session: {}", session_id);
                    if let Err(e) = manager.stop_session(&session_id).await {
                        println!("Error stopping {}: {}", session_id, e);
                    }
                }
                println!("✓ Goodbye!");
                break;
            }

            _ => {
                println!("Unknown command: {}. Type 'help' for available commands.", command);
            }
        }
    }
}

fn print_help() {
    println!("Available commands:");
    println!("  help                          - Show this help");
    println!("  register <name>               - Register session from /sessions");
    println!("  start <id>                    - Start session");
    println!("  stop <id>                     - Stop session");
    println!("  remove <id> [delete_file]     - Remove session (delete_file: true/false)");
    println!("  status <id>                   - Get session status");
    println!("  list                          - List active (running) sessions");
    println!("  list-registered               - List registered sessions in DB");
    println!("  list-available                - List available sessions in /storage");
    println!("  list-pending                  - List pending sessions in /sessions");
    println!("  exit / quit                   - Shutdown and exit");
    println!();
}
