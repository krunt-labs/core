---
tags: [concept, compiler, types]
---

# Unification / Type Inference

The algorithmic heart of a relation between: given partially known types, deduce and **check the unknown ones by constraint solving**.

## Key idea (TAPL Ch 22, Hindley–Milner)
- Represent unknown types as **type variables**; treat the program as a set of equations between types.
- **Unify** two types: try to make them equal, substituting a variable for a concrete type (or bind two variables), failing gracefully when they cannot match (e.g., `int` vs `string`).
- Iteratively solve the constraints; the result is an assignment of types to every variable, or a "cannot unify here" diagnostic that the compiler can source-map to a specific edge/port.
- A *constraint-based* inference is the practical anchor of the RFC's "type inference propagates automatically across compatible edges."

## Why it matters here
Graph Giraffe's type system must support primitives, struct/record, collections, `Option<T>`, `Result<T,E>`, `Future`, `Stream`, unions and intersections, and **infer across data edges**. Unification is the engine; assignability/subtyping extends it for unions, and generics (`Option`/`Result`) require a quantifier-aware variant.

## Appears in
- [[TAPL Ch 22 - Type Reconstruction]]
- [[Phase 3 - Type System and Inference]]
- [[TAPL Ch22 - graph-types]]
- Skill `graph-giraffe-type-system`