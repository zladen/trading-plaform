# 0004 — PostgreSQL as the primary store

- **Status:** accepted
- **Date:** 2026-01-01
- **Context:** V0.1, persistence

## Context

Trading data has properties that narrow the store choice:

- orders and fills must be durably recorded and reconciled against the exchange;
- money must be exact, so numeric types must be exact;
- many records are naturally time series (fills, candles, balance snapshots);
- several writes are related and must be atomic (an order plus its state
  transition, for example);
- queries will be ad hoc during incident investigation, when nobody wants to
  write a map-reduce job to answer "what happened to order X".

## Decision

PostgreSQL 16 as the single primary store, accessed through SQLx from
`platform-storage` only.

- Migrations live in `migrations/` and are embedded at compile time.
- Queries use bind parameters; no string interpolation.
- Timestamps are `TIMESTAMPTZ` and the server runs with `timezone=UTC`.
- Monetary values will use `rust_decimal`/`NUMERIC`, never floating point.
- Redis is auxiliary only and is never the source of truth for durable state.

## Rationale

- **Exact numerics.** `NUMERIC` is arbitrary-precision decimal. No float
  surprises in balances.
- **Real transactions with real isolation levels.** An order state transition and
  its audit row commit together or not at all.
- **Native time-series support.** `TIMESTAMPTZ`, range partitioning when volume
  demands it, and window functions for the analytics that will be needed for
  reconciliation and reporting. No second store has to be introduced to answer
  "fills per hour".
- **`JSONB` for exchange payloads.** Raw exchange responses must be stored
  verbatim for later reconciliation. `JSONB` keeps them queryable without
  inventing a schema for every venue.
- **Operational maturity.** Backups, point-in-time recovery, replication and
  monitoring are well understood.

## Consequences

**Positive**

- One store covers transactional, relational and analytical needs at this scale.
- Constraints and foreign keys make invalid states hard to persist.
- Operators already know how to run it.

**Negative**

- Compile-time query checking (`sqlx::query!`) needs a live database or offline
  metadata in CI. V0.1 has no such queries; when they arrive, CI will need either
  a service container plus `cargo sqlx prepare`, or the offline cache.
- Very high write rates on market data would eventually need partitioning or a
  dedicated store. That is a volume problem, not a V0.1 problem, and it is
  measurable before it is architectural.

## Alternatives considered

- **MongoDB.** Flexible schema, but weak relational guarantees and no exact
  decimal type that is as natural. Order/position consistency is the wrong place
  for eventual consistency.
- **SQLite.** Excellent for a single-process tool, wrong for a service with
  concurrent writers and a deployment topology.
- **A time-series database plus a relational store.** Two stores means two
  consistency models and two backup stories, for a requirement that PostgreSQL
  already meets. Revisit only with measured volume.
- **Redis as primary.** Rejected outright: durability and query capability are
  both insufficient, and the project rules forbid it.
