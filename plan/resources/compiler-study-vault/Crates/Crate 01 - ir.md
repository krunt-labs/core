---
tags: [crate, compiler, ir]
---

# Crate 01 — `ir`

The canonical, validated intermediary representation.

## Definition of Done
- [ ] `IROp` enum + `IRNode` struct with control + data edges
- [ ] Source-map to graph element IDs (see [[Source Mapping]])
- [ ] Invariant-guarded builder / checked constructors (unresolved types, dangling edges rejected)
- [ ] Deterministic `serde` (stable field order, no unordered maps exposed)
- [ ] Immutable after construction
- [ ] Construction-rejection tests for each invariant
- [ ] Serialization-stability tests (byte-for-byte across runs/machines)

## Requirements (from `graph-giraffe-ir-optimization` skill)
- Deterministic in identity and order; independent of editor/UI.
- Expressive enough for multiple generators (see [[Target Contract]]).
- Explicit about control flow, data dependencies, types, effects, async.
- Traceable to source node/port IDs.
- No Express/Fastify names, filenames, timestamps, or random IDs.

## Build order
Built in [[Phase 0 - Orientation and Rust]] (skeleton) and **[[Phase 1 - Canonical IR]]** (full).

## Cross-links
- [[IR Taxonomy]] · [[Source Mapping]] · [[Determinism]] · [[EAC Ch5 - IR]]