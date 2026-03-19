# AGENTS.md

## Purpose

This file defines how coding agents should operate inside the **FA Local** repository.

FA Local is a **governed local execution control service** for Forge applications. It is not a general agent runtime, not a planner, not a semantic authority, and not a hidden orchestration monolith. Agent work in this repo must preserve that posture at all times.

This document is a repository control surface. Treat it as binding guidance for implementation, review, refactor, planning, and documentation work.

---

## 1. Repo mission

Build and maintain FA Local as a **bounded, fail-closed, policy-first local execution service** that:

- accepts only trusted local execution requests
- evaluates requests against explicit policy and admitted capabilities
- coordinates only bounded, approved execution plans
- reports truthful status for denied, canceled, failed, partial, degraded, and completed execution
- emits structured review packages when authority must hand back to a human
- produces minimal, open-format, locally auditable forensic records

Do not turn this repo into:

- a generic local agent framework
- an open-ended planner
- a business-logic authority layer
- a semantic memory layer
- a hidden service-integration brain
- a convenience abstraction that absorbs Cortex, NeuronForge Local, or DF Local ownership

---

## 2. Non-negotiable doctrine

### 2.1 Fail closed

If trust, policy, capability admission, coupling, environment, resource budget, or dependency readiness is invalid, unknown, malformed, expired, mismatched, revoked, or not explicitly allowed, the result must be **deny**, **review-gated**, or **explicit approval required** according to the governing contract.

Never default to permissive execution.

### 2.2 Policy before execution

Execution is impossible without:

- valid request trust
- valid policy artifact
- admitted capability
- coherent policy/capability coupling
- valid approval posture
- bounded execution plan when multi-step

### 2.3 Correct ownership over convenience

FA Local owns:

- trusted request intake
- request validation
- requester trust evaluation
- policy evaluation
- capability admission checks
- approval posture resolution
- bounded execution-plan validation
- execution coordination across approved local routes
- truthful execution state
- review package emission
- bounded forensic event generation
- correlation propagation

FA Local does **not** own:

- app business semantics
- durable semantic memory
- content intelligence semantics
- inference/prompt intelligence semantics
- open-ended planning
- hidden surveillance
- uncontrolled capability discovery
- durable workflow authorship

### 2.4 Truthful status only

Do not hide degraded, partial, fallback, or denied truth behind smooth summaries.

If degraded, the subtype must be explicit.
If fallback occurred, it must be policy-valid and surfaced truthfully.
If verification is incomplete, say so.

### 2.5 Open, minimal forensics

Forensics exist for execution truth and operational accountability, not surveillance.
Prefer minimal payloads, bounded retention, redaction, and locally auditable open formats.

---

## 3. Required working posture for agents

When working in this repo, always do the following before proposing or making changes:

1. Identify the owning boundary.
2. Identify the non-owning boundaries.
3. Identify the contract surfaces affected.
4. Identify the smallest safe change set.
5. Identify the main drift risk.
6. Identify required validation before claiming completion.

Do not behave like a blank-slate code generator.
Do not redesign architecture unless the task explicitly authorizes architectural change.
Do not widen scope because a broader cleanup feels nicer.

---

## 4. Boundary map

### 4.1 FA Local

Owns governance and bounded execution control only.

### 4.2 DF Local

May be used for:

- readiness checks
- registration lookup
- bounded persistence
- forensic event persistence and retrieval where allowed
- open-format local audit access

Do not turn DF Local into workflow memory or semantic orchestration storage.

### 4.3 Cortex

May be used for:

- approved Cortex contracts
- dependency readiness for Cortex routes
- correlation propagation into Cortex calls
- truthful state around Cortex-dependent execution

Do not absorb content or retrieval semantics into FA Local.

### 4.4 NeuronForge Local

May be used for:

- approved inferential contracts
- bounded invocation within validated plans
- readiness checks and truthful fallback/degraded handling
- correlation propagation into NeuronForge Local calls

Do not absorb prompt intelligence, lane intelligence, or inference authority into FA Local.

---

## 5. Core repo invariants

