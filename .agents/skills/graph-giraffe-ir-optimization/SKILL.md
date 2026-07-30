---
name: graph-giraffe-ir-optimization
description: Design, implement, serialize, transform, test, or review Graph Giraffe's validated canonical intermediate representation and optimization passes. Use for IR nodes and operations, lowering contracts, source mappings, control/data representation, IR invariants, dead-node elimination, constant folding, subgraph inlining, dependency pruning, scheduling, parallelization, memoization, and semantic-preservation checks.
---

# Build Canonical IR and Optimization

Own the compiler representation after semantic and type validation. Distinguish the graph as canonical user source from IR as canonical compiler input and generated files as artifacts.

## Define validated IR

Make IR:

- immutable after construction;
- serializable and versioned where persisted or exposed;
- independent of the editor and UI;
- deterministic in identity and order;
- expressive enough for multiple generators;
- explicit about control flow, data dependencies, types, effects, and async behavior;
- traceable to source graph node and port IDs.

Construct executable IR only through validated lowering APIs. Use builders or checked constructors to prevent unresolved types, missing dependencies, and invalid operations.

## Exclude unrelated state

Do not include:

- canvas coordinates, groups, selection, or undo history;
- AI prompts and provider metadata;
- unresolved plugin property bags;
- Express/Fastify imports or filenames;
- wall-clock timestamps, random IDs, or absolute paths.

Keep backend operations in a domain extension or well-defined semantic operation set rather than contaminating the universal core.

## Design pass contracts

Each optimization pass must declare:

- required input invariants;
- produced output invariants;
- observable semantics it preserves;
- source-map behavior;
- deterministic ordering behavior;
- whether it is safe for incremental or parallel execution.

Prefer `IR -> Result<IR, InternalDiagnostic>` pure transformations. Treat broken IR invariants as compiler defects, not user graph errors.

## Optimize conservatively

Implement only semantics-backed transformations such as:

- dead-node and unreachable-operation elimination;
- constant folding;
- graph flattening and subgraph inlining;
- common subgraph elimination;
- dependency pruning;
- execution scheduling;
- safe parallelization;
- memoization for explicitly pure operations.

Never reorder impure effects without a proven ordering rule. Never infer purity from absence of known effects; consume resolved capability/effect facts.

## Implement an IR change

1. Define the invariant and compatibility impact.
2. Update lowering at semantic-plugin boundaries.
3. Update every affected pass and generator.
4. Preserve or deliberately combine source mappings.
5. Add construction rejection tests and serialization stability tests.
6. Add before/after pass fixtures proving semantic preservation.

## Verify determinism

Run repeated lowering and optimization with different insertion orders. Compare canonical IR and generator-visible order exactly. Test idempotence where a pass claims it.
