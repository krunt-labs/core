---
tags: [concept, compiler, graph]
---

# Dominance

A structural relation on a **control-flow graph** that says which nodes must run before (or are guaranteed to run before) which others.

## Key idea (EAC Ch 9.2.1)
- Node *d* **dominates** node *n* if every path from the entry to *n* passes through *d*.
- From dominance we derive:
  - the **dominator tree** (immediate dominators),
  - **dominance frontiers** — the joins where a value defined in one block can first be "seen" by another — which exactly drive where to place φ-functions in [[Static Single Assignment Form]],
  - reachability and post-dominance for dead-code / loop analysis.

## Why it matters here
Dominance underpins the reaches/liveness concepts the compiler uses for [[Phase 2 - Data-Flow and SSA]]: reachability of nodes/subgraphs, dead-branch detection, and the ordering guarantees that make optimization sound.

## Appears in
- [[EAC Ch9 - Data-Flow Analysis]]
- [[Phase 2 - Data-Flow and SSA]]
- [[Static Single Assignment Form]] (φ-placement via dominance frontiers), [[IR Taxonomy]]