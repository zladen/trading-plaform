# 0001 — Modular monolith instead of microservices

- **Status:** accepted
- **Date:** 2026-01-01
- **Context:** V0.1, infrastructure foundation

## Context

The platform must eventually run a strategy engine, a risk engine, an execution
engine, exchange adapters, portfolio management and market data. Those are
genuinely different concerns with different failure modes.

The obvious temptation is to start with separate services for each, on the
assumption that a trading platform needs them. Nothing in V0.1 does anything yet,
and the boundaries between those future components are not yet informed by real
constraints: which state is shared, which calls are hot, where the consistency
boundaries actually fall.

Starting with distributed boundaries chosen in advance would force network calls,
serialisation, retries and deployment coordination between components whose
interfaces we would still be guessing. Operational cost would be paid before any
trading logic exists.

## Decision

Build one deployable backend process, split internally into layered crates with a
strict dependency direction:

```
api ──▶ application ──▶ domain
             ▲
infrastructure (SQLx, Redis)
```

Crate boundaries — not folder conventions — enforce the split. `platform-domain`
cannot call SQLx because the dependency is not declared; a violation fails the
build rather than a review.

## Consequences

**Positive**

- One process to build, deploy, observe and debug.
- A use case call is an in-process call: no serialisation, no network failure
  mode, no distributed transaction.
- Refactoring a boundary is a compile-time change, not a versioned API migration
  between services.
- The later extraction of a component into its own service is straightforward
  *because* the boundary is already an explicit port. `DependencyProbe` is the
  pattern: the interface exists, the implementation is swappable.

**Negative**

- A single process cannot be scaled per component; the whole backend scales
  together.
- One crash takes down every capability in that process.
- Language choice is shared, which is fine here because the whole backend is
  Rust.

## Alternatives considered

- **Microservices from the start.** Rejected: it front-loads operational and
  consistency complexity for boundaries that are not yet validated by real use.
- **Single crate with modules.** Rejected: module visibility alone does not stop
  the domain layer from importing `sqlx`. Crate boundaries make the rule
  mechanical.

## Revisit when

A component has measurably different scaling, availability or release-cadence
requirements from the rest, or its domain model genuinely conflicts with the
others. At that point, extract it along the port that already exists.
