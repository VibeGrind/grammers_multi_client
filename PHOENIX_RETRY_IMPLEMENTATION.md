# Phoenix Retry Logic Implementation

## Overview

This document describes the retry logic with exponential backoff implementation for Phoenix operations in `src/phoenix_bridge.rs`.

## What Was Implemented

### 1. Retry Configuration (`RetryConfig`)

A configurable struct for controlling retry behavior:

```rust
pub struct RetryConfig {
    pub initial_delay_secs: u64,  // Starting delay (default: 1s)
    pub max_delay_secs: u64,      // Maximum delay cap (default: 30s)
    pub max_attempts: u32,        // Maximum retry attempts (default: 5)
}
```

**Default values:**
- Initial delay: 1 second
- Max delay: 30 seconds
- Max attempts: 5

### 2. Generic Retry Helper (`retry_with_backoff`)

A generic async function that can retry any operation with exponential backoff:

```rust
async fn retry_with_backoff<F, Fut, T, E>(
    operation: F,
    config: RetryConfig,
    operation_name: &str,
) -> Result<T, E>
```

**Features:**
- Accepts any async closure/future
- Implements exponential backoff: delay doubles on each retry
- Caps delay at `max_delay_secs`
- Logs all retry attempts with clear messages
- Returns error after `max_attempts` failures

**Exponential backoff sequence (with defaults):**
- Attempt 1: immediate
- Attempt 2: 1s delay (1 × 2^0)
- Attempt 3: 2s delay (1 × 2^1)
- Attempt 4: 4s delay (1 × 2^2)
- Attempt 5: 8s delay (1 × 2^3)
- Attempt 6+: 30s delay (capped)

### 3. Connection Retry in `PhoenixBridge::new()`

The connection logic now includes automatic retry:

```rust
// Old behavior:
PhoenixBridge::new(url, topic).await  // Failed immediately on error

// New behavior:
PhoenixBridge::new(url, topic).await  // Retries 5 times with backoff
```

**What gets retried:**
- Phoenix websocket connection
- Channel join operation

**Benefits:**
- Handles transient network issues
- Works around temporary Phoenix server unavailability
- No code changes needed in existing usage

### 4. Retry Queue for Failed Sends

A background retry system for send operations:

**Components:**
- `retry_queue_tx`: Channel sender (part of PhoenixBridge)
- `retry_queue_rx`: Channel receiver (owned by background task)
- `process_retry_queue()`: Background task that processes failed sends
- `QueuedUpdate`: Internal struct tracking update + attempt count

**How it works:**
1. Failed send is queued with attempt number
2. Background task picks up queued updates
3. Waits with exponential backoff before retry
4. Retries up to `max_attempts` times
5. Drops update after max attempts (with error log)

### 5. New Method: `send_with_retry()`

A non-blocking send method with automatic retry:

```rust
pub async fn send_with_retry(&self, update: TelegramUpdate)
```

**Behavior:**
1. Tries to send immediately
2. On success: returns immediately
3. On failure:
   - Logs warning
   - Queues update for retry
   - Returns immediately (non-blocking)
   - Background task handles retries

**Key features:**
- Non-blocking: doesn't wait for retries
- Efficient: doesn't block main update processing
- Automatic: retries happen in background
- Resilient: survives temporary failures

### 6. Updated Methods

**`send_update()` (legacy)**
- Kept for backward compatibility
- Marked as "Legacy method" in docs
- Still works exactly as before
- Does NOT use retry queue
- Just logs errors

**`send_update_with_confirmation()` (unchanged)**
- No changes to this method
- Still waits for server confirmation
- No automatic retry

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ PhoenixBridge::new()                                        │
│   ├─ retry_with_backoff(connect + join)                    │
│   │    └─ Retries: 1s, 2s, 4s, 8s, 16s delays             │
│   └─ Spawns background task: process_retry_queue()         │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ PhoenixBridge                                               │
│   ├─ channel: Arc<Channel>                                 │
│   ├─ topic: String                                          │
│   ├─ retry_queue_tx: mpsc::UnboundedSender<QueuedUpdate>   │
│   └─ retry_config: RetryConfig                             │
└─────────────────────────────────────────────────────────────┘
                           │
            ┌──────────────┴──────────────┐
            │                             │
            ▼                             ▼
