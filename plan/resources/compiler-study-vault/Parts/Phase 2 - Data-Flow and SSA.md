---
tags: [phase, compiler, graph-giraffe, dataflow, ssa]
---

# Phase 2 — Data-Flow & SSA

> Status: not started
- [ ] Learn completed
- [ ] Dominance + reachability + liveness on IR completed

## Goal
Gain the analysis backbone that powers reachability/cycle/dead-node validation and the use–def chains that later drive type inference across data edges.

## Learn (must)
- **EAC Ch 9** — *Data-Flow Analysis* (pp.475):
  - iterative analysis + **dominance** (§9.2.1),
  - live-variable analysis (§9.2.2),
  - reaching definitions / "other data-flow problems" (§9.2.4),
  - **SSA construction** — building SSA (§9.3.1), dominance frontiers (§9.3.2), φ-placement (§9.3.3), renaming (§9.3.4), out-of-SSA (§9.3.5).

## Learn (companion)
- **CI Ch 11** — *Resolving and Binding* (scope resolution) as the simpler analog — see [[CI Ch11 - Scope]].
- **Skills** — `graph-giraffe-ir-optimization` ("deterministic ordering", "passes are pure"), `graph-giraffe-compiler` (reachability, cycles).

## Concepts to write
[[Dominance]] · [[Static Single Assignment Form]]

## Build
On the [[Crate 01 - ir]] graph:
- dominance + reachability (use `petgraph` if it fits);
- cycle / dead-node / unreachable detection (compiler-skill validations);
- liveness over data edges (the seed of type-flow analysis).

## Verify
- Determinism under shuffled insertion orders — repeat analysis, compare result order exactly.
- Invalid fixtures produce deterministic diagnostics; legal cycles limited to modeled `Loop`.

**Next → [[Phase 3 - Type System and Inference]]**