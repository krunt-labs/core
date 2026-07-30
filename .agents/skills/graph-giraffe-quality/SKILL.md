---
name: graph-giraffe-quality
description: Test, review, verify, or establish quality gates for Graph Giraffe. Use for unit and integration strategy, invalid-graph fixtures, serialization tests, compiler diagnostics, deterministic repeat compilation, IR pass verification, golden generated projects, generated TypeScript type-checking and runtime smoke tests, architecture dependency checks, performance tests, fuzzing, security review, and release readiness.
---

# Verify Graph Giraffe Quality

Prove correctness at architectural boundaries. Do not use snapshot updates as a substitute for understanding behavior.

## Build the test pyramid by concern

Require:

- graph-model tests for IDs, references, edges, subgraphs, versions, migrations, and canonical serialization;
- type-system tests for assignability, inference, generics, ambiguity, and contradictory constraints;
- plugin-system tests for registration, contracts, versions, missing plugins, and duplicate IDs;
- Backend DSL tests for every node contract, invalid combination, effect, and lowering rule;
- compiler tests for stage order, graph-wide validation, and structured diagnostics;
- IR tests for construction invariants, source maps, and pass semantics;
- generator golden tests for complete deterministic file trees;
- editor tests for commands, undo/redo, generic descriptors, and diagnostic mapping;
- AI tests treating every model response as untrusted;
- end-to-end graph-to-project fixtures.

## Verify determinism explicitly

For the same semantic graph:

1. Vary insertion order where document semantics allow it.
2. Compile repeatedly.
3. Compare diagnostic order.
4. Compare canonical IR.
5. Compare generated relative paths and file bytes.
6. Fail on timestamps, random IDs, absolute paths, hash iteration, or unstable parallel merges.

Do not define determinism merely as equivalent runtime behavior.

## Test invalid input as a product feature

Cover:

- malformed and future graph schemas;
- invalid or missing IDs and ports;
- unknown and incompatible plugins;
- type and contract violations;
- illegal cycles and unreachable graphs;
- unsafe effect ordering;
- unsupported generator operations;
- malformed AI patches.

Assert stable diagnostic codes and graph source references, not only message prose.

## Check architecture

Inspect dependency manifests or add automated checks ensuring:

- graph model imports no DSL, editor, AI, compiler, or generator;
- type system imports no Backend DSL;
- compiler core imports no editor or AI;
- Backend DSL emits IR, not TypeScript;
- generator consumes IR and does not infer semantics;
- editor consumes descriptors and diagnostics rather than domain implementations;
- AI uses public schemas and ordinary validation.

## Validate generated projects

For representative golden projects:

- format and type-check TypeScript;
- install from a locked dependency graph when authorized;
- run generated tests;
- smoke-start the service;
- exercise at least one success and error route;
- inspect readability and node-ID traceability;
- verify no editor or AI dependency exists.

## Report release evidence

Report commands run, fixtures covered, deterministic comparisons, generated-runtime checks, known gaps, and whether failures are product errors or test-infrastructure errors. Never claim full validation when required toolchains or dependencies were unavailable.
