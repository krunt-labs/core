---
tags: [mirror, compiler, crafting-interpreters, scope]
---

# Mirror — Crafting Interpreters Ch 11 "Resolving and Binding" → Scope

Maps scope-resolution intuition onto graph/module name spaces. (Book: *Crafting Interpreters*, Ch 11 — a tree-walk interpreter doing a separate analysis pass.)

## The mapping
| CI concept | Graph Giraffe equivalent |
|---|---|
| **Separate resolve pass after parse** | Our semantic analysis runs *after* structural validation + plugin resolution, never inline with editing |
| **Static (lexical) scope** | Subgraph/module name spaces are determined by structure, not by execution path |
| **Resolver errors pointing at a definition** | Diagnostics that source-map to the graph element that owns the conflicting name |
| **Late resolution surprises** | Exactly why we resolve names *before* type-checking, as its own stage |

## Key lesson
Even a tiny interpreter keeps resolution as a dedicated pass with a clear input (syntax) and output (resolved references). Graph Giraffe's graph-front-end needs the same discipline: [[Phase 4 - Semantic Analysis and Graph Front-End]] resolves plugin-defined node/port identities before any typing or contract work.

## Cross-links
- [[Phase 2 - Data-Flow and SSA]] · [[Phase 4 - Semantic Analysis and Graph Front-End]] · [[Source Mapping]]