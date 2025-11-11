use crate::error::SessionError;
use crate::domain::{SessionData as DomainSessionData, ApiId, ProxyUrl};
use sqlite::Connection;
use std::path::Path;
use std::fs::{File, OpenOptions};
use fs2::FileExt;

/// Data structure for device information
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub device_model: String,
    pub sdk: String,
    pub app_version: String,
    pub lang_code: String,
    pub system_lang_code: String,
}

/// Complete session data structure
/// This is a storage-layer DTO that gets converted to domain::SessionData
#[derive(Debug, Clone)]
pub struct SessionData {
    pub app_id: ApiId,
    pub device_info: DeviceInfo,
    pub proxy: Option<ProxyUrl>,
}

/// Session file lock guard to prevent multiple instances from using the same session
///
/// This struct holds an exclusive lock on the session file. The lock is automatically
/// released when the guard is dropped.
///
/// # Example
///
/// ```ignore
/// let lock = SessionLock::acquire("session/my.session")?;
/// // Lock is held here
/// // ...
/// drop(lock); // Lock is released
/// ```
pub struct SessionLock {
    #[allow(dead_code)] // Keep the file handle to maintain the lock
    file: File,
    path: String,
}

impl SessionLock {
    /// Acquire an exclusive lock on the session file
    ///
    /// This will create a .lock file next to the session file and acquire an exclusive
    /// lock on it. If another instance already holds the lock, this will fail immediately.
    ///
    /// # Arguments
    ///
    /// * `session_path` - Path to the session file
    ///
    /// # Returns
    ///
    /// A SessionLock guard that will release the lock when dropped
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The session file doesn't exist
    /// - Another instance already holds the lock
    /// - Failed to create or open the lock file
    pub fn acquire(session_path: impl AsRef<Path>) -> Result<Self, SessionError> {
        let session_path = session_path.as_ref();

        // Check if session file exists
        if !session_path.exists() {
            return Err(SessionError::FileNotFound {
                path: session_path.display().to_string(),
            });
        }

        // Create lock file path (session.db.lock)
        let lock_path = format!("{}.lock", session_path.display());

        // Open or create the lock file
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&lock_path)
            .map_err(|e| SessionError::FileLockError {
                path: lock_path.clone(),
                reason: format!("Failed to open lock file: {}", e),
            })?;

        // Try to acquire exclusive lock (non-blocking)
        file.try_lock_exclusive()
            .map_err(|e| SessionError::FileLockError {
                path: lock_path.clone(),
                reason: format!(
                    "Another instance is already using this session. \
                     Please close the other instance or wait for it to finish. \
                     Lock error: {}",
                    e
                ),
            })?;

        log::info!("✓ Acquired exclusive lock on session file: {}", lock_path);

        Ok(Self {
            file,
            path: lock_path,
        })
    }
}

impl Drop for SessionLock {
    fn drop(&mut self) {
        // Unlock the file (automatically done when file handle is dropped)
        // We explicitly unlock to log it
        if let Err(e) = self.file.unlock() {
            log::warn!("Failed to unlock session file {}: {}", self.path, e);
        } else {
            log::info!("✓ Released lock on session file: {}", self.path);
        }
    }
}

/// Repository trait for session data access
///
/// This trait abstracts the storage layer, allowing different implementations
/// (e.g., SQLite, in-memory, mock for testing) to be swapped without changing
/// business logic.
pub trait SessionRepository: Send + Sync {
    /// Load complete session data including API ID, device info, and proxy settings
    fn load_session_data(&self) -> Result<SessionData, SessionError>;

    /// Get API ID from session storage
    fn get_api_id(&self) -> Result<ApiId, SessionError>;

    /// Get device information from session storage
    fn get_device_info(&self) -> Result<DeviceInfo, SessionError>;

    /// Get proxy URL from session storage (if configured)
    fn get_proxy_url(&self) -> Result<Option<ProxyUrl>, SessionError>;
}

/// SQLite implementation of SessionRepository
///
/// This implementation reads session data from a SQLite database file that
/// follows the grammers session format with a 'body' table containing
/// key-value pairs.
pub struct SqliteSessionRepository {
    session_path: String,
}

impl SqliteSessionRepository {
    /// Create a new SQLite session repository
    ///
    /// # Arguments
    ///
    /// * `session_path` - Path to the SQLite session file
    ///
    /// # Example
    ///
    /// ```ignore
    /// let repo = SqliteSessionRepository::new("session/my.session");
    /// ```
    pub fn new(session_path: impl Into<String>) -> Self {
        Self {
            session_path: session_path.into(),
        }
    }

