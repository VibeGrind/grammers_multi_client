//! Domain models and types for the Telegram client application.
//!
//! This module provides strongly-typed wrappers around primitive types to catch
//! errors at compile time and ensure type safety throughout the application.

use serde::{Deserialize, Serialize};
use std::fmt;

// =============================================================================
// Domain Errors
// =============================================================================

/// Errors that can occur during domain object validation
#[derive(Debug, Clone)]
pub enum DomainError {
    /// Invalid API ID (must be positive)
    InvalidApiId(i32),

    /// Invalid Chat ID (cannot be zero)
    InvalidChatId(i64),

    /// Invalid Message ID (must be positive)
    InvalidMessageId(i32),

    /// Invalid User ID (cannot be zero)
    InvalidUserId(i64),

    /// Invalid Proxy URL with reason
    InvalidProxyUrl(String),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::InvalidApiId(id) => {
                write!(f, "Invalid API ID: {} (must be positive)", id)
            }
            DomainError::InvalidChatId(id) => {
                write!(f, "Invalid Chat ID: {} (cannot be zero)", id)
            }
            DomainError::InvalidMessageId(id) => {
                write!(f, "Invalid Message ID: {} (must be positive)", id)
            }
            DomainError::InvalidUserId(id) => {
                write!(f, "Invalid User ID: {} (cannot be zero)", id)
            }
            DomainError::InvalidProxyUrl(reason) => {
                write!(f, "Invalid Proxy URL: {}", reason)
            }
        }
    }
}

impl std::error::Error for DomainError {}

// =============================================================================
// NewType Wrappers
// =============================================================================

/// Telegram API application ID.
///
/// This is provided by Telegram when you register your application at
/// https://my.telegram.org/apps
///
/// # Example
/// ```
/// use telegram_minimal_client::domain::ApiId;
///
/// let api_id = ApiId::new(12345).expect("Valid API ID");
/// assert_eq!(api_id.as_i32(), 12345);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApiId(i32);

impl ApiId {
    /// Creates a new ApiId after validation.
    ///
    /// # Errors
    /// Returns `DomainError::InvalidApiId` if the ID is not positive.
    pub fn new(id: i32) -> Result<Self, DomainError> {
        if id <= 0 {
            return Err(DomainError::InvalidApiId(id));
        }
        Ok(Self(id))
    }

    /// Creates an ApiId without validation. Use with caution.
    ///
    /// # Safety
    /// Caller must ensure the ID is valid (positive).
    pub const fn new_unchecked(id: i32) -> Self {
        Self(id)
    }

    /// Returns the inner i32 value.
    pub const fn as_i32(&self) -> i32 {
        self.0
    }

    /// Converts into the inner i32 value.
    pub fn into_inner(self) -> i32 {
        self.0
    }
}

impl fmt::Display for ApiId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<i32> for ApiId {
    type Error = DomainError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

// =============================================================================

/// Telegram chat identifier.
///
/// This uniquely identifies a chat, channel, group, or private conversation.
/// The format follows Telegram's Bot API chat_id convention.
///
/// # Example
/// ```
/// use telegram_minimal_client::domain::ChatId;
///
/// let chat_id = ChatId::new(-1001234567890).expect("Valid chat ID");
/// assert_eq!(chat_id.as_i64(), -1001234567890);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ChatId(i64);

impl ChatId {
    /// Creates a new ChatId after validation.
    ///
    /// # Errors
    /// Returns `DomainError::InvalidChatId` if the ID is zero.
    pub fn new(id: i64) -> Result<Self, DomainError> {
        if id == 0 {
            return Err(DomainError::InvalidChatId(id));
        }
        Ok(Self(id))
    }

    /// Creates a ChatId without validation. Use with caution.
    ///
    /// # Safety
    /// Caller must ensure the ID is valid (non-zero).
    pub const fn new_unchecked(id: i64) -> Self {
        Self(id)
    }

    /// Returns the inner i64 value.
    pub const fn as_i64(&self) -> i64 {
        self.0
    }

