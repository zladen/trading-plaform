# 0005 — REST for commands and queries, WebSocket for live updates

- **Status:** accepted
- **Date:** 2026-01-01
- **Context:** V0.1, API design

## Context

The frontend needs two different things from the backend:

1. **Request/response operations** — fetch system state, submit a command, read a
   resource. The caller wants an answer bound to the request, with a status code
   and an error it can act on.
2. **Server-pushed updates** — a fill, an order state change, a price tick. The
   caller does not know when these happen and must not poll for them.

Forcing one transport to do both produces either polling (wasteful and slow) or
request/response over a socket (no natural status codes, no caching, awkward
tooling).

## Decision

- **REST under `/api/v1`** for commands and queries. Versioned path prefix,
  resource-oriented routes, HTTP status codes, a unified response envelope.
- **WebSocket at `/api/v1/ws`** for server-pushed events and liveness.
- **OpenAPI** describes the REST surface and is the generated contract for the
  frontend. The WebSocket protocol is described in this document and typed on
  both sides.

V0.1 implements the transport for both but carries no trading events: the socket
serves `connected`, `heartbeat`, `pong` and `closing` frames only.

## Why both, with these responsibilities

- REST gets caching, status codes, browser devtools, `curl`, load balancers and an
  OpenAPI contract for free. It is the right tool for "do this, then tell me what
  happened".
- A WebSocket keeps one connection open and lets the server push. That is the only
  sane way to deliver "an order filled 40 ms ago" without polling.
- The split is also a reliability boundary: the dashboard can show accurate
  health data over REST even when the socket is down, which is exactly the state
  an operator most needs to see.

## Protocol

Server → client, all JSON text frames:

| `type` | Fields | Meaning |
| --- | --- | --- |
| `connected` | `connection_id`, `protocol_version` | Sent immediately after the upgrade. `protocol_version` is `1`. |
| `heartbeat` | `server_time` | Sent every `APP_WS_HEARTBEAT_SECONDS`. |
| `pong` | `payload` | Reply to a client `ping`. |
| `closing` | `reason` | Sent before a deliberate close. |

Client → server: `{ "type": "ping", "payload": ... }`.

Behaviour that is part of the contract:

- binary frames are rejected;
- unrecognised text produces a `closing` frame with `unrecognised message`, then a
  clean close;
- client disconnect ends the task and releases the connection;
- the server never sends an unversioned message, so adding a field is safe and a
  breaking change requires a new `protocol_version`.

## Consequences

**Positive**

- Each concern uses the transport that fits it.
- REST errors are HTTP errors with stable codes, not in-band failures.
- The socket protocol is explicit and versioned, so both sides can evolve.
- The frontend's HTTP client and socket client are separate: a socket outage does
  not degrade REST.

**Negative**

- Two transports to secure, observe and test. V0.1 covers both with integration
  tests, but the burden is real.
- WebSocket authentication is not solved by cookies the way REST is. When auth
  arrives it must be handled explicitly for the socket — that is an argument for
  a deliberate design, not for multiplexing everything over the socket.
- Proxies must forward the upgrade. The bundled nginx configuration sets
  `Upgrade`/`Connection` and disables buffering; this is verified, not assumed.

## Alternatives considered

- **REST with polling.** Simple, but the latency and request cost of polling order
  state is unacceptable for a trading UI.
- **GraphQL subscriptions.** Would unify queries and pushes, but adds a schema,
  resolver layer and client tooling for a system with a small, stable API surface.
- **Everything over WebSocket.** Loses HTTP semantics: no status codes, no
  caching, no OpenAPI, harder debugging.
- **Server-Sent Events.** Good for one-way pushes, but the frontend also needs to
  send pings and, later, subscription commands. WebSocket covers both directions
  with one connection.
