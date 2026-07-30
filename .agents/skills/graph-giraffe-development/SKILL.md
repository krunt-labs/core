---
name: graph-giraffe-development
description: Coordinate cross-layer Graph Giraffe architecture, implementation, review, and planning. Use when a task spans multiple concerns, introduces a vertical feature, changes public boundaries, or needs routing to the graph model, type system, plugin system, Rust compiler, IR and optimization, Backend DSL, TypeScript codegen, editor, AI authoring, or quality skills.
---

# Coordinate Graph Giraffe Development

Treat Graph Giraffe as a deterministic graph compiler platform. Use this skill to route work and preserve system-wide boundaries; load the focused skills that own the actual change.

## Read the source of truth

For architectural or cross-layer work, read:

- `plan/architecture/RFC-001.md`
- `plan/architecture/graph-giraffe-backend-tool-prd.pdf`

Treat the RFC as the platform design and the PRD as the v1 Backend DSL scope. Follow explicit user decisions over draft documents. Surface contradictions and open questions instead of silently creating a durable platform contract.

## Preserve global invariants

- Treat the versioned graph as user source and generated code as an artifact.
- Implement the compiler in Rust.
- Generate TypeScript/Node backends for the first target.
- Produce identical output for equivalent graph input.
- Separate execution flow from typed data flow.
- Validate before lowering and code generation.
- Keep the graph core and editor domain-agnostic.
- Put semantics in plugins.
- Let AI author graphs but never own semantics, validation, compilation, or execution.
- Keep generated projects readable, independently runnable, and user-owned.
- Never execute arbitrary graph-provided JavaScript.

## Route work by concern

Load every skill whose owned boundary the task changes:

- `.agents/skills/graph-giraffe-graph-model/SKILL.md` — graph documents, IDs, nodes, ports, edges, subgraphs, annotations, serialization.
- `.agents/skills/graph-giraffe-type-system/SKILL.md` — type representation, assignability, constraints, inference, and type diagnostics.
- `.agents/skills/graph-giraffe-plugin-system/SKILL.md` — semantic API, descriptors, contracts, capabilities, registration, lifecycle, and versioning.
- `.agents/skills/graph-giraffe-compiler/SKILL.md` — Rust pipeline orchestration, semantic analysis, graph validation, and diagnostics.
- `.agents/skills/graph-giraffe-ir-optimization/SKILL.md` — validated canonical IR, lowering boundary, source maps, and optimization passes.
- `.agents/skills/graph-giraffe-backend-dsl/SKILL.md` — backend node families, contracts, semantic rules, and IR lowering.
- `.agents/skills/graph-giraffe-typescript-codegen/SKILL.md` — deterministic, readable TypeScript backend generation.
- `.agents/skills/graph-giraffe-editor/SKILL.md` — domain-neutral canvas, editing, presentation state, and compiler integration.
- `.agents/skills/graph-giraffe-ai-authoring/SKILL.md` — prompt-to-graph, graph completion, explanation, and safe proposals.
- `.agents/skills/graph-giraffe-quality/SKILL.md` — test strategy, determinism, architecture checks, golden output, and release evidence.

## Apply dependency direction

```text
editor / AI / CLI / service adapters
                 ↓
compiler orchestration
                 ↓
semantic plugins ──→ type and contract APIs
                 ↓
validated canonical IR
                 ↓
optimization
                 ↓
runtime generators

structural graph model: no dependency on layers above
```

Move shared primitives downward only when they are truly domain-independent. Do not create a generic utilities dumping ground.

## Coordinate a vertical change

1. Identify the source representation, semantic owner, compiler stage, IR effect, generator effect, and user surface.
2. Load each owning skill.
3. Define the smallest interface change at every crossed boundary.
4. Implement from the lowest changed stable layer upward.
5. Test each boundary independently.
6. Add an end-to-end fixture when behavior crosses layers.
7. Run the quality skill's determinism and architecture checks.
8. Record any new durable decision in the RFC or an architecture decision.

For a new Backend DSL node, usually load the graph model only if document shape changes; otherwise load plugin system, type system, Backend DSL, compiler or IR when lowering changes, TypeScript codegen, and quality.

## Keep open decisions explicit

Do not settle these in unrelated work:

- Express, Fastify, or framework-neutral adapters;
- legal cycles and recursion;
- pull-based or push-based data evaluation;
- public or private IR;
- partial compilation behavior;
- generated-code regeneration and ownership;
- async execution representation;
- plugin sandboxing and distribution;
- cross-DSL graphs.

## Finish cross-layer work

Confirm:

- ownership is unambiguous;
- dependency direction has no cycle;
- schemas and public data are versioned;
- errors map to graph element IDs;
- invalid graphs cannot generate runnable output;
- generated files remain deterministic and readable;
- tests cover the narrow layers and the integrated flow.
