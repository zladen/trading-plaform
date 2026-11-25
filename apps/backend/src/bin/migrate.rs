//! Applies the embedded database migrations and exits.
//!
//! The backend applies migrations during startup by default, so this binary is
//! not required for a normal run. It exists for operators who keep
//! `DATABASE_MIGRATE=false` and want an explicit, auditable migration step
//! (for example in a deployment pipeline), and for `make migrate`.
//!
//! Only `DATABASE_URL` and, optionally, the pool sizing variables are read. The
//! HTTP and CORS settings are irrelevant to a migration, so requiring them would
//! make the command needlessly hard to use.

use std::process::ExitCode;
use std::time::Duration;

use platform_storage::PostgresSettings;
use tracing::info;

const DEFAULT_MAX_CONNECTIONS: u32 = 2;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("migration failed: {error:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let settings = PostgresSettings {
        url: require("DATABASE_URL")?,
        max_connections: read_optional("DATABASE_MAX_CONNECTIONS")?
            .unwrap_or(DEFAULT_MAX_CONNECTIONS),
        min_connections: 1,
        acquire_timeout: Duration::from_secs(
            read_optional::<u64>("DATABASE_ACQUIRE_TIMEOUT_SECONDS")?.unwrap_or(5),
        ),
        run_migrations: true,
    };

    let pool = platform_storage::postgres::connect(&settings).await?;
    platform_storage::postgres::run_migrations(&pool).await?;

    info!(target: "platform.migrate", "migrations applied");
    println!("migrations applied");

    pool.close().await;

    Ok(())
}

fn require(key: &str) -> anyhow::Result<String> {
    std::env::var(key)
        .map(|value| value.trim().to_owned())
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing required environment variable {key}"))
}

fn read_optional<T>(key: &str) -> anyhow::Result<Option<T>>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let Ok(raw) = std::env::var(key) else {
        return Ok(None);
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    trimmed
        .parse::<T>()
        .map(Some)
        .map_err(|error| anyhow::anyhow!("invalid value for {key}: {error}"))
}
