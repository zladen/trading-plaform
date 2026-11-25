//! Typed, validated configuration.

use std::fmt;
use std::time::Duration;

use serde::Deserialize;
use url::Url;

use super::error::ConfigError;

/// Deployment environment the process runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEnvironment {
    /// Local development with permissive defaults and Swagger UI enabled.
    Development,
    /// Automated test runs; Swagger UI enabled, no outbound calls.
    Test,
    /// Production; Swagger UI disabled and stricter validation.
    Production,
}

impl AppEnvironment {
    /// Parses an environment name.
    pub fn parse(raw: &str) -> Result<Self, ConfigError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "development" | "dev" => Ok(Self::Development),
            "test" => Ok(Self::Test),
            "production" | "prod" => Ok(Self::Production),
            _ => Err(ConfigError::Invalid {
                key: "APP_ENV".to_owned(),
                reason: format!("expected one of development, test, production; received {raw:?}"),
            }),
        }
    }

    /// Stable identifier used in API responses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Test => "test",
            Self::Production => "production",
        }
    }
}

impl fmt::Display for AppEnvironment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Human readable, colourised output for local development.
    Pretty,
    /// Newline delimited JSON for log collectors.
    Json,
}

impl LogFormat {
    /// Parses a log format name.
    pub fn parse(raw: &str) -> Result<Self, ConfigError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "pretty" | "text" => Ok(Self::Pretty),
            "json" => Ok(Self::Json),
            _ => Err(ConfigError::Invalid {
                key: "APP_LOG_FORMAT".to_owned(),
                reason: format!("expected one of pretty, json; received {raw:?}"),
            }),
        }
    }
}

/// HTTP listener and middleware settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpConfig {
    /// Bind address.
    pub host: String,
    /// Bind port.
    pub port: u16,
    /// Maximum accepted request body size in bytes.
    pub request_body_limit_bytes: usize,
    /// Origins allowed by CORS.
    pub cors: CorsConfig,
}

/// CORS settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorsConfig {
    /// Explicit list of allowed origins. Wildcards are rejected.
    pub allowed_origins: Vec<String>,
}

/// PostgreSQL settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseConfig {
    /// Connection URL including credentials.
    pub url: String,
    /// Maximum pooled connections.
    pub max_connections: u32,
    /// Minimum warm connections.
    pub min_connections: u32,
    /// Deadline for acquiring a pooled connection.
    pub acquire_timeout: Duration,
    /// Whether embedded migrations run at startup.
    pub run_migrations: bool,
}

/// Redis settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisConfig {
    /// Connection URL including credentials.
    pub url: String,
    /// Deadline for a single command or connect attempt.
    pub timeout: Duration,
}

/// Logging settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoggingConfig {
    /// `tracing` filter directive, for example `info` or `platform=debug`.
    pub filter: String,
    /// Output format.
    pub format: LogFormat,
}

/// Graceful shutdown settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShutdownConfig {
    /// Maximum time to wait for in-flight work during shutdown.
    pub timeout: Duration,
}

/// WebSocket settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WebSocketConfig {
    /// Interval between server initiated heartbeats.
    pub heartbeat_interval: Duration,
}

/// Settings for the `/info` use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoConfig {
    /// Environment name exposed to clients.
    pub environment: AppEnvironment,
}

/// Fully validated application configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Deployment environment.
    pub environment: AppEnvironment,
    /// HTTP listener and middleware settings.
    pub http: HttpConfig,
    /// Logging settings.
    pub logging: LoggingConfig,
    /// PostgreSQL settings.
    pub database: DatabaseConfig,
    /// Redis settings.
    pub redis: RedisConfig,
    /// Origin of the frontend application, used by CORS defaults.
    pub frontend_url: String,
    /// Graceful shutdown settings.
    pub shutdown: ShutdownConfig,
    /// Per-dependency readiness probe deadline.
    pub readiness_timeout: Duration,
    /// WebSocket settings.
    pub websocket: WebSocketConfig,
    /// Whether Swagger UI is mounted.
    pub swagger_enabled: bool,
}

impl Config {
    /// Returns `true` when running in production.
    #[must_use]
    pub fn is_production(&self) -> bool {
        matches!(self.environment, AppEnvironment::Production)
    }

