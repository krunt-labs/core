---
tags: [chapter, eac, compiler, optimization]
---

# Chapter — EAC Ch 8: Introduction to Optimization

**Book:** *Engineering a Compiler* 2nd ed., Ch 8 (pp.405). Orientation to optimization.

## While reading, capture
- Scope of optimization — §8.3 (local / regional / global / interprocedural).
- **Local value numbering** — §8.4.1 (a concrete first optimization).
- Using liveness for global analysis — §8.6.1.
- **Inline substitution** and interprocedural context — §8.7.1.

## How this feeds the project
Framing for [[Phase 5 - Optimization Passes]]: DCE, constant folding, subgraph inlining, dependency pruning.

## Links
- [[Phase 5 - Optimization Passes]] · [[Determinism]]
- **Pages:** scope §8.3 p417 · local value numbering §8.4 p420 · global §8.6 p445 · interprocedural/inline §8.7 p457