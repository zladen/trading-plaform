//! Centralised backend configuration.
//!
//! Configuration is resolved once at startup from environment variables (and an
//! optional `.env` file), validated, and then passed to the runtime as an
//! immutable value. Nothing below the startup layer reads the environment.

mod error;
mod settings;
mod source;

pub use error::ConfigError;
pub use settings::{
    AppEnvironment, Config, CorsConfig, DatabaseConfig, HttpConfig, InfoConfig, LogFormat,
    LoggingConfig, RedisConfig, ShutdownConfig, WebSocketConfig,
};
pub use source::{LoadOptions, load};
