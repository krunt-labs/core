---
tags: [concept, compiler, diagnostics, traceability]
---

# Source Mapping

The discipline of keeping every IR construct and generated artifact **traceable back to the graph element that produced it** (node/port/edge IDs).

## Key idea
- The **graph is the source**; IR and generated files are artifacts.
- Every IR operation and every emitted line carries a *source span*: stable graph element IDs (and for generated files, file/line in the artifact plan).
- Diagnostics use the mapping to report against graph elements the user actually edits.
- Traceability is how **N:M mapping** stays safe: one Subgraph → many files, many nodes → one file, and we can still explain any output.

## Why it matters here
Without source mapping, a deterministic compile is still opaque: you cannot tell a user *why* a line was generated, and validation errors can't point at the node. Source mapping is the thread that ties [[IR Taxonomy]] → optimization passes → [[Target Contract]] output together.

## Appears in
- [[Phase 1 - Canonical IR]] (IR requirement)
- [[Phase 6 - Lowering and Multi-Target Codegen]]
- [[02 Architecture - IR to Target Mapping]]
- Skill `graph-giraffe-ir-optimization` ("traceable to source graph node and port IDs")