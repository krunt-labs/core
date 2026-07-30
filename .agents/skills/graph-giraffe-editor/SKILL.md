---
name: graph-giraffe-editor
description: Build, change, test, or review Graph Giraffe's domain-agnostic visual graph editor. Use for canvas rendering, node and port presentation, typed connection UX, execution/data edge styling, selection, grouping, subgraphs, annotations, property panels, copy/paste, undo/redo, commands, serialization integration, diagnostics display, incremental validation requests, performance, accessibility, and compiler or plugin-catalog integration.
---

# Build the Domain-Agnostic Editor

Treat the editor as a generic client of graph documents, semantic descriptors, and compiler diagnostics.

## Own interaction and presentation

Own:

- canvas layout, viewport, rendering, and hit testing;
- selection, multi-select, grouping, and annotations;
- add, connect, disconnect, delete, copy/paste, and undo/redo commands;
- subgraph navigation and reusable composition UX;
- distinct execution-edge and data-edge presentation;
- generic property panels driven by plugin schemas;
- validation and compilation requests;
- diagnostics mapped to graph IDs;
- document persistence through the graph-model API;
- incremental rendering and accessible interaction.

Keep transient state such as selection, hover, drag previews, and open panels outside the canonical executable graph.

## Forbid semantic duplication

Do not:

- hard-code HTTP, Entity, Repository, or response behavior;
- create editor-only backend node classes;
- implement independent type inference or compiler validation;
- generate TypeScript directly;
- special-case AI-created graphs;
- treat a visual group as executable control flow.

The editor may reject obviously structural invalid actions locally, but compiler diagnostics remain semantic authority.

## Consume generic catalogs

Render nodes from serializable plugin descriptors:

- stable node type ID and version;
- labels, categories, and documentation;
- port schemas and data-type display;
- property schemas and generic editor hints;
- capability/effect badges;
- defaults and deprecation state.

Do not require the UI bundle to import Rust plugin implementation code. Use generated schemas, a service API, or WebAssembly bindings.

## Implement an editor feature

1. Decide whether state is canonical graph data, presentation metadata, or transient UI state.
2. Express mutations as reversible commands.
3. Preserve stable graph IDs.
4. Call graph-model serialization rather than inventing a second format.
5. Request compiler validation after affected changes.
6. Render diagnostics without parsing message text.
7. Test undo/redo, copy/paste ID remapping, keyboard access, and large-graph behavior.

## Verify editor boundaries

Test with at least two synthetic plugin catalogs so generic behavior is proven. Confirm that adding a Backend DSL node type requires descriptor data, not editor source changes.
