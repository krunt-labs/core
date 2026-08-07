---
tags: [phase, compiler, graph-giraffe, codegen, multi-target]
---

# Phase 6 — Lowering & Multi-Target Codegen

> Status: not started
- [ ] Learn completed
- [ ] [[Crate 06 - codegen]] Build + Verify (first generator behind the [[Target Contract]]) completed
- [ ] *Proof:* a **library** target (not a second app framework) generated from the same IR

## Goal
Lower validated IR into an artifact set — starting with a TypeScript/Node generator — behind the  Target Contract boundary so more targets (libraries, other languages) slot in later. This is where [[02 Architecture - IR to Target Mapping]] becomes concrete.

## Learn (must)
- **EAC Ch 11** — *Instruction Selection* (pp.597):
  - code generation (§11.2),
  - extending the simple treewalk scheme (§11.3),
  - **instruction selection via tree-pattern matching / tiling** (§11.4),
  - peephole optimization (§11.5) — a template for local output cleanup.

## Learn (skim)
- **EAC Ch 12** — *Instruction Scheduling* (§12.3 local list scheduling) as the analogy for ordering operations within a generated handler.

## Skip
- **EAC Ch 13** — Register Allocation (irrelevant to high-level output).

## Map to Graph Giraffe
- IR → **artifact plan** (files, dirs, routes, handlers, models, deps) against the  Target Contract.
- Mapping is **N:M** (Subgraph → folder+function; Entity → model+migration+repo).
- Node-ID traceability must survive into generated output — [[Source Mapping]].
- Determinism: identical graphs → byte-identical files.

## Build
Per [[Crate 06 - codegen]] DoD: first generator behind the Target Contract; then a **library** target to prove the model (per [[02 Architecture - IR to Target Mapping]] practice step).

## Verify
- **Golden projects**: generated tree compared byte-for-byte.
- Generated TypeScript type-checks (`tsc`) and runs a smoke test.
- Add a second target only after the first two are stable.

**Next → [[Phase 7 - Quality and Integration]]**