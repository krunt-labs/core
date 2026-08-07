---
tags: [phase, compiler, graph-giraffe, quality]
---

# Phase 7 — Quality & Integration

> Status: not started
- [ ] Learn completed
- [ ] Full-pipeline e2e tests green
- [ ] Determinism / golden / WASM smoke verified

## Goal
Lock the guarantees that make Graph Giraffe trustworthy: determinism, validation-before-generation, and clean public boundaries (CLI / WASM / service).

## Learn
- **Skill** — `graph-giraffe-quality` (read end-to-end: invalid-graph fixtures, serialization tests, deterministic repeat compile, IR pass verification, golden generated projects, generated TypeScript type-check + runtime smoke, architecture dependency checks, security review, release readiness).
- **Skills** — `graph-giraffe-compiler` ("verify compilation" section) and `graph-giraffe-typescript-codegen` (archive export, deterministic golden output).

## Build (test strategy)
- Invalid-graph fixtures at *each* stage.
- Deterministic repeat compilation (byte-for-byte).
- IR pass verification (before/after fixtures).
- Golden generated projects.
- Generated TypeScript `tsc` + runtime smoke.

## Verify
- Repeat compilation from a clean checkout is identical.
- No generated output on semantic errors (CLI, service, WASM surfaces consistent).
- Architecture dependency check: core crates never reference target frameworks.

## Concepts to finalize
[[Determinism]] · [[Source Mapping]] · [[Target Contract]]

**Done → revisit [[00 MOC - Compiler Study]] and extend into a new domain / target.**