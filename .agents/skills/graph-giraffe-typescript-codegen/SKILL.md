---
name: graph-giraffe-typescript-codegen
description: Build, change, test, or review Graph Giraffe's TypeScript/Node backend generator. Use for generated project layout, routes, handlers, models, repositories, validation, effects, error handling, migration scaffolds, tests, README content, framework adapters, templates, formatting, dependency versions, node-ID traceability, archive export, and deterministic golden output.
---

# Generate TypeScript Backends

Consume validated or optimized IR and return a deterministic map of relative paths to file contents. Keep source generation separate from compiler analysis and Backend DSL semantics.

## Own generator decisions

Own:

- project and module layout;
- TypeScript syntax and imports;
- framework adapter behavior;
- generated dependencies and pinned versions;
- routes, handlers, models, persistence, effects, and errors;
- validation code at runtime boundaries;
- migration scaffold generation;
- generated tests;
- graph-derived README content;
- formatting and source-node traceability.

Until Express versus Fastify is decided, isolate framework behavior behind a generator adapter. Do not spread either framework through IR or shared compiler crates.

## Consume trusted input

Generate only from validated IR. Do not:

- re-run semantic inference;
- inspect canvas/editor state;
- accept AI-produced source code as trusted input;
- guess missing types or contracts;
- mutate graph documents;
- execute generated applications inside the generator library.

If IR contains an unsupported operation, return a structured internal/generator diagnostic rather than silently omitting it.

## Guarantee deterministic output

- Sort files, declarations, imports, routes, fields, and dependencies canonically.
- Use fixed formatting and normalized line endings.
- Exclude timestamps, random values, machine paths, and environment-dependent comments.
- Pin template and dependency versions.
- Generate stable identifiers from semantic/source identities.
- Emit node traceability comments consistently.
- Return files in a deterministic ordered collection.

## Keep generated code user-owned

Produce readable, idiomatic code that:

- runs without the editor or AI layer;
- has explicit dependencies;
- contains understandable module boundaries;
- validates external input;
- exposes useful error and logging behavior;
- avoids a hidden proprietary runtime requirement.

Default to replacing managed output on recompilation. Do not implement round-trip merging until ownership markers and conflict semantics are designed.

## Implement a generator change

1. Identify the IR operation and framework adapter involved.
2. Update the smallest template or renderer.
3. Add a focused rendering test.
4. Update a complete golden project.
5. Format and type-check generated output.
6. Run generated tests or a smoke boot when tooling exists.
7. Compile the same fixture repeatedly and compare paths and bytes.

Treat readability regressions as failures even when snapshots can be updated.
