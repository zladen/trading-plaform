//! Connection settings for infrastructure adapters.

/// PostgreSQL connection settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresSettings {
    /// Connection URL including credentials.
    pub url: String,
    /// Maximum number of pooled connections.
    pub max_connections: u32,
    /// Minimum number of connections kept warm.
    pub min_connections: u32,
    /// Deadline for acquiring a connection from the pool.
    pub acquire_timeout: std::time::Duration,
    /// Whether embedded migrations run at startup.
    pub run_migrations: bool,
}

impl PostgresSettings {
    /// Returns the connection URL with credentials masked.
    ///
    /// Used for diagnostics only. The full URL must never be logged.
    #[must_use]
    pub fn redacted_url(&self) -> String {
        redact_url(&self.url)
    }
}

/// Redis connection settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisSettings {
    /// Connection URL including credentials.
    pub url: String,
    /// Deadline for a single command or connection attempt.
    pub timeout: std::time::Duration,
}

impl RedisSettings {
    /// Returns the connection URL with credentials masked.
    #[must_use]
    pub fn redacted_url(&self) -> String {
        redact_url(&self.url)
    }
}

/// Replaces the password component of a URL with a placeholder.
///
/// The function is defensive: it never panics on malformed input and returns a
/// fully masked string when parsing fails, because a partially valid URL may
/// still contain a secret.
#[must_use]
pub fn redact_url(raw: &str) -> String {
    let Some(scheme_end) = raw.find("://") else {
        return "<invalid-url>".to_owned();
    };

    let (scheme, rest) = raw.split_at(scheme_end);
    let rest = &rest["://".len()..];

    let (authority, tail) = match rest.find('/') {
        Some(index) => rest.split_at(index),
        None => (rest, ""),
    };

    let Some(at_index) = authority.rfind('@') else {
        return format!("{scheme}://{authority}{tail}");
    };

    let (credentials, host) = authority.split_at(at_index);
    let host = &host["@".len()..];

    match credentials.split_once(':') {
        Some((user, _password)) => format!("{scheme}://{user}:***@{host}{tail}"),
        None => format!("{scheme}://***@{host}{tail}"),
    }
}

#[cfg(test)]
mod tests {
    use super::redact_url;

    #[test]
    fn masks_password() {
        assert_eq!(
            redact_url("postgres://platform:supersecret@postgres:5432/platform"),
            "postgres://platform:***@postgres:5432/platform"
        );
    }

    #[test]
    fn masks_password_without_path() {
        assert_eq!(
            redact_url("redis://default:secret@redis:6379"),
            "redis://default:***@redis:6379"
        );
    }

    #[test]
    fn keeps_urls_without_credentials_intact() {
        assert_eq!(redact_url("redis://redis:6379/0"), "redis://redis:6379/0");
    }

    #[test]
    fn masks_user_only_credentials() {
        assert_eq!(
            redact_url("postgres://token@db:5432/platform"),
            "postgres://***@db:5432/platform"
        );
    }

    #[test]
    fn never_returns_partial_secret_for_invalid_input() {
        assert_eq!(redact_url("supersecret"), "<invalid-url>");
    }
}
