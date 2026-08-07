---
tags: [phase, compiler, graph-giraffe, types]
---

# Phase 3 — Type System & Inference

> Status: not started
- [ ] Learn completed
- [ ] [[Crate 02 - graph-types]] Build + Verify completed

## Goal
Build the reusable compile-time type system: representation, assignability, unification, constraint solving, and inference across data edges.

## Learn (must)
- **TAPL Ch 3** — *Untyped Arithmetic Expressions* (terms as the substrate).
- **TAPL Ch 9** — *The Simply Typed Lambda-Calculus* (the core type-checking discipline).
- **TAPL Ch 22** — *Type Reconstruction* (Hindley–Milner unification — *the single most important chapter for you*).

## Learn (skim)
- TAPL Ch 11 *Simple Extensions* (records/units → your struct/record) — skim §11 on base types, unit, sequencing, ascription.

## Deferred (not now — revisit later)
- TAPL Ch 15 *Subtyping* + Ch 16 *Metatheory of Subtyping* → revisit for union/intersection modeling.
- TAPL Ch 20 *Recursive Types*, Ch 23–24 *Universal/Existential Types* → revisit for generics (`Option<T>`, `Result<T,E>`) and `Future`/`Stream`.

## Map to Graph Giraffe
- Type representation must cover: primitives, struct/record, collections, `Option`/`Result`/`Future`/`Stream`, unions, intersections — RFC §9.
- Inference **propagates automatically across compatible data edges** — this is the "type system on a graph" novelty.
- Contract input/output typing (RFC §10) reuses the same machinery.

## Build
Per [[Crate 02 - graph-types]] DoD: `Type` enum, `unify`, constraint solve, assignability, diagnostics.

## Verify
- Edge-typing fixtures: valid chains infer; invalid connections produce source-mapped diagnostics.
- Determinism of inference (solve order does not change results).

## Concepts to write
[[Unification - Type Inference]] · [[IR Taxonomy]] (types as IR-carrying facts)

**Next → [[Phase 4 - Semantic Analysis and Graph Front-End]]**