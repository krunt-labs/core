---
tags: [mirror, compiler, eac, codegen, target]
---

# Mirror — EAC Ch 11 "Instruction Selection" → Target Contract / Codegen

Maps instruction-selection theory (one IR, many backends) onto multi-target codegen. (Book: *Engineering a Compiler* 2nd ed., Ch 11, pp.597.)

## The mapping
| EAC concept | Graph Giraffe equivalent |
|---|---|
| **Code generation** (§11.2) | Lowering validated IR → an artifact set for one target |
| **Treewalk scheme** (§11.3) | Walking IR ops and emitting target structures |
| **Tree-pattern matching / tiling** (§11.4) | Matching IR sub-graphs and choosing which template to emit (this is how you express a flow in the *target's* idioms) |
| **Peephole optimization** (§11.5) | Local cleanup of the generated text/artifacts before write-out |
| **Target selection (one IR → many) throughout** | [[Target Contract]]: one IR, many generators |

## Key lesson
The classic compiler does not change its IR per machine; each **backend** "tilings" the IR into that machine's idiom. This is the exact precedent for multi-target Graph Giraffe — extend to libraries/other languages by adding a generator behind the Target Contract, not by changing the IR. See [[02 Architecture - IR to Target Mapping]].

## Cross-links
- [[Target Contract]] · [[Source Mapping]] · [[Phase 6 - Lowering and Multi-Target Codegen]] · [[Crate 06 - codegen]]

**Book pages (approx):** codegen §11.2 p600 · treewalk extension §11.3 p603 · pattern matching/tiling §11.4 p610 · peephole §11.5 p621 · scheduling Ch 12 (skim) p639 · register allocation Ch 13 (skip) p679