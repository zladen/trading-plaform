# 0003 — React with Feature-Sliced Design

- **Status:** accepted
- **Date:** 2026-01-01
- **Context:** V0.1, frontend structure

## Context

The dashboard must eventually show live positions, order state, risk limits and
charts. Even now it composites several independent concerns: backend health,
PostgreSQL and Redis state, version information and WebSocket liveness.

The usual failure pattern in a React codebase is a `components/` and `hooks/`
folder that grows until every module depends on every other one. The cost appears
late, when a change to a status badge requires understanding half the app.

## Decision

React 19 with TypeScript in strict mode, built by Vite, structured by
Feature-Sliced Design:

```
app → pages → widgets → features → entities → shared
```

Rules, enforced by ESLint rather than by review:

- a layer imports only from itself and lower layers;
- a slice is imported through its `index.ts` only — reaching into
  `@/entities/system-status/model` from a widget is an error;
- no circular dependencies;
- generic components in `shared/ui` contain no business logic.

Slices are created when they have a genuine responsibility, not to fill the
folder structure. V0.1 has one entity, two features, two widgets and one page,
which is the honest minimum for the dashboard.

## Enforcement

`eslint-plugin-boundaries` is configured with `boundaries/dependencies` policies
and, critically, with `eslint-import-resolver-typescript`. Without that resolver
the plugin cannot follow the `@/` alias, classifies aliased imports as external
and never reports anything — a rule that silently does nothing is worse than no
rule. The configuration was verified by introducing deliberate violations (a
`shared → pages` import, a `features → widgets` import, a cross-slice internal
import) and confirming each produced an error.

## Consequences

**Positive**

- The cost of a change is bounded by its slice.
- Deleting a feature is deleting a folder; nothing else references its internals.
- Layer violations fail CI instead of accumulating.
- The structure will absorb trading modules without rearrangement: a positions
  entity, a portfolio widget, an order-entry feature.

**Negative**

- FSD has real ceremony: public API files, occasional awkward placement
  decisions, a learning curve.
- Cross-imports that are genuinely needed (two entities sharing a type) require
  moving the shared part down to `shared` — correct, but slower than a direct
  import.
- Over-applying FSD creates empty layers. Mitigated by creating slices only when
  justified.

## Alternatives considered

- **`components/` + `hooks/`.** Rejected: no dependency direction, and the
  boundary erosion is invisible until it is expensive.
- **Feature folders without layers.** Simpler, but does not express that a widget
  may compose features while a feature must not compose widgets. The rule that is
  hardest to keep by discipline is exactly the one worth mechanising.
