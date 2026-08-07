---
tags: [crate, compiler, types]
---

# Crate 02 — `graph-types`

The reusable compile-time type system.

## Definition of Done
- [ ] `Type` enum covering RFC §9 primitives + composites + collections + generics
- [ ] Assignability / consistency check
- [ ] `unify` + constraint solver (see [[Unification - Type Inference]])
- [ ] Inference **across data edges** (the graph-native novelty)
- [ ] Contract input/output typing support (RFC §10)
- [ ] Type diagnostics source-mapped to edges/ports
- [ ] Edge-typing fixtures: valid chains infer; invalid produce diagnostics
- [ ] Deterministic inference (solve order does not change results)

## Requirements (from `graph-giraffe-type-system` skill)
- Type representations, variables, assignability, unification, constraint solving, inference across data edges, generics, collections, unions, intersections, `Option`/`Result`/`Future`/`Stream`, contract input/output typing, type diagnostics.

## Build order
Built in **[[Phase 3 - Type System and Inference]]**.

## Cross-links
- [[Unification - Type Inference]] · [[TAPL Ch22 - graph-types]] · [[Phase 3 - Type System and Inference]]