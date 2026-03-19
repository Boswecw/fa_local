# FA Local - System Documentation

**Document version:** 0.3 (2026-03-19) - Contract, denial, and posture-resolution slice aligned to current repo state
**Protocol:** Forge Documentation Protocol v1

| Key | Value |
|-----|-------|
| **Project** | FA Local |
| **Prefix** | `fa` |
| **Output** | `doc/faSYSTEM.md` |

This `doc/system/` tree is the assembled system reference for FA Local as a bounded local execution-control service.
It reflects the current repository state after the standalone crate scaffold, the schema-backed Phase 0.5 contract slice, the opening of Phase 1 requester/policy/capability deny logic, the pure route-decision and approval-posture slice, and the current fail-closed test coverage.

Assembly contract:

- Command: `bash doc/system/BUILD.sh`
- Output: `doc/faSYSTEM.md`

| Part | File | Contents |
|------|------|----------|
| SS1 | [01-overview-charter.md](01-overview-charter.md) | Mission, role, success posture, and current bounded baseline |
| SS2 | [02-boundaries-and-doctrine.md](02-boundaries-and-doctrine.md) | Authority boundaries, policy-before-execution doctrine, and anti-drift posture |
| SS3 | [03-contract-surface.md](03-contract-surface.md) | Implemented contract inventory, typed validation surfaces, and current execution-control vocabulary |
| SS4 | [04-validation-and-delivery.md](04-validation-and-delivery.md) | Build/test wiring, delivered contract slice, and current delivery posture |

## Quick Assembly

```bash
bash doc/system/BUILD.sh
```

*Last updated: 2026-03-19*

---

# 1. Overview and Charter

## Purpose

FA Local is the bounded local execution-control service for Forge applications.

Its current MVP purpose is narrow:

- accept trusted execution requests only
- enforce policy before side effects
- admit execution only through registered capabilities
- require bounded execution plans for multi-step work
- preserve truthful denial, degraded, partial, and completion state
- hand back to human review when direct execution is not admissible
- keep local forensics minimal and auditable

## Constitutional role

FA Local is a service/library implementation repository for the governed FA Local boundary.

It must not become:

- a standalone product UI
- a semantic authority
- a workflow memory surface
- a hidden planner
- a generic agent runtime
- an unbounded plugin executor

## Success posture

FA Local is only successful if it remains:

- bounded by contract
- fail-closed by default
- policy-first before execution
- capability-scoped rather than request-trusting
- truthful about degraded and denied posture
- explicit about human approval and handoff
- unable to drift into hidden orchestration or semantic control

## Current bounded baseline

The currently delivered implementation baseline is no longer scaffold-only.
It currently includes:

- standalone Rust crate and repo framing
- top-level governance and boundary docs
- domain/app/adapter/integration module seams
- typed runtime vocabulary for environment, requester, posture, denial, and degraded state
- typed UUID-backed identity primitives
- fail-closed denial guards and helpers
- schema-backed contracts for requester trust, policy artifact, capability registry, execution request, route decision, and denial guard
- valid and invalid fixtures for those contract surfaces
- pure schema loading and validation helpers
- pure requester-trust evaluation and capability-admission deny logic
- pure approval-posture resolution and typed route-decision output
- deny smoke tests for the current fail-closed baseline rules

What is still intentionally not delivered:

- runtime coordination
- execution-plan control
- adapters or cross-service invocation
- CLI, daemon, or API surfaces
- review package emission
- forensic persistence or export

This is the current bounded baseline, not a claim that later execution-facing phases are already delivered.

## Foundational references

This section is grounded in:

- `README.md`
- `SYSTEM.md`
- `BOUNDARIES.md`
- `ROADMAP.md`

---

# 2. Boundaries and Doctrine

## Authority line

FA Local owns:

- requester-trust-gated execution intake
- requester trust posture evaluation
- policy-before-execution enforcement
- capability admission checks
- approval posture resolution
- bounded execution-plan validation
- controlled execution coordination for admitted routes
- review-package handoff support
- minimized forensic event generation for execution truth

FA Local does not own:

- application business semantics
- syntax authority
- model or inference authority
- durable workflow memory
- hidden workflow policy authority
- open-ended planning
- ungoverned tool access
- canonical business truth

## Doctrine line

The governing doctrines are:

