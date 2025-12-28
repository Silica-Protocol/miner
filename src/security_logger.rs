/// Security utilities for sanitizing and redacting sensitive data in logs
pub struct SecurityLogger;

impl SecurityLogger {
    /// Redact sensitive information from a string for safe logging
    pub fn redact_sensitive(data: &str) -> String {
        // Common patterns for sensitive data
        let sensitive_patterns = [
            "password",
            "passwd",
            "secret",
            "key",
            "token",
            "auth",
            "credential",
            "private",
            "api_key",
            "access_token",
        ];

        let data_lower = data.to_lowercase();
        for pattern in &sensitive_patterns {
            if data_lower.contains(pattern) {
                return format!(
                    "{}***{}",
                    &data[..data.len().min(3)],
                    &data[data.len().saturating_sub(3)..]
                );
            }
        }

        data.to_string()
    }

    /// Redact user ID for logging (show first 3 and last 3 characters)
    pub fn redact_user_id(user_id: &str) -> String {
        if user_id.len() < 6 {
            // For short user IDs, show first and last character only
            if user_id.len() <= 2 {
                "*".repeat(user_id.len())
            } else {
                format!("{}***{}", &user_id[..1], &user_id[user_id.len() - 1..])
            }
        } else {
            // For longer user IDs, show first 3 and last 3 characters
            format!("{}***{}", &user_id[..3], &user_id[user_id.len() - 3..])
        }
    }

    /// Redact URL for logging (hide sensitive parts)
    pub fn redact_url(url: &str) -> String {
        // Simple URL redaction without external dependencies
        if url.contains('?') {
            let parts: Vec<&str> = url.split('?').collect();
            if parts.len() >= 2 {
                let base_url = parts[0];
                let query = parts[1];

                let safe_query = query
                    .split('&')
                    .map(|param| {
                        if let Some((key, _)) = param.split_once('=') {
                            let key_lower = key.to_lowercase();
                            if key_lower.contains("token")
                                || key_lower.contains("key")
                                || key_lower.contains("secret")
                                || key_lower.contains("auth")
                                || key_lower.contains("password")
                                || key_lower.contains("credential")
                            {
                                format!("{}=***", key)
                            } else {
                                param.to_string()
                            }
                        } else {
                            param.to_string()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("&");

                format!("{}?{}", base_url, safe_query)
            } else {
                url.to_string()
            }
        } else {
            // No query parameters, URL is likely safe to log as-is
            url.to_string()
        }
    }

    /// Log an info message with automatic sensitive data redaction
    pub fn log_info(message: &str) {
        let redacted_message = Self::redact_sensitive(message);
        tracing::info!("{}", redacted_message);
    }

    /// Log a warning message with automatic sensitive data redaction
    pub fn log_warn(message: &str) {
        let redacted_message = Self::redact_sensitive(message);
        tracing::warn!("{}", redacted_message);
    }

    /// Log an error message with automatic sensitive data redaction
    pub fn log_error(message: &str) {
        let redacted_message = Self::redact_sensitive(message);
        tracing::error!("{}", redacted_message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_user_id() {
        assert_eq!(SecurityLogger::redact_user_id("user123456"), "use***456");
        assert_eq!(SecurityLogger::redact_user_id("ab"), "**");
        assert_eq!(SecurityLogger::redact_user_id("abc"), "a***c");
        assert_eq!(SecurityLogger::redact_user_id("abcd"), "a***d");
        assert_eq!(SecurityLogger::redact_user_id("a"), "*");
        assert_eq!(SecurityLogger::redact_user_id("abcdef"), "abc***def");
    }

    #[test]
    fn test_redact_sensitive() {
        assert_eq!(
            SecurityLogger::redact_sensitive("my_password_123"),
            "my_***123"
        );
        assert_eq!(SecurityLogger::redact_sensitive("secretValue"), "sec***lue");
        assert_eq!(
            SecurityLogger::redact_sensitive("api_key_secret"),
            "api***ret"
        );
        assert_eq!(
            SecurityLogger::redact_sensitive("normal_data"),
            "normal_data"
        );
    }

    #[test]
    fn test_redact_url() {
        let url = "https://api.example.com/endpoint?user=test&token=secret123";
        let redacted = SecurityLogger::redact_url(url);
        assert!(redacted.contains("token=***"));
        assert!(redacted.contains("user=test"));
    }
}
