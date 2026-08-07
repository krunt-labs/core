---
tags: [crate, compiler, codegen, multi-target]
---

# Crate 06 — `codegen`

Generate artifact sets from validated IR, behind the Target Contract boundary.

## Definition of Done (first generator: TypeScript/Node service)
- [ ] Define the Target Contract (artifact-set schema): files, dirs, entrypoint, routes, handlers, models, repos, deps
- [ ] Generator translates IR → artifact plan → concrete files
- [ ] Node-ID traceability retained in output (see [[Source Mapping]])
- [ ] Deterministic formatting (byte-identical for identical graphs)
- [ ] **Golden projects** compared byte-for-byte
- [ ] Generated TypeScript type-checks (`tsc`) + runtime smoke test
- [ ] *Proof target:* a **library**-kind target generated from the same IR (per [[02 Architecture - IR to Target Mapping]]) — before any second app framework

## Requirements (from `graph-giraffe-typescript-codegen` skill)
Generated project layout, routes, handlers, models, repositories, validation, effects, error handling, migration scaffolds, tests, README content, framework adapters, templates, formatting, dependency versions, node-ID traceability, archive export, deterministic golden output.

## Build order
First generator in **[[Phase 6 - Lowering and Multi-Target Codegen]]**.

## Cross-links
- [[Target Contract]] · [[Source Mapping]] · [[Crate 05 - backend-dsl]] · [[EAC Ch11 - Target Contract]]