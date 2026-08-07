---
tags: [mirror, compiler, tapl, types]
---

# Mirror — TAPL Ch 22 "Type Reconstruction" → graph-types

Maps Hindley–Milner type inference onto the reusable type system crate. (Book: *Types and Programming Languages*, Ch 22.)

## The mapping
| TAPL concept | Graph Giraffe equivalent |
|---|---|
| **Type variables** | Placeholders on data edges/ports awaiting inference |
| **Constraint generation from syntax** | Constraints generated from port schemas + edge connections |
| **Unification** | Assignability/consistency between producer and consumer ports |
| **Inference failure → diagnostic** | A source-mapped type error on the offending edge/node |
| **`let`-polymorphism** (the HM trick) | Generics like `Option<T>` / `Result<T,E>` at node interfaces |

## Key lesson
Type inference is not a "type checker pass" bolted on at the end — it is a constraint solver the whole [[Phase 3 - Type System and Inference]] stage is built around. Deferred from v1: subtyping (Ch 15), recursive (Ch 20), universal/existential (Ch 23–24).

## Cross-links
- [[Unification - Type Inference]] · [[Phase 3 - Type System and Inference]] · [[Crate 02 - graph-types]]