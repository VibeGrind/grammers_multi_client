# Storage Layer Architecture

## Overview

The storage layer provides a clean abstraction for session data access, completely separating data persistence concerns from business logic. This architecture allows for easy testing, mocking, and future implementation swapping.

## Architecture

### Core Components

1. **SessionRepository Trait** (`src/storage.rs`)
   - Defines the interface for session data access
   - Allows multiple implementations (SQLite, in-memory, mock, etc.)
   - Thread-safe (`Send + Sync`)

2. **SqliteSessionRepository** (`src/storage.rs`)
   - Concrete implementation using SQLite
   - Handles all SQL queries and database connections
   - Implements proper error handling with `SessionError`

3. **Data Structures**
   - `SessionData`: Complete session information
   - `DeviceInfo`: Device fingerprint details
   - `SessionError`: Strongly-typed errors (from `error.rs`)

## Usage Example

```rust
use storage::{SessionRepository, SqliteSessionRepository};

// Create a repository instance
let repository = SqliteSessionRepository::new("session/my.session");

// Load complete session data
let session_data = repository.load_session_data()?;

// Or load specific components
let api_id = repository.get_api_id()?;
let device_info = repository.get_device_info()?;
let proxy_url = repository.get_proxy_url()?;

// Use the data
println!("API ID: {}", session_data.app_id);
println!("Device: {}", session_data.device_info.device_model);
```

## Benefits

### 1. Separation of Concerns
- **Before**: SQL queries mixed with business logic in `main.rs`
- **After**: All data access isolated in `storage.rs`

### 2. Testability
```rust
// Easy to create mock implementations for testing
struct MockSessionRepository {
    data: SessionData,
}

impl SessionRepository for MockSessionRepository {
    fn load_session_data(&self) -> Result<SessionData, SessionError> {
        Ok(self.data.clone())
    }
    // ... other methods
}
```

### 3. Type Safety
- Uses `SessionError` instead of `Box<dyn Error>`
- Compile-time checking of error types
- Better error messages and handling

### 4. Security
- All SQL queries use parameterized queries (prevents SQL injection)
- Proxy URL validation with detailed error messages
- Input validation at the storage layer

### 5. Maintainability
- Single Responsibility Principle: each component has one job
- Easy to modify storage implementation without touching business logic
- Clear API contract via the trait

## Data Flow

```
main.rs
  └─> SqliteSessionRepository (storage.rs)
       └─> SQLite Database (session/my.session)
            ├─> body table (key-value pairs)
            │    ├─> app_id
            │    ├─> device
            │    ├─> sdk
            │    ├─> app_version
            │    ├─> lang_code
            │    ├─> system_lang_code
            │    └─> proxy (optional)
            └─> JSON parsing and validation
```

## Error Handling

The storage layer uses the `SessionError` enum from `error.rs`:

```rust
pub enum SessionError {
    DatabaseOpen(sqlite::Error),           // Database connection issues
    FileNotFound { path: String },         // Session file missing
    FieldNotFound { field: String },       // Required field missing
    InvalidProxyUrl { reason: String },    // Proxy validation failed
    UnsupportedProxyProtocol,              // Only SOCKS5 supported
    InvalidProxyPort { port: String },     // Port out of range
    ParseError(serde_json::Error),         // JSON parsing failed
    ValidationFailed(String),              // Generic validation error
    TaskJoinError(String),                 // Tokio task join error
    StorageError(String),                  // SQLite storage error
}
```

## Implementation Details

### Proxy URL Validation

The storage layer validates proxy URLs with the following rules:

1. **Protocol**: Must start with `socks5://`
2. **Format**: `socks5://[user:pass@]host:port`
3. **Host**: Cannot be empty
4. **Port**: Must be 1-65535

Examples:
- ✅ `socks5://proxy.example.com:1080`
- ✅ `socks5://user:pass@proxy.example.com:1080`
- ✅ `socks5://127.0.0.1:9050`
- ❌ `http://proxy.example.com:1080` (wrong protocol)
- ❌ `socks5://proxy.example.com` (missing port)
- ❌ `socks5://proxy.example.com:0` (invalid port)

### JSON String Parsing

Values in the SQLite `body` table are stored as JSON-encoded strings:
- The storage layer automatically handles JSON parsing
- Handles escape sequences: `\"`, `\n`, `\t`, `\\`, etc.
- Returns clean, unescaped strings to the caller

### Database Initialization

The repository automatically:
1. Checks if the session file exists
2. Opens the SQLite connection
3. Sets `PRAGMA user_version = 1` for grammers compatibility
4. Prevents table recreation on subsequent opens

## Testing

The module includes comprehensive unit tests:

```bash
cargo test storage
```

Test coverage includes:
- Valid proxy URL formats
- Invalid proxy URLs (protocol, format, port)
- Empty host validation
- Boundary conditions (port 0, 65535)

## Future Enhancements

Potential improvements:
1. **Connection Pooling**: Reuse database connections
2. **Caching**: Cache frequently accessed data
3. **Write Operations**: Add methods to update session data
4. **Alternative Backends**: Redis, PostgreSQL, or cloud storage
5. **Async Operations**: Make trait methods async for better performance

## Migration Guide

### Before (Old Code)

```rust
// main.rs - old approach
fn load_session_data(session_path: &str) -> Result<SessionData, Box<dyn Error>> {
    let conn = Connection::open(session_path)?;
    conn.execute("PRAGMA user_version = 1")?;

    let mut stmt = conn.prepare("SELECT value FROM body WHERE key = 'app_id'")?;
    let app_id = stmt.read::<i64, _>(0)? as i32;

    let device = read_string(&conn, "device")?;
    // ... more direct SQL calls
}
```

### After (New Code)

```rust
// main.rs - new approach
use storage::{SessionRepository, SqliteSessionRepository};

let repository = SqliteSessionRepository::new(SESSION_FILE);
let session_data = repository.load_session_data()?;
```

### Changes Required

1. **Add module declaration**: `mod storage;`
2. **Import types**: `use storage::{SessionRepository, SqliteSessionRepository};`
3. **Create repository**: `SqliteSessionRepository::new(path)`
4. **Load data**: `repository.load_session_data()?`
5. **Update field access**: `session_data.device_info.device_model` instead of `session_data.device`
6. **Remove old functions**: `load_session_data()`, `read_string()`, `validate_proxy_url()`

## API Reference

### Trait: `SessionRepository`

```rust
pub trait SessionRepository: Send + Sync {
    fn load_session_data(&self) -> Result<SessionData, SessionError>;
    fn get_api_id(&self) -> Result<i32, SessionError>;
    fn get_device_info(&self) -> Result<DeviceInfo, SessionError>;
    fn get_proxy_url(&self) -> Result<Option<String>, SessionError>;
}
```

### Struct: `SqliteSessionRepository`

```rust
pub struct SqliteSessionRepository {
    session_path: String,
}

impl SqliteSessionRepository {
    pub fn new(session_path: impl Into<String>) -> Self
}
```

### Struct: `SessionData`

```rust
pub struct SessionData {
    pub app_id: i32,
    pub device_info: DeviceInfo,
    pub proxy: Option<String>,
}
```

### Struct: `DeviceInfo`

```rust
pub struct DeviceInfo {
    pub device_model: String,
    pub sdk: String,
    pub app_version: String,
    pub lang_code: String,
    pub system_lang_code: String,
}
```

## Conclusion

The storage layer abstraction provides a robust, maintainable, and testable approach to session data management. It follows SOLID principles and best practices for Rust development, making the codebase more professional and easier to maintain.
