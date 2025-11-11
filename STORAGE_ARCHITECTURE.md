# Storage Layer Architecture Diagram

## Component Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         Application Layer                        │
│                          (main.rs)                              │
│                                                                  │
│  ┌────────────────────────────────────────────────────────┐   │
│  │                                                          │   │
│  │  let repository = SqliteSessionRepository::new(path);   │   │
│  │  let session_data = repository.load_session_data()?;    │   │
│  │                                                          │   │
│  │  // Use session_data for Telegram connection           │   │
│  │                                                          │   │
│  └────────────────────────────────────────────────────────┘   │
│                               │                                  │
│                               │ Uses trait interface             │
│                               ▼                                  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                │
┌─────────────────────────────────────────────────────────────────┐
│                    Storage Abstraction Layer                     │
│                        (storage.rs)                             │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │         trait SessionRepository                           │  │
│  │  ┌──────────────────────────────────────────────────┐   │  │
│  │  │  + load_session_data() -> Result<SessionData>    │   │  │
│  │  │  + get_api_id() -> Result<i32>                   │   │  │
│  │  │  + get_device_info() -> Result<DeviceInfo>       │   │  │
│  │  │  + get_proxy_url() -> Result<Option<String>>     │   │  │
│  │  └──────────────────────────────────────────────────┘   │  │
│  └──────────────────────────────────────────────────────────┘  │
│                          ▲                                       │
│                          │ implements                            │
│                          │                                       │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │    struct SqliteSessionRepository                         │  │
│  │  ┌──────────────────────────────────────────────────┐   │  │
│  │  │  - session_path: String                          │   │  │
│  │  │                                                   │   │  │
│  │  │  + new(path) -> Self                             │   │  │
│  │  │  - connect() -> Connection                       │   │  │
│  │  │  - read_string() -> String                       │   │  │
│  │  │  - validate_proxy_url() -> String                │   │  │
│  │  └──────────────────────────────────────────────────┘   │  │
│  └──────────────────────────────────────────────────────────┘  │
│                          │                                       │
│                          │ Uses                                  │
│                          ▼                                       │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Data Structures                              │  │
│  │  ┌──────────────────────────────────────────────────┐   │  │
│  │  │  struct SessionData {                            │   │  │
│  │  │    app_id: i32,                                  │   │  │
│  │  │    device_info: DeviceInfo,                      │   │  │
│  │  │    proxy: Option<String>,                        │   │  │
│  │  │  }                                               │   │  │
│  │  └──────────────────────────────────────────────────┘   │  │
│  │  ┌──────────────────────────────────────────────────┐   │  │
│  │  │  struct DeviceInfo {                             │   │  │
│  │  │    device_model: String,                         │   │  │
│  │  │    sdk: String,                                  │   │  │
│  │  │    app_version: String,                          │   │  │
│  │  │    lang_code: String,                            │   │  │
│  │  │    system_lang_code: String,                     │   │  │
│  │  │  }                                               │   │  │
│  │  └──────────────────────────────────────────────────┘   │  │
│  └──────────────────────────────────────────────────────────┘  │
│                          │                                       │
│                          │ Uses                                  │
│                          ▼                                       │
└─────────────────────────────────────────────────────────────────┘
                           │
                           │
┌─────────────────────────────────────────────────────────────────┐
│                     Error Handling Layer                         │
│                        (error.rs)                               │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │         enum SessionError                                 │  │
│  │  ┌──────────────────────────────────────────────────┐   │  │
│  │  │  DatabaseOpen(sqlite::Error)                     │   │  │
│  │  │  FileNotFound { path: String }                   │   │  │
│  │  │  FieldNotFound { field: String }                 │   │  │
│  │  │  InvalidProxyUrl { reason: String }              │   │  │
│  │  │  UnsupportedProxyProtocol                        │   │  │
│  │  │  InvalidProxyPort { port: String }               │   │  │
│  │  │  ParseError(serde_json::Error)                   │   │  │
│  │  │  ValidationFailed(String)                        │   │  │
│  │  │  TaskJoinError(String)                           │   │  │
│  │  │  StorageError(String)                            │   │  │
│  │  └──────────────────────────────────────────────────┘   │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                           │
                           │