The following invariants must remain true unless a deliberate governance change updates the repo doctrine.

### 5.1 Request legitimacy invariant

Unknown requester => deny.
Malformed requester envelope => deny.
Environment mismatch => deny.
Expired or invalid nonce/token => deny.
Missing or unverifiable trust basis => deny.
Trust provenance mismatch => deny.
Approval-worthy action without adequate user-intent basis => review or deny.

### 5.2 Policy invariant

Invalid policy => deny.
Missing required policy => deny.
Policy load failure => deny.
Policy/service mismatch => deny or degrade only if doctrine explicitly allows it.

### 5.3 Capability invariant

Unregistered capability => deny.
Disabled capability => deny.
Revoked capability => deny.
Requester not in allowed class => deny.
Policy/capability mismatch => deny.
Version negotiation failure => deny.

### 5.4 Execution-plan invariant

If execution is multi-step, it must use a bounded validated execution plan.
No dynamic step invention after acceptance.
No unadmitted capability in any step.
Fallbacks must be declared, admitted, policy-valid, and reported truthfully.
Accepted plans must have a stable plan hash.

### 5.5 Status invariant

Execution status must be truthful about:

- denied
- allowed
- allowed_with_audit
- review_required
- explicit_approval_required
- canceled
- failed
- partial
- degraded
- completed

Generic “degraded” without subtype is not acceptable.

### 5.6 Handover invariant

When posture is `explicit_approval_required`, FA Local must emit a structured review package.
It must not silently wait, guess, or proceed beyond authority.

### 5.7 Forensic invariant

Every downstream call must carry the originating correlation ID or a formally derived child correlation reference.
Forensics must stay minimal, inspectable, and non-proprietary.

---

## 6. Agent rules for implementation work

### 6.1 Start with recon

Before editing:

- inspect relevant modules
- inspect schemas/contracts involved
- inspect tests covering the area
- identify existing patterns to preserve
- identify whether the task is boundary-sensitive, contract-sensitive, or execution-sensitive

### 6.2 Preserve architecture first

Prefer coherence with the existing module structure over clever abstractions.
Do not introduce shared helpers that quietly centralize ownership from the wrong layer.
Do not move behavior across modules unless the task explicitly requires it and the ownership analysis supports it.

### 6.3 Contracts before logic

If a task affects data flow, events, statuses, plans, capabilities, or handoff payloads, identify and update the contract surface first:

- schema
- Rust type(s)
- example fixtures
- validation logic
- downstream consumers/tests

### 6.4 Bounded change sets only

Keep edits narrowly scoped and reviewable.
Do not fold in opportunistic cleanup unless required for correctness, contract integrity, or testability.
State what is intentionally out of scope.

### 6.5 No fake completion

Do not claim:

- done
- complete
- production-ready
- fully implemented

unless validation actually supports that claim.
Use explicit status language such as:

- proposed
- partially implemented
- implemented but not fully verified
- verified by tests
- deferred
- blocked by missing contract/input

---

## 7. Agent rules for review and refactor work

### 7.1 Review priorities

When reviewing code or plans in this repo, inspect for:

1. ownership drift
2. fail-open behavior
3. silent contract changes
4. capability admission bypass
5. policy/capability coupling gaps
6. degraded/fallback truth violations
7. hidden planning behavior
8. unbounded state growth
9. forensic over-collection
10. widened scope disguised as cleanup

### 7.2 Refactor posture

Refactors must:

- preserve intended behavior unless change is explicitly authorized
- stay bounded
- improve clarity, determinism, or guardrails
- not widen authority
- not introduce stealth abstractions

If a refactor exposes an architectural issue beyond scope, list it under deferred work rather than smuggling the redesign into the patch.

---

## 8. Required output structure for serious tasks

For non-trivial tasks, agents should structure work in this order:

1. current understanding
2. relevant files/modules/contracts
3. ownership and non-ownership
4. risks / likely failure modes
5. bounded plan
6. implementation or analysis result
7. validation/tests
8. self-review against repo doctrine
9. deferred work / residual risk

This applies to:

- implementation
- refactor
- architecture review
- contract work
- governance changes