    /// Settings consumed by the `/info` use case.
    #[must_use]
    pub fn info(&self) -> InfoConfig {
        InfoConfig {
            environment: self.environment,
        }
    }

    /// Socket address description for logging.
    #[must_use]
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.http.host, self.http.port)
    }

    /// Converts the validated settings into storage settings.
    #[must_use]
    pub fn postgres_settings(&self) -> platform_storage::PostgresSettings {
        platform_storage::PostgresSettings {
            url: self.database.url.clone(),
            max_connections: self.database.max_connections,
            min_connections: self.database.min_connections,
            acquire_timeout: self.database.acquire_timeout,
            run_migrations: self.database.run_migrations,
        }
    }

    /// Converts the validated settings into Redis settings.
    #[must_use]
    pub fn redis_settings(&self) -> platform_storage::RedisSettings {
        platform_storage::RedisSettings {
            url: self.redis.url.clone(),
            timeout: self.redis.timeout,
        }
    }
}

/// Deserialisation target for the resolved configuration tree.
#[derive(Debug, Deserialize)]
pub(crate) struct RawConfig {
    pub(crate) app: RawApp,
    pub(crate) database: RawDatabase,
    pub(crate) redis: RawRedis,
    pub(crate) frontend_url: String,
    pub(crate) rust_log: String,
}

/// Raw application section.
#[derive(Debug, Deserialize)]
pub(crate) struct RawApp {
    pub(crate) env: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) log_format: String,
    pub(crate) request_body_limit_bytes: u64,
    pub(crate) cors_allowed_origins: Vec<String>,
    pub(crate) shutdown_timeout_seconds: u64,
    pub(crate) readiness_timeout_ms: u64,
    pub(crate) ws_heartbeat_seconds: u64,
    pub(crate) swagger_enabled: Option<bool>,
}

/// Raw database section.
#[derive(Debug, Deserialize)]
pub(crate) struct RawDatabase {
    pub(crate) url: String,
    pub(crate) max_connections: u32,
    pub(crate) min_connections: u32,
    pub(crate) acquire_timeout_seconds: u64,
    pub(crate) migrate: bool,
}

/// Raw Redis section.
#[derive(Debug, Deserialize)]
pub(crate) struct RawRedis {
    pub(crate) url: String,
    pub(crate) timeout_ms: u64,
}

/// Maximum accepted request body size: 16 MiB.
const MAX_BODY_LIMIT_BYTES: u64 = 16 * 1024 * 1024;

/// Lowest acceptable request body limit: 1 KiB.
const MIN_BODY_LIMIT_BYTES: u64 = 1024;

