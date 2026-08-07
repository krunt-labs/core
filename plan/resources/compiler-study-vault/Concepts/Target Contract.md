---
tags: [concept, compiler, multi-target, architecture]
---

# Target Contract

The **second public boundary** (after the plugin API) that lets one validated IR drive many output targets without coupling the compiler to any framework.

## Key idea
- The IR is target-independent. The **Target Contract** is a schema for what an *artifact set* looks like: files, directories, entrypoints, routes, handlers, models, repositories, dependencies, config.
- A **generator** implements the Target Contract for one target kind: a `Service` (app with entrypoint) vs a `Library` (importable modules) vs another language/crate.
- Generators translate IR → **artifact plan** → concrete files, using **generator-local framework adapters** (express, fastify, ...). Adapters never affect IR or analysis.
- Target-specific constraints are expressed as **generator-declared required IR properties** (e.g., "no cycles", "effect order provable") — validation still lives in the compiler.

## Why it matters here
This is what lets Graph Giraffe extend beyond TypeScript into libraries and other codebases without breaking the mapping. See [[02 Architecture - IR to Target Mapping]] for the N:M (traceability-not-1:1) argument.

## Design rules
1. IR never leaks target idioms.
2. The Target Contract is versioned like the plugin API.
3. Libraries are easier than app frameworks — prove the model with a library target before the second framework.
4. Determinism and [[Source Mapping]] hold per target.

## Appears in
- [[Phase 6 - Lowering and Multi-Target Codegen]]
- [[Phase 4 - Semantic Analysis and Graph Front-End]]
- [[EAC Ch11 - Target Contract]]
- Skills `graph-giraffe-typescript-codegen` (adapters, golden output) and `graph-giraffe-ir-optimization` (multi-generator IR)