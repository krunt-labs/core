---
tags: [concept, crafting-interpreters]
---

# Jump Instruction & Backpatching

Bytecode instructions that alter control flow by changing the instruction pointer; because jump distances aren't known until later code is compiled, the compiler emits a placeholder and patches in the real offset afterward.

## Appears in
- [[23 Jumping Back and Forth]]
