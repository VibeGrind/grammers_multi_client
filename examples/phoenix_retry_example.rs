// Example demonstrating the new retry logic in PhoenixBridge
//
// This example shows how to use:
// 1. PhoenixBridge::new() with automatic connection retry
// 2. send_with_retry() for automatic retry of failed sends
// 3. Custom RetryConfig for different retry strategies

use telegram_minimal_client::domain::{TelegramUpdate, UserInfo};
// Note: PhoenixBridge is internal and not exported from the library

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // NOTE: This example is for documentation purposes only.
    // PhoenixBridge is now internal and not exported from the library.
    // The retry logic demonstrated here is integrated into the main application.

    println!("=== Phoenix Retry Logic Examples ===\n");
    println!("Note: PhoenixBridge is internal. See src/main.rs for actual usage.\n");

    /* Example code commented out since PhoenixBridge is internal:

    // Initialize logger
    simple_logger::SimpleLogger::new().init().unwrap();

    // Example 1: Using default retry configuration
    println!("Example 1: Default retry configuration");
    println!("- Initial delay: 1s");
    println!("- Max delay: 30s");
    println!("- Max attempts: 5");
    println!("- Backoff: exponential (1s, 2s, 4s, 8s, 16s, 30s cap)\n");

    let phoenix_url = "ws://localhost:4000/socket/websocket";
    let phoenix_topic = "telegram:updates";

    // This will automatically retry connection with exponential backoff
    let bridge = match PhoenixBridge::new(phoenix_url, phoenix_topic, None).await {
        Ok(b) => {
            println!("✓ Connected successfully!\n");
            b
        }
        Err(e) => {
            println!("✗ Failed to connect after retries: {}\n", e);
            return Err(e);
        }
    };

    // Example 2: Using custom retry configuration
    println!("\nExample 2: Custom retry configuration");
    let custom_config = RetryConfig {
        initial_delay_secs: 2,  // Start with 2s
        max_delay_secs: 60,     // Cap at 60s
        max_attempts: 3,        // Only try 3 times
    };
    println!("- Initial delay: {}s", custom_config.initial_delay_secs);
    println!("- Max delay: {}s", custom_config.max_delay_secs);
    println!("- Max attempts: {}\n", custom_config.max_attempts);

    // This would use custom retry config
    // let bridge = PhoenixBridge::new_with_config(phoenix_url, phoenix_topic, custom_config).await?;

    // Example 3: Sending updates with retry logic
    println!("\nExample 3: Sending updates with retry");

    let update = TelegramUpdate {
        update_type: "new_message".to_string(),
        chat_id: Some(123456),
        message_id: Some(789),
        text: Some("Test message".to_string()),
        from_user: Some(UserInfo {
            id: 111,
            first_name: "Test User".to_string(),
            username: Some("testuser".to_string()),
        }),
        timestamp: Some(1234567890),
        raw_data: None,
    };

    // Method 1: send_update() - Legacy, no retry (just logs errors)
    println!("Method 1: send_update() - fire and forget, no retry");
    bridge.send_update(update.clone()).await;

    // Method 2: send_with_retry() - Recommended, automatic retry with exponential backoff
    println!("Method 2: send_with_retry() - tries immediately, queues on failure");
    bridge.send_with_retry(update.clone()).await;
    println!("  - If send fails, update is queued for background retry");
    println!("  - Retries happen with exponential backoff: 1s, 2s, 4s, 8s, 16s, 30s");
    println!("  - Non-blocking: returns immediately");
    println!("  - Background task handles retries\n");

    // Example 4: How retry queue works
    println!("\nExample 4: Retry queue mechanics");
    println!("When send_with_retry() fails:");
    println!("  1. Update is immediately queued with attempt=1");
    println!("  2. Background task picks it up");
    println!("  3. Waits: initial_delay * 2^(attempt-1), capped at max_delay");
    println!("     - Attempt 1: 1s * 2^0 = 1s");
    println!("     - Attempt 2: 1s * 2^1 = 2s");
    println!("     - Attempt 3: 1s * 2^2 = 4s");
    println!("     - Attempt 4: 1s * 2^3 = 8s");
    println!("     - Attempt 5: 1s * 2^4 = 16s");
    println!("     - Further: capped at 30s");
    println!("  4. If all attempts fail, update is dropped and logged\n");

    // Keep alive to allow background retries to complete
    println!("Keeping connection alive for 60 seconds to demonstrate retries...");
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

    println!("\nDone!");
    */

    Ok(())
}
