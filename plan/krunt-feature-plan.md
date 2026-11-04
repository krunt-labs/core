# Krunt Feature Plan

**Status:** Implementation baseline

**Related architecture:** [RFC-001](architecture/RFC-001.md)

## Product direction

Krunt is a graph-native backend compiler with multiple language and framework
targets. The first target is TypeScript with NestJS. React Flow is the initial
editor renderer; the canonical graph document and compiler remain independent
of React Flow so Graph Giraffe can replace it later.

```text
Canonical graph document
        ↓
Backend semantic plugin
        ↓
Universal semantic IR
        ↓
Language backend (TypeScript first)
        ↓
Framework backend (NestJS first)
        ↓
Generated project
```

## Phase 0 — Boundaries and vertical slice

- [ ] Define canonical graph document ownership and versioning.
- [ ] Define stable node, port, edge, subgraph, plugin, and diagnostic IDs.
- [ ] Define the editor adapter interface.
- [ ] Add React Flow as an internal editor dependency in `krunt/client/v1`.
- [ ] Render a Route → Query → Transform → Response example.
- [ ] Prove controlled editing, save/load, and diagnostic highlighting.

**Exit criteria:** React Flow is only a projection of the Krunt graph document;
no React Flow node or edge object is used as compiler source.

## Phase 1 — Structural graph model

- [ ] Implement the versioned graph document in Rust.
- [ ] Model explicit semantic ports instead of visual handle sides.
- [ ] Distinguish execution edges from data edges.
- [ ] Implement deterministic serialization and migrations.
- [ ] Add structural validation for IDs, references, ports, and subgraphs.
- [ ] Preserve source mappings for nodes, ports, and edges.

### Implementation checkpoint

The first structural graph-model slice now exists in
`crates/graph-model`:

- [x] Define typed graph, node, port, edge, group, and subgraph IDs.
- [x] Represent explicit port direction and execution/data edge kinds.
- [x] Keep node properties and namespaced extensions opaque to this layer.
- [x] Validate references, directions, edge kinds, duplicate IDs, and schema
      versions.
- [x] Serialize and parse canonical graph documents deterministically.
- [ ] Add migrations and connect structural documents to compiler lowering.

**Exit criteria:** valid documents round-trip deterministically; malformed
documents return structured diagnostics without panics.

## Phase 2 — React Flow editor adapter

- [ ] Map canonical nodes to custom React Flow nodes.
- [ ] Map semantic port IDs to `sourceHandle` and `targetHandle`.
- [ ] Render execution and data edges distinctly.
- [ ] Convert editor changes into Krunt commands.
- [ ] Add grouping, subgraphs, selection, undo/redo, and viewport persistence.
- [ ] Add generic diagnostic badges, overlays, and navigation.
- [ ] Keep compiler validation authoritative over editor-side validation.

**Exit criteria:** editor interaction mutates the canonical graph through
commands and supports the initial Backend DSL node set.

## Phase 3 — Types and semantic plugins

- [ ] Define primitive, composite, collection, `Option`, `Result`, `Future`,
  `Stream`, union, and intersection types.
- [ ] Define contracts, requirements, capabilities, effects, guarantees, and
  execution semantics.
- [ ] Define serializable plugin catalogs for the editor and AI.
- [ ] Implement deterministic plugin registration and version resolution.
- [ ] Define Backend DSL descriptors and contracts.
- [ ] Implement the initial Route, Cron, Branch, Merge, Transform, Loop,
  Subgraph, Entity, Repository, Query, Mutation, HTTP Request, Logging,
  Storage, Success Response, Error Response, Try/Catch, and Validation Error
  nodes.

**Exit criteria:** a synthetic second plugin can use the generic editor and
compiler interfaces without importing Backend DSL code.

### Implementation checkpoint

Task 2 is implemented in `crates/plugin-api`:

- [x] Define serializable plugin identity and supported node-type manifests.
- [x] Define target-neutral semantic lowering results.
- [x] Register and resolve plugins through an explicit deterministic registry.
- [x] Reject duplicate plugin IDs and node-type ownership conflicts.
- [x] Connect the registry to compiler orchestration and backend diagnostics.

Task 4's initial type-system slice is implemented in `crates/type-system`:

- [x] Keep the initial type relation independent of Backend DSL semantics.
- [x] Represent named types and an explicit `ANY` target relation.
- [x] Reject incompatible named data connections before IR lowering.
- [ ] Add composite types, generics, inference, and richer constraints.

## Phase 4 — Compiler and universal IR

- [ ] Implement Rust compiler orchestration.
- [ ] Build separate execution-flow and data-flow views.
- [ ] Implement type constraints and inference.
- [ ] Validate contracts, capabilities, requirements, dependencies, reachability,
  and legal control flow.
- [ ] Reject arbitrary cycles; allow structured cycles only through `Loop`.
- [ ] Lower valid graphs into immutable, deterministic, source-mapped universal
  IR.
