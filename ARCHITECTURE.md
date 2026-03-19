# FA Local Architecture

The crate is structured inside-out:

- `domain/` owns core vocabulary and pure decision primitives.
- `app/` will host orchestration services that compose domain logic without absorbing policy authority.
- `adapters/` will isolate storage, schema, clock, hashing, and export boundaries.
- `integrations/` will keep Cortex, NeuronForge Local, and DF Local behind explicit contracts.

The current scaffold only establishes the bounded module seams and fail-closed primitives.
