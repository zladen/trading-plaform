# Architecture overview

V0.1 establishes the foundation only. There is no trading logic, no exchange
integration and no authentication. Everything below describes what actually
exists in the repository today.

## 1. System shape

```
┌──────────────────────────────────────────────────────────────┐
│ Browser                                                      │
│  React SPA — one origin, REST + WebSocket                    │
└───────────────┬──────────────────────────────────────────────┘
                │ HTTP, WebSocket
┌───────────────▼────────────┐        ┌────────────────────────┐
│ frontend container         │        │ backend container      │
│ nginx :8080 (uid 101)      │        │ Axum :8080 (uid 10001) │
│  ├─ static bundle          │  /api  │  ├─ REST /api/v1       │
│  └─ reverse proxy ─────────┼───────▶│  ├─ WS   /api/v1/ws    │
└────────────────────────────┘        │  └─ /swagger-ui (dev)  │
                                      └───┬────────────────┬───┘
                                          │ SQLx           │ redis-rs
                              ┌───────────▼──────┐  ┌──────▼─────────┐
                              │ postgres:16      │  │ redis:8        │
                              │ named volume     │  │ named volume   │
                              │ internal network │  │ internal net   │
                              └──────────────────┘  └────────────────┘
```

Only two ports are published, both bound to loopback by default: the backend
(`8080`) and the frontend (`8081`). PostgreSQL and Redis have no `ports:` entry
at all, so they can only be reached from the `internal` Docker network.

In development the browser talks to the Vite dev server, which proxies `/api` to
the backend. In containers nginx does the same. Either way the frontend uses a
same-origin base URL, so no host is hardcoded and no CORS preflight is needed for
the application itself.

## 2. Backend: modular monolith

One deployable process, internally split into layers with a strict dependency
direction. The split is enforced by crate boundaries, not by convention alone.

```
apps/backend        api  ──▶ application ──▶ domain
                     │            ▲
                     └──▶ infrastructure ──┘        (platform-storage)
```

| Crate | Responsibility | May depend on |
| --- | --- | --- |
| `platform-domain` | Entities, value objects, domain errors. No I/O. | `serde`, `thiserror` |
| `platform-application` | Use cases and ports (`DependencyProbe`, services). | domain, shared |
| `platform-shared` | Cross-cutting primitives: build info, request ids. | `serde`, `uuid` |
| `platform-storage` | SQLx pool, migrations, Redis client, probes. | application, domain |
| `platform-backend` | Config, router, middleware, handlers, WebSocket, startup. | all of the above |

`platform-domain` does not depend on Axum, SQLx, Redis or any HTTP client. The
build fails if that changes, because the dependency simply is not declared.

Within `apps/backend`:

```
src/
├── main.rs            Thin entry point: config → logging → startup
├── lib.rs             Library target so tests build the real router
├── config/            Explicit env-var parsing, typed and validated
├── http/
│   ├── router.rs      Router, middleware order, body limit, CORS, Swagger
│   ├── dto.rs         Request/response DTOs (never domain types)
│   ├── response.rs    Unified `{ success, data | error, request_id }` envelope
│   ├── error.rs       Error mapping to HTTP with a stable error code
│   ├── middleware/    request id, timeout
│   ├── handlers/      Thin handlers; they call use cases only
│   ├── websocket/     Upgrade, message protocol, heartbeat, clean close
│   └── openapi.rs     Utoipa document assembly
├── state/             Immutable AppState holding use cases, never pools
└── startup/           Logging, dependency connection with retry, shutdown
```

Handlers receive `AppState`, which exposes use cases (`HealthService`,
`ReadinessService`, `InfoService`). They cannot obtain a database pool or a Redis
connection, so business logic cannot leak into the HTTP layer.

### Request lifecycle

Middleware order, outermost first:

1. **request id** — accepts a caller-provided `x-request-id` when it is a valid
   value and generates one otherwise. Every response echoes it.