---

## 9. Testing and validation expectations

### 9.1 Minimum expectation

Every meaningful change must include validation appropriate to its risk.

Examples:

- unit tests for pure logic and enums
- contract/schema validation tests
- deny-path tests for fail-closed rules
- determinism tests for plan hashing
- posture-resolution golden tests
- integration tests for bounded execution flow
- review package tests for explicit approval cases
- forensic write/export tests where relevant

### 9.2 Mandatory deny coverage mindset

If a change touches any of these areas, include or update deny-path tests:

- requester trust
- policy loading
- capability admission
- execution-plan validation
- dependency readiness and fallback posture
- review gating / explicit approval

### 9.3 Evidence-sensitive completion

A change is stronger when it includes:

- tests or validation updates
- fixture updates where contracts changed
- explicit statement of residual risk
- explicit statement of what was not verified

---

## 10. Contract-sensitive zones

Treat the following as high-discipline control surfaces:

- `schemas/`
- request trust contracts
- policy artifact contracts
- capability registry contracts
- route decision contracts
- execution plan contracts
- execution status contracts
- denial/guard contracts
- forensic event contracts
- review package contracts
- friction payload contracts

When these change, agents must identify:

- producers
n- consumers
- compatibility posture
- tests/fixtures needing updates
- whether human review is mandatory

---

## 11. Forbidden drift patterns

Do not introduce any of the following without explicit governance approval:

- business-semantic durable state inside FA Local
- open-ended planning or replanning logic
- semantic interpretation that belongs to app/domain layers
- service-specific routing heuristics that belong elsewhere
- capability discovery that bypasses registry governance
- hidden fallback substitution
- generic agent runtime features
- surveillance-oriented forensic capture
- broad workflow logic centralized in FA Local
- dynamic step invention after plan acceptance

If a task pressures the repo toward one of these, stop and surface it as a drift risk.

---

## 12. Human review is mandatory for

Always require explicit human review for:

- architecture changes
- contract surface changes
- state model changes
- persistence model changes
- approval posture changes
- policy loading or coupling changes
- capability admission or revocation behavior
- security-sensitive logic
- destructive operations
- workflow/orchestration authority shifts
- performance-sensitive or resource-budget-sensitive paths

---

## 13. Repository working agreements for coding agents

### 13.1 Preferred implementation order

When building new core capability in this repo, prefer this order:

1. doctrine / ownership confirmation
2. contract definition or update
3. type/model alignment
4. validation logic
5. core execution logic
6. adapter boundary updates
7. tests
8. documentation update

### 13.2 Adapter discipline

Keep integrations with DF Local, Cortex, and NeuronForge Local behind explicit adapters.
Do not leak their semantics into FA Local core domain logic.

### 13.3 Determinism bias

Prefer deterministic behavior over “smart” behavior.
Prefer explicit enums over vague strings.
Prefer typed denial reasons over booleans.
Prefer bounded plans over runtime invention.

### 13.4 Minimal state bias

Do not introduce durable state unless it is clearly required by FA Local’s bounded operational role.
If adding state, justify:

- why it belongs here
- retention needs
- minimization/redaction posture
- auditability implications

---

## 14. Prompting guidance inside this repo

When using AI assistance for repo work, prompts should reinforce:

- repo awareness before edits
- ownership before implementation
- contract awareness before code changes
- bounded scope
- verification before completion claims
- truthful reporting of assumptions, limits, and deferred work

Do not use blank-slate prompts for repo work.
Do not ask for “production-ready” output without explicit verification requirements.

---

## 15. Suggested completion statement format

When reporting task results in this repo, prefer a format like:

- **What changed**
- **Why it belongs here**
- **Contracts touched**
- **Validation performed**
- **Residual risk / deferred work**

This keeps completion claims evidence-sensitive and reviewable.

---

## 16. Final rule

A good agent contribution in this repo is not the one that produces the most code.
It is the one that:

- preserves FA Local’s true boundary
- strengthens fail-closed governance
- keeps ownership correct
- stays within scope
- improves verification
- tells the truth about what was and was not proven
