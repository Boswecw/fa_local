# 4. Validation and Delivery

## Validation surface

FA Local currently includes:

- Rust build metadata in `Cargo.toml`
- JSON schemas in `schemas/`
- valid fixtures in `tests/contracts/fixtures/valid/`
- invalid fixtures in `tests/contracts/fixtures/invalid/`
- schema loading and validation tests in `tests/contracts_schema.rs`
- typed contract loading tests in `tests/contracts_loading.rs`
- deny smoke tests in `tests/denial_smoke.rs`
- deterministic enum serialization tests in `tests/enums_roundtrip.rs`
- fail-closed guard tests in `tests/guard_helpers.rs`
- repo-local assembly for system documentation through `doc/system/BUILD.sh`

The current machine-checked layer covers:

- schema validation for the six implemented contract surfaces
- valid and invalid fixture coverage for each implemented schema
- typed contract deserialization after schema validation
- requester-trust fail-closed rules
- policy artifact fail-closed rules
- capability admission fail-closed rules
- route-decision schema invariants for posture/bool consistency
- golden approval-posture resolution for all five posture outcomes
- deny-to-posture mapping and invalid-input fail-closed posture behavior
- stable snake-case serialization for baseline enums
- unknown-enum rejection behavior
- typed guard creation
- fail-closed helper behavior
- UTC timestamp stamping on denials

## Delivered slice

The currently delivered implementation slice is Phase 0.5 plus the opening of Phase 1 only.

It adds:

- standalone `fa-local` repository framing
- top-level repo docs and ADR stubs
- bounded source-tree layout for domain, app, adapters, and integrations
- shared runtime vocabulary aligned to the FA Local doctrine
- typed denial/error primitives
- schema-backed contracts for requester trust, policy artifact, capability registry, execution request, route decision, and denial guard
- pure schema loading and validation helpers
- pure requester-trust evaluation
- pure policy-required loading
- pure capability-admission deny logic
- pure approval-posture resolution
- typed route-decision output with deterministic posture flags
- deterministic contract fixtures and deny smoke coverage
- latest `jsonschema` validator release aligned in the crate dependency set

## Not yet delivered

The following planned surfaces are explicitly not delivered yet:

- bounded execution-plan hashing
- execution coordinator
- execution routing
- CLI, daemon, or API surface
- adapters and cross-service invocation
- review package emitter
- forensic recorder and export

## Current delivery posture

The repo currently supports:

- `cargo fmt`
- `cargo test`
- `bash doc/system/BUILD.sh`

The current delivered state should be described as:

- governance scaffold present
- typed baseline present
- first contract layer present
- first deny-path admission layer present
- first machine-checked route-decision layer present
- no executable FA Local runtime slice admitted yet

That wording matters because the crate now has meaningful contract, deny-path, and posture-resolution behavior, but the execution-control service itself is still not implemented beyond bounded validation and decision output.
