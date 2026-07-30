---
name: graph-giraffe-compiler
description: Build, debug, optimize, test, or review Graph Giraffe's deterministic Rust compiler orchestration. Use for parsing and schema checks, plugin resolution, semantic analysis, execution/data graph analysis, type-check coordination, contract and capability validation, whole-graph validation, compiler diagnostics, pass ordering, incremental compilation, CLI/WASM/service compiler APIs, and compile failure policy.
---

# Build the Rust Compiler

Own pipeline orchestration and graph-wide analysis. Use the graph, type, plugin, IR, Backend DSL, and codegen skills when changing their owned data or behavior.

## Run one explicit pipeline

Execute stages in this order:

1. Parse the versioned graph document.
2. Run structural schema validation.
3. Resolve plugin and node definitions.
4. Build separate execution-flow and data-flow views.
5. Run semantic analysis.
6. Generate and solve type constraints.
7. Validate contracts and requirements.
8. Resolve capabilities and side effects.
9. Validate whole-graph reachability, dependencies, and control-flow legality.
10. Lower valid semantics into canonical IR.
11. Invoke IR optimization.
12. Dispatch the selected generator.

Do not generate runnable files after error-severity diagnostics invalidate semantics.

## Own compiler-wide validation

Detect:

- missing nodes, ports, plugins, and dependencies;
- incompatible port connections and types;
- contract, requirement, and capability violations;
- illegal cycles and recursion;
- unreachable nodes and subgraphs;
- dead branches where statically knowable;
- invalid execution/data-flow combinations.

Treat Loop as structured control flow. Do not infer that arbitrary cycles are legal.

## Produce structured diagnostics

Return diagnostics with:

- stable code and severity;
- concise message;
- primary graph element ID;
- related element IDs;
- compiler stage;
- optional structured help or proposed fix.

Sort diagnostics canonically. Avoid panics for user-authored invalid graphs. Keep rendering and localization outside compiler core.

## Keep orchestration separate

The compiler:

- calls type and semantic services;
- controls stage order;
- aggregates deterministic diagnostics;
- exposes pure compile/validate APIs where possible.

It does not:

- own graph serialization schemas;
- embed Backend DSL match statements in generic stages;
- define canonical IR data owned by the IR crate;
- format TypeScript;
- access editor state or AI prompts;
- hide filesystem or network activity inside library passes.

Put CLI, WebAssembly, server, filesystem, and archive behavior in adapters around compiler libraries.

## Implement a compiler change

1. Name the owning stage and its input/output invariant.
2. Preserve earlier-stage guarantees.
3. Add source mappings for every new diagnostic.
4. Keep the stage deterministic and side-effect free.
5. Test invalid and valid fixtures at that stage.
6. Add an end-to-end compile test when stage ordering or output changes.

## Verify compilation

Test:

- stage order and short-circuit behavior;
- multiple simultaneous diagnostics in stable order;
- data and execution graph analysis independently;
- invalid AI and manual graphs identically;
- repeat compilation byte-for-byte;
- no generated output on semantic errors;
- adapter consistency across CLI, service, and WASM surfaces.
