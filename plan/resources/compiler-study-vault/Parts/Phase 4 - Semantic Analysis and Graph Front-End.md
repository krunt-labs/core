---
tags: [phase, compiler, graph-giraffe, semantics]
---

# Phase 4 — Semantic Analysis & Graph Front-End

> Status: not started
- [ ] Learn completed
- [ ] [[Crate 03 - semantic-api]] + [[Crate 04 - compiler-core]] Build + Verify completed

## Goal
Build the compiler's "front-end" (structural validation → plugin/node resolution → execution/data-flow views → semantic analysis → contract/capability validation → whole-graph validation) and its deterministic diagnostics.

## Learn (must)
- **EAC Ch 4** — *Context-Sensitive Analysis* (pp.161):
  - intro to type systems (§4.2),
  - **attribute-grammar framework** (§4.3) — a useful mental model for propagating facts across a graph,
  - ad hoc syntax-directed translation (§4.4),
  - *harder problems in type inference* (§4.5.1).
- **EAC Ch 5.5** — *Symbol Tables* (pp.253): scopes, name resolution (analog for subgraph/module name spaces).

## Map to Graph Giraffe
- Because there is no parser, this stage *is* parsing-analogous analysis: structural validation, plugin resolution, execution/data-flow view construction.
- Contract validation (RFC §10), capability resolution (RFC §11), whole-graph reachability all land here.
- Every diagnostic carries stable code, severity, primary element ID, related IDs, stage — the [[Source Mapping]] discipline.

## Build
Per [[Crate 03 - semantic-api]] and [[Crate 04 - compiler-core]] DoD:
- `semantic-api` plugin/contract interfaces a backend-dsl implements;
- `compiler-core` stage orchestration with deterministic, canonically-sorted diagnostics;
- short-circuit on error severity (no emit after semantic errors).

## Verify
- Multiple simultaneous diagnostics in stable order.
- Invalid AI-authored and manual graphs produce identical diagnostics.
- Plugin resolution is deterministic (same graph → same resolve order).

## Concepts to write
[[Target Contract]] (how semantic plugins extend without contaminating IR) · [[Determinism]]

**Next → [[Phase 5 - Optimization Passes]]**