┌─────────────────────────────────────────────────────────────────┐
│                      Database Layer                              │
│                  (SQLite - session file)                        │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                  body table                               │  │
│  │  ┌──────────────┬─────────────────────────────────────┐ │  │
│  │  │     key      │            value                     │ │  │
│  │  ├──────────────┼─────────────────────────────────────┤ │  │
│  │  │  app_id      │  123456 (integer)                   │ │  │
│  │  │  device      │  "Samsung Galaxy S21" (JSON string) │ │  │
│  │  │  sdk         │  "Android 11" (JSON string)         │ │  │
│  │  │  app_version │  "8.4.0" (JSON string)              │ │  │
│  │  │  lang_code   │  "en" (JSON string)                 │ │  │
│  │  │  system_lang │  "en-US" (JSON string)              │ │  │
│  │  │  proxy       │  "socks5://..." (JSON string)       │ │  │
│  │  └──────────────┴─────────────────────────────────────┘ │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Data Flow Diagram

### Loading Session Data

```
User Request
    │
    ▼
┌────────────────────────────────────────┐
│ main.rs                                │
│ tokio::spawn_blocking(|| {             │
│   let repo = SqliteSessionRepository   │
│   repo.load_session_data()             │
│ })                                     │
└────────────────────────────────────────┘
    │
    ▼
┌────────────────────────────────────────┐
│ SessionRepository::load_session_data() │
│ (trait method)                         │
└────────────────────────────────────────┘
    │
    ▼
┌────────────────────────────────────────┐
│ SqliteSessionRepository                │
│   ├─→ get_api_id()                    │
│   ├─→ get_device_info()               │
│   └─→ get_proxy_url()                 │
└────────────────────────────────────────┘
    │
    ├──────────────────┬──────────────────┐
    ▼                  ▼                  ▼
┌─────────┐    ┌──────────────┐   ┌──────────────┐
│ connect │    │ read_string  │   │ validate_    │
│   ()    │───▶│   (conn,key) │   │ proxy_url()  │
└─────────┘    └──────────────┘   └──────────────┘
    │                  │                  │
    ▼                  ▼                  ▼
┌──────────────────────────────────────────────┐
│         SQLite Database                      │
│  SELECT value FROM body WHERE key = ?        │
│  (Parameterized query - SQL injection safe)  │
└──────────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────────┐
│  JSON Parsing & Validation                   │
│  serde_json::from_str(&value)?               │
└──────────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────────┐
│  SessionData { app_id, device_info, proxy }  │
└──────────────────────────────────────────────┘
    │
    ▼
┌────────────────────────────────────────┐
│ Return to main.rs                      │
│ Use for Telegram connection            │
└────────────────────────────────────────┘
```

## Error Propagation

```
SQLite Error
    │
    ▼
SessionError::DatabaseOpen
    │
    ▼
Result<T, SessionError>
    │
    ▼
main.rs (handles error)
    │
    ├─→ Log error
    ├─→ Display user message
    └─→ Exit gracefully
```

## Security Features

```
┌─────────────────────────────────────────────┐
│         User Input (Proxy URL)              │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│      validate_proxy_url(proxy_str)          │
│  ┌───────────────────────────────────────┐ │
│  │ 1. Check protocol (socks5:// only)    │ │
│  │ 2. Validate format (host:port)        │ │
│  │ 3. Validate host (not empty)          │ │
│  │ 4. Validate port (1-65535)            │ │
│  │ 5. Return validated URL               │ │
│  └───────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
                    │
                    ├─→ ✅ Valid: Return URL
                    │
                    └─→ ❌ Invalid: Return Error
                            │
                            ▼
                ┌──────────────────────────┐
                │ SessionError::           │
                │ - UnsupportedProtocol   │
                │ - InvalidProxyUrl       │
                │ - InvalidProxyPort      │
                └──────────────────────────┘
```

