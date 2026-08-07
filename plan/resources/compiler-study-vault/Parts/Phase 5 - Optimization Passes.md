---
tags: [phase, compiler, graph-giraffe, optimization]
---

# Phase 5 — Optimization Passes

> Status: not started
- [ ] Learn completed
- [ ] [[Phase 6 - Lowering and Multi-Target Codegen]] gated on passes passing

## Goal
Implement deterministic, semantics-preserving optimization passes on the canonical IR: dead-node elimination, constant folding, subgraph inlining, dependency pruning, scheduling, memoization.

## Learn (must)
- **EAC Ch 8** — *Introduction to Optimization* (pp.405):
  - scope of optimization (§8.3),
  - **local value numbering** (§8.4.1),
  - global optimization / liveness (§8.6.1),
  - interprocedural + **inline substitution** (§8.7.1).
- **EAC Ch 10** — *Scalar Optimizations* (pp.539):
  - **eliminating useless & unreachable code** (§10.2),
  - code motion (§10.3),
  - redundancy elimination (§10.5).

## Map to Graph Giraffe
- **DCE / dead-branch / unreachable** → the compiler-skill whole-graph validations, as transforms.
- **Constant folding** → RFC "constant folding".
- **Subgraph inlining** → RFC "subgraph inlining".
- **Dependency pruning / scheduling** → RFC "dependency pruning", "execution scheduling".

## Rule (from the ir-optimization skill)
Optimize conservatively. Never reorder impure effects without a proven ordering rule; never infer purity — consume resolved capability/effect facts.

## Build
Passes as pure `IR -> Result<IR>` with declared invariants:
- required input invariants;
- produced output invariants;
- observable semantics preserved;
- source-map behavior;
- deterministic ordering;
- safe for incremental/parallel execution or not.

## Verify
- Before/after fixtures proving semantic preservation.
- Idempotence where a pass claims it; determinism under different input insertion orders.
- IR invariants hold after every pass (broken invariants = compiler defect, not user error).

**Next → [[Phase 6 - Lowering and Multi-Target Codegen]]**