2. **catch panic** — a panic becomes a unified `500` instead of a dropped
   connection.
3. **tracing** — structured log line with method, path, status, latency.
4. **timeout** — bounds handler execution and returns a unified `408`.
5. **CORS** — explicit origin list; a preflight is answered before the body
   limit applies.
6. **body limit** — rejects oversized payloads early.

### Endpoints

| Method | Path | Semantics |
| --- | --- | --- |
| GET | `/api/v1/health` | Process liveness. Touches no dependency. Always `200` while the process serves requests. |
| GET | `/api/v1/ready` | Probes PostgreSQL and Redis concurrently with a per-probe deadline. `200` when all are up, `503` with the same envelope when one is down. |
| GET | `/api/v1/info` | Name, version, environment, Rust version, start time, uptime, WebSocket path. |
| GET | `/api/v1/openapi.json` | Generated OpenAPI 3.1 document. |
| GET | `/api/v1/ws` | WebSocket upgrade. |
| GET | `/swagger-ui` | Swagger UI, enabled outside production only. |

Every response uses one envelope, including errors:

```json
{ "success": true,  "data": { }, "request_id": "..." }
{ "success": false, "error": { "code": "not_found", "message": "..." }, "request_id": "..." }
```

`/ready` returning `503` is a *successful observation*: the body is the normal
success envelope describing which dependency is down. The frontend parses it as
data instead of treating it as a transport failure.

### Startup and shutdown

Startup fails fast and loudly. Configuration is validated before logging exists,
so a missing variable produces a precise message on stderr rather than a
partially initialised process. Then:

1. validated configuration (required variables, URL shapes, CORS not a wildcard);
2. `tracing` subscriber (pretty locally, JSON in containers);
3. PostgreSQL pool, then embedded migrations (unless `DATABASE_MIGRATE=false`);
4. Redis connection;
5. router and listener;
6. `SIGINT`/`SIGTERM` trigger a `CancellationToken`, in-flight requests drain
   within `APP_SHUTDOWN_TIMEOUT_SECONDS`, the listener closes, pools close.

Dependency connections are retried with exponential backoff for up to 60 seconds,
which absorbs the case where Compose reports a container healthy slightly before
it accepts application connections. A dependency that never answers is fatal: the
process must not advertise readiness it cannot honour.

### Readiness port

```rust
#[async_trait]
pub trait DependencyProbe: Send + Sync {
    fn dependency(&self) -> Dependency;
    async fn probe(&self, timeout: Duration) -> DependencyCheck;
}
```

`ReadinessService` holds the probes, runs them concurrently and aggregates the
result. An `up` check carries its latency; a `down` check carries a reason. The
domain decides what "ready" means, so a new dependency is a new probe, not a new
branch in a handler.

## 3. Frontend: Feature-Sliced Design

```
app → pages → widgets → features → entities → shared
```

A layer may import only from itself and lower layers. Cross-slice imports must go
through the slice's `index.ts`.

| Slice | Responsibility |
| --- | --- |
| `shared/api` | Typed client, error taxonomy, WebSocket client |
| `shared/ui` | Design primitives (button, card, badge, chart wrapper) |
| `shared/lib` | Formatting and `cn` |
| `entities/system-status` | Health/readiness/info mapping and the query |
| `features/refresh-system-status` | Manual re-check button |
| `features/live-connection` | WebSocket transport state in Zustand |
| `widgets/system-status` | Loading, success, error and degraded rendering |
| `widgets/live-channel` | Protocol version, connection id, heartbeat |
| `pages/dashboard` | Composition |

Zod is used at the network boundary: `shared/api/websocket.ts` parses every
server frame with a discriminated union, so a malformed or unexpected frame is
dropped instead of reaching the store as a cast. React Hook Form and
`@hookform/resolvers` are installed for the first form V0.1 needs; there is no
form on the page yet, so no form package is imported by application code.