## Extensibility Model

```
┌─────────────────────────────────────────────┐
│      trait SessionRepository                │
│      (Interface/Contract)                   │
└─────────────────────────────────────────────┘
         ▲           ▲           ▲
         │           │           │
         │           │           │
    ┌────┴────┐ ┌────┴────┐ ┌────┴────┐
    │ SQLite  │ │  Redis  │ │  Mock   │
    │  Impl   │ │  Impl   │ │  Impl   │
    └─────────┘ └─────────┘ └─────────┘
         │           │           │
         ▼           ▼           ▼
    ┌─────────────────────────────────┐
    │   All return same types:        │
    │   Result<SessionData>           │
    │   Result<DeviceInfo>            │
    │   Result<Option<String>>        │
    └─────────────────────────────────┘
```

## Testing Strategy

```
┌─────────────────────────────────────────────┐
│            Unit Tests                       │
│  ┌───────────────────────────────────────┐ │
│  │ test_validate_proxy_url_valid()       │ │
│  │ test_validate_proxy_url_invalid()     │ │
│  │ test_validate_proxy_url_incomplete()  │ │
│  │ test_validate_proxy_url_bad_port()    │ │
│  └───────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│         Integration Tests                   │
│  ┌───────────────────────────────────────┐ │
│  │ Create test SQLite database           │ │
│  │ Test SqliteSessionRepository          │ │
│  │ Verify data loading                   │ │
│  └───────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│         Mock Testing                        │
│  ┌───────────────────────────────────────┐ │
│  │ struct MockSessionRepository          │ │
│  │ impl SessionRepository                │ │
│  │ Test business logic without DB        │ │
│  └───────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
```

## Performance Considerations

```
Current: Sequential Reads
┌─────┐    ┌─────┐    ┌─────┐    ┌─────┐
│ API │ -> │ Dev │ -> │ SDK │ -> │Proxy│
│ ID  │    │ ice │    │     │    │     │
└─────┘    └─────┘    └─────┘    └─────┘
  10ms       10ms       10ms       10ms
                Total: 40ms

Future: Parallel Reads (with connection pool)
┌─────┐
│ API │ \
│ ID  │  \
└─────┘   \     ┌────────────┐
           \    │   Merge    │
┌─────┐     ├──▶│  Results   │
│ Dev │────/    └────────────┘
│ ice │    /          │
└─────┘   /           ▼
         /      SessionData
┌─────┐ /
│Proxy│/
└─────┘
  10ms (concurrent)
```

## Comparison: Before vs After

### Before (Monolithic)
```
main.rs (458 lines)
├── fn load_session_data()
│   ├── Connection::open()
│   ├── execute("PRAGMA...")
│   ├── prepare("SELECT...")
│   ├── read_string()
│   ├── validate_proxy_url()
│   └── Return SessionData
├── fn read_string()
│   ├── prepare("SELECT...")
│   ├── bind()
│   ├── next()
│   └── parse JSON
├── fn validate_proxy_url()
│   ├── Check protocol
│   ├── Parse host:port
│   └── Validate port
└── Business logic mixed with SQL
```

### After (Layered)
```
main.rs (328 lines)
└── Uses SessionRepository trait
    └── Business logic only

storage.rs (340 lines)
├── trait SessionRepository
├── impl SqliteSessionRepository
│   ├── fn connect()
│   ├── fn read_string()
│   └── fn validate_proxy_url()
├── struct SessionData
├── struct DeviceInfo
└── Unit tests

Separation Achieved! ✅
```

## Summary

The storage layer abstraction provides:

1. **Clean Architecture**: Clear separation between layers
2. **Type Safety**: Strongly typed errors and data structures
3. **Security**: SQL injection prevention, input validation
4. **Testability**: Easy to mock and test
5. **Extensibility**: Easy to add new implementations
6. **Maintainability**: Single responsibility principle

The system is now production-ready with proper error handling, validation, and documentation.
