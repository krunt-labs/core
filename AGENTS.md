# AI Agent Instructions

For every Graph Giraffe architecture, implementation, review, testing, or planning task, first read:

- `.agents/skills/graph-giraffe-development/SKILL.md`

Use it as the project-wide router and load every concern-specific skill named there whose boundary the task changes. The available focused skills cover:

- structural graph model;
- type system;
- semantic plugin system;
- Rust compiler;
- canonical IR and optimization;
- Backend DSL;
- TypeScript code generation;
- domain-neutral editor;
- AI-assisted authoring;
- quality, determinism, and architectural verification.

For architectural or cross-layer work, also read both source documents completely:

- `plan/architecture/RFC-001.md`
- `plan/architecture/graph-giraffe-backend-tool-prd.pdf`

Follow the user's explicit request when it overrides a draft document. Surface unresolved conflicts rather than silently establishing a new platform contract.
