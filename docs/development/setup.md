# Local setup

Everything below was executed against this repository. Where a step depends on
your machine, it says so.

## 1. Requirements

| Tool | Version | Check |
| --- | --- | --- |
| Rust | 1.97.1 (pinned) | `rustc --version` |
| Node.js | 24 or newer | `node --version` |
| pnpm | 12.16.1 (pinned) | `pnpm --version` |
| Docker Engine | 24 or newer | `docker version` |
| Docker Compose | v2 | `docker compose version` |

`rust-toolchain.toml` pins the toolchain, so `rustup` installs 1.97.1
automatically on the first `cargo` invocation. `packageManager` in
`package.json` lets Corepack provide the pinned pnpm.

## 2. Dependencies

```bash
# Rust workspace
cargo fetch

# Frontend workspace
pnpm install --frozen-lockfile
```

`pnpm install` also installs the root `prettier`, because the workspace shares one
lockfile.

## 3. Configuration

`.env` is git-ignored and empty in a fresh checkout. Create it from the template:

```bash
make env          # copies .env.example to .env, only when .env is empty
```

For a first local run the template values are sufficient. What each group does:

| Variable | Required | Purpose |
| --- | --- | --- |
| `APP_ENV` | yes | `development`, `test` or `production`. Controls Swagger UI and validation strictness. |
| `APP_HOST`, `APP_PORT` | yes | Listener address. |
| `APP_CORS_ALLOWED_ORIGINS` | yes | Comma-separated exact origins. A wildcard is rejected at startup. |
| `DATABASE_URL` | yes | PostgreSQL connection URL. |
| `REDIS_URL` | yes | Redis connection URL. |
| `FRONTEND_URL` | yes | Frontend origin, used for CORS defaults and diagnostics. |
| `APP_LOG_FORMAT` | no | `pretty` (default) or `json`. |
| `APP_REQUEST_BODY_LIMIT_BYTES` | no | Default 1 MiB. |
| `APP_SHUTDOWN_TIMEOUT_SECONDS` | no | Default 30. |
| `APP_READINESS_TIMEOUT_MS` | no | Per-dependency probe deadline, default 2000. |
| `APP_WS_HEARTBEAT_SECONDS` | no | Server heartbeat interval, default 30. |
| `APP_SWAGGER_ENABLED` | no | Defaults to enabled outside production. |
| `DATABASE_MAX_CONNECTIONS` / `_MIN_CONNECTIONS` / `_ACQUIRE_TIMEOUT_SECONDS` | no | Pool sizing, defaults 10 / 1 / 5. |
| `DATABASE_MIGRATE` | no | Default `true`: apply embedded migrations at startup. |
| `REDIS_TIMEOUT_MS` | no | Default 2000. |
| `RUST_LOG` | no | Default `info`. |
| `VITE_API_URL` | no | Defaults to the current origin (dev proxy / nginx). |
| `VITE_DEV_API_TARGET` | no | Vite proxy target, default `http://127.0.0.1:8080`. |

`.env.example` additionally documents the Compose-only variables
(`POSTGRES_*`, `BACKEND_PORT`, `FRONTEND_PORT`, `APP_BIND_ADDRESS`) and the
commented host-side `DATABASE_URL` / `REDIS_URL` used when the applications run
outside Docker.

Configuration is validated at startup. A missing required variable or an invalid
value aborts the process with a message that names the variable and never prints
its value.

## 4. Running everything in Docker

```bash
make up
```

This builds both images and starts PostgreSQL, Redis, the backend and the
frontend. `--wait` makes the target return only after every healthcheck passes.

| Service | URL |
| --- | --- |
| Dashboard | <http://localhost:8081> |
| API | <http://localhost:8080/api/v1/health> |
| Swagger UI | <http://localhost:8080/swagger-ui> |

Both published ports bind to `127.0.0.1`. PostgreSQL and Redis are not published
at all; they are reachable only from the internal Docker network.

```bash
make ps                # health status
make logs              # all logs
make logs-backend      # backend only
make restart           # recreate
make down              # stop, keep data
```

## 5. Running the applications on the host

Useful for a fast Rust edit-compile cycle and for the Vite dev server. Docker
still supplies the dependencies.

