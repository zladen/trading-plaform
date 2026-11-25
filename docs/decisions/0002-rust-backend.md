# 0002 — Rust for the backend

- **Status:** accepted
- **Date:** 2026-01-01
- **Context:** V0.1, technology selection

## Context

The backend must eventually handle market data, order lifecycle and money. Even
at the infrastructure stage, the properties that matter are already visible:
long-running processes, concurrent I/O, strict correctness, predictable latency,
and a low tolerance for "it worked in staging".

The team's stack rules already name Rust. This ADR records *why*, so that the
choice is revisitable rather than dogmatic.

## Decision

Use stable Rust (pinned 1.97.1) with Tokio, Axum, SQLx and `redis-rs`, organised
as the layered workspace described in ADR 0001.

## Rationale

- **Memory and thread safety without a garbage collector.** Trading runtimes hold
  state that is constantly mutated by concurrent tasks. Rust makes data races a
  compile error instead of an intermittent incident.
- **No GC pauses.** Latency-sensitive paths — order submission, risk checks — get
  predictable timing. Nothing in V0.1 needs this yet, but the language choice
  will not have to be revisited when it does.
- **Errors as values.** `Result` plus `thiserror` in libraries and `anyhow` at
  boundaries means every failure path is explicit. The configuration loader, for
  example, cannot silently fall back to a default when a required variable is
  missing.
- **Types carry invariants.** Newtypes for identifiers and `rust_decimal` for
  monetary values make whole classes of bug unrepresentable. Financial `f64` is
  excluded by the type system rather than by review comment.
- **One static binary.** Small runtime image, no interpreter, fast cold start.

## Consequences

**Positive**

- Compilation catches boundary violations, missing error handling and precision
  mistakes.
- The production image is a ~15 MB layer over `debian-slim` with no runtime
  dependencies.
- Clippy with `-D warnings` is a meaningful gate, and `unsafe_code = "forbid"` is
  enforced workspace-wide.

**Negative**

- Compile times are longer than for a dynamic language, and the Docker build has
  to cache the dependency graph carefully.
- The ecosystem is smaller for some domains; a crate must be evaluated rather
  than assumed.
- Contributors need Rust fluency. The learning curve is real.
- `sqlx` compile-time query checking would need either a live database or offline
  metadata in CI. V0.1 has no compile-time-checked queries, so this is not yet a
  cost.

## Alternatives considered

- **TypeScript/NestJS for both sides.** Rejected: one language is convenient, but
  a garbage-collected runtime for the execution path and weaker compile-time
  guarantees on financial arithmetic are the wrong trade for this domain.
- **Go.** Viable and simpler to learn, with good concurrency. Rejected because
  the type system cannot express the invariants that matter here as precisely —
  no newtype distinction between a price and a quantity without ceremony, and nil
  handling is a runtime concern.
