//! Configuration sources.
//!
//! Environment variables are the single supported configuration source. They are
//! read explicitly so that every required variable produces a precise error
//! instead of a generic deserialisation failure.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::error::ConfigError;
use super::settings::{Config, RawApp, RawConfig, RawDatabase, RawRedis};

/// Options that control configuration loading.
#[derive(Debug, Clone, Default)]
pub struct LoadOptions {
    /// Path to a dotenv file to load before reading variables.
    pub env_file: Option<PathBuf>,
}

impl LoadOptions {
    /// Loads the default `.env` file when it exists.
    #[must_use]
    pub fn with_default_env_file() -> Self {
        Self {
            env_file: Some(PathBuf::from(".env")),
        }
    }

    /// Disables dotenv loading; only the process environment is used.
    #[must_use]
    pub fn without_env_file() -> Self {
        Self { env_file: None }
    }
}

/// Loads and validates configuration from the environment.
pub fn load(options: &LoadOptions) -> Result<Config, ConfigError> {
    load_dotenv(options.env_file.as_deref())?;

    let collected: HashMap<String, String> = std::env::vars().collect();
    from_map(&collected)
}

fn load_dotenv(path: Option<&Path>) -> Result<(), ConfigError> {
    let Some(path) = path else {
        return Ok(());
    };

    if !path.exists() {
        tracing::debug!(
            target: "platform.config",
            path = %path.display(),
            "dotenv file not found, relying on the process environment"
        );
        return Ok(());
    }

    dotenvy::from_path(path).map_err(|error| ConfigError::Invalid {
        key: "APP_ENV_FILE".to_owned(),
        reason: format!("failed to read {}: {error}", path.display()),
    })?;

    Ok(())
}

/// Validates configuration from an explicit variable map.
///
/// Kept public to the crate so that tests can exercise loading without mutating
/// process-wide state.
pub(crate) fn from_map(variables: &HashMap<String, String>) -> Result<Config, ConfigError> {
    let raw = RawConfig {
        app: RawApp {
            env: required(variables, "APP_ENV")?,
            host: required(variables, "APP_HOST")?,
            port: parse_required(variables, "APP_PORT")?,
            log_format: optional(variables, "APP_LOG_FORMAT")
                .unwrap_or_else(|| "pretty".to_owned()),
            request_body_limit_bytes: parse_optional(variables, "APP_REQUEST_BODY_LIMIT_BYTES")?
                .unwrap_or(1024 * 1024),
            cors_allowed_origins: split_list(&required(variables, "APP_CORS_ALLOWED_ORIGINS")?),
            shutdown_timeout_seconds: parse_optional(variables, "APP_SHUTDOWN_TIMEOUT_SECONDS")?
                .unwrap_or(30),
            readiness_timeout_ms: parse_optional(variables, "APP_READINESS_TIMEOUT_MS")?
                .unwrap_or(2000),
            ws_heartbeat_seconds: parse_optional(variables, "APP_WS_HEARTBEAT_SECONDS")?
                .unwrap_or(30),
            swagger_enabled: parse_optional(variables, "APP_SWAGGER_ENABLED")?,
        },
        database: RawDatabase {
            url: required(variables, "DATABASE_URL")?,
            max_connections: parse_optional(variables, "DATABASE_MAX_CONNECTIONS")?.unwrap_or(10),
            min_connections: parse_optional(variables, "DATABASE_MIN_CONNECTIONS")?.unwrap_or(1),
            acquire_timeout_seconds: parse_optional(variables, "DATABASE_ACQUIRE_TIMEOUT_SECONDS")?
                .unwrap_or(5),
            migrate: parse_optional(variables, "DATABASE_MIGRATE")?.unwrap_or(true),
        },
        redis: RawRedis {
            url: required(variables, "REDIS_URL")?,
            timeout_ms: parse_optional(variables, "REDIS_TIMEOUT_MS")?.unwrap_or(2000),
        },
        frontend_url: required(variables, "FRONTEND_URL")?,
        rust_log: optional(variables, "RUST_LOG").unwrap_or_else(|| "info".to_owned()),
    };

    raw.validate()
}

fn required(variables: &HashMap<String, String>, key: &str) -> Result<String, ConfigError> {
    match optional(variables, key) {
        Some(value) => Ok(value),
        None => Err(ConfigError::Missing(key.to_owned())),
    }
}

fn optional(variables: &HashMap<String, String>, key: &str) -> Option<String> {
    variables
        .get(key)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn parse_required<T>(variables: &HashMap<String, String>, key: &str) -> Result<T, ConfigError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = required(variables, key)?;
    value.parse::<T>().map_err(|error| ConfigError::Invalid {
        key: key.to_owned(),
        reason: error.to_string(),
    })
}