┌────────────────────┐      ┌──────────────────────────┐
│ send_update()      │      │ send_with_retry()        │
│ (legacy)           │      │ (recommended)            │
│                    │      │                          │
│ Try send           │      │ 1. Try send immediately  │
│ Log on error       │      │ 2. On error: queue       │
│ Return             │      │ 3. Return (non-blocking) │
└────────────────────┘      └──────────────────────────┘
                                       │
                                       ▼
                          ┌─────────────────────────┐
                          │ retry_queue_tx.send()   │
                          └─────────────────────────┘
                                       │
                                       ▼
                          ┌─────────────────────────────────┐
                          │ Background Task                 │
                          │ process_retry_queue()           │
                          │                                 │
                          │ Loop:                           │
                          │   1. Receive queued update      │
                          │   2. Wait with exp backoff      │
                          │   3. Try send                   │
                          │   4. Success: log & continue    │
                          │   5. Failure: requeue or drop   │
                          └─────────────────────────────────┘
```

## Migration Guide

### For Existing Code

**No changes required!** The implementation is backward compatible:

```rust
// This still works exactly as before
let bridge = PhoenixBridge::new(url, topic).await?;
bridge.send_update(update).await;
```

**But now with automatic connection retry:**
- `PhoenixBridge::new()` will retry up to 5 times
- Connection failures are handled gracefully

### Recommended Migration

For better reliability, migrate to `send_with_retry()`:

**Before:**
```rust
// Old: just logs errors, no retry
bridge.send_update(update).await;
```

**After:**
```rust
// New: automatic retry with exponential backoff
bridge.send_with_retry(update).await;
```

**Benefits:**
- Automatic retry on failure
- Non-blocking (won't slow down processing)
- Exponential backoff prevents overwhelming the server
- Updates survive temporary network issues

### Using Custom Retry Configuration

```rust
use phoenix_bridge::{PhoenixBridge, RetryConfig};

// Create custom config
let config = RetryConfig {
    initial_delay_secs: 2,   // Start with 2s delay
    max_delay_secs: 60,      // Cap at 60s
    max_attempts: 10,        // Try up to 10 times
};

// Use custom config
let bridge = PhoenixBridge::new_with_config(url, topic, config).await?;

// Sends will use the custom retry config
bridge.send_with_retry(update).await;
```

## Logging

The implementation provides comprehensive logging:

### Connection Retry Logs

```
INFO Phoenix connection: Attempt 1/5
INFO Connecting to Phoenix Channel: ws://...
INFO Joining channel: telegram:updates
INFO Successfully joined Phoenix channel

// On failure:
WARN Phoenix connection: Attempt 1 failed: connection refused. Retrying in 1s...
INFO Phoenix connection: Attempt 2/5
...

// After success:
INFO Phoenix connection: Succeeded on attempt 3

// After all failures:
ERROR Phoenix connection: Failed after 5 attempts. Last error: ...
```

### Send Retry Logs

```
// Immediate send failure:
WARN Failed to send update immediately: connection closed. Queueing for retry...

// Background retry:
INFO Retrying queued update (attempt 1/5), waiting 1s...
INFO Successfully sent queued update on attempt 1

// Or on continued failure:
WARN Retry attempt 2 failed: connection closed. Requeueing...
INFO Retrying queued update (attempt 3/5), waiting 4s...

