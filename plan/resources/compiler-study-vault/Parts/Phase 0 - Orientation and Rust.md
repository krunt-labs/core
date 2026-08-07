---
tags: [phase, compiler, graph-giraffe, rust]
---

# Phase 0 — Orientation & Rust On-ramp

> Status: not started
- [ ] Learn completed
- [ ] Build `/ir` crate skeleton completed

## Goal
Gain the minimum orientation to direct the project, and get enough Rust to stand up the first crate. The point is *ability to build*, not fluency.

## Learn (must)
- **RFC** — `plan/architecture/RFC-001.md` §5–14 (graph model, ports, contracts, capabilities, pipeline).
- **Skills** — read `graph-giraffe-development`, `graph-giraffe-compiler`, `graph-giraffe-ir-optimization`, `graph-giraffe-type-system`.
- **MOC & Protocol** — [[00 MOC - Compiler Study]], [[01 Protocol - Study While Building]].
- **EAC Ch 1** — skim *Introduction* (pp.1) to see the compiler-shaped vocabulary.
- **Rust** — get comfortable with: ownership/borrowing, `enum` + `match`, `Result`/`Option`, `serde` derives. (Learn by writing the skeleton, not by a reading tour.)

## Build
Stand up the `ir` crate (see [[Crate 01 - ir]]) with:
- a placeholder `IROp` `enum` and `IRNode` struct;
- `serde` derive + a round-trip test;
- a cargo workspace so later crates (types, compiler-core) slot in.

## Verify
- `cargo test` passes; serde JSON round-trip is stable.
- Workspace compiles from a clean checkout with `--locked`.

## Outputs
- A working `ir` crate skeleton in a `crates/` workspace.
- First `Concept` notes: you can jot [[IR Taxonomy]] and [[Determinism]] early.

**Next → [[Phase 1 - Canonical IR]]**