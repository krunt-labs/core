---
tags: [compiler, graph-giraffe, protocol]
---

# Protocol — Study While Building

The goal is to *understand enough to direct the build*, not to exhaust a textbook. Each phase follows a tight loop: **Learn → Build → Verify**, gated by a **Definition of Done (DoD)**.

## The loop
1. **Learn** — read the tagged chapters in the `Chapters/` folder and the concept notes.
2. **Build** — implement the crate milestone the phase gates. Use the crate note's DoD.
3. **Verify** — run the skill-mandated tests (determinism, rejection tests, golden output, invalid fixtures).
4. **Mirror** — write a `Mirrors/` note capturing "this book idea shows up here in Graph Giraffe." This is the learning payoff.

## Rules that protect the architecture
- **IR stays target-independent.** Never let a framework name (`express`, `fastify`) enter IR or analysis. See [[Target Contract]].
- **Mapping is N:M, not 1:1.** Graphs are *source*; generated files are *artifacts*. Traceability via [[Source Mapping]].
- **Determinism is a guarantee, not a hope.** Every phase's verify step includes a repeat-compile / byte-for-byte check. See [[Determinism]].
- **Validation before generation.** No emit after semantic errors.
- **Passes are pure** `IR -> Result<IR>` with declared invariants. See [[Static Single Assignment Form]] (as a model of invariant-rich representation).

## Phase gating
Each Part is a stage that cannot be skipped. Complete the DoD in the Crate note before moving to the next Phase.

## Reading discipline
- **Must-read** chapters are few; **skims** and **deferred** items are marked in each Part. Do not read textbooks from cover to cover.
- Classic parsers/scanners are **skipped** for this project (see MOC premise) — do not spend time there.

## Progress tracking
Check the checklist boxes at the top of each Part as you finish Build + Verify. Update the Crates with the current status. Keep a running `Mirrors/` log of "aha" mappings — those are what make this project a compiler, not just a translator.