    /// Converts into the inner i64 value.
    pub fn into_inner(self) -> i64 {
        self.0
    }
}

impl fmt::Display for ChatId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<i64> for ChatId {
    type Error = DomainError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

// =============================================================================

/// Telegram message identifier.
///
/// This uniquely identifies a message within a chat.
///
/// # Example
/// ```
/// use telegram_minimal_client::domain::MessageId;
///
/// let msg_id = MessageId::new(42).expect("Valid message ID");
/// assert_eq!(msg_id.as_i32(), 42);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageId(i32);

impl MessageId {
    /// Creates a new MessageId after validation.
    ///
    /// # Errors
    /// Returns `DomainError::InvalidMessageId` if the ID is not positive.
    pub fn new(id: i32) -> Result<Self, DomainError> {
        if id <= 0 {
            return Err(DomainError::InvalidMessageId(id));
        }
        Ok(Self(id))
    }

    /// Creates a MessageId without validation. Use with caution.
    ///
    /// # Safety
    /// Caller must ensure the ID is valid (positive).
    pub const fn new_unchecked(id: i32) -> Self {
        Self(id)
    }

    /// Returns the inner i32 value.
    pub const fn as_i32(&self) -> i32 {
        self.0
    }

    /// Converts into the inner i32 value.
    pub fn into_inner(self) -> i32 {
        self.0
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<i32> for MessageId {
    type Error = DomainError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

// =============================================================================

/// Telegram user identifier.
///
/// This uniquely identifies a user account in Telegram.
///
/// # Example
/// ```
/// use telegram_minimal_client::domain::UserId;
///
/// let user_id = UserId::new(123456789).expect("Valid user ID");
/// assert_eq!(user_id.as_i64(), 123456789);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(i64);

impl UserId {
    /// Creates a new UserId after validation.
    ///
    /// # Errors
    /// Returns `DomainError::InvalidUserId` if the ID is zero.
    pub fn new(id: i64) -> Result<Self, DomainError> {
        if id == 0 {
            return Err(DomainError::InvalidUserId(id));
        }
        Ok(Self(id))
    }

    /// Creates a UserId without validation. Use with caution.
    ///
    /// # Safety
    /// Caller must ensure the ID is valid (non-zero).
    pub const fn new_unchecked(id: i64) -> Self {
        Self(id)
    }

    /// Returns the inner i64 value.
    pub const fn as_i64(&self) -> i64 {
        self.0
    }

    /// Converts into the inner i64 value.
    pub fn into_inner(self) -> i64 {
        self.0
    }
}

impl fmt::Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<i64> for UserId {
    type Error = DomainError;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

// =============================================================================

/// Validated SOCKS5 proxy URL.
///
/// This type ensures that proxy URLs are properly formatted and valid before use.
/// Expected format: `socks5://[user:pass@]host:port`
///
/// # Example
/// ```
/// use telegram_minimal_client::domain::ProxyUrl;
///
/// let proxy = ProxyUrl::new("socks5://user:pass@proxy.example.com:1080".to_string())
///     .expect("Valid proxy URL");
/// assert_eq!(proxy.as_str(), "socks5://user:pass@proxy.example.com:1080");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProxyUrl(String);

impl ProxyUrl {
    /// Creates a new ProxyUrl after validation.
    ///
    /// # Errors
    /// Returns `DomainError::InvalidProxyUrl` if:
    /// - URL doesn't start with "socks5://"
    /// - Host or port is missing
    /// - Port is not a valid number (1-65535)
    ///
    /// # Example
    /// ```
    /// use telegram_minimal_client::domain::ProxyUrl;
    ///
    /// // Valid proxy URL
    /// let proxy = ProxyUrl::new("socks5://proxy.example.com:1080".to_string());
    /// assert!(proxy.is_ok());
    ///
    /// // Invalid - missing port
    /// let proxy = ProxyUrl::new("socks5://proxy.example.com".to_string());
    /// assert!(proxy.is_err());
    /// ```
    pub fn new(url: String) -> Result<Self, DomainError> {
        Self::validate(&url)?;
        Ok(Self(url))
    }

    /// Validates a proxy URL string.
    fn validate(proxy_str: &str) -> Result<(), DomainError> {
        // Check if it starts with socks5://
        if !proxy_str.starts_with("socks5://") {
            return Err(DomainError::InvalidProxyUrl(
                "Proxy URL must start with 'socks5://'".to_string()
            ));
        }

        // Remove the scheme
        let after_scheme = &proxy_str[9..]; // "socks5://" is 9 characters

        if after_scheme.is_empty() {
            return Err(DomainError::InvalidProxyUrl(
                "Proxy URL is incomplete after 'socks5://'".to_string()
            ));
        }

        // Split by @ to separate auth from host:port
        let host_port = if let Some(at_pos) = after_scheme.rfind('@') {
            &after_scheme[at_pos + 1..]
        } else {
            after_scheme
        };

        // Split by : to get host and port
        let parts: Vec<&str> = host_port.rsplitn(2, ':').collect();

        if parts.len() != 2 {
            return Err(DomainError::InvalidProxyUrl(
                "Proxy URL must include port in format 'host:port'".to_string()
            ));
        }

        let port_str = parts[0];
        let host = parts[1];

        // Validate host is not empty
        if host.is_empty() {
            return Err(DomainError::InvalidProxyUrl(
                "Proxy URL host cannot be empty".to_string()
            ));
        }

        // Validate port
        let port: u16 = port_str.parse()
            .map_err(|_| DomainError::InvalidProxyUrl(
                format!("Invalid proxy port '{}', must be a number between 1 and 65535", port_str)
            ))?;

        if port == 0 {
            return Err(DomainError::InvalidProxyUrl(
                "Proxy port must be between 1 and 65535".to_string()
            ));
        }

        Ok(())
    }

    /// Returns the proxy URL as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Converts into the inner String.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for ProxyUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "socks5://***:***@***:***") // Redact for security
    }
}

impl TryFrom<String> for ProxyUrl {
    type Error = DomainError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ProxyUrl> for String {
    fn from(proxy: ProxyUrl) -> Self {
        proxy.0
    }
}

// =============================================================================
// Domain Structs
// =============================================================================

/// Session fingerprint data loaded from the SQLite session file.
///
/// This contains all the device and application information needed to
/// establish a Telegram connection with proper device fingerprinting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    /// Telegram API application ID
    pub app_id: ApiId,

    /// Device model (e.g., "Samsung Galaxy S21")
    pub device: String,

    /// SDK/OS version (e.g., "Android 12")
    pub sdk: String,

    /// Application version (e.g., "1.0.0")
    pub app_version: String,

    /// Primary language code (e.g., "en")
    pub lang_code: String,

    /// System language code (e.g., "en-US")
    pub system_lang_code: String,

    /// Optional SOCKS5 proxy URL
    pub proxy: Option<ProxyUrl>,
}

impl SessionData {
    /// Creates a new SessionData instance.
    pub fn new(
        app_id: ApiId,
        device: String,
        sdk: String,
        app_version: String,
        lang_code: String,
        system_lang_code: String,
        proxy: Option<ProxyUrl>,
    ) -> Self {
        Self {
            app_id,
            device,
            sdk,
            app_version,
            lang_code,
            system_lang_code,
            proxy,
        }
    }

    /// Returns whether a proxy is configured.
    pub fn has_proxy(&self) -> bool {
        self.proxy.is_some()
    }
}

// =============================================================================

/// Information about a Telegram user.
///
/// This represents a user that sent a message or is involved in an update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    /// Unique user identifier
    pub id: UserId,

    /// User's first name
    pub first_name: String,

    /// User's username (without @)
    pub username: Option<String>,
}

impl UserInfo {
    /// Creates a new UserInfo instance.
    pub fn new(id: UserId, first_name: String, username: Option<String>) -> Self {
        Self {
            id,
            first_name,
            username,
        }
    }

    /// Returns the display name for this user (first name or username).
    pub fn display_name(&self) -> &str {
        if !self.first_name.is_empty() {
            &self.first_name
        } else if let Some(ref username) = self.username {
            username
        } else {
            "Unknown"
        }
    }
}

impl fmt::Display for UserInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref username) = self.username {
            write!(f, "{} (@{})", self.first_name, username)
        } else {
            write!(f, "{}", self.first_name)
        }
    }
}

// =============================================================================

/// Telegram update event to be sent to Phoenix Channel.
///
/// This represents various types of events from Telegram (new messages,
/// message edits, deletions, etc.) in a format suitable for Phoenix Channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramUpdate {
    /// Type of update (e.g., "new_message", "message_edited", "message_deleted")
    pub update_type: String,

