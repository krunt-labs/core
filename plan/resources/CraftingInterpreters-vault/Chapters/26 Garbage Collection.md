---
tags: [chapter, "III. A Bytecode Virtual Machine"]
part: "III. A Bytecode Virtual Machine"
---

# 26. Garbage Collection

**Part:** [[III. A Bytecode Virtual Machine]]
**Previous:** [[25 Closures]] · **Next:** [[27 Classes and Instances]]

## Summary
Implements a mark-sweep garbage collector for clox so long-running Lox programs don't leak memory as they allocate strings, closures, and instances.

## Key concepts
- [[Garbage Collection (Mark-Sweep)]]
