---
tags: [chapter, "II. A Tree-Walk Interpreter"]
part: "II. A Tree-Walk Interpreter"
---

# 11. Resolving and Binding

**Part:** [[II. A Tree-Walk Interpreter]]
**Previous:** [[10 Functions]] · **Next:** [[12 Classes]]

## Summary
Fixes subtle scoping bugs by adding a static resolution pass between parsing and interpreting, so each variable reference is bound once, consistently, rather than dynamically at runtime.

## Key concepts
- [[Static Scope Resolution]]
- [[Environment (Scope)]]