    /// Chat where the update occurred (if applicable)
    pub chat_id: Option<ChatId>,

    /// Message ID involved in the update (if applicable)
    pub message_id: Option<MessageId>,

    /// Text content of the message (if applicable)
    pub text: Option<String>,

    /// User who triggered the update (if applicable)
    pub from_user: Option<UserInfo>,

    /// Unix timestamp of the event (if applicable)
    pub timestamp: Option<i64>,

    /// Raw debug data for unhandled update types
    pub raw_data: Option<String>,
}

impl TelegramUpdate {
    /// Creates a new TelegramUpdate for a new message.
    pub fn new_message(
        chat_id: ChatId,
        message_id: MessageId,
        text: String,
        from_user: Option<UserInfo>,
        timestamp: i64,
    ) -> Self {
        Self {
            update_type: "new_message".to_string(),
            chat_id: Some(chat_id),
            message_id: Some(message_id),
            text: Some(text),
            from_user,
            timestamp: Some(timestamp),
            raw_data: None,
        }
    }

    /// Creates a new TelegramUpdate for a message edit.
    pub fn message_edited(
        chat_id: ChatId,
        message_id: MessageId,
        text: String,
        timestamp: i64,
    ) -> Self {
        Self {
            update_type: "message_edited".to_string(),
            chat_id: Some(chat_id),
            message_id: Some(message_id),
            text: Some(text),
            from_user: None,
            timestamp: Some(timestamp),
            raw_data: None,
        }
    }

