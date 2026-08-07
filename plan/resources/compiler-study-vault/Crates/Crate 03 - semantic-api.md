---
tags: [crate, compiler, plugins]
---

# Crate 03 — `semantic-api`

The plugin and contract interfaces a semantic domain (e.g. the Backend DSL) implements.

## Definition of Done
- [ ] Node descriptor + port/property schema abstractions
- [ ] Contract input/output typing surface (RFC §10)
- [ ] Capability + requirement + side-effect declarations (RFC §11)
- [ ] Semantic validator hook points
- [ ] Compiler hook points (lowering)
- [ ] Versioned compatibility in the manifest (plugin-system skill)
- [ ] Deterministic registration order

## Requirements (from `graph-giraffe-plugin-system` skill)
Node descriptors, port and property schemas, contracts, capabilities, requirements, side effects, guarantees, semantic validators, compiler hooks, plugin registries, lifecycle, compatibility, manifests.

## Build order
Interfaces first, in **[[Phase 4 - Semantic Analysis and Graph Front-End]]**; the concrete Backend DSL implementation lives in [[Crate 05 - backend-dsl]].

## Cross-links
- [[Target Contract]] · [[Phase 4 - Semantic Analysis and Graph Front-End]] · [[Crate 05 - backend-dsl]]