Enforcement is real, not aspirational. `eslint-plugin-boundaries` is configured
with a TypeScript resolver so `@/` aliases are classified; without it the plugin
silently treats aliased imports as external and the rules never fire. The
configuration disallows both upward layer imports and reaching into another
slice's internals.

Server state lives in TanStack Query. Zustand holds only transport state that the
server does not own (socket status, last heartbeat). No server data is duplicated
into a client store.

### Data flow

```
useSystemSnapshotQuery (TanStack Query)
  └─ loadSystemSnapshot
       ├─ GET /api/v1/health
       ├─ GET /api/v1/ready   (503 parsed as data)
       └─ GET /api/v1/info
  └─ buildSnapshot → SystemSnapshot  (backend + mapped dependencies)
       └─ deriveSystemState → operational | degraded | unavailable
```

The dashboard renders four explicit states: loading, error (no answer at all),
success, and degraded/unavailable (an answer that reports a down dependency).
Nothing is mocked: with the backend stopped the page shows an error panel, not
plausible-looking numbers.

### API client

`shared/api/client.ts` is a small typed wrapper rather than a framework:

- one `VITE_API_URL` base, defaulting to the current origin;
- `AbortController` per request, joined with the caller's signal via
  `AbortSignal.any`, so React Query cancellation and the deadline both work;
- an `ApiError` with `kind` (`network | timeout | http | parse`), status, stable
  error code, request id and an `isRetryable` classification;
- envelope validation, because a valid JSON body that is not the documented
  envelope is still a contract violation;
- `acceptedStatuses` so `/ready`'s `503` payload is returned instead of thrown.

Types come from the generated OpenAPI schema. No response shape is written twice.

### WebSocket

`/api/v1/ws` speaks a small versioned protocol:

| Direction | Message |
| --- | --- |
| server → client | `connected` (`connection_id`, `protocol_version: 1`) |
| server → client | `heartbeat` (`server_time`) |
| server → client | `pong` (`payload`) |
| server → client | `closing` (`reason`) |
| client → server | `ping` |

The server rejects binary frames, answers unrecognised text with a `closing`
frame and closes cleanly on client disconnect. No market or trading events exist
yet; the channel carries liveness only.

## 4. Data

### PostgreSQL

The single store for durable state. Access is confined to `platform-storage`;
`platform-domain` has no SQL. Migrations are embedded at compile time
(`sqlx::migrate!`) from the repository-root `migrations/` directory, which is why
the Docker build context is the repository root.

V0.1 migration content: one infrastructure table, `platform_meta`, holding a
`schema_version` row. No strategies, orders, trades or positions tables, because
nothing stores them yet. Queries use bind parameters; timestamps are UTC
(`TIMESTAMPTZ`, `timezone=UTC` in the container command).

### Redis

Auxiliary infrastructure only: connection, configuration, availability check and
clean shutdown. No queues, Pub/Sub, streams or cache layer, because no use case
needs one. Persistence is disabled (`--appendonly no --save ""`) so it cannot
accidentally become a source of truth.

## 5. Contract

The OpenAPI document is generated from the Rust definitions by Utoipa and is the
only API contract:

```
Rust types + utoipa annotations
  └─ platform-openapi (binary) ──▶ apps/frontend/src/shared/api/generated/openapi.json
       └─ openapi-typescript ──▶ schema.d.ts
            └─ imported by shared/api
```

A separate binary means the document can be diffed without starting a server, so
CI can fail a pull request that changes the API without regenerating the
artefacts.

## 6. Deliberately deferred

Exchange adapters, market data, strategy and risk engines, execution, backtesting,
portfolio and position management, users, authentication and API keys. The layering
above leaves room for each — a strategy engine becomes an application module with
its own ports; authentication becomes an API middleware plus a session adapter —
without restructuring what exists. No placeholder modules are created in advance.
