---
tags: [chapter, tapl, compiler, types]
---

# Chapter — TAPL Ch 22: Type Reconstruction

**Book:** *Types and Programming Languages*, Ch 22. Hindley–Milner inference — the deepest, most important chapter for Graph Giraffe's type system.

## While reading, capture
- Type variables and constraint generation.
- **Unification** — the substitution engine.
- The inference algorithm and its source of unpredictable failure (the "occurs check," cycle detection).
- `let`-polymorphism and how generics generalize.

## How this feeds the project
Direct blueprint for [[Crate 02 - graph-types]] type inference across data edges. See [[TAPL Ch22 - graph-types]] and [[Unification - Type Inference]].

## Links
- [[Phase 3 - Type System and Inference]] · [[Phase 2 - Data-Flow and SSA]] (edge-based constraint flow)
- Deferred partners: Subtyping (Ch 15), Recursive (Ch 20), Universal/Existential (Ch 23–24)