- [ ] Block generation on error-severity diagnostics.

### Implementation checkpoint

The first IR slice now exists in `crates/ir`:

- [x] Keep the IR independent of TypeScript, NestJS, and editor details.
- [x] Represent separate control-flow and typed data-flow edges.
- [x] Preserve source mappings to graph node, port, and edge IDs.
- [x] Build immutable graphs through a validating builder.
- [x] Produce deterministic node and edge ordering and JSON serialization.
- [x] Reject duplicate nodes, dangling edges, invalid ports, and type mismatches.
- [ ] Add compiler orchestration, diagnostics, cycle validation, and lowering
      from the canonical graph model.

### Compiler-boundary checkpoint

Task 1 is implemented in `crates/diagnostics`:

- [x] Define stable diagnostic codes, severities, and compiler stages.
- [x] Link diagnostics to primary and related graph element IDs.
- [x] Support renderer-neutral structured help and fix proposals.
- [x] Canonically sort diagnostics and detect error-severity findings.
- [x] Aggregate diagnostics in compiler orchestration and block lowering on
      errors.

### Compiler orchestration checkpoint

Task 3 is implemented in `crates/compiler`:

- [x] Resolve node types through an explicit plugin registry.
- [x] Lower plugin operations into validated IR nodes with source mappings.
- [x] Wire execution and data edges independently.
- [x] Aggregate and canonically sort diagnostics.
- [x] Block IR output when compilation has error-severity diagnostics.
- [x] Run plugin semantic analysis before lowering.
- [x] Run initial data-edge type analysis before lowering.
- [x] Add initial contract validation before IR construction.
- [ ] Add capability resolution and whole-graph validation.

### Contract-validation checkpoint

The first target-neutral contract slice is implemented across
`crates/plugin-api` and `crates/compiler`:

- [x] Declare input and output port shapes in semantic plugins.
- [x] Declare requirements, capabilities, guarantees, execution semantics, and
      side effects in node contracts.
- [x] Validate graph port counts and kinds before lowering.
- [x] Validate lowered data types, async semantics, and effects against the
      declared contract before IR construction.
- [ ] Resolve capabilities and requirements across connected nodes.
- [ ] Add whole-graph reachability, dependency, and cycle validation.

**Exit criteria:** repeated compilation produces identical diagnostics and IR;
invalid graphs never reach a generator.

## Phase 5 — TypeScript and NestJS backends

- [ ] Define the TypeScript language IR.
- [ ] Define the NestJS target contract and generator plugin.
- [ ] Generate modules, controllers, providers, models, repositories,
  validation, errors, tests, and README content.
- [ ] Preserve node-ID traceability in generated output.
- [ ] Generate deterministic project archives.
- [ ] Type-check and smoke-test representative generated projects.

**Exit criteria:** a representative graph generates a readable, runnable NestJS
project with byte-identical repeated output.

## Phase 6 — Graph Giraffe replacement

- [ ] Implement the Graph Giraffe adapter against the same canonical fixtures.
- [ ] Add semantic port support where the adapter requires it.
- [ ] Add execution/data edge presentation.
- [ ] Add generic diagnostic rendering.
- [ ] Add renderer-neutral graph projection and command APIs where required.
- [ ] Compare React Flow and Graph Giraffe using the editor conformance suite.
- [ ] Switch Krunt only after behavior and source mapping parity is proven.

**Exit criteria:** both editors can edit the same graph documents and emit
equivalent commands and source mappings.

## Phase 7 — Quality and future preview runtime

- [ ] Add deterministic compilation and insertion-order tests.
- [ ] Add invalid graph fixtures and stable diagnostic assertions.
- [ ] Add golden NestJS projects.
- [ ] Type-check generated projects and run runtime smoke tests.
- [ ] Add large-graph performance tests.
- [ ] Design sandboxed backend preview without coupling it to compilation.
- [ ] Define explicit network, filesystem, database, secret, CPU, and memory
  capabilities for future sandbox execution.

## Cross-cutting rules

- The graph document is source; generated code is an artifact.
- The editor never owns semantic validation or code generation.
- Universal IR never contains NestJS, Express, Laravel, filenames, or framework
  imports.
- The language backend and framework backend are separate extension points.
- AI output is untrusted and follows the same validation path as manual edits.
- Diagnostics carry stable source IDs and generation stops on errors.
- Compiler and generator output is deterministic.
- Krunt does not execute arbitrary graph-provided JavaScript.

## Initial vertical-slice acceptance test

The first end-to-end fixture will contain an HTTP route, request validation,
one repository query, a typed transform, a success response, and an error path.
It must:

1. render in React Flow;
2. serialize and restore deterministically;
3. show a deliberate invalid-connection diagnostic;
4. compile into universal IR;
5. lower into TypeScript IR;
6. generate a NestJS project;
7. retain node IDs in diagnostics and generated comments; and
8. produce no output when the graph contains an error-severity diagnostic.
