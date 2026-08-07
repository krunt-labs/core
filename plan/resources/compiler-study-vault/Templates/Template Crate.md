---
tags: [template, crate]
---

# Template — Crate

Use for each Rust crate's Definition of Done + progress. Copy to `Crates/` and rename `Crate NN - <crate>`.

```markdown
---
tags: [crate, compiler, <area>]
---

# Crate NN — `<crate>`

<One-line purpose.>

## Definition of Done
- [ ] <item>
- [ ] <item>

## Requirements (from `<skill-file>`)
Anything the skill mandates that the DoD must reflect.

## Build order
Built/assembled in **[[Phase N - <Name>]]**.

## Cross-links
- [[<concept>]] · [[<phase>]] · [[<other crate>]]
```

## Cross-links
- [[00 MOC - Compiler Study]]