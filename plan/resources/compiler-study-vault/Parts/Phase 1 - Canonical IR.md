---
tags: [phase, compiler, graph-giraffe, ir]
---

# Phase 1 — Canonical IR

> Status: not started
- [ ] Learn completed
- [ ] [[Crate 01 - ir]] Build + Verify completed

## Goal
Design the canonical, validated, deterministic IR that every generator consumes. *This is the intellectual heart of your focus.*

## Learn (must)
- **EAC Ch 5** — *Intermediate Representations* (pp.221): IR taxonomy (5.1), graphical vs linear IRs (5.2–5.3), building CFG from linear code (5.3.4), naming temps + SSA (5.4), symbol tables (5.5).
- **EAC Appendix A** — *ILOC* (pp.725), **Appendix B.3** — *Implementing Intermediate Representations* (pp.743).
- **Skills** — `graph-giraffe-ir-optimization` (the definitive requirements list) and `graph-giraffe-graph-model` (what the source graph document provides).

## Learn (skim)
- EAC Ch 7 *Code Shape* (pp.331) — only the **control-flow constructs** §7.8 (conditional, loops, case) for intuition on lowering Branch/Merge/Loop shapes.

## Map to Graph Giraffe
- Your IR is a **graph IR (CFG-like)** whose nodes are *domain ops* (not ILOC machine ops) — see [[EAC Ch5 - IR]].
- IR must be immutable, deterministic, source-mapped, versioned — see [[Source Mapping]] and [[Determinism]].

## Build
Per [[Crate 01 - ir]] DoD:
- `IROp`, `IRNode`, control + data edges, source-map to graph element IDs;
- invariant-guarded builder / checked constructors (no unresolved types, no dangling edges);
- deterministic `serde`.

## Verify
- Construction-rejection tests (invariant violations panic as compiler defects).
- Serialization-stability tests (byte-for-byte across runs and machines).
- Accept an invalid graph only as `Err` with a diagnostic; never emit.

## Concepts to write
[[IR Taxonomy]] · [[Source Mapping]] · [[Determinism]]

**Next → [[Phase 2 - Data-Flow and SSA]]**