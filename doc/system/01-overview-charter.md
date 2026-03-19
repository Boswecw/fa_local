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
- schema-backed contracts for requester trust, policy artifact, capability registry, execution request, execution plan, execution status, route decision, and denial guard
- valid and invalid fixtures for those contract surfaces
- pure schema loading and validation helpers
- pure requester-trust evaluation and capability-admission deny logic
- pure approval-posture resolution and typed route-decision output
- pure bounded execution-plan validation and stable plan hashing
- pure execution-status validation and construction helpers
- deny smoke tests for the current fail-closed baseline rules

What is still intentionally not delivered:

- runtime coordination
- runtime execution-plan coordination
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
