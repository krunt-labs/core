---
tags: [chapter, eac, compiler, ir]
---

# Chapter — EAC Ch 5: Intermediate Representations

**Book:** *Engineering a Compiler* 2nd ed., Ch 5 (pp.221). Core IR reading.

## While reading, capture
- The spectrum: graphical (trees, DAGs, graphs) vs linear (stack, three-address, ILOC) — §5.1 taxonomy.
- How a CFG is built from a linear program — §5.3.4.
- Naming temporary values and **SSA form** — §5.4.
- What symbol tables solve and where scopes break — §5.5.

## How this feeds the project
Design of the [[Crate 01 - ir]] canonical IR; see [[EAC Ch5 - IR]] and [[IR Taxonomy]].

## Links
- [[Phase 1 - Canonical IR]] · [[Static Single Assignment Form]] · [[Source Mapping]]
- **Pages:** taxonomy §5.1 p223 · graphical §5.2 p226 · linear §5.3 p235 · names/SSA §5.4 p243 · symbols §5.5 p253