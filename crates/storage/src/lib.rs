//! Infrastructure adapters.
//!
//! This crate implements the application ports with concrete technology: SQLx
//! for PostgreSQL and `redis-rs` for Redis. It is the only place allowed to know
//! those technologies.

pub mod error;
pub mod postgres;
pub mod redis;
pub mod settings;

pub use error::StorageError;
pub use postgres::{PostgresPool, PostgresProbe};
pub use redis::{RedisClient, RedisProbe};
pub use settings::{PostgresSettings, RedisSettings};
