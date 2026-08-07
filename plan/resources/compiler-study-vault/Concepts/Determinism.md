---
tags: [concept, compiler, determinism]
---

# Determinism

A guarantee the whole platform rests on: **the same graph compiles to byte-identical output, every time.**

## Key idea
Determinism is not a single step — it must be preserved at *every* stage:
- **No hidden inputs**: no wall-clock time, random IDs, absolute paths, or environment-dependent ordering leaking in.
- **Stable ordering**: iteration, passes, and diagnostics are canonically sorted; unordered containers are replaced with ordered ones or sorted before serialization.
- **Pure passes**: optimizations are `IR -> Result<IR>` transforms with no side effects, so any pass ordering question is a pure function of its input.
- **Incremental-safe**: passes declare whether re-running is safe; idempotence where claimed.

## Why it matters here
Determinism is a core sales point (RFC §5.2) and the thing that separates graph-as-source codegen from AI-generated white-space. It is verified by repeat-compile tests at every phase and by golden projects.

## Appears in
- [[Phase 1 - Canonical IR]] (requirements + verify)
- [[Phase 4 - Semantic Analysis and Graph Front-End]]
- [[Phase 7 - Quality and Integration]]
- Skill `graph-giraffe-ir-optimization` ("deterministic in identity and order") and `graph-giraffe-quality`