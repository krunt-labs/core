---
tags: [concept, crafting-interpreters]
---

# NaN Boxing

A value-representation trick that packs any Lox value — including doubles, booleans, and pointers — into the 64 bits of a double, exploiting unused bit patterns in NaN values for speed and compactness.

## Appears in
- [[30 Optimization]]