impl RawConfig {
    /// Validates the raw values and produces typed settings.
    pub(crate) fn validate(self) -> Result<Config, ConfigError> {
        let environment = AppEnvironment::parse(&self.app.env)?;
        let log_format = LogFormat::parse(&self.app.log_format)?;

        validate_non_empty("APP_HOST", &self.app.host)?;
        validate_non_empty("FRONTEND_URL", &self.frontend_url)?;
        validate_non_empty("RUST_LOG", &self.rust_log)?;

        if self.app.port == 0 {
            return Err(ConfigError::Invalid {
                key: "APP_PORT".to_owned(),
                reason: "port must be between 1 and 65535".to_owned(),
            });
        }

        if !(MIN_BODY_LIMIT_BYTES..=MAX_BODY_LIMIT_BYTES)
            .contains(&self.app.request_body_limit_bytes)
        {
            return Err(ConfigError::Invalid {
                key: "APP_REQUEST_BODY_LIMIT_BYTES".to_owned(),
                reason: format!(
                    "must be between {MIN_BODY_LIMIT_BYTES} and {MAX_BODY_LIMIT_BYTES} bytes"
                ),
            });
        }

        if self.app.cors_allowed_origins.is_empty() {
            return Err(ConfigError::Missing("APP_CORS_ALLOWED_ORIGINS".to_owned()));
        }

        for origin in &self.app.cors_allowed_origins {
            validate_origin(origin)?;
        }

        if self.app.shutdown_timeout_seconds == 0 {
            return Err(ConfigError::Invalid {
                key: "APP_SHUTDOWN_TIMEOUT_SECONDS".to_owned(),
                reason: "must be greater than zero".to_owned(),
            });
        }

        if self.app.readiness_timeout_ms == 0 {
            return Err(ConfigError::Invalid {
                key: "APP_READINESS_TIMEOUT_MS".to_owned(),
                reason: "must be greater than zero".to_owned(),
            });
        }

        if self.app.ws_heartbeat_seconds == 0 {
            return Err(ConfigError::Invalid {
                key: "APP_WS_HEARTBEAT_SECONDS".to_owned(),
                reason: "must be greater than zero".to_owned(),
            });
        }

        validate_url_scheme(
            "DATABASE_URL",
            &self.database.url,
            &["postgres", "postgresql"],
        )?;
        validate_url_scheme("REDIS_URL", &self.redis.url, &["redis", "rediss"])?;

        if self.database.max_connections == 0 {
            return Err(ConfigError::Invalid {
                key: "DATABASE_MAX_CONNECTIONS".to_owned(),
                reason: "must be greater than zero".to_owned(),
            });
        }

        if self.database.min_connections > self.database.max_connections {
            return Err(ConfigError::Invalid {
                key: "DATABASE_MIN_CONNECTIONS".to_owned(),
                reason: "must not exceed DATABASE_MAX_CONNECTIONS".to_owned(),
            });
        }

        if self.database.acquire_timeout_seconds == 0 {
            return Err(ConfigError::Invalid {
                key: "DATABASE_ACQUIRE_TIMEOUT_SECONDS".to_owned(),
                reason: "must be greater than zero".to_owned(),
            });
        }

        if self.redis.timeout_ms == 0 {
            return Err(ConfigError::Invalid {
                key: "REDIS_TIMEOUT_MS".to_owned(),
                reason: "must be greater than zero".to_owned(),
            });
        }

        let swagger_enabled = self
            .app
            .swagger_enabled
            .unwrap_or(!matches!(environment, AppEnvironment::Production));

        if swagger_enabled && matches!(environment, AppEnvironment::Production) {
            return Err(ConfigError::Invalid {
                key: "APP_SWAGGER_ENABLED".to_owned(),
                reason: "Swagger UI must stay disabled in production".to_owned(),
            });
        }

        Ok(Config {
            environment,
            http: HttpConfig {
                host: self.app.host.trim().to_owned(),
                port: self.app.port,
                request_body_limit_bytes: usize::try_from(self.app.request_body_limit_bytes)
                    .map_err(|_| ConfigError::Invalid {
                        key: "APP_REQUEST_BODY_LIMIT_BYTES".to_owned(),
                        reason: "value does not fit the platform usize".to_owned(),
                    })?,
                cors: CorsConfig {
                    allowed_origins: self
                        .app
                        .cors_allowed_origins
                        .iter()
                        .map(|origin| origin.trim().to_owned())
                        .collect(),
                },
            },
            logging: LoggingConfig {
                filter: self.rust_log.trim().to_owned(),
                format: log_format,
            },
            database: DatabaseConfig {
                url: self.database.url.trim().to_owned(),
                max_connections: self.database.max_connections,
                min_connections: self.database.min_connections,
                acquire_timeout: Duration::from_secs(self.database.acquire_timeout_seconds),
                run_migrations: self.database.migrate,
            },
            redis: RedisConfig {
                url: self.redis.url.trim().to_owned(),
                timeout: Duration::from_millis(self.redis.timeout_ms),
            },
            frontend_url: self.frontend_url.trim().trim_end_matches('/').to_owned(),
            shutdown: ShutdownConfig {
                timeout: Duration::from_secs(self.app.shutdown_timeout_seconds),
            },
            readiness_timeout: Duration::from_millis(self.app.readiness_timeout_ms),
            websocket: WebSocketConfig {
                heartbeat_interval: Duration::from_secs(self.app.ws_heartbeat_seconds),
            },
            swagger_enabled,
        })
    }
}

fn validate_non_empty(key: &str, value: &str) -> Result<(), ConfigError> {
    if value.trim().is_empty() {
        return Err(ConfigError::Invalid {
            key: key.to_owned(),
            reason: "must not be empty".to_owned(),
        });
    }

    Ok(())
}