    /// Get database connection with proper initialization
    ///
    /// This method:
    /// 1. Checks if the session file exists
    /// 2. Opens the SQLite connection
    /// 3. Sets PRAGMA user_version = 1 to mark it as an initialized grammers session
    fn connect(&self) -> Result<Connection, SessionError> {
        if !Path::new(&self.session_path).exists() {
            return Err(SessionError::FileNotFound {
                path: self.session_path.clone(),
            });
        }

        let conn = Connection::open(&self.session_path)?;

        // IMPORTANT: Set user_version = 1 for existing grammers session
        // This prevents re-creating tables when SqliteSession::open() is called
        conn.execute("PRAGMA user_version = 1")
            .map_err(SessionError::DatabaseOpen)?;

        Ok(conn)
    }

    /// Read a string value from the body table
    ///
    /// Values in the body table are stored as JSON-encoded strings,
    /// so this method parses the JSON to extract the actual value.
    ///
    /// # Arguments
    ///
    /// * `conn` - Active database connection
    /// * `key` - Key to look up in the body table
    ///
    /// # Returns
    ///
    /// The parsed string value, or an error if the key is not found or parsing fails
    fn read_string(&self, conn: &Connection, key: &str) -> Result<String, SessionError> {
        use sqlite::State;

        // Use parameterized query to prevent SQL injection
        let mut stmt = conn.prepare("SELECT value FROM body WHERE key = ?")
            .map_err(SessionError::DatabaseOpen)?;
        stmt.bind((1, key))
            .map_err(SessionError::DatabaseOpen)?;

        let value = if let State::Row = stmt.next()
            .map_err(SessionError::DatabaseOpen)?
        {
            stmt.read::<String, _>(0)
                .map_err(SessionError::DatabaseOpen)?
        } else {
            return Err(SessionError::FieldNotFound {
                field: key.to_string(),
            });
        };

        // Parse JSON string properly (handles escapes like \", \n, \t, \\, etc.)
        let cleaned: String = serde_json::from_str(&value)?;
        Ok(cleaned)
    }

    /// Validate and convert proxy URL string to ProxyUrl domain type
    ///
    /// Expected format: socks5://[user:pass@]host:port
    ///
    /// # Arguments
    ///
    /// * `proxy_str` - The proxy URL string to validate
    ///
    /// # Returns
    ///
    /// The validated ProxyUrl domain object, or an error if validation fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The URL doesn't start with "socks5://"
    /// - The host or port is missing
    /// - The port is invalid (not a number, 0, or > 65535)
    fn validate_proxy_url(&self, proxy_str: &str) -> Result<ProxyUrl, SessionError> {
        // Use domain validation logic
        ProxyUrl::new(proxy_str.to_string())
            .map_err(|e| SessionError::Domain(e))
    }
}

impl SessionRepository for SqliteSessionRepository {
    fn load_session_data(&self) -> Result<SessionData, SessionError> {
        let app_id = self.get_api_id()?;
        let device_info = self.get_device_info()?;
        let proxy = self.get_proxy_url()?;

        Ok(SessionData {
            app_id,
            device_info,
            proxy,
        })
    }

    fn get_api_id(&self) -> Result<ApiId, SessionError> {
        use sqlite::State;

        let conn = self.connect()?;

        // Use parameterized query (though not strictly necessary for hardcoded key)
        let mut stmt = conn.prepare("SELECT value FROM body WHERE key = 'app_id'")
            .map_err(SessionError::DatabaseOpen)?;

        let app_id_raw: i32 = if let State::Row = stmt.next()
            .map_err(SessionError::DatabaseOpen)?
        {
            stmt.read::<i64, _>(0)
                .map_err(SessionError::DatabaseOpen)? as i32
        } else {
            return Err(SessionError::FieldNotFound {
                field: "app_id".to_string(),
            });
        };

        // Validate and wrap in ApiId domain type
        ApiId::new(app_id_raw)
            .map_err(|e| SessionError::Domain(e))
    }

    fn get_device_info(&self) -> Result<DeviceInfo, SessionError> {
        let conn = self.connect()?;

        let device_model = self.read_string(&conn, "device")?;
        let sdk = self.read_string(&conn, "sdk")?;
        let app_version = self.read_string(&conn, "app_version")?;
        let lang_code = self.read_string(&conn, "lang_code")?;
        let system_lang_code = self.read_string(&conn, "system_lang_code")?;

        Ok(DeviceInfo {
            device_model,
            sdk,
            app_version,
            lang_code,
            system_lang_code,
        })
    }

    fn get_proxy_url(&self) -> Result<Option<ProxyUrl>, SessionError> {
        let conn = self.connect()?;

        // Proxy is optional, so we allow it to be missing
        let proxy = match self.read_string(&conn, "proxy") {
            Ok(proxy_str) => {
                if proxy_str.is_empty() {
                    None
                } else {
                    // Validate proxy URL format and convert to domain type
                    Some(self.validate_proxy_url(&proxy_str)?)
                }
            }
            Err(SessionError::FieldNotFound { .. }) => None, // Proxy is optional
            Err(e) => return Err(e), // Other errors should be propagated
        };

        Ok(proxy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_proxy_url_valid() {
        let repo = SqliteSessionRepository::new("test.db");

        // Valid URLs
        assert!(repo.validate_proxy_url("socks5://proxy.example.com:1080").is_ok());
        assert!(repo.validate_proxy_url("socks5://user:pass@proxy.example.com:1080").is_ok());
        assert!(repo.validate_proxy_url("socks5://127.0.0.1:9050").is_ok());
        assert!(repo.validate_proxy_url("socks5://192.168.1.1:8080").is_ok());
    }

    #[test]
    fn test_validate_proxy_url_invalid_protocol() {
        let repo = SqliteSessionRepository::new("test.db");

        let result = repo.validate_proxy_url("http://proxy.example.com:1080");
        assert!(matches!(result, Err(SessionError::Domain(_))));
    }

    #[test]
    fn test_validate_proxy_url_incomplete() {
        let repo = SqliteSessionRepository::new("test.db");

        assert!(repo.validate_proxy_url("socks5://").is_err());
        assert!(repo.validate_proxy_url("socks5://proxy.example.com").is_err());
    }

    #[test]
    fn test_validate_proxy_url_invalid_port() {
        let repo = SqliteSessionRepository::new("test.db");

        assert!(repo.validate_proxy_url("socks5://proxy.example.com:0").is_err());
        assert!(repo.validate_proxy_url("socks5://proxy.example.com:99999").is_err());
        assert!(repo.validate_proxy_url("socks5://proxy.example.com:abc").is_err());
    }

    #[test]
    fn test_validate_proxy_url_empty_host() {
        let repo = SqliteSessionRepository::new("test.db");

        let result = repo.validate_proxy_url("socks5://:1080");
        assert!(matches!(result, Err(SessionError::Domain(_))));
    }

    // ============================================================================
    // Additional Comprehensive Tests
    // ============================================================================

    // DeviceInfo Tests
    #[test]
    fn test_device_info_creation() {
        let device_info = DeviceInfo {
            device_model: "Android".to_string(),
            sdk: "SDK 30".to_string(),
            app_version: "1.0.0".to_string(),
            lang_code: "en".to_string(),
            system_lang_code: "en-US".to_string(),
        };

        assert_eq!(device_info.device_model, "Android");
        assert_eq!(device_info.sdk, "SDK 30");
        assert_eq!(device_info.app_version, "1.0.0");
        assert_eq!(device_info.lang_code, "en");
        assert_eq!(device_info.system_lang_code, "en-US");
    }

    // SessionData Tests
    #[test]
    fn test_session_data_creation() {
        let api_id = ApiId::new(12345).unwrap();
        let device_info = DeviceInfo {
            device_model: "Android".to_string(),
            sdk: "SDK 30".to_string(),
            app_version: "1.0.0".to_string(),
            lang_code: "en".to_string(),
            system_lang_code: "en-US".to_string(),
        };
        let proxy = ProxyUrl::new("socks5://proxy.com:1080".to_string()).unwrap();

        let session_data = SessionData {
            app_id: api_id,
            device_info: device_info.clone(),
            proxy: Some(proxy.clone()),
        };

        assert_eq!(session_data.app_id, api_id);
        assert_eq!(session_data.device_info.device_model, device_info.device_model);
        assert!(session_data.proxy.is_some());
    }

    // SqliteSessionRepository Tests
    #[test]
    fn test_sqlite_repository_new() {
        let repo = SqliteSessionRepository::new("test.db");
        // Verify it's created without panic
        assert!(true);
    }

    #[test]
    fn test_validate_proxy_url_with_ipv4() {
        let repo = SqliteSessionRepository::new("test.db");

        assert!(repo.validate_proxy_url("socks5://127.0.0.1:1080").is_ok());
        assert!(repo.validate_proxy_url("socks5://192.168.1.1:9050").is_ok());
        assert!(repo.validate_proxy_url("socks5://10.0.0.1:8080").is_ok());
    }

    #[test]
    fn test_validate_proxy_url_with_ipv6() {
        let repo = SqliteSessionRepository::new("test.db");

        assert!(repo.validate_proxy_url("socks5://[::1]:1080").is_ok());
        assert!(repo.validate_proxy_url("socks5://[2001:db8::1]:1080").is_ok());
        assert!(repo.validate_proxy_url("socks5://[fe80::1]:9050").is_ok());
    }

    #[test]
    fn test_validate_proxy_url_with_auth() {
        let repo = SqliteSessionRepository::new("test.db");

        assert!(repo.validate_proxy_url("socks5://user:pass@proxy.com:1080").is_ok());
        assert!(repo.validate_proxy_url("socks5://username:password123@proxy.com:1080").is_ok());
        assert!(repo.validate_proxy_url("socks5://admin:p@ssw0rd!@proxy.com:1080").is_ok());
    }

    #[test]
    fn test_validate_proxy_url_port_boundaries() {
        let repo = SqliteSessionRepository::new("test.db");

        // Valid ports
        assert!(repo.validate_proxy_url("socks5://proxy.com:1").is_ok());
        assert!(repo.validate_proxy_url("socks5://proxy.com:80").is_ok());
        assert!(repo.validate_proxy_url("socks5://proxy.com:443").is_ok());
        assert!(repo.validate_proxy_url("socks5://proxy.com:8080").is_ok());
        assert!(repo.validate_proxy_url("socks5://proxy.com:65535").is_ok());

        // Invalid ports
        assert!(repo.validate_proxy_url("socks5://proxy.com:0").is_err());
        assert!(repo.validate_proxy_url("socks5://proxy.com:65536").is_err());
        assert!(repo.validate_proxy_url("socks5://proxy.com:-1").is_err());
        assert!(repo.validate_proxy_url("socks5://proxy.com:99999").is_err());
    }

    #[test]
    fn test_validate_proxy_url_malformed() {
        let repo = SqliteSessionRepository::new("test.db");

        // Wrong protocol
        assert!(repo.validate_proxy_url("http://proxy.com:1080").is_err());
        assert!(repo.validate_proxy_url("https://proxy.com:1080").is_err());
        assert!(repo.validate_proxy_url("socks4://proxy.com:1080").is_err());

        // Missing parts
        assert!(repo.validate_proxy_url("socks5://").is_err());
        assert!(repo.validate_proxy_url("socks5://proxy.com").is_err());
        assert!(repo.validate_proxy_url("socks5://:1080").is_err());

        // Invalid characters in port
        assert!(repo.validate_proxy_url("socks5://proxy.com:abc").is_err());
        assert!(repo.validate_proxy_url("socks5://proxy.com:12.34").is_err());
        assert!(repo.validate_proxy_url("socks5://proxy.com:12a").is_err());
    }

    // ============================================================================
    // Mock SessionRepository for Testing
    // ============================================================================

    /// Mock implementation of SessionRepository for testing business logic
    /// without requiring actual database files
    #[derive(Clone)]
    struct MockSessionRepository {
        app_id: Option<Result<ApiId, SessionError>>,
        device_info: Option<Result<DeviceInfo, SessionError>>,
        proxy_url: Option<Result<Option<ProxyUrl>, SessionError>>,
    }

    impl MockSessionRepository {
        fn new() -> Self {
            Self {
                app_id: None,
                device_info: None,
                proxy_url: None,
            }
        }

        fn with_app_id(mut self, result: Result<ApiId, SessionError>) -> Self {
            self.app_id = Some(result);
            self
        }

        fn with_device_info(mut self, result: Result<DeviceInfo, SessionError>) -> Self {
            self.device_info = Some(result);
            self
        }

        fn with_proxy_url(mut self, result: Result<Option<ProxyUrl>, SessionError>) -> Self {
            self.proxy_url = Some(result);
            self
        }
    }

    impl SessionRepository for MockSessionRepository {
        fn load_session_data(&self) -> Result<SessionData, SessionError> {
            let app_id = self.get_api_id()?;
            let device_info = self.get_device_info()?;
            let proxy = self.get_proxy_url()?;

            Ok(SessionData {
                app_id,
                device_info,
                proxy,
            })
        }

        fn get_api_id(&self) -> Result<ApiId, SessionError> {
            self.app_id.clone().unwrap_or_else(|| {
                ApiId::new(12345).map_err(|e| SessionError::Domain(e))
            })
        }

        fn get_device_info(&self) -> Result<DeviceInfo, SessionError> {
            self.device_info.clone().unwrap_or_else(|| {
                Ok(DeviceInfo {
                    device_model: "Test Device".to_string(),
                    sdk: "Test SDK".to_string(),
                    app_version: "1.0.0".to_string(),
                    lang_code: "en".to_string(),
                    system_lang_code: "en-US".to_string(),
                })
            })
        }

        fn get_proxy_url(&self) -> Result<Option<ProxyUrl>, SessionError> {
            self.proxy_url.clone().unwrap_or(Ok(None))
        }
    }

    // Mock Repository Tests
    #[test]
    fn test_mock_repository_happy_path() {
        let mock = MockSessionRepository::new();
        let session_data = mock.load_session_data().unwrap();

        assert_eq!(session_data.app_id.as_i32(), 12345);
        assert_eq!(session_data.device_info.device_model, "Test Device");
        assert!(session_data.proxy.is_none());
    }

    #[test]
    fn test_mock_repository_with_custom_api_id() {
        let mock = MockSessionRepository::new()
            .with_app_id(ApiId::new(99999).map_err(|e| SessionError::Domain(e)));

        let api_id = mock.get_api_id().unwrap();
        assert_eq!(api_id.as_i32(), 99999);
    }

    #[test]
    fn test_mock_repository_with_proxy() {
        let proxy = ProxyUrl::new("socks5://proxy.com:1080".to_string()).unwrap();
        let mock = MockSessionRepository::new()
            .with_proxy_url(Ok(Some(proxy.clone())));

        let proxy_result = mock.get_proxy_url().unwrap();
        assert!(proxy_result.is_some());
    }

    #[test]
    fn test_mock_repository_error_propagation() {
        let mock = MockSessionRepository::new()
            .with_app_id(Err(SessionError::FieldNotFound {
                field: "app_id".to_string(),
            }));

        let result = mock.load_session_data();
        assert!(result.is_err());
        assert!(matches!(result, Err(SessionError::FieldNotFound { .. })));
    }

    #[test]
    fn test_mock_repository_device_info_error() {
        let mock = MockSessionRepository::new()
            .with_device_info(Err(SessionError::FieldNotFound {
                field: "device".to_string(),
            }));

        let result = mock.get_device_info();
        assert!(result.is_err());
    }

    #[test]
    fn test_mock_repository_proxy_error() {
        let mock = MockSessionRepository::new()
            .with_proxy_url(Err(SessionError::InvalidProxyUrl {
                reason: "Invalid format".to_string(),
            }));

        let result = mock.get_proxy_url();
        assert!(result.is_err());
    }

    // SessionLock Edge Cases
    #[test]
    fn test_session_lock_with_nonexistent_file() {
        let result = SessionLock::acquire("/nonexistent/path/session.db");
        assert!(result.is_err());
        assert!(matches!(result, Err(SessionError::FileNotFound { .. })));
    }

    // Test that multiple SessionRepository implementations work with the same trait
    #[test]
    fn test_repository_trait_polymorphism() {
        fn load_from_repo(repo: &dyn SessionRepository) -> Result<ApiId, SessionError> {
            repo.get_api_id()
        }

        let mock = MockSessionRepository::new();
        let result = load_from_repo(&mock);
        assert!(result.is_ok());
    }

    // Test Clone implementation for structs
    #[test]
    fn test_device_info_clone() {
        let device_info = DeviceInfo {
            device_model: "Android".to_string(),
            sdk: "SDK 30".to_string(),
            app_version: "1.0.0".to_string(),
            lang_code: "en".to_string(),
            system_lang_code: "en-US".to_string(),
        };

        let cloned = device_info.clone();
        assert_eq!(cloned.device_model, device_info.device_model);
        assert_eq!(cloned.sdk, device_info.sdk);
    }

    #[test]
    fn test_session_data_clone() {
        let api_id = ApiId::new(12345).unwrap();
        let device_info = DeviceInfo {
            device_model: "Android".to_string(),
            sdk: "SDK 30".to_string(),
            app_version: "1.0.0".to_string(),
            lang_code: "en".to_string(),
            system_lang_code: "en-US".to_string(),
        };

        let session_data = SessionData {
            app_id: api_id,
            device_info: device_info.clone(),
            proxy: None,
        };

        let cloned = session_data.clone();
        assert_eq!(cloned.app_id, session_data.app_id);
        assert_eq!(cloned.device_info.device_model, session_data.device_info.device_model);
    }
}
