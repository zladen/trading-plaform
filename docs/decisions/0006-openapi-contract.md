# 0006 — OpenAPI as the single API contract

- **Status:** accepted
- **Date:** 2026-01-01
- **Context:** V0.1, API contract

## Context

The frontend and the backend both describe the same JSON payloads. When those
descriptions are maintained separately, they drift — and the drift is invisible
until a field is missing in production. Handwritten TypeScript interfaces are the
most common way this happens: they look authoritative and are wrong.

The API surface is small and will grow deliberately. It does not need a
schema-first design process, but it does need one source of truth.

## Decision

Generate the OpenAPI 3.1 document **from the Rust source** with Utoipa. That
document is the contract. The frontend's types are generated from it.

```
Rust handler + DTO + #[utoipa::path] annotations
  └─ platform-openapi binary ──▶ apps/frontend/src/shared/api/generated/openapi.json
       └─ openapi-typescript ──▶ schema.d.ts
            └─ imported by shared/api/client.ts
```

Mechanisms that keep it honest:

- A **separate binary** (`platform-openapi`) renders the document without
  starting a server, so CI can diff it.
- **CI regenerates both artefacts** and fails when the committed files differ.
  A pull request that changes a response shape without regenerating cannot merge.
- The document is served at `/api/v1/openapi.json` from the same code path that
  the binary uses, so what clients read is what was generated.
- Swagger UI is mounted at `/swagger-ui` outside production, and reads *that*
  route rather than registering a second handler for the same path.

## Why code-first rather than schema-first

- The Rust types already exist and are already validated. A schema-first workflow
  would duplicate them and add a generation step in the opposite direction.
- Utoipa derives the document from the same annotations that document the handler,
  so the doc and the code are physically adjacent.
- For a backend in one language with a single known client, code-first removes a
  synchronisation problem instead of adding one.

## Consequences

**Positive**

- One place to change a contract: the Rust type.
- The frontend cannot compile against a stale shape; types are regenerated.
- The contract is reviewable as a diff in pull requests.
- Third-party clients get a real OpenAPI document for free.

**Negative**

- Utoipa annotations are extra code to maintain. A new endpoint is not done until
  it is annotated, and CI enforces the artefacts but not the annotation quality.
- `openapi-typescript` output is verbose and must not be edited by hand; it is
  excluded from ESLint and Prettier.
- Regeneration requires a Rust build, so the contract job in CI compiles the
  workspace.

## Alternatives considered

- **Handwritten TypeScript types.** Rejected: this is the drift problem itself.
- **Schema-first with a generator.** Rejected for V0.1: it would mean maintaining
  the schema and the Rust types, and the client is first-party. Worth revisiting if
  external clients need to drive the contract.
- **tRPC-style shared types.** Not applicable across Rust and TypeScript without a
  bridge, and it would tie the client to a specific runtime rather than an open
  contract.
- **Skipping codegen and validating at runtime only.** Rejected: runtime validation
  catches bad data but not a missing field, and it fails in production rather than
  at build time.
