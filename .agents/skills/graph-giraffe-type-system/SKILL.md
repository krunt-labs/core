---
name: graph-giraffe-type-system
description: Design, implement, test, or review Graph Giraffe's reusable compile-time type system. Use for type representations, type variables, assignability, unification, constraint solving, inference across data edges, generics, collections, unions, intersections, Option, Result, Future, Stream, type diagnostics, and contract input/output typing.
---

# Build the Graph Type System

Implement a DSL-independent type engine in Rust. Read the RFC type-system and contract sections before changing public semantics. Load the compiler or plugin-system skill when the change crosses those boundaries.

## Own type semantics

Represent graph-language types explicitly:

- primitives: string, boolean, integer, float, number, null, and void;
- composites: struct, record, and object;
- collections: array, set, and map;
- generics: option, result, future, and stream;
- unions and intersections;
- named types, type variables, and resolved type IDs.

Own:

- normalization and equality;
- assignability and variance rules;
- constraint generation primitives;
- deterministic unification and inference;
- substitutions and generic instantiation;
- type-related diagnostic facts.

## Keep boundaries clean

Do not:

- encode Backend DSL node names or framework types;
- traverse editor components;
- decide execution scheduling;
- perform capability or side-effect analysis;
- lower directly to TypeScript;
- use Rust's own type system as the graph type representation.

Contracts may reference graph types, but the plugin system owns full contracts, capabilities, requirements, effects, and guarantees.

## Make inference deterministic

- Give constraints stable identities or stable source ordering.
- Resolve equally valid candidates by documented canonical rules.
- Avoid hash iteration in substitution or diagnostic ordering.
- Preserve source node and port IDs on constraints.
- Report ambiguity instead of making environment-dependent guesses.
- Bound recursive expansion and detect occurs-check failures.

## Implement a type feature

1. Define syntax-independent semantics first.
2. Specify equality, assignability, inference, and display behavior.
3. Decide variance for generic parameters.
4. Add structured errors with graph source references.
5. Integrate through type and contract APIs rather than domain switches.
6. Add positive, negative, ambiguous, and recursive tests.

## Verify the type engine

Test:

- primitive widening and rejection;
- record field compatibility;
- generic substitution;
- nested option/result/future/stream types;
- union and intersection normalization;
- collection element/key constraints;
- multi-edge propagation;
- contradictory constraints;
- stable diagnostics under reordered input;
- no Backend DSL imports.

Expose deterministic values and diagnostics; do not print, exit, or mutate editor state.
