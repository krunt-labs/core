---
tags: [crate, compiler]
---

# Crate 04 — `compiler-core`

Pipeline orchestration, analysis, and deterministic diagnostics.

## Definition of Done
- [ ] Explicit stage ordering per RFC §13 (structural → plugin → flow views → semantic → type → contract → capability → whole-graph → IR → optimize → generate)
- [ ] Deterministic, canonically-sorted diagnostics
- [ ] Short-circuit on error-severity diagnostics (no emit)
- [ ] Stable code + severity + primary element ID + related IDs + stage + help/fix
- [ ] Pure compile/validate APIs where possible
- [ ] Stage-gate tests (each stage's invalid + valid fixtures)
- [ ] End-to-end compile test when stage order / output changes

## Requirements (from `graph-giraffe-compiler` skill)
Own pipeline orchestration; call type/semantic services; control stage order; aggregate diagnostics; handle cycles (only modeled `Loop` legal), reachability, dead-branch; keep rendering/localization out; keep plugin/IR/codegen/match-statements out of generic stages.

## Build order
Assembled in **[[Phase 4 - Semantic Analysis and Graph Front-End]]**; hardened in [[Phase 7 - Quality and Integration]].

## Cross-links
- [[Determinism]] · [[Source Mapping]] · [[Crate 03 - semantic-api]] · [[Phase 4 - Semantic Analysis and Graph Front-End]]