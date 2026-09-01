# 0004 — Deliver The CLI Before Language Bindings

## Status

Partially superseded by [0005 — Validate an independent engine through its first
consumer](0005-independent-engine-consumer-validation.md).
CLI-first remains accepted and is implemented. The mandatory process-next order
below is historical guidance, no longer the current integration sequence.

## Context

The parser must be independently usable and later integrate with TypeScript. Starting with WebAssembly, native Node bindings, and a service simultaneously would create packaging and deployment complexity before the parser request and response contracts are proven.

## Historical decision

Build the first end-to-end integration through a CLI with structured JSON input and output. Add TypeScript process integration next, then WebAssembly or native bindings when the core contract and fixtures are stable.

## Consequences

- End-to-end behavior can be tested without a graphical interface.
- The project gains an independent tool immediately.
- TypeScript applications can integrate through a child process before optimized bindings exist.
- Process startup overhead is accepted during early integration.
- Language bindings must remain thin wrappers around the same library API and fixture corpus.
