---
tags: [mirror, compiler, eac, ir]
---

# Mirror — EAC Ch 5 "Intermediate Representations" → Graph Giraffe

Maps textbook IR theory onto the project's canonical IR. (Book: *Engineering a Compiler* 2nd ed., Ch 5, pp.221.)

## The mapping
| EAC concept | Graph Giraffe equivalent |
|---|---|
| **IR taxonomy** (graphical vs linear, §5.1) | Our IR is a **graph IR** (CFG-like); we keep this as the primary form because we need dependency structure for type-check + scheduling |
| **Graphical IR — DAGs** (§5.2) | Shared sub-expressions across data edges (optimization/redundancy) |
| **Graphical IR — graphs / CFG** (§5.2.2) | The IR's control structure: branch, merge, loop, subgraph |
| **Linear IR — three-address / ILOC** (§5.3, App A) | A target-specific *artifact* shape used at codegen time, not in analysis |
| **Naming temporary values** (§5.4.1) | Internal value nodes on data edges |
| **SSA form** (§5.4.2) / Ch 9 | Data edges are already near-SSA: a value on an edge has one producer |
| **Symbol tables / scopes** (§5.5) | Namespaces of subgraph/module nodes |

## Key lesson
**Do not flatten the graph into linear code too early.** If analysis collapses into a linear IR, the type system and scheduler lose the dependency structure they depend on. Keep a graph IR as the canonical form; linearize only at the generator/target boundary ([[Target Contract]]).

## Cross-links
- [[IR Taxonomy]] · [[Static Single Assignment Form]] · [[Phase 1 - Canonical IR]] · [[Crate 01 - ir]]

**Book pages (approx):** taxonomy §5.1 p223 · graphical §5.2 p226 · linear §5.3 p235 · values/names §5.4 p243 · SSA §5.4.2 p246 · symbols §5.5 p253 · ILOC Appendix A p725 · implementing IRs Appendix B.3 p743