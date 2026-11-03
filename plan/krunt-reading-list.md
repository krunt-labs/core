# Krunt Reading List

This list is ordered so learning can proceed alongside implementation. The
compiler-study vault is the primary curriculum; the other materials fill in
Rust, editor, backend, and framework knowledge.

## 1. Project orientation

- `plan/architecture/RFC-001.md`
- `plan/resources/compiler-study-vault/00 MOC - Compiler Study.md`
- `plan/resources/compiler-study-vault/01 Protocol - Study While Building.md`
- `plan/resources/compiler-study-vault/02 Architecture - IR to Target Mapping.md`
- `plan/resources/compiler-study-vault/Parts/Phase 0 - Orientation and Rust.md`

Learn the project boundaries first: graph as source, compiler ownership,
determinism, plugins, IR, generators, and source traceability.

## 2. Rust foundations

Study ownership and borrowing, enums and pattern matching, traits, generics,
`Result`-based error handling, iterators, deterministic collections, `serde`,
testing, property-based testing, and async Rust.

The goal is not general Rust mastery before work begins. The immediate goal is
being able to model graph documents, diagnostics, compiler stages, plugin
traits, and immutable IR safely.

## 3. Graph and editor architecture

Study stable identifiers, adjacency indexes, parent/child scene graphs, command
history, event sourcing, controlled React state, projection versus source of
truth, undo/redo, accessibility, and keyboard interaction.

Review the existing Graph Giraffe node-editor material:

- `../graph-giraffe/architecture/architecture.txt`
- `../graph-giraffe/.agents/rfc-replace-reactflow-with-graphgiraffe-0773830.md`
- `../graph-giraffe/.agents/architecture-configurable-primitives-plan.md`

Also learn React Flow’s controlled state, custom nodes, identified handles,
connection validation, custom edges, grouping, and subflows. Use the public core
APIs rather than Pro examples or templates.

## 4. Type systems

- `plan/resources/compiler-study-vault/Chapters/TAPL Ch3 - Untyped Arithmetic Expressions.md`
- `plan/resources/compiler-study-vault/Chapters/TAPL Ch9 - Simply Typed Lambda-Calculus.md`
- `plan/resources/compiler-study-vault/Chapters/TAPL Ch 22 - Type Reconstruction.md`
- `plan/resources/compiler-study-vault/Concepts/Unification - Type Inference.md`
- `plan/resources/compiler-study-vault/Parts/Phase 3 - Type System and Inference.md`
- `plan/resources/Benjamin_C._Pierce-Types_and_Programming_Languages-The_MIT_Press(2002).txt`

Focus on type representation, assignability, unification, constraint solving,
generics, unions, intersections, `Option`, `Result`, `Future`, `Stream`, and
inference across data edges.

## 5. Compiler and IR design

- `plan/resources/compiler-study-vault/Chapters/EAC Ch4 - Context-Sensitive Analysis.md`
- `plan/resources/compiler-study-vault/Chapters/EAC Ch5 - Intermediate Representations.md`
- `plan/resources/compiler-study-vault/Chapters/EAC Ch8 - Introduction to Optimization.md`
- `plan/resources/compiler-study-vault/Chapters/EAC Ch9 - Data-Flow Analysis.md`
- `plan/resources/compiler-study-vault/Chapters/EAC Ch10 - Scalar Optimizations.md`
- `plan/resources/compiler-study-vault/Chapters/EAC Ch11 - Instruction Selection.md`
- `plan/resources/compiler-study-vault/Concepts/IR Taxonomy.md`
- `plan/resources/compiler-study-vault/Concepts/Static Single Assignment Form.md`
- `plan/resources/compiler-study-vault/Concepts/Dominance.md`
- `plan/resources/compiler-study-vault/Concepts/Source Mapping.md`
- `plan/resources/compiler-study-vault/Concepts/Determinism.md`
- `plan/resources/compiler-study-vault/Concepts/Target Contract.md`
- `plan/resources/compiler-study-vault/Parts/Phase 1 - Canonical IR.md`
- `plan/resources/compiler-study-vault/Parts/Phase 2 - Data-Flow and SSA.md`
- `plan/resources/compiler-study-vault/Parts/Phase 4 - Semantic Analysis and Graph Front-End.md`
- `plan/resources/compiler-study-vault/Parts/Phase 5 - Optimization Passes.md`
- `plan/resources/compiler-study-vault/Parts/Phase 6 - Lowering and Multi-Target Codegen.md`
- `plan/resources/compiler-study-vault/Parts/Phase 7 - Quality and Integration.md`

Learn semantic analysis, control-flow graphs, data-flow, SSA concepts, source
maps, optimization safety, target contracts, and deterministic code generation.

## 6. Backend engineering

Study HTTP request/response lifecycles, routing, middleware, authentication,
authorization, validation, dependency injection, repositories, persistence,
transactions, effects, error propagation, async execution, retries, timeouts,
and cancellation.

Then study NestJS modules, controllers, providers, guards, pipes, interceptors,
exception filters, and testing. Keep these concepts separate from NestJS
syntax so they remain representable in the universal IR.

## 7. Dynamic values and future preview execution

- `plan/resources/CraftingInterpreters-vault/Chapters/7 Evaluating Expressions.md`
- `plan/resources/CraftingInterpreters-vault/Chapters/18 Types of Values.md`
- `plan/resources/CraftingInterpreters-vault/Concepts/Tagged Union.md`

These are supporting materials for a future preview/runtime system. They are
not prerequisites for the initial compiler, which should rely on static types
and validated IR rather than a Lox-style dynamic value model.

## Suggested study rhythm

- Read the orientation material before each implementation phase.
- Read the relevant compiler concept immediately before building its crate.
- Write a short project-specific mirror note after each major concept.
- Keep unresolved decisions in RFCs instead of settling them in implementation.
