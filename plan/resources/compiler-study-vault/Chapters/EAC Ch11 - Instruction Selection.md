---
tags: [chapter, eac, compiler, codegen]
---

# Chapter — EAC Ch 11: Instruction Selection

**Book:** *Engineering a Compiler* 2nd ed., Ch 11 (pp.597). Code generation fundamentals.

## While reading, capture
- The **treewalk code generation** scheme — §11.2–11.3.
- **Tree-pattern matching / tiling** — §11.4 (choosing how to express a flow in target idioms).
- **Peephole optimization** — §11.5 (local output cleanup).

## How this feeds the project
Direct model for lowering IR into generated code behind the [[Target Contract]]. See [[EAC Ch11 - Target Contract]].

## Links
- [[Phase 6 - Lowering and Multi-Target Codegen]] · [[Target Contract]]
- **Pages:** codegen §11.2 p600 · treewalk §11.3 p603 · pattern matching/tiling §11.4 p610 · peephole §11.5 p621