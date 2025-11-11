/// Example demonstrating the storage layer abstraction
///
/// This example shows how to:
/// 1. Create a session repository
/// 2. Load session data
/// 3. Access individual components
/// 4. Handle errors properly
///
/// Run with: cargo run --example storage_example

use telegram_minimal_client::storage::{SessionRepository, SqliteSessionRepository};
use telegram_minimal_client::error::SessionError;

fn main() -> Result<(), SessionError> {
    println!("=== Storage Layer Example ===\n");

    // Path to your session file
    let session_path = "session/my.session";

    // Create a repository instance
    println!("Creating repository for: {}", session_path);
    let repository = SqliteSessionRepository::new(session_path);

    // Method 1: Load complete session data at once
    println!("\n--- Loading Complete Session Data ---");
    match repository.load_session_data() {
        Ok(session_data) => {
            println!("✓ Session loaded successfully!");
            println!("  API ID: {}", session_data.app_id);
            println!("  Device: {}", session_data.device_info.device_model);
            println!("  SDK: {}", session_data.device_info.sdk);
            println!("  App Version: {}", session_data.device_info.app_version);
            println!("  Language: {}", session_data.device_info.lang_code);
            println!("  System Language: {}", session_data.device_info.system_lang_code);

            if let Some(proxy) = &session_data.proxy {
                println!("  Proxy: {} (validated)", proxy);
            } else {
                println!("  Proxy: None");
            }
        }
        Err(e) => {
            println!("✗ Failed to load session: {}", e);
            return Err(e);
        }
    }

    // Method 2: Load specific components individually
    println!("\n--- Loading Individual Components ---");

    match repository.get_api_id() {
        Ok(api_id) => println!("✓ API ID: {}", api_id),
        Err(e) => println!("✗ Failed to get API ID: {}", e),
    }

    match repository.get_device_info() {
        Ok(device_info) => {
            println!("✓ Device Info:");
            println!("    Model: {}", device_info.device_model);
            println!("    SDK: {}", device_info.sdk);
            println!("    Version: {}", device_info.app_version);
        }
        Err(e) => println!("✗ Failed to get device info: {}", e),
    }

    match repository.get_proxy_url() {
        Ok(Some(proxy)) => println!("✓ Proxy URL: {}", proxy),
        Ok(None) => println!("✓ No proxy configured"),
        Err(e) => println!("✗ Failed to get proxy URL: {}", e),
    }

    // Demonstrate error handling
    println!("\n--- Error Handling Examples ---");

    // Try to load a non-existent session
    let bad_repo = SqliteSessionRepository::new("nonexistent.session");
    match bad_repo.load_session_data() {
        Ok(_) => println!("Unexpected success!"),
        Err(SessionError::FileNotFound { path }) => {
            println!("✓ Correctly caught FileNotFound error for: {}", path);
        }
        Err(e) => println!("Different error: {}", e),
    }

    println!("\n=== Example Complete ===");
    Ok(())
}
