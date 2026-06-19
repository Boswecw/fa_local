# FA Local GNAT Dispatch Target Role

## Status

Runtime and schema promoted as a bounded FA Local GNAT dispatch support surface.

## Source Authority

The local-system proving repo at
`/home/charlie/Forge/ecosystem/local-systems/fa-local-operator` owns the current
GNAT dispatch contract, validator, and proof.

Current source authority evidence:

- `schemas/gnat-dispatch-envelope.schema.json`
- `tests/gnat_dispatch.rs`
- `src/integrations/cortex/mod.rs`
- `src/bin/fa_local_run.rs`
- `ci_gate.sh`

## Promoted Support Role

FA Local app support owns the bounded execution-routing side of the Cortex GNAT
surface.

The promoted support role is limited to:

- validating a Cortex-originated `GnatDispatchEnvelope.v1`
- enforcing that FA Local owns execution routing
- clamping effective concurrency to admitted local capability
- making serial fallback explicit when the contract permits it
- denying unsupported worker types, unsupported contract versions, and malformed
  shard plans
- preserving Cortex ownership of source eligibility and receipt validation

## Explicit Non-Goals

This promotion does not authorize:

- changing execution service behavior
- adding queue, watcher, retry, or scheduler ownership
- letting Cortex own integrated execution routing
- storing durable GNAT records in FA Local
- emitting semantic labels or candidate meaning

## Promoted Files

This slice promotes:

- `schemas/gnat-dispatch-envelope.schema.json`
- `src/integrations/cortex/mod.rs`
- `src/domain/shared/schema.rs`
- `src/bin/fa_local_run.rs`
- `tests/gnat_dispatch.rs`
- GNAT dispatch contract fixtures under `tests/contracts/fixtures`

## Promotion Gate

The promotion remains valid only while source and support proof commands pass
and the promotion ledger classifies this drift as intentional app support
adaptation rather than source-local hold.
