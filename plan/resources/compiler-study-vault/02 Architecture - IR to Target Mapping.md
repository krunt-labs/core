---
tags: [architecture, compiler, graph-giraffe, multi-target]
---

# 02 Architecture — IR to Target Mapping (N:M)

Question this note answers: *"If we extend code generation to framework codebases and libraries beyond TypeScript, won't the extra abstraction layer break the mapping between our computed flow and the generated code?"*

**Answer: No — provided we keep the mapping N:M with traceability, and add one explicit boundary: the Target Contract.**

## Why it does not break
The IR is **target-independent**; every generator handles the "idiom" mapping. This is the standard compiler pattern — x86 vs ARM vs RISC-V do not express a compiler's flow one-to-one either. Each backend transforms and "tilings" it locally. See [[EAC Ch11 - Instruction Selection]].

The mapping between graph elements and generated artifacts is **N:M, not 1:1**:
- one Subgraph node → folder + function(s)
- one Entity node → model file + migration + repository
- many nodes → one file

That is fine because the *source mapping* (traceability) is preserved: every emitted artifact can trace back to graph element IDs. That is what [[Source Mapping]] exists for — not 1:1 emission.

## The layered model
```
Graph (source)
 └─ compiler-core  → target-independent analysis + validation
     └─ Canonical IR → NO framework knowledge (no "express", no filenames)
         └─ Target Contract  ← schema of an artifact set
         │     "Service" (app: entrypoint/routes/handlers) | "Library" (importable modules) | ...
         └─ Generator per target kind
               └─ framework adapters (express / fastify / ...) — generator-local
```

## Invariants that protect the mapping
1. **IR never leaks target idioms upward** — the highest-leverage rule. A "middleware" IR op would couple analysis to Express.
2. **The Target Contract is a second public boundary**, versioned like the plugin API. Generators emit an *artifact plan* against it.
3. **Libraries are easier targets than framework apps** — no idioms, just modules. Extending to libraries (Rust crate, Python package) is a lower-risk generator than a second app framework.
4. **Validation stays in the compiler** — target-specific constraints are checked by the generator *declaring required IR properties* (e.g., "no cycles", "effects ordered").

## Risks (scope, not architecture)
- Stabilizing the Target Contract while shipping the first TypeScript generator (version it early, resist premature generality).
- Letting target idioms creep up into IR/analysis (the invariant above).

## Relation to RFC
- RFC "Open decisions": *framework-neutral TypeScript adapter*, *public vs internal canonical IR*, *composition of multiple DSLs* — this note resolves the first two deliberately.
- Skills: [[Target Contract]] ties to `graph-giraffe-typescript-codegen` ("framework adapters") and `graph-giraffe-ir-optimization` ("expressive enough for multiple generators").

## Practice
When you reach [[Phase 6 - Lowering and Multi-Target Codegen]], implement the first generator behind the Target Contract boundary, then add a *library* target (not a second app framework) to prove the model before touching a second framework.