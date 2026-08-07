---
tags: [concept, compiler, ir]
---

# IR Taxonomy

A way of classifying intermediate representations by the "shape" of the data you store and the operations you allow.

## Key idea (EAC Ch 5.1)
IRs fall on a spectrum:
- **Graphical** — trees and **DAGs** (compact, share common subexpressions) and **graphs** (explicit control flow via a **control-flow graph / CFG**).
- **Linear** — stack-machine code (postfix), three-address code (one op, at most three operands), and register transfer / ILOC-style.

The right choice depends on what the later stages (optimization, type-check, codegen) need easy access to.

## Why it matters here
A graph compiler's natural IR is a **graph IR** — a CFG-like structure whose nodes are *semantic operations* rather than machine instructions. Understanding the taxonomy warns against collapsing the representation too early into linear code, which would bury the dependency structure the type system and scheduling rely on.

## Appears in
- [[EAC Ch5 - Intermediate Representations]]
- [[Phase 1 - Canonical IR]]
- [[EAC Ch5 - IR]]
- [[Dominance]] (graph-based analyses), [[Static Single Assignment Form]] (structural refinement of a graph IR)