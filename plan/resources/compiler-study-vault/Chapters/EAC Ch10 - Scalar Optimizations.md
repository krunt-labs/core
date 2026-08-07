---
tags: [chapter, eac, compiler, optimization]
---

# Chapter — EAC Ch 10: Scalar Optimizations

**Book:** *Engineering a Compiler* 2nd ed., Ch 10 (pp.539). Concrete optimization passes.

## While reading, capture
- **Eliminating useless & unreachable code** — §10.2 (the direct model for dead-node elimination).
- Code motion — §10.3 (hoisting, lazy code motion) — use sparingly; never reorder impure effects.
- Redundancy elimination / value numbering — §10.5.

## How this feeds the project
The exact transforms in RFC optimization list: DCE, dead-branch, unreachable, dependency pruning. Pass discipline in [[Phase 5 - Optimization Passes]].

## Links
- [[Phase 5 - Optimization Passes]] · [[Determinism]]
- **Pages:** useless/unreachable §10.2 p544 · code motion §10.3 p551 · redundancy §10.5 p565