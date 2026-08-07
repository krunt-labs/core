---
tags: [moc, compiler, graph-giraffe]
---

# 00 MOC — Graph Giraffe Compiler Study

Map of content for the IR → Compiler → Codegen study plan. Study is done **while building**: each phase reads the concepts, then builds (or verifies) a crate. Open Graph View to see the clustering around IR, Type System, and Codegen.

## Where this sits in the project
- **Project RFC:** `plan/architecture/RFC-001.md`
- **Skills router:** `.agents/skills/graph-giraffe-development/SKILL.md`
- **Concern skills:** `graph-giraffe-ir-optimization`, `graph-giraffe-compiler`, `graph-giraffe-type-system`, `graph-giraffe-plugin-system`, `graph-giraffe-backend-dsl`, `graph-giraffe-typescript-codegen`, `graph-giraffe-quality`, `graph-giraffe-graph-model`

## Core premise
A graph compiler has **no scanner/parser** — the "source" is a structured graph document. So everything from classic compiler theory transfers starting at context-sensitive analysis and intermediate representations; the parsing chapters are skipped. The "front-end" becomes structural validation + plugin resolution. See [[02 Architecture - IR to Target Mapping]].

## Study protocol
[[01 Protocol - Study While Building]]

## Parts (read in order)
- [[Phase 0 - Orientation and Rust]]
- [[Phase 1 - Canonical IR]]
- [[Phase 2 - Data-Flow and SSA]]
- [[Phase 3 - Type System and Inference]]
- [[Phase 4 - Semantic Analysis and Graph Front-End]]
- [[Phase 5 - Optimization Passes]]
- [[Phase 6 - Lowering and Multi-Target Codegen]]
- [[Phase 7 - Quality and Integration]]

## Entry points by focus
- **IR:** [[Phase 1 - Canonical IR]] · [[IR Taxonomy]] · [[EAC Ch5 - IR]]
- **Compiler orchestration:** [[Phase 4 - Semantic Analysis and Graph Front-End]]
- **Type inference (deepest):** [[Phase 3 - Type System and Inference]] · [[Unification - Type Inference]]
- **Multi-target / beyond TypeScript:** [[Phase 6 - Lowering and Multi-Target Codegen]] · [[Target Contract]]

## Concepts
[[IR Taxonomy]] · [[Static Single Assignment Form]] · [[Dominance]] · [[Source Mapping]] · [[Determinism]] · [[Unification - Type Inference]] · [[Target Contract]]

## Crates this builds
[[Crate 01 - ir]] → [[Crate 02 - graph-types]] → [[Crate 03 - semantic-api]] → [[Crate 04 - compiler-core]] → [[Crate 05 - backend-dsl]] → [[Crate 06 - codegen]] → [[Crate 07 - cli-wasm]]