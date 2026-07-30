---
name: graph-giraffe-ai-authoring
description: Design, implement, secure, test, or review Graph Giraffe's AI-assisted graph authoring. Use for prompt-to-graph generation, graph completion, subgraph explanation, validation-error fix proposals, refactoring suggestions, optimization proposals, node discovery, schema-constrained output, graph patches, review UX, AI provider adapters, and evaluation of AI-generated graphs.
---

# Build AI-Assisted Graph Authoring

Keep AI beside the trusted compiler. AI may propose graph source; it must never become semantic or validation authority.

## Allow bounded authoring tasks

Support:

- prompt-to-graph generation;
- completion of an existing graph;
- subgraph explanation and documentation;
- proposed fixes for compiler diagnostics;
- proposed refactors and optimizations;
- node discovery from the current plugin catalog.

Return ordinary graph documents or explicit graph patches. Do not create a separate AI graph type.

## Constrain AI with public schemas

Provide:

- graph schema and supported version;
- active plugin and node catalog versions;
- property and port schemas;
- relevant contracts and capabilities;
- current graph or selected subgraph;
- structured compiler diagnostics for fix requests.

Require stable existing IDs in edits and explicit new IDs for additions. Validate patch preconditions before application.

## Preserve trust boundaries

AI must not:

- compile or execute graphs;
- define new semantics or capabilities;
- bypass structural, type, contract, or graph validation;
- directly emit trusted runtime source code;
- silently apply destructive graph edits;
- treat its explanation as proof of correctness.

Send AI output through the same graph parser and compiler validation as manual edits. Show resulting errors through normal editor diagnostics.

## Implement an AI workflow

1. Define the exact graph operation and allowed scope.
2. Build input from versioned public schemas.
3. Request schema-constrained output.
4. Parse into an untrusted graph or patch.
5. Check IDs, versions, and patch preconditions.
6. Validate with the ordinary compiler.
7. Present a reviewable diff or proposal.
8. Apply only through normal editor commands.
9. Record metrics without storing sensitive prompts or graphs by default.

## Evaluate behavior

Test malformed output, nonexistent IDs, outdated plugin versions, invalid ports, semantic errors, partial patches, and provider failure. Measure validation pass rate and user acceptance separately; a graph passing validation may still be semantically wrong for the user's intent.
