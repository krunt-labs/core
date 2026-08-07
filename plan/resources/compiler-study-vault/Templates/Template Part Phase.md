---
tags: [template, phase]
---

# Template — Part / Phase

Use for one phase of the study-while-building plan. Copy to `Parts/` and rename `Phase N - <Name>`.

```markdown
---
tags: [phase, compiler, <area>]
---

# Phase N — <Name>

> Status: not started
- [ ] Learn completed
- [ ] Build + Verify completed

## Goal
One sentence: what you'll be able to direct/build afterward.

## Learn (must)
- **Book Ch** — exact sections with page anchors.
- **Skills** — the skill file names that own this concern.

## Learn (skim / deferred)
Anything optional, and what to revisit later.

## Map to Graph Giraffe
The translation to the project's terminology (nodes, edges, IR, targets).

## Build
The [[Crate NN - <crate>]] DoD this phase gates.

## Verify
The skill-mandated tests this phase requires.

## Concepts to write
Back-links to the Concept notes this phase creates.

**Next → [[Phase N+1 - <Name>]]**
```

## Cross-links
- [[00 MOC - Compiler Study]] · [[01 Protocol - Study While Building]]