- policy before execution
- requester trust before admission
- fail closed over convenience
- bounded execution rather than runtime invention
- truthful degraded-state reporting
- explicit approval and review handoff
- privacy-preserving, minimized forensics
- explicit adapters rather than absorbed cross-service semantics

## Cross-service boundaries

### DF Local Foundation

Provides bounded substrate support for readiness, persistence, and local records.
It does not become execution authority.

### NeuronForge Local

May be invoked only through admitted inference contracts where policy allows.
It does not transfer final execution authority away from FA Local.

### Cortex

May provide approved preparation or readiness contracts only.
It does not make FA Local a syntax authority, and FA Local does not delegate execution authority back into Cortex.

## Anti-drift warning

Any proposal that turns FA Local into a planner, semantic authority, broad agent substrate, generic tool governor, or stealth orchestrator should be rejected unless the architecture is explicitly reworked.

No automatic expansion is implied by the current scaffold.
Further implementation must stay inside the constitutional boundaries already established in the repo and the shared runtime doctrine.

---

# 3. Contract Surface

## Implemented and planned contract set

The intended FA Local contract surface covers:

- requester trust
- policy artifact
- capability registry
- execution request
- route decision
- execution plan
- execution status
- denial guard
- forensic event
- review package
- friction payload

The currently implemented schema-backed subset is:

- requester trust
- policy artifact
- capability registry
- execution request
- route decision
- denial guard

The remaining contract surfaces are still deferred.

## Current typed surface

The current machine-checked typed surface includes:

- runtime vocabulary enums
- UUID-backed identity types
- UTC timestamp utility
- structured denial guard payloads
- fail-closed helper functions
- requester trust envelope and trust-evaluation context
- policy artifact and capability-rule types
- capability registry and capability-record types
- execution request type
- route-decision, policy-reference, and capability-decision-summary types
- pure approval-posture resolver inputs and context
- schema-name dispatch plus contract load/deserialize helpers

This gives FA Local a stable baseline for deny-by-default behavior with the first contract layer and the first machine-checked decision layer already in place.

## Approval and execution posture

The current vocabulary distinguishes:

- approval posture: `denied`, `review_required`, `explicit_operator_approval`, `policy_preapproved`, `execute_allowed`
- execution state: `denied`, `review_required`, `waiting_explicit_approval`, `admitted_not_started`, `in_progress`, `degraded`, `partial_success`, `completed_with_constraints`, `completed`, `failed`, `canceled`
- degraded subtype: `degraded_pre_start`, `degraded_in_flight`, `degraded_fallback_equivalent`, `degraded_fallback_limited`, `degraded_partial`, `unavailable_dependency_block`

That split keeps approval authority distinct from execution truth rather than collapsing them into one label set.

## Denial surface

The current denial guard preserves:

- denial reason class
- denial scope
- denial basis
- remediable flag
- review-available flag
- operator-visible summary
- UTC timestamp

This is intentionally narrow, but it already supports fail-closed truth without reducing all denials to generic errors.

## Current pure validation and admission logic

The current pure logic layer can already:

- validate requester-trust envelopes against schema and typed rules
- deny unknown requesters
- deny malformed requester envelopes
- deny environment mismatch
- deny invalid or expired nonce/token posture
- deny missing required policy
- deny invalid policy artifacts
- deny unregistered capabilities
- deny disabled or revoked capabilities
- deny policy/capability mismatch
- resolve deterministic approval posture from requester trust, policy, capability admission, review class, and side-effect posture
- produce typed route decisions for `denied`, `review_required`, `explicit_operator_approval`, `policy_preapproved`, and `execute_allowed`

These checks remain bounded to validation, deny-path admission, and pure decision output.
They do not coordinate execution.

## Current implementation boundary

Schema-backed execution-plan, execution-status, review-package, forensic-event, and friction-payload contracts do not exist yet.

There is also no CLI, daemon, API surface, adapter implementation, execution routing, or runtime coordinator in the current baseline.

## Supporting references

This section is grounded in:

- `src/domain/shared/schema.rs`
- `src/domain/shared/vocabulary.rs`
- `src/domain/shared/ids.rs`
- `src/domain/guards/mod.rs`
- `src/domain/requester_trust/mod.rs`
- `src/domain/policy/mod.rs`
- `src/domain/capabilities/mod.rs`
- `src/domain/execution/mod.rs`
- `src/domain/posture/mod.rs`
- `src/domain/routing/mod.rs`
- `docs/fa_local_codex_build_plan_v_1.md`

---

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