```bash
# 1. Dependencies only, published on loopback (compose.dev.yaml)
make dev-deps

# 2. Point the backend at them
export DATABASE_URL=postgres://platform:change-me-development-only@127.0.0.1:5432/platform
export REDIS_URL=redis://127.0.0.1:6379
export APP_ENV=development
export APP_HOST=127.0.0.1
export APP_PORT=8080
export APP_CORS_ALLOWED_ORIGINS=http://localhost:5173
export FRONTEND_URL=http://localhost:5173

# 3. Backend
make backend

# 4. Frontend, in a second terminal
make frontend        # http://localhost:5173, proxies /api to :8080
```

`compose.dev.yaml` publishes `5432` and `6379` on `127.0.0.1` and disables the
backend/frontend services through a profile. It is a development file; do not
deploy with it.

## 6. Migrations

Migrations live in `migrations/` and are embedded into the binary at compile
time, so the container needs no extra files.

- The backend applies them at startup by default (`DATABASE_MIGRATE=true`).
- To apply them explicitly — for example with `DATABASE_MIGRATE=false` in a
  deployment pipeline — use `make migrate` or
  `cargo run --package platform-backend --bin platform-migrate`.

Applied migrations must never be edited in a shared environment; add a new file
with a later timestamp prefix instead.

## 7. Tests

```bash
make test            # cargo test --workspace + frontend vitest
make lint            # clippy -D warnings, fmt check, tsc, eslint, prettier check
make check           # lint + test
make smoke           # end-to-end verification of a running stack
```

Rust tests:

```bash
cargo test --workspace
cargo test --package platform-backend --test api        # HTTP contract
cargo test --package platform-backend --test websocket  # WS protocol
cargo test --package platform-storage                   # adapters
```

The storage integration suite needs live services. Without them it prints
`skipping: TEST_DATABASE_URL is not set` and passes, so `cargo test` stays usable
offline; with them it proves connectivity, migration idempotency and a real
`SELECT` through a bind parameter:

```bash
TEST_DATABASE_URL=postgres://platform:change-me-development-only@127.0.0.1:5432/platform \
TEST_REDIS_URL=redis://127.0.0.1:6379 \
  cargo test --package platform-storage --test infrastructure
```

Frontend tests:

```bash
pnpm --filter @platform/frontend test           # once
pnpm --filter @platform/frontend test:watch     # watch mode
pnpm --filter @platform/frontend test src/entities/system-status/model.test.ts
```

## 8. Regenerating the API contract

```bash
pnpm api:openapi     # Rust → openapi.json
pnpm api:types       # openapi.json → schema.d.ts
```

Run both after changing a handler signature, a DTO or a Utoipa annotation. CI
regenerates them and fails if the committed files differ.

## 9. Troubleshooting

**The backend exits with `configuration error: missing required configuration
variable: ...`** — `.env` was not created, or the variable is missing from the
process environment. Run `make env` and review it.

**`failed to connect to PostgreSQL` at startup** — the container is not healthy
yet or `DATABASE_URL` uses the wrong host. Inside Compose the host is `postgres`;
on the host it is `127.0.0.1`. Startup retries for 60 seconds before failing.

**The dashboard shows "System status unavailable"** — the browser could not reach
the API. Check `make ps`, then `curl http://localhost:8080/api/v1/health`. In host
development confirm the Vite proxy target (`VITE_DEV_API_TARGET`).

**The dashboard shows "Degraded" with a dependency disabled** — the API answered
and reported a down dependency. Check `docker compose logs postgres` or
`docker compose logs redis`.

**Swagger UI returns 404** — `APP_ENV=production` disables it by design. Set
`APP_SWAGGER_ENABLED=true` to override.

**`cargo build` fails inside Docker with a missing archive member** — a stale
BuildKit cache. Run `docker build --no-cache -f
infrastructure/docker/backend.Dockerfile .`.

**Tests skipped with `TEST_DATABASE_URL is not set`** — expected without live
services. Start `make dev-deps` and export the variables to run them.

## 10. Resetting data

```bash
make down            # containers gone, volumes kept
make clean           # build artefacts only; volumes untouched
make clean-volumes   # deletes platform-postgres-data and platform-redis-data
```

`make clean-volumes` asks for typed confirmation and deletes the named volumes
explicitly. Nothing else deletes user data.