// After max attempts:
ERROR Failed to send update after 5 attempts. Dropping update. Last error: ...
```

## Performance Considerations

### Non-Blocking Design

- `send_with_retry()` returns immediately
- Retries happen in background task
- Main update processing never blocks on retries

### Memory Usage

- Unbounded channel for retry queue
- Each queued update clones the TelegramUpdate struct
- Consider: Updates are relatively small (~100-1KB each)
- Risk: If Phoenix is down for extended period, queue grows

### Potential Improvements

If memory is a concern, consider:

1. **Bounded channel:** Replace unbounded with bounded
   ```rust
   let (tx, rx) = mpsc::channel(100); // Max 100 queued updates
   ```

2. **TTL for queued updates:** Drop old updates
   ```rust
   struct QueuedUpdate {
       update: TelegramUpdate,
       attempt: u32,
       queued_at: Instant,  // Add timestamp
   }
   ```

3. **Backpressure:** Drop new updates if queue is full
   ```rust
   if retry_queue_tx.try_send(queued).is_err() {
       log::warn!("Retry queue full, dropping update");
   }
   ```

## Testing

See `examples/phoenix_retry_example.rs` for usage examples.

### Manual Testing

1. Start Phoenix server
2. Run the example: `cargo run --example phoenix_retry_example`
3. Observe retry logs

To test retry logic:
1. Start without Phoenix server
2. Observe connection retry attempts
3. Start Phoenix server mid-retry
4. Observe successful connection

To test send retry:
1. Connect successfully
2. Stop Phoenix server
3. Send updates with `send_with_retry()`
4. Observe queue building up
5. Restart Phoenix server
6. Observe queued updates being sent

## Error Handling

### Connection Errors

All connection errors are retried automatically:
- Network unreachable
- Connection refused
- DNS resolution failures
- Timeout errors

After max attempts, error is returned to caller.

### Send Errors

With `send_with_retry()`:
- Serialization errors: logged, not queued
- Network errors: queued for retry
- After max attempts: logged and dropped

With `send_update()` (legacy):
- All errors: logged only, no retry

## Summary of Changes

### Files Modified
- `src/phoenix_bridge.rs` - Complete retry implementation

### New Types
- `RetryConfig` - Configurable retry parameters
- `QueuedUpdate` - Internal struct for retry queue

### New Functions
- `retry_with_backoff()` - Generic retry helper
- `PhoenixBridge::new_with_config()` - Custom retry config
- `PhoenixBridge::send_with_retry()` - Send with auto-retry
- `PhoenixBridge::process_retry_queue()` - Background retry task
- `PhoenixBridge::try_send_update()` - Helper for retryable send

### Modified Functions
- `PhoenixBridge::new()` - Now uses retry logic
- `PhoenixBridge` struct - Added retry_queue_tx and retry_config fields
- `Clone` impl - Updated to include new fields

### Traits Added
- `Clone` for `TelegramUpdate` (required for queue)
- `Clone` for `UserInfo` (required for TelegramUpdate clone)

### Backward Compatibility
- ✅ 100% backward compatible
- ✅ All existing code works unchanged
- ✅ New features are opt-in via `send_with_retry()`

## Configuration Recommendations

### Production Settings

```rust
RetryConfig {
    initial_delay_secs: 1,    // Quick retry for transient issues
    max_delay_secs: 30,       // Don't wait too long
    max_attempts: 5,          // Reasonable retry count
}
```

### High-Reliability Settings

```rust
RetryConfig {
    initial_delay_secs: 2,    // Slightly more patient
    max_delay_secs: 60,       // Willing to wait longer
    max_attempts: 10,         // More retry attempts
}
```

### Fast-Fail Settings

```rust
RetryConfig {
    initial_delay_secs: 1,    // Quick attempts
    max_delay_secs: 10,       // Give up faster
    max_attempts: 3,          // Fewer attempts
}
```

## Conclusion

The retry logic implementation provides:
- ✅ Automatic connection retry with exponential backoff
- ✅ Non-blocking retry queue for failed sends
- ✅ Configurable retry parameters
- ✅ Comprehensive logging
- ✅ Backward compatibility
- ✅ Production-ready error handling

The implementation efficiently handles transient failures without blocking the main update processing loop, making the Phoenix bridge more resilient and reliable.
