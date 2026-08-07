---
tags: [chapter, eac, compiler, semantics]
---

# Chapter — EAC Ch 4: Context-Sensitive Analysis

**Book:** *Engineering a Compiler* 2nd ed., Ch 4 (pp.161). Semantic analysis for the front-end.

## While reading, capture
- Intro to type systems — §4.2 (purpose, components).
- The **attribute-grammar framework** — §4.3 (propagating facts across a structure; a strong model for graph-wide semantic analysis).
- Ad hoc syntax-directed translation — §4.4.
- Harder problems in type inference — §4.5.1.

## How this feeds the project
The model for [[Phase 4 - Semantic Analysis and Graph Front-End]]: attribute-like propagation of types/contracts/capabilities over the graph, and the discipline of resolving names before typing. See [[CI Ch11 - Scope]] for the resolution discipline and [[Crate 04 - compiler-core]] for the stage it lands in.

## Links
- [[Phase 4 - Semantic Analysis and Graph Front-End]] · [[Crate 03 - semantic-api]] · [[Crate 05 - backend-dsl]]
- **Pages:** type systems §4.2 p164 · attribute grammar §4.3 p182 · ad hoc SDT §4.4 p198 · harder type inference §4.5.1 p211