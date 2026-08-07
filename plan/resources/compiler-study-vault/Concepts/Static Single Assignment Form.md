---
tags: [concept, compiler, ssa]
---

# Static Single Assignment Form (SSA)

An IR invariant: **every variable is assigned exactly once in the program text**, and a special **φ-function** (`phi`) merges the values arriving at a control-flow join.

## Key idea (EAC Ch 5.4.2, 9.3)
- Single assignment makes every value's "define" point unique, so reaching uses (use–def chains) are trivial: whoever reads a name learns its one defining operation directly.
- Building SSA requires three steps: place φ-functions (guided by **dominance frontiers**), rename variables consistently, and optionally convert back out of SSA afterward.

## What SSA buys
- Cheap, precise reaching-definition / use–def chains.
- Simpler and safer optimization (constant propagation, redundancy elimination), because facts attach to one point.
- A strong model for *invariant-rich representations*: the discipline of "a representation whose structure makes certain errors impossible."

## Why it matters here
The graph's **data edges** are already close to SSA: a value flowing on an edge has a unique producer. Formalizing the SSA idea clarifies how [[Phase 2 - Data-Flow and SSA]] type propagation and schedules across data edges can exploit this.

## Appears in
- [[EAC Ch9 - Data-Flow Analysis]]
- [[Phase 2 - Data-Flow and SSA]]
- [[Dominance]] (φ-placement), [[IR Taxonomy]]