---
tags: [crate, compiler, backend-dsl]
---

# Crate 05 — `backend-dsl`

The first semantic domain: Backend DSL nodes, validation rules, and IR lowering.

## Definition of Done
- [ ] v1 node families registered (Entry, Flow, Persistence, Effects, Response, Error handling)
- [ ] Contracts + capabilities + side effects declared per node
- [ ] Semantic validators
- [ ] Lowering hook: semantic graph → canonical IR (the plugin boundary in [[Crate 03 - semantic-api]] implemented)
- [ ] Deterministic IR output with source mappings preserved

## Requirements (from `graph-giraffe-backend-dsl` skill)
HTTP Route, Cron, Branch, Merge, Transform, Loop, Subgraph, Entity, Repository, Query, Mutation, HTTP Request, Logging, Storage, Success Response, Error Response, Try/Catch, Validation Error — properties, ports, contracts, capabilities, effects, guarantees, diagnostics, and lowering into canonical IR.

## Build order
Interface + descriptors first; **full lowering in [[Phase 4 - Semantic Analysis and Graph Front-End]]** and consumption by [[Crate 06 - codegen]] in [[Phase 6 - Lowering and Multi-Target Codegen]].

## Cross-links
- [[Crate 03 - semantic-api]] · [[Crate 01 - ir]] · [[Target Contract]] · [[EAC Ch4 - Context-Sensitive Analysis]] (not yet created — add when studying)