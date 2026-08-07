---
tags: [chapter, eac, compiler, dataflow]
---

# Chapter — EAC Ch 9: Data-Flow Analysis

**Book:** *Engineering a Compiler* 2nd ed., Ch 9 (pp.475). The analysis backbone.

## While reading, capture
- Iterative data-flow analysis and its fixed-point shape — §9.2.
- **Dominance**: definitions, the dominator tree — §9.2.1.
- **Live-variable analysis** — §9.2.2.
- Reaching definitions / "other data-flow problems" — §9.2.4.
- **SSA construction**: dominance frontiers, φ-placement, renaming, out-of-SSA — §9.3.
- Interprocedural analysis / call-graph — §9.4.

## How this feeds the project
Reachability, cycle/dead-node detection, and the use–def chains behind type propagation. See [[EAC Ch9 - Type Inference and Dead-Node]].

## Links
- [[Phase 2 - Data-Flow and SSA]] · [[Dominance]] · [[Static Single Assignment Form]]
- **Pages:** iterative + dominance §9.2 p477 · SSA §9.3 p495 · interprocedural §9.4 p519