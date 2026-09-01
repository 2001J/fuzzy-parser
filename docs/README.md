# Fuzzy Parser Documentation

Choose the path that matches what you are trying to do. You do not need to read
the entire repository documentation before using the parser.

## I want to try the parser

1. [Getting started](getting-started.md)
2. [Capability matrix](current-state.md)
3. [Results and review](results-and-review.md)

This path explains the CLI, the shape of a result, and what `needs_review`
means.

## I am integrating an application

1. [Integration guide](integration-strategy.md)
2. [Application profiles](application-profiles.md)
3. [Results and review](results-and-review.md)
4. [Advanced data contracts](data-contracts.md)
5. [Errors and confidence](error-and-confidence-model.md)

An application defines and versions a profile once. Its end users paste or
upload data; they do not construct a schema for every import.

## I am contributing code

- [Contributor guide](contributing.md)
- [Architecture](architecture.md)
- [Parsing pipeline](parsing-pipeline.md)
- [Testing strategy](testing-strategy.md)
- [File validation](file-validation.md)
- [Cross-profile conformance](conformance.md)

## I maintain releases and infrastructure

- [Continuous integration](ci.md)
- [Release operator guide](releasing.md)
- [Release and environment policy](release-and-environment-strategy.md)

## Product direction

- [Product direction](product-direction.md)
- [Roadmap](roadmap.md)

## Internal and historical records

These records explain how decisions were reached. They are not API guides,
current capability specifications, or required integration reading:

- [Internal documentation index](internal/README.md)
- [Architecture decisions](decisions/README.md)

Public capability claims belong in [current state](current-state.md). Exact wire
contracts belong in [data contracts](data-contracts.md). Planned work belongs in
the [roadmap](roadmap.md). Avoid copying the same status into multiple documents.