fn parse_optional<T>(
    variables: &HashMap<String, String>,
    key: &str,
) -> Result<Option<T>, ConfigError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let Some(value) = optional(variables, key) else {
        return Ok(None);
    };

    value
        .parse::<T>()
        .map(Some)
        .map_err(|error| ConfigError::Invalid {
            key: key.to_owned(),
            reason: error.to_string(),
        })
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{LoadOptions, from_map};
    use crate::config::{AppEnvironment, ConfigError, LogFormat};

    fn base_variables() -> HashMap<String, String> {
        HashMap::from([
            ("APP_ENV".to_owned(), "development".to_owned()),
            ("APP_HOST".to_owned(), "0.0.0.0".to_owned()),
            ("APP_PORT".to_owned(), "8080".to_owned()),
            (
                "APP_CORS_ALLOWED_ORIGINS".to_owned(),
                "http://localhost:5173, http://localhost:4173".to_owned(),
            ),
            (
                "DATABASE_URL".to_owned(),
                "postgres://platform:password@postgres:5432/platform".to_owned(),
            ),
            ("REDIS_URL".to_owned(), "redis://redis:6379".to_owned()),
            (
                "FRONTEND_URL".to_owned(),
                "http://localhost:5173".to_owned(),
            ),
            ("RUST_LOG".to_owned(), "info,platform=debug".to_owned()),
        ])
    }

    #[test]
    fn loads_minimal_environment_with_defaults() {
        let config = from_map(&base_variables()).expect("configuration should load");

        assert_eq!(config.environment, AppEnvironment::Development);
        assert_eq!(config.http.port, 8080);
        assert_eq!(
            config.http.cors.allowed_origins,
            vec![
                "http://localhost:5173".to_owned(),
                "http://localhost:4173".to_owned()
            ]
        );
        assert_eq!(config.logging.filter, "info,platform=debug");
        assert_eq!(config.logging.format, LogFormat::Pretty);
        assert_eq!(config.database.max_connections, 10);
        assert!(config.database.run_migrations);
        assert_eq!(config.redis.timeout.as_millis(), 2000);
    }

    #[test]
    fn missing_required_variable_names_the_variable() {
        let mut variables = base_variables();
        variables.remove("DATABASE_URL");

        let error = from_map(&variables).expect_err("missing variable must fail");

        assert!(matches!(
            &error,
            ConfigError::Missing(key) if key == "DATABASE_URL"
        ));
    }

    #[test]
    fn blank_required_variable_is_reported_as_missing() {
        let mut variables = base_variables();
        variables.insert("REDIS_URL".to_owned(), "   ".to_owned());

        let error = from_map(&variables).expect_err("blank variable must fail");
        assert_eq!(
            error.to_string(),
            "missing required configuration variable: REDIS_URL"
        );
    }

    #[test]
    fn non_numeric_port_is_rejected() {
        let mut variables = base_variables();
        variables.insert("APP_PORT".to_owned(), "eighty".to_owned());

        let error = from_map(&variables).expect_err("non numeric port must fail");
        assert!(error.to_string().contains("APP_PORT"));
    }

    #[test]
    fn non_boolean_migration_flag_is_rejected() {
        let mut variables = base_variables();
        variables.insert("DATABASE_MIGRATE".to_owned(), "yes-please".to_owned());

        let error = from_map(&variables).expect_err("invalid boolean must fail");
        assert!(error.to_string().contains("DATABASE_MIGRATE"));
    }

    #[test]
    fn json_log_format_is_supported() {
        let mut variables = base_variables();
        variables.insert("APP_LOG_FORMAT".to_owned(), "json".to_owned());

        let config = from_map(&variables).unwrap();
        assert_eq!(config.logging.format, LogFormat::Json);
    }

    #[test]
    fn test_environment_keeps_swagger_enabled() {
        let mut variables = base_variables();
        variables.insert("APP_ENV".to_owned(), "test".to_owned());

        let config = from_map(&variables).unwrap();
        assert!(config.swagger_enabled);
    }

    #[test]
    fn explicit_swagger_flag_is_honoured_outside_production() {
        let mut variables = base_variables();
        variables.insert("APP_SWAGGER_ENABLED".to_owned(), "false".to_owned());

        let config = from_map(&variables).unwrap();
        assert!(!config.swagger_enabled);
    }

    #[test]
    fn missing_env_file_is_not_fatal() {
        let options = LoadOptions {
            env_file: Some(std::path::PathBuf::from("/nonexistent/.env.platform-test")),
        };

        let result = super::load_dotenv(options.env_file.as_deref());
        assert!(result.is_ok(), "absent dotenv files must be tolerated");
    }

    #[test]
    fn invalid_env_file_is_reported() {
        let directory = std::env::temp_dir().join("platform-config-test");
        std::fs::create_dir_all(&directory).expect("temp dir should be created");
        let path = directory.join(".env");
        std::fs::write(&path, "INVALID_LINE_WITHOUT_EQUALS\n").expect("file should be written");

        let result = super::load_dotenv(Some(&path));
        assert!(result.is_err(), "malformed dotenv files must be reported");

        let _ = std::fs::remove_file(&path);
    }
}
