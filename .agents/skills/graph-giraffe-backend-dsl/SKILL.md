---
name: graph-giraffe-backend-dsl
description: Design, implement, validate, document, or review Graph Giraffe's v1 Backend DSL semantic plugin. Use for HTTP Route, Cron, Branch, Merge, Transform, Loop, Subgraph, Entity, Repository, Query, Mutation, HTTP Request, Logging, Storage, Success Response, Error Response, Try/Catch, Validation Error, their properties, ports, contracts, capabilities, effects, guarantees, diagnostics, and lowering into canonical IR.
---

# Build the Backend DSL

Own backend meaning without leaking it into the graph model, editor, type engine, or compiler orchestration.

## Keep v1 scope explicit

Implement these families:

- Entry: HTTP Route and Cron.
- Flow: Branch, Merge, Transform, Loop, and Subgraph.
- Persistence: Entity, Repository, Query, and Mutation.
- Effects: HTTP Request, Logging, and Storage.
- Response: Success Response and Error Response.
- Error handling: Try/Catch and Validation Error.

Defer WebSocket, Queue, GraphQL, RPC, first-class Transaction and Migration nodes, and non-backend DSLs unless scope is explicitly expanded.

## Define every node completely

For each node type, specify:

- stable type ID and version;
- user-facing descriptor and category;
- versioned property schema and defaults;
- execution inputs and outputs;
- typed data inputs and outputs;
- requirements and dependencies;
- capabilities;
- side effects;
- guarantees;
- execution semantics;
- semantic validation and diagnostic codes;
- lowering behavior;
- source-node traceability.

Do not mark a node pure if its observable read, write, time, randomness, network, or logging behavior violates the chosen purity definition. Resolve that definition explicitly before relying on purity optimizations.

## Respect dependency boundaries

Depend on structural read-only graph views, graph types, semantic plugin APIs, diagnostics, and lowering interfaces.

Do not:

- import editor components;
- orchestrate the compiler pipeline;
- emit TypeScript files or framework imports;
- execute backend operations during compilation;
- hide arbitrary JavaScript transforms in node properties.

Represent Transform through a constrained, typed expression model or supported declarative operations.

## Implement a node

1. Write its contract and examples before implementation.
2. Define valid and invalid port combinations.
3. Define effects, failure paths, and async behavior.
4. Register the descriptor deterministically.
5. Implement semantic validation.
6. Lower to validated IR without codegen knowledge.
7. Add focused contract and lowering tests.
8. Add a TypeScript generator golden case separately.
9. Add one graph-level invalid fixture.

## Preserve backend coherence

Validate relationships such as:

- routes reaching an explicit response on every legal path;
- repository/query/mutation nodes referencing valid entities and connections;
- response payloads matching declared route contracts;
- failure paths reaching valid error handling;
- execution order covering impure effects;
- loops having explicit structured termination semantics;
- entry nodes not being reached as ordinary downstream operations.

Finish only when the node is usable by the generic editor through descriptors and by generators through IR.
