---
tags: [mirror, compiler, eac, dataflow, types]
---

# Mirror — EAC Ch 9 "Data-Flow Analysis" → Type Inference & Dead-Node

Maps data-flow theory onto the project's graph-wide analysis. (Book: *Engineering a Compiler* 2nd ed., Ch 9, pp.475.)

## The mapping
| EAC concept | Graph Giraffe equivalent |
|---|---|
| **Dominance** (§9.2.1) | Reachability + dead-branch/unreachable-node detection across the execution graph |
| **Live-variable analysis** (§9.2.2) | Which data-edge values are still needed downstream → drives pruning and diagnostics |
| **Reaching definitions** (§9.2.4) | Use–def chains along data edges → the backbone of type propagation across the data graph |
| **SSA construction** (dominance frontiers, φ, renaming, §9.3) | Formalizes how joins merge values/types on convergent data edges |
| **Interprocedural / call-graph** (§9.4) | Subgraph boundaries: how types reach across a subgraph interface |

## Key lesson
The data graph's acyclic nature simplifies these analyses, but the phrasing stays the same: we compute what *reaches* each node/port so the type checker and scheduler can work on real, per-edge facts rather than guesses.

## Cross-links
- [[Dominance]] · [[Static Single Assignment Form]] · [[Phase 2 - Data-Flow and SSA]] · [[Phase 4 - Semantic Analysis and Graph Front-End]]

**Book pages (approx):** iterative + dominance §9.2 p477 · SSA §9.3 p495 · interprocedural §9.4 p519