    /// Creates a new TelegramUpdate for a message deletion.
    pub fn message_deleted(
        chat_id: Option<ChatId>,
        message_id: Option<MessageId>,
        raw_data: String,
    ) -> Self {
        Self {
            update_type: "message_deleted".to_string(),
            chat_id,
            message_id,
            text: None,
            from_user: None,
            timestamp: None,
            raw_data: Some(raw_data),
        }
    }

    /// Creates a new TelegramUpdate for an unhandled update type.
    pub fn other(raw_data: String) -> Self {
        Self {
            update_type: "other".to_string(),
            chat_id: None,
            message_id: None,
            text: None,
            from_user: None,
            timestamp: None,
            raw_data: Some(raw_data),
        }
    }

    /// Returns whether this is a new message update.
    pub fn is_new_message(&self) -> bool {
        self.update_type == "new_message"
    }

    /// Returns whether this is a message edit update.
    pub fn is_message_edited(&self) -> bool {
        self.update_type == "message_edited"
    }

    /// Returns whether this is a message deletion update.
    pub fn is_message_deleted(&self) -> bool {
        self.update_type == "message_deleted"
    }
}

impl fmt::Display for TelegramUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TelegramUpdate({})", self.update_type)?;
        if let Some(ref chat_id) = self.chat_id {
            write!(f, " chat={}", chat_id)?;
        }
        if let Some(ref msg_id) = self.message_id {
            write!(f, " msg={}", msg_id)?;
        }
        Ok(())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_id_validation() {
        assert!(ApiId::new(12345).is_ok());
        assert!(ApiId::new(0).is_err());
        assert!(ApiId::new(-1).is_err());
    }

    #[test]
    fn test_chat_id_validation() {
        assert!(ChatId::new(12345).is_ok());
        assert!(ChatId::new(-12345).is_ok());
        assert!(ChatId::new(0).is_err());
    }

    #[test]
    fn test_message_id_validation() {
        assert!(MessageId::new(42).is_ok());
        assert!(MessageId::new(0).is_err());
        assert!(MessageId::new(-1).is_err());
    }

    #[test]
    fn test_user_id_validation() {
        assert!(UserId::new(123456789).is_ok());
        assert!(UserId::new(-123456789).is_ok());
        assert!(UserId::new(0).is_err());
    }

    #[test]
    fn test_proxy_url_validation() {
        // Valid URLs
        assert!(ProxyUrl::new("socks5://proxy.example.com:1080".to_string()).is_ok());
        assert!(ProxyUrl::new("socks5://user:pass@proxy.example.com:1080".to_string()).is_ok());

        // Invalid URLs
        assert!(ProxyUrl::new("http://proxy.example.com:1080".to_string()).is_err());
        assert!(ProxyUrl::new("socks5://proxy.example.com".to_string()).is_err());
        assert!(ProxyUrl::new("socks5://proxy.example.com:0".to_string()).is_err());
        assert!(ProxyUrl::new("socks5://proxy.example.com:99999".to_string()).is_err());
        assert!(ProxyUrl::new("socks5://".to_string()).is_err());
    }

    #[test]
    fn test_user_info_display_name() {
        let user = UserInfo::new(
            UserId::new(123).unwrap(),
            "John".to_string(),
            Some("john_doe".to_string()),
        );
        assert_eq!(user.display_name(), "John");

        let user_no_name = UserInfo::new(
            UserId::new(123).unwrap(),
            "".to_string(),
            Some("john_doe".to_string()),
        );
        assert_eq!(user_no_name.display_name(), "john_doe");
    }

    #[test]
    fn test_telegram_update_constructors() {
        let chat_id = ChatId::new(123).unwrap();
        let msg_id = MessageId::new(456).unwrap();

        let update = TelegramUpdate::new_message(
            chat_id,
            msg_id,
            "Hello".to_string(),
            None,
            1234567890,
        );
        assert!(update.is_new_message());
        assert!(!update.is_message_edited());

        let edit = TelegramUpdate::message_edited(
            chat_id,
            msg_id,
            "Hello edited".to_string(),
            1234567890,
        );
        assert!(edit.is_message_edited());

        let delete = TelegramUpdate::message_deleted(
            Some(chat_id),
            Some(msg_id),
            "deleted".to_string(),
        );
        assert!(delete.is_message_deleted());
    }

    #[test]
    fn test_serde_serialization() {
        let user_id = UserId::new(123).unwrap();
        let json = serde_json::to_string(&user_id).unwrap();
        assert_eq!(json, "123");

        let deserialized: UserId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, user_id);
    }

    // ============================================================================
    // Additional Comprehensive Tests
    // ============================================================================

    // NewType Edge Cases
    #[test]
    fn test_api_id_edge_cases() {
        // Maximum i32 value
        assert!(ApiId::new(i32::MAX).is_ok());

        // Minimum valid value (1)
        assert!(ApiId::new(1).is_ok());

        // Just below minimum (0)
        assert!(ApiId::new(0).is_err());

        // Negative values
        assert!(ApiId::new(i32::MIN).is_err());
    }

    #[test]
    fn test_chat_id_edge_cases() {
        // Maximum i64 value
        assert!(ChatId::new(i64::MAX).is_ok());

        // Minimum i64 value
        assert!(ChatId::new(i64::MIN).is_ok());

        // Zero (invalid)
        assert!(ChatId::new(0).is_err());

        // Positive and negative (both valid)
        assert!(ChatId::new(1).is_ok());
        assert!(ChatId::new(-1).is_ok());
    }

    #[test]
    fn test_message_id_edge_cases() {
        // Maximum i32 value
        assert!(MessageId::new(i32::MAX).is_ok());

        // Minimum valid value (1)
        assert!(MessageId::new(1).is_ok());

        // Zero (invalid)
        assert!(MessageId::new(0).is_err());

        // Negative (invalid)
        assert!(MessageId::new(-1).is_err());
        assert!(MessageId::new(i32::MIN).is_err());
    }

    #[test]
    fn test_user_id_edge_cases() {
        // Maximum i64 value
        assert!(UserId::new(i64::MAX).is_ok());

        // Minimum i64 value
        assert!(UserId::new(i64::MIN).is_ok());

        // Zero (invalid)
        assert!(UserId::new(0).is_err());

        // Positive and negative (both valid)
        assert!(UserId::new(1).is_ok());
        assert!(UserId::new(-1).is_ok());
    }

    // ProxyUrl Edge Cases
    #[test]
    fn test_proxy_url_edge_cases() {
        // IPv6 addresses
        assert!(ProxyUrl::new("socks5://[::1]:1080".to_string()).is_ok());
        assert!(ProxyUrl::new("socks5://[2001:db8::1]:1080".to_string()).is_ok());

        // With credentials containing special characters
        assert!(ProxyUrl::new("socks5://user%40domain:p@ss:w0rd@proxy.com:1080".to_string()).is_ok());

        // Maximum port (65535)
        assert!(ProxyUrl::new("socks5://proxy.com:65535".to_string()).is_ok());

        // Minimum port (1)
        assert!(ProxyUrl::new("socks5://proxy.com:1".to_string()).is_ok());

        // Invalid port (0)
        assert!(ProxyUrl::new("socks5://proxy.com:0".to_string()).is_err());

        // Invalid port (too large)
        assert!(ProxyUrl::new("socks5://proxy.com:65536".to_string()).is_err());

        // Invalid port (negative)
        assert!(ProxyUrl::new("socks5://proxy.com:-1".to_string()).is_err());

        // Missing scheme
        assert!(ProxyUrl::new("proxy.com:1080".to_string()).is_err());

        // Wrong scheme
        assert!(ProxyUrl::new("http://proxy.com:1080".to_string()).is_err());
        assert!(ProxyUrl::new("https://proxy.com:1080".to_string()).is_err());

        // Missing host
        assert!(ProxyUrl::new("socks5://:1080".to_string()).is_err());

        // Missing port
        assert!(ProxyUrl::new("socks5://proxy.com".to_string()).is_err());

        // Empty after scheme
        assert!(ProxyUrl::new("socks5://".to_string()).is_err());

        // With path (should still work as host can contain /)
        assert!(ProxyUrl::new("socks5://proxy.com:1080/path".to_string()).is_err());
    }

    #[test]
    fn test_proxy_url_complex_usernames() {
        // Colon in password
        assert!(ProxyUrl::new("socks5://user:pass:word@proxy.com:1080".to_string()).is_ok());

        // At sign in username (URL encoded)
        assert!(ProxyUrl::new("socks5://user%40domain:pass@proxy.com:1080".to_string()).is_ok());

        // Multiple colons in auth
        assert!(ProxyUrl::new("socks5://user:pass:word:123@proxy.com:1080".to_string()).is_ok());
    }

    // Display and Format Tests
    #[test]
    fn test_display_implementations() {
        let api_id = ApiId::new(12345).unwrap();
        assert_eq!(format!("{}", api_id), "12345");

        let chat_id = ChatId::new(-1001234567890).unwrap();
        assert_eq!(format!("{}", chat_id), "-1001234567890");

        let msg_id = MessageId::new(42).unwrap();
        assert_eq!(format!("{}", msg_id), "42");

        let user_id = UserId::new(123456789).unwrap();
        assert_eq!(format!("{}", user_id), "123456789");
    }

    #[test]
    fn test_proxy_url_display_redacted() {
        let proxy = ProxyUrl::new("socks5://user:password@proxy.com:1080".to_string()).unwrap();
        let display = format!("{}", proxy);

        // Should not contain actual credentials
        assert!(!display.contains("user"));
        assert!(!display.contains("password"));
        assert!(!display.contains("proxy.com"));

        // Should be redacted
        assert!(display.contains("***"));
    }

    #[test]
    fn test_user_info_display_with_username() {
        let user = UserInfo::new(
            UserId::new(123).unwrap(),
            "John Doe".to_string(),
            Some("johndoe".to_string()),
        );

        let display = format!("{}", user);
        assert!(display.contains("John Doe"));
        assert!(display.contains("@johndoe"));
    }

    #[test]
    fn test_user_info_display_without_username() {
        let user = UserInfo::new(
            UserId::new(123).unwrap(),
            "John Doe".to_string(),
            None,
        );

        let display = format!("{}", user);
        assert_eq!(display, "John Doe");
        assert!(!display.contains("@"));
    }

    #[test]
    fn test_user_info_display_name_edge_cases() {
        // Empty first name but has username
        let user = UserInfo::new(
            UserId::new(123).unwrap(),
            "".to_string(),
            Some("johndoe".to_string()),
        );
        assert_eq!(user.display_name(), "johndoe");

        // Empty first name and no username
        let user = UserInfo::new(
            UserId::new(123).unwrap(),
            "".to_string(),
            None,
        );
        assert_eq!(user.display_name(), "Unknown");

        // Has first name (should prefer it)
        let user = UserInfo::new(
            UserId::new(123).unwrap(),
            "John".to_string(),
            Some("johndoe".to_string()),
        );
        assert_eq!(user.display_name(), "John");
    }

    // TelegramUpdate Tests
    #[test]
    fn test_telegram_update_display() {
        let chat_id = ChatId::new(123).unwrap();
        let msg_id = MessageId::new(456).unwrap();

        let update = TelegramUpdate::new_message(
            chat_id,
            msg_id,
            "Hello".to_string(),
            None,
            1234567890,
        );

        let display = format!("{}", update);
        assert!(display.contains("TelegramUpdate"));
        assert!(display.contains("new_message"));
        assert!(display.contains("chat=123"));
        assert!(display.contains("msg=456"));
    }

    #[test]
    fn test_telegram_update_type_checks() {
        let chat_id = ChatId::new(123).unwrap();
        let msg_id = MessageId::new(456).unwrap();

        // Test new_message
        let update = TelegramUpdate::new_message(
            chat_id,
            msg_id,
            "Hello".to_string(),
            None,
            1234567890,
        );
        assert!(update.is_new_message());
        assert!(!update.is_message_edited());
        assert!(!update.is_message_deleted());

        // Test message_edited
        let update = TelegramUpdate::message_edited(
            chat_id,
            msg_id,
            "Hello edited".to_string(),
            1234567890,
        );
        assert!(!update.is_new_message());
        assert!(update.is_message_edited());
        assert!(!update.is_message_deleted());

        // Test message_deleted
        let update = TelegramUpdate::message_deleted(
            Some(chat_id),
            Some(msg_id),
            "raw data".to_string(),
        );
        assert!(!update.is_new_message());
        assert!(!update.is_message_edited());
        assert!(update.is_message_deleted());

        // Test other
        let update = TelegramUpdate::other("unknown update".to_string());
        assert!(!update.is_new_message());
        assert!(!update.is_message_edited());
        assert!(!update.is_message_deleted());
        assert_eq!(update.update_type, "other");
    }

    #[test]
    fn test_telegram_update_with_user_info() {
        let chat_id = ChatId::new(123).unwrap();
        let msg_id = MessageId::new(456).unwrap();
        let user = UserInfo::new(
            UserId::new(789).unwrap(),
            "John".to_string(),
            Some("john".to_string()),
        );

        let update = TelegramUpdate::new_message(
            chat_id,
            msg_id,
            "Hello".to_string(),
            Some(user.clone()),
            1234567890,
        );

        assert!(update.from_user.is_some());
        assert_eq!(update.from_user.unwrap().id, user.id);
    }

    // SessionData Tests
    #[test]
    fn test_session_data_creation() {
        let api_id = ApiId::new(12345).unwrap();
        let proxy = ProxyUrl::new("socks5://proxy.com:1080".to_string()).unwrap();

        let session = SessionData::new(
            api_id,
            "Android".to_string(),
            "SDK 30".to_string(),
            "1.0.0".to_string(),
            "en".to_string(),
            "en-US".to_string(),
            Some(proxy),
        );

        assert_eq!(session.app_id, api_id);
        assert_eq!(session.device, "Android");
        assert_eq!(session.sdk, "SDK 30");
        assert_eq!(session.app_version, "1.0.0");
        assert_eq!(session.lang_code, "en");
        assert_eq!(session.system_lang_code, "en-US");
        assert!(session.has_proxy());
    }

    #[test]
    fn test_session_data_without_proxy() {
        let api_id = ApiId::new(12345).unwrap();

        let session = SessionData::new(
            api_id,
            "Android".to_string(),
            "SDK 30".to_string(),
            "1.0.0".to_string(),
            "en".to_string(),
            "en-US".to_string(),
            None,
        );

        assert!(!session.has_proxy());
    }

    // Serde Serialization Tests
    #[test]
    fn test_api_id_serde() {
        let api_id = ApiId::new(12345).unwrap();
        let json = serde_json::to_string(&api_id).unwrap();
        assert_eq!(json, "12345");

        let deserialized: ApiId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, api_id);
    }

    #[test]
    fn test_chat_id_serde() {
        let chat_id = ChatId::new(-1001234567890).unwrap();
        let json = serde_json::to_string(&chat_id).unwrap();
        assert_eq!(json, "-1001234567890");

        let deserialized: ChatId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, chat_id);
    }

    #[test]
    fn test_message_id_serde() {
        let msg_id = MessageId::new(42).unwrap();
        let json = serde_json::to_string(&msg_id).unwrap();
        assert_eq!(json, "42");

        let deserialized: MessageId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, msg_id);
    }

    #[test]
    fn test_user_id_serde() {
        let user_id = UserId::new(123456789).unwrap();
        let json = serde_json::to_string(&user_id).unwrap();
        assert_eq!(json, "123456789");

        let deserialized: UserId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, user_id);
    }

    #[test]
    fn test_proxy_url_serde() {
        let proxy = ProxyUrl::new("socks5://user:pass@proxy.com:1080".to_string()).unwrap();
        let json = serde_json::to_string(&proxy).unwrap();

        // Should serialize the actual URL (not redacted)
        assert!(json.contains("socks5://user:pass@proxy.com:1080"));

        let deserialized: ProxyUrl = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.as_str(), proxy.as_str());
    }

    #[test]
    fn test_proxy_url_serde_validation() {
        // Invalid proxy URL should fail deserialization
        let invalid_json = r#""http://proxy.com:1080""#;
        let result: Result<ProxyUrl, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_telegram_update_serde() {
        let chat_id = ChatId::new(123).unwrap();
        let msg_id = MessageId::new(456).unwrap();

        let update = TelegramUpdate::new_message(
            chat_id,
            msg_id,
            "Hello".to_string(),
            None,
            1234567890,
        );

        let json = serde_json::to_string(&update).unwrap();
        let deserialized: TelegramUpdate = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.update_type, update.update_type);
        assert_eq!(deserialized.chat_id, update.chat_id);
        assert_eq!(deserialized.message_id, update.message_id);
        assert_eq!(deserialized.text, update.text);
    }

    // TryFrom Tests
    #[test]
    fn test_api_id_try_from() {
        let result: Result<ApiId, _> = 12345_i32.try_into();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().as_i32(), 12345);

        let result: Result<ApiId, _> = 0_i32.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_chat_id_try_from() {
        let result: Result<ChatId, _> = 123_i64.try_into();
        assert!(result.is_ok());

        let result: Result<ChatId, _> = 0_i64.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_message_id_try_from() {
        let result: Result<MessageId, _> = 42_i32.try_into();
        assert!(result.is_ok());

        let result: Result<MessageId, _> = 0_i32.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_user_id_try_from() {
        let result: Result<UserId, _> = 123456789_i64.try_into();
        assert!(result.is_ok());

        let result: Result<UserId, _> = 0_i64.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_proxy_url_try_from_string() {
        let result: Result<ProxyUrl, _> = "socks5://proxy.com:1080".to_string().try_into();
        assert!(result.is_ok());

        let result: Result<ProxyUrl, _> = "http://proxy.com:1080".to_string().try_into();
        assert!(result.is_err());
    }

    // NewType Inner Value Access
    #[test]
    fn test_into_inner_methods() {
        let api_id = ApiId::new(12345).unwrap();
        assert_eq!(api_id.into_inner(), 12345);

        let chat_id = ChatId::new(123).unwrap();
        assert_eq!(chat_id.into_inner(), 123);

        let msg_id = MessageId::new(42).unwrap();
        assert_eq!(msg_id.into_inner(), 42);

        let user_id = UserId::new(789).unwrap();
        assert_eq!(user_id.into_inner(), 789);

        let proxy = ProxyUrl::new("socks5://proxy.com:1080".to_string()).unwrap();
        assert_eq!(proxy.into_inner(), "socks5://proxy.com:1080");
    }

    // Domain Error Tests
    #[test]
    fn test_domain_error_display() {
        let err = DomainError::InvalidApiId(-1);
        assert!(err.to_string().contains("Invalid API ID"));
        assert!(err.to_string().contains("-1"));

        let err = DomainError::InvalidChatId(0);
        assert!(err.to_string().contains("Invalid Chat ID"));
        assert!(err.to_string().contains("0"));

        let err = DomainError::InvalidMessageId(-1);
        assert!(err.to_string().contains("Invalid Message ID"));
        assert!(err.to_string().contains("-1"));

        let err = DomainError::InvalidUserId(0);
        assert!(err.to_string().contains("Invalid User ID"));
        assert!(err.to_string().contains("0"));

        let err = DomainError::InvalidProxyUrl("Missing port".to_string());
        assert!(err.to_string().contains("Invalid Proxy URL"));
        assert!(err.to_string().contains("Missing port"));
    }

    #[test]
    fn test_domain_error_is_error_trait() {
        use std::error::Error;

        let err: Box<dyn Error> = Box::new(DomainError::InvalidApiId(-1));
        assert!(err.to_string().len() > 0);
    }
}
