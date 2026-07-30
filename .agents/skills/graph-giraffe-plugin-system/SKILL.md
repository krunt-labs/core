---
name: graph-giraffe-plugin-system
description: Design, implement, version, register, or review Graph Giraffe's semantic plugin system and extension API. Use for node descriptors, port and property schemas, contracts, capabilities, requirements, side effects, guarantees, semantic validators, compiler hooks, plugin registries, lifecycle, compatibility, manifests, and future sandbox boundaries.
---

# Build the Semantic Plugin System

Define how domains add meaning above the structural graph without coupling the editor or compiler core to a domain.

## Own the semantic extension boundary

Provide versioned interfaces for:

- plugin identity, manifest, semantic version, and compatibility;
- node-type descriptors and stable type IDs;
- port and property schemas;
- contract construction;
- requirements, capabilities, side effects, guarantees, and execution semantics;
- semantic validators and diagnostic registration;
- lowering registration;
- optional optimization and generator target hooks;
- generic editor descriptors such as labels, categories, and property schemas.

Keep descriptor data serializable so editor and AI clients can consume a catalog without loading Rust implementation code.

## Keep v1 loading simple

Use Rust crates linked at build time behind stable internal traits for v1. Do not introduce a native dynamic-library ABI as an accidental commitment.

Design interfaces so a later versioned protocol or WebAssembly component can replace in-process registration. Treat sandboxing, distribution, signing, and marketplaces as unresolved future work.

## Enforce ownership

The plugin system may depend on:

- structural graph IDs and read-only views;
- reusable graph types;
- shared diagnostics;
- stable semantic and lowering interfaces.

It must not depend on:

- editor components;
- AI providers;
- a specific backend framework;
- filesystem output conventions;
- generated TypeScript templates.

The compiler calls plugins through interfaces. Plugins must not take over pipeline orchestration.

## Register deterministically

- Reject duplicate plugin and node-type IDs.
- Resolve versions explicitly.
- Define stable registration and diagnostic ordering.
- Make missing or incompatible plugins a structured compilation error.
- Avoid global mutable registries in library code.
- Pass registries or immutable catalogs explicitly.
- Keep plugin initialization free from network and filesystem side effects unless an adapter authorizes them.

## Add or change an extension point

1. Identify which domain needs the extension and whether a generic abstraction is justified.
2. Define data contracts before Rust traits.
3. Specify compatibility and failure behavior.
4. Keep core orchestration in the compiler.
5. Implement one Backend DSL use case to prove the boundary.
6. Test duplicate IDs, incompatible versions, missing implementations, and deterministic registry output.

Finish only when another DSL could implement the interface without importing Backend DSL code.