fn validate_origin(origin: &str) -> Result<(), ConfigError> {
    let parsed = Url::parse(origin.trim()).map_err(|error| ConfigError::Invalid {
        key: "APP_CORS_ALLOWED_ORIGINS".to_owned(),
        reason: format!("{origin:?} is not a valid origin: {error}"),
    })?;

    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ConfigError::Invalid {
            key: "APP_CORS_ALLOWED_ORIGINS".to_owned(),
            reason: format!("{origin:?} must use http or https"),
        });
    }

    if origin.contains('*') {
        return Err(ConfigError::Invalid {
            key: "APP_CORS_ALLOWED_ORIGINS".to_owned(),
            reason: "wildcard origins are not allowed".to_owned(),
        });
    }

    if parsed.path() != "/" || parsed.query().is_some() || parsed.fragment().is_some() {
        return Err(ConfigError::Invalid {
            key: "APP_CORS_ALLOWED_ORIGINS".to_owned(),
            reason: format!("{origin:?} must be an origin without path, query or fragment"),
        });
    }

    Ok(())
}

fn validate_url_scheme(key: &str, raw: &str, allowed: &[&str]) -> Result<(), ConfigError> {
    validate_non_empty(key, raw)?;

    let parsed = Url::parse(raw).map_err(|error| ConfigError::Invalid {
        key: key.to_owned(),
        reason: format!("not a valid URL: {error}"),
    })?;

    if !allowed.contains(&parsed.scheme()) {
        return Err(ConfigError::Invalid {
            key: key.to_owned(),
            reason: format!(
                "scheme {:?} is not supported; expected one of {:?}",
                parsed.scheme(),
                allowed
            ),
        });
    }

    if parsed.host_str().is_none_or(str::is_empty) {
        return Err(ConfigError::Invalid {
            key: key.to_owned(),
            reason: "URL must contain a host".to_owned(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{AppEnvironment, Config, LogFormat, RawApp, RawConfig, RawDatabase, RawRedis};

    fn raw_config() -> RawConfig {
        RawConfig {
            app: RawApp {
                env: "development".to_owned(),
                host: "0.0.0.0".to_owned(),
                port: 8080,
                log_format: "pretty".to_owned(),
                request_body_limit_bytes: 1024 * 1024,
                cors_allowed_origins: vec!["http://localhost:5173".to_owned()],
                shutdown_timeout_seconds: 30,
                readiness_timeout_ms: 2000,
                ws_heartbeat_seconds: 30,
                swagger_enabled: None,
            },
            database: RawDatabase {
                url: "postgres://platform:password@postgres:5432/platform".to_owned(),
                max_connections: 10,
                min_connections: 1,
                acquire_timeout_seconds: 5,
                migrate: true,
            },
            redis: RawRedis {
                url: "redis://redis:6379".to_owned(),
                timeout_ms: 2000,
            },
            frontend_url: "http://localhost:5173/".to_owned(),
            rust_log: "info".to_owned(),
        }
    }

    fn validate(mutate: impl FnOnce(&mut RawConfig)) -> Result<Config, super::ConfigError> {
        let mut raw = raw_config();
        mutate(&mut raw);
        raw.validate()
    }

    #[test]
    fn valid_configuration_is_accepted() {
        let config = validate(|_| {}).expect("configuration should be valid");

        assert_eq!(config.environment, AppEnvironment::Development);
        assert_eq!(config.logging.format, LogFormat::Pretty);
        assert_eq!(config.http.port, 8080);
        assert_eq!(config.http.request_body_limit_bytes, 1024 * 1024);
        assert_eq!(config.database.acquire_timeout, Duration::from_secs(5));
        assert_eq!(config.redis.timeout, Duration::from_millis(2000));
        assert_eq!(config.frontend_url, "http://localhost:5173");
        assert!(config.swagger_enabled, "dev keeps Swagger UI enabled");
        assert!(!config.is_production());
    }

    #[test]
    fn environment_aliases_are_supported() {
        assert_eq!(
            AppEnvironment::parse("DEV").unwrap(),
            AppEnvironment::Development
        );
        assert_eq!(
            AppEnvironment::parse("prod").unwrap(),
            AppEnvironment::Production
        );
        assert!(AppEnvironment::parse("staging").is_err());
    }

    #[test]
    fn invalid_port_is_rejected() {
        let error = validate(|raw| raw.app.port = 0).expect_err("port 0 must be rejected");
        assert_eq!(
            error.to_string(),
            "invalid value for configuration variable APP_PORT: port must be between 1 and 65535"
        );
    }

    #[test]
    fn empty_database_url_is_rejected() {
        let error =
            validate(|raw| raw.database.url = "   ".to_owned()).expect_err("blank URL must fail");
        assert!(error.to_string().contains("DATABASE_URL"));
    }

    #[test]
    fn non_postgres_database_scheme_is_rejected() {
        let error = validate(|raw| raw.database.url = "mysql://db/platform".to_owned())
            .expect_err("mysql must be rejected");
        assert!(error.to_string().contains("DATABASE_URL"));
    }

    #[test]
    fn non_redis_scheme_is_rejected() {
        let error = validate(|raw| raw.redis.url = "http://redis:6379".to_owned())
            .expect_err("http must be rejected");
        assert!(error.to_string().contains("REDIS_URL"));
    }

    #[test]
    fn wildcard_cors_origin_is_rejected() {
        let error = validate(|raw| {
            raw.app.cors_allowed_origins = vec!["*".to_owned()];
        })
        .expect_err("wildcard CORS must be rejected");
        assert!(error.to_string().contains("APP_CORS_ALLOWED_ORIGINS"));
    }

    #[test]
    fn cors_origin_with_path_is_rejected() {
        let error = validate(|raw| {
            raw.app.cors_allowed_origins = vec!["http://localhost:5173/app".to_owned()];
        })
        .expect_err("origin with path must be rejected");
        assert!(error.to_string().contains("path"));
    }

    #[test]
    fn empty_cors_list_is_rejected() {
        let error = validate(|raw| raw.app.cors_allowed_origins.clear())
            .expect_err("empty CORS list must fail");
        assert_eq!(
            error.to_string(),
            "missing required configuration variable: APP_CORS_ALLOWED_ORIGINS"
        );
    }

    #[test]
    fn min_connections_above_max_is_rejected() {
        let error = validate(|raw| {
            raw.database.min_connections = 20;
            raw.database.max_connections = 10;
        })
        .expect_err("min>max must be rejected");
        assert!(error.to_string().contains("DATABASE_MIN_CONNECTIONS"));
    }

    #[test]
    fn body_limit_outside_bounds_is_rejected() {
        assert!(validate(|raw| raw.app.request_body_limit_bytes = 0).is_err());
        assert!(validate(|raw| raw.app.request_body_limit_bytes = 64 * 1024 * 1024).is_err());
    }

    #[test]
    fn production_disables_swagger_ui() {
        let config = validate(|raw| raw.app.env = "production".to_owned())
            .expect("production config should be valid");
        assert!(!config.swagger_enabled);
        assert!(config.is_production());
    }

    #[test]
    fn production_rejects_explicit_swagger_enablement() {
        let error = validate(|raw| {
            raw.app.env = "production".to_owned();
            raw.app.swagger_enabled = Some(true);
        })
        .expect_err("production must not enable Swagger UI");
        assert!(error.to_string().contains("APP_SWAGGER_ENABLED"));
    }

    #[test]
    fn zero_timeouts_are_rejected() {
        assert!(validate(|raw| raw.app.readiness_timeout_ms = 0).is_err());
        assert!(validate(|raw| raw.app.shutdown_timeout_seconds = 0).is_err());
        assert!(validate(|raw| raw.app.ws_heartbeat_seconds = 0).is_err());
        assert!(validate(|raw| raw.redis.timeout_ms = 0).is_err());
        assert!(validate(|raw| raw.database.acquire_timeout_seconds = 0).is_err());
    }

    #[test]
    fn bind_address_joins_host_and_port() {
        let config = validate(|_| {}).unwrap();
        assert_eq!(config.bind_address(), "0.0.0.0:8080");
    }

    #[test]
    fn storage_settings_are_derived_from_configuration() {
        let config = validate(|_| {}).unwrap();
        let postgres = config.postgres_settings();
        let redis = config.redis_settings();

        assert_eq!(postgres.max_connections, 10);
        assert!(postgres.run_migrations);
        assert_eq!(redis.timeout, Duration::from_millis(2000));
    }

    #[test]
    fn invalid_log_format_is_rejected() {
        let error = validate(|raw| raw.app.log_format = "xml".to_owned())
            .expect_err("xml must be rejected");
        assert!(error.to_string().contains("APP_LOG_FORMAT"));
    }
}
