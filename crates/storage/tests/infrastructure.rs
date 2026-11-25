//! Integration tests against real PostgreSQL and Redis instances.
//!
//! Unlike the unit tests in the crate, these exercise the actual drivers,
//! migrations and probes. They need a live dependency, so they are skipped with
//! an explanatory message when the corresponding variable is absent. CI sets
//! both variables; a developer can point them at `make dev-deps`.
//!
//! ```text
//! TEST_DATABASE_URL=postgres://platform:change-me@127.0.0.1:5432/platform \
//! TEST_REDIS_URL=redis://127.0.0.1:6379 \
//!   cargo test --package platform-storage --test infrastructure
//! ```

use std::time::Duration;

use platform_application::DependencyProbe;
use platform_domain::Dependency;
use platform_storage::{PostgresProbe, PostgresSettings, RedisClient, RedisProbe, RedisSettings};

/// How long a probe is allowed to take before it is reported as down.
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

fn database_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn redis_url() -> Option<String> {
    std::env::var("TEST_REDIS_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn postgres_settings(url: String) -> PostgresSettings {
    PostgresSettings {
        url,
        max_connections: 2,
        min_connections: 1,
        acquire_timeout: Duration::from_secs(5),
        run_migrations: true,
    }
}

#[tokio::test]
async fn postgres_connection_and_migrations_are_usable() {
    let Some(url) = database_url() else {
        eprintln!("skipping: TEST_DATABASE_URL is not set");
        return;
    };

    let pool = platform_storage::postgres::connect(&postgres_settings(url))
        .await
        .expect("PostgreSQL must accept the configured connection");

    platform_storage::postgres::run_migrations(&pool)
        .await
        .expect("embedded migrations must apply");

    // Migrations are idempotent: applying them twice must not fail.
    platform_storage::postgres::run_migrations(&pool)
        .await
        .expect("re-applying embedded migrations must be a no-op");

    // A real query through a bind parameter, so the schema is proven usable.
    let value: String = sqlx::query_scalar("SELECT value FROM platform_meta WHERE key = $1")
        .bind("schema_version")
        .fetch_one(&pool)
        .await
        .expect("the migrations must populate platform_meta");

    assert_eq!(value, "v0.1");

    let check = PostgresProbe::new(pool.clone()).probe(PROBE_TIMEOUT).await;

    assert_eq!(check.dependency(), Dependency::Postgres);
    assert!(check.state().is_up(), "probe should report PostgreSQL up");
    assert_eq!(check.error(), None);

    pool.close().await;
}

#[tokio::test]
async fn redis_connection_and_ping_are_usable() {
    let Some(url) = redis_url() else {
        eprintln!("skipping: TEST_REDIS_URL is not set");
        return;
    };

    let settings = RedisSettings {
        url,
        timeout: PROBE_TIMEOUT,
    };

    let client = RedisClient::connect(&settings)
        .await
        .expect("Redis must accept the configured connection");

    let check = RedisProbe::new(client).probe(PROBE_TIMEOUT).await;

    assert_eq!(check.dependency(), Dependency::Redis);
    assert!(check.state().is_up(), "probe should report Redis up");
    assert_eq!(check.error(), None);
}

#[tokio::test]
async fn postgres_probe_reports_down_when_the_server_is_unreachable() {
    // Always runs: a closed port must yield a `down` check, never a panic or a
    // hang, so readiness fails closed when a dependency disappears.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_millis(200))
        .connect_lazy("postgres://platform:unused@127.0.0.1:1/platform")
        .expect("a lazy pool does not connect eagerly");

    let check = PostgresProbe::new(pool).probe(PROBE_TIMEOUT).await;

    assert!(!check.state().is_up());
    assert!(
        check.error().is_some(),
        "the failure reason must be reported"
    );
}

#[tokio::test]
async fn redis_connect_fails_fast_for_an_unreachable_server() {
    let settings = RedisSettings {
        url: "redis://127.0.0.1:1".to_owned(),
        timeout: Duration::from_millis(300),
    };

    let outcome = RedisClient::connect(&settings).await;

    assert!(outcome.is_err(), "connecting to a closed port must fail");
}
