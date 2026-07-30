---
name: graph-giraffe-graph-model
description: Design, implement, migrate, serialize, or review Graph Giraffe's domain-neutral structural graph model in Rust. Use for Graph, Node, Port, Edge, Group, Subgraph, Annotation, stable IDs, execution-versus-data edge representation, graph schema versions, structural validation, deterministic JSON, graph diffs, and document migrations.
---

# Build the Structural Graph Model

Own the stable, domain-independent source document. Read `plan/architecture/RFC-001.md` sections on structural graphs, serialization, and versioning before changing public shape. Load `graph-giraffe-development` for cross-layer changes.

## Own only structural concerns

Model:

- graph metadata, schema version, plugin version references, and semantic version;
- stable `GraphId`, `NodeId`, `PortId`, `EdgeId`, and `SubgraphId` newtypes;
- nodes with type references, property data, port instances, and annotations;
- directed edges with explicit execution or data kind;
- groups as presentation-only membership;
- subgraphs with explicit boundary ports;
- extension metadata with versioned namespaces.

Validate:

- unique and well-formed IDs;
- endpoint and container existence;
- legal input/output direction;
- edge-kind compatibility at the structural level;
- valid subgraph ownership and boundary references;
- schema shape and supported version.

## Exclude upper-layer knowledge

Do not encode:

- HTTP, repositories, entities, responses, or other Backend DSL concepts;
- type inference or domain contract validation;
- compiler pass state or code-generation details;
- canvas components, selection, or transient editor state in executable structure;
- AI prompts or provider metadata;
- runtime scheduling behavior.

Store plugin node-type identifiers and properties without interpreting their domain meaning. Let semantic plugins resolve them.

## Design the Rust boundary

Prefer:

- immutable reads and explicit mutation commands;
- typed ID newtypes instead of interchangeable strings;
- explicit enums for port direction and edge kind;
- ordered collections where order is semantic;
- canonical sorting where order is not semantic;
- `serde` representations with intentional field names and defaults;
- private fields when construction must enforce invariants.

Separate the canonical graph document from editor presentation data when presentation changes should not affect compilation. If positions remain in one serialized file, exclude them explicitly during semantic hashing.

## Preserve deterministic serialization

- Define canonical ordering for nodes, ports, edges, subgraphs, annotations, and map keys.
- Avoid emitting `HashMap` iteration order.
- Normalize optional/default fields.
- Reject duplicate IDs rather than choosing a winner.
- Add migrations for supported old schema versions.
- Preserve unknown namespaced extension data when safe.
- Compare serialization byte-for-byte in repeat tests.

## Implement a change

1. Identify whether the change affects canonical semantics, presentation metadata, or extensions.
2. Define compatibility and migration behavior.
3. Update constructors and structural validation.
4. Update canonical serialization.
5. Notify direct consumers through explicit API changes.
6. Test round trips, invalid documents, migrations, and stable output.

## Verify the boundary

Test:

- every ID and reference invariant;
- data and execution edges independently;
- nested subgraph boundaries;
- groups having no execution effect;
- deterministic JSON under different insertion orders;
- unknown, missing, and future versions;
- malformed and duplicate elements.

Finish only when backend-specific imports and compiler/runtime types remain absent.
