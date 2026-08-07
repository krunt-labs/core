---
tags: [crate, compiler, cli, wasm, service]
---

# Crate 07 — `cli` / `wasm` / `service`

Adapter surfaces around the compiler libraries. No pipeline logic lives here.

## Definition of Done
- [ ] CLI adapter (compile graph → generated project / diagnostics)
- [ ] WASM adapter (in-browser validation) — see note below
- [ ] Service adapter (optional server-side compile)
- [ ] Adapter consistency: identical diagnostics + output across CLI / WASM / service
- [ ] Filesystem + archive behavior inside adapters only (never in library passes)

## Requirements (from `graph-giraffe-compiler` skill)
Put CLI, WASM, server, filesystem, and archive behavior in adapters around compiler libraries. Hide no filesystem/network activity inside library passes.

## WASM note
Rust → WASM interop (`wasm-bindgen`) is one of the reasons the compiler is in Rust (see the "Rust vs C" decision). It lets the TypeScript editor validate the graph in-browser without a server.

## Cross-links
- [[Crate 04 - compiler-core]] · [[Phase 7 - Quality and Integration]] · [[Determinism]]