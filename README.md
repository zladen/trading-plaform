# Platform

Infrastructure foundation for an automated cryptocurrency trading platform.

This repository currently contains **no trading functionality**. V0.1 delivers a
verified, runnable skeleton that later stages build on: a Rust backend with a
versioned REST API and a WebSocket channel, a React dashboard that renders the
real state of the backend and its dependencies, PostgreSQL and Redis, Docker
Compose, CI and documentation.

## What exists today

| Area | Content |
| --- | --- |
| Backend | Rust 2024 edition, Axum, layered modular monolith, `/api/v1` REST, WebSocket, OpenAPI 3.1, Swagger UI outside production |
| Frontend | React 19, TypeScript strict, Vite, Feature-Sliced Design, TanStack Query, Zustand, Zod, React Hook Form, Tailwind CSS v4, shadcn/ui-style primitives, TradingView Lightweight Charts |
| Data | PostgreSQL 16 via SQLx with embedded migrations, Redis 8 for auxiliary infrastructure only |
| Infrastructure | Multi-stage Dockerfiles, non-root containers, Compose with healthchecks, named volumes, no published database ports |
| Quality | Rust unit and integration tests, frontend unit and component tests, ESLint with enforced FSD boundaries, Clippy with `-D warnings`, GitHub Actions |

Deliberately absent: exchange integrations, strategies, risk and execution
engines, backtesting, portfolio management, users, authentication, API keys.
See `docs/decisions/0001-modular-monolith.md` and the scope section of
`docs/development/AGENT_TASK_V0.1.md`.

## Architecture

```
Browser
  │  HTTP + WebSocket, one origin
  ▼
Frontend container (nginx)  ──/api/*──▶  Backend container (Axum)
  │                                          │
  │  static bundle                           ├──▶ PostgreSQL (internal network only)
  ▼                                          └──▶ Redis (internal network only)
```

Backend layers, dependencies pointing inwards:

```
api ──▶ application ──▶ domain
             ▲
infrastructure (SQLx, Redis)
```

The frontend follows Feature-Sliced Design. Full detail:
[`docs/architecture/overview.md`](docs/architecture/overview.md).

## Requirements

- Rust 1.97.1 (pinned by `rust-toolchain.toml`)
- Node.js 24+ and pnpm 12 (`packageManager` in `package.json`)
- Docker 24+ with Compose v2
- `make` (optional, but every documented command has a target)

## Quick start with Docker

```bash
make env      # creates .env from .env.example, only when .env is empty
make up       # builds and starts postgres, redis, backend, frontend
```

Then open <http://localhost:8081>. The dashboard shows the backend, PostgreSQL,
Redis, the version and the WebSocket state, all read from the live API.

```bash
make ps       # service and health status
make logs     # follow all logs
make down     # stop and remove containers; named volumes are kept
```

| Endpoint | URL |
| --- | --- |
| Dashboard | <http://localhost:8081> |
| Health | <http://localhost:8080/api/v1/health> |
| Readiness | <http://localhost:8080/api/v1/ready> |
| Info | <http://localhost:8080/api/v1/info> |
| OpenAPI document | <http://localhost:8080/api/v1/openapi.json> |
| Swagger UI | <http://localhost:8080/swagger-ui> (development only) |
| WebSocket | `ws://localhost:8080/api/v1/ws` |

## Local development without containers for the applications

```bash
make dev-deps                 # PostgreSQL and Redis on loopback only
cp .env.example .env          # then set DATABASE_URL / REDIS_URL for the host
make backend                  # cargo run, http://127.0.0.1:8080
make frontend                 # vite dev server, http://127.0.0.1:5173
```

The Vite dev server proxies `/api` (including the WebSocket upgrade) to
`http://127.0.0.1:8080`, so the browser still talks to a single origin.

## Verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

pnpm install --frozen-lockfile
pnpm --filter @platform/frontend typecheck
pnpm --filter @platform/frontend lint
pnpm --filter @platform/frontend test
pnpm --filter @platform/frontend build

docker compose --env-file .env.example config
```

`make check` runs the lint and test suites; `make lint`, `make test` and
`make build` run the individual groups. Integration tests against real services
read `TEST_DATABASE_URL` and `TEST_REDIS_URL` and are skipped with a message when
those are absent.

With the stack running, `make smoke` verifies what a deployment actually
exposes — every endpoint, the unified envelope, the OpenAPI document, the
WebSocket handshake and `ping`/`pong`, error handling, and the frontend bundle
plus its `/api` proxy. It exits non-zero on the first failed assertion, so it is
usable as a post-deployment check.

## API contract

The OpenAPI document is generated from the Rust source, which is the single
source of truth. The frontend consumes types generated from it — no handwritten
duplicates.

```bash
pnpm api:openapi   # regenerate apps/frontend/src/shared/api/generated/openapi.json
pnpm api:types     # regenerate schema.d.ts from that document
```

CI fails when either artefact is stale.

## Security posture for V0.1

- PostgreSQL and Redis have no published ports; they are reachable only from an
  internal Docker network.
- Published ports bind to `127.0.0.1` unless `APP_BIND_ADDRESS` is changed.
- Backend and frontend containers run as non-root; the backend container has a
  read-only root filesystem.
- CORS allows an explicit list of origins; wildcards are rejected at startup.
- Secrets live in `.env` (git-ignored) and never in source, images or logs.
  Connection URLs are logged with the password redacted.

There is no authentication yet. Do not expose this stack to an untrusted network
until the authentication stage is implemented.

## Repository layout

```
apps/backend/          Rust API and WebSocket server
apps/frontend/         React application (FSD)
crates/domain/         Entities, value objects, domain errors
crates/application/    Use cases and dependency ports
crates/storage/        SQLx and Redis adapters
crates/shared/         Cross-cutting primitives (build info, request ids)
migrations/            SQLx migrations
infrastructure/docker/ Dockerfiles and nginx configuration
docs/                  Architecture, decisions, development guides
```

## Documentation

- [Architecture overview](docs/architecture/overview.md)
- [Local setup](docs/development/setup.md)
- [Architecture decisions](docs/decisions/)
- [V0.1 technical specification](docs/development/AGENT_TASK_V0.1.md)
