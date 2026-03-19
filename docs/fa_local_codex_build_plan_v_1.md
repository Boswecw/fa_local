# FA Local — Codex Build Plan v1

**Purpose:** Implementation-ready build plan for VS Code Codex based on the FA Local v1.2 constitutional plan.  
**Source basis:** FA Local — Updated Plan v1.2.  
**Operating posture:** local-first, fail-closed, policy-first, bounded, non-authoritative over app business truth.

---

# 1. Build objective

Build **FA Local** as a **governed local execution control service** for Forge applications.

FA Local must:
- accept only trusted local execution requests
- validate request trust, environment, policy, and admitted capabilities before execution
- resolve every request into an approval posture
- coordinate only bounded, policy-valid execution plans
- report truthful execution state including denied, canceled, failed, partial, degraded, and completed
- emit structured review packages when authority must hand back to a human
- generate minimal, open-format local forensic records

FA Local must not:
- become a planner
- become a semantic authority
- become app workflow memory
- absorb Cortex or NeuronForge Local intelligence
- execute unregistered or revoked capabilities
- proceed on malformed, unverifiable, or policy-mismatched requests

---

# 2. Codex mission framing

Give this to Codex as the governing instruction:

> Build FA Local as a bounded Rust service that enforces trusted request intake, fail-closed policy/capability admission, approval posture resolution, bounded execution coordination, truthful execution state, structured human handover, and minimal local forensics. Do not add autonomous planning, business semantics, durable semantic memory, or hidden orchestration behavior. Any unknown, malformed, mismatched, untrusted, or policy-invalid request must deny by default.

---

# 3. Target stack

## Core implementation
- **Language:** Rust
- **Edition:** 2024 if repo baseline already uses it; otherwise stable current Rust edition aligned with your local Forge standards
- **Primary form:** local service/library crate usable by consuming desktop apps
- **Serialization:** serde / serde_json
- **Schema validation:** JSON Schema validation for artifact loading and contract tests
- **Storage:** SQLite for bounded local operational persistence and forensic records, with JSONL export/readability support
- **IDs / hashes:** UUIDs for IDs, SHA-256 for stable plan fingerprinting where canonicalization is defined
- **Time:** UTC timestamps only

## Design posture
- prefer pure domain modules + adapters
- no framework-heavy abstraction unless required
- explicit enums over free-form strings where possible
- every boundary object must be schema-backed
- every decision path must be testable without UI

---

# 4. Repo shape

```text
fa-local/
  Cargo.toml
  README.md
  SYSTEM.md
  ARCHITECTURE.md
  BOUNDARIES.md
  POLICY.md
  CAPABILITIES.md
  FORENSICS.md
  REVIEWS.md
  ROADMAP.md
  DECISIONS/
    0001-service-only-framing.md
    0002-requester-trust-model.md
    0003-capability-admission-governance.md
    0004-bounded-execution-plan-model.md
    0005-review-package-handover.md
    0006-open-format-local-forensics.md
  docs/
    doctrine/
    architecture/
    contracts/
    controls/
    risks/
  schemas/
    requester-trust.schema.json
    policy-artifact.schema.json
    capability-registry.schema.json
    execution-request.schema.json
    route-decision.schema.json
    execution-plan.schema.json
    execution-status.schema.json
    denial-guard.schema.json
    forensic-event.schema.json
    review-package.schema.json
    friction-payload.schema.json
    examples/
  src/
    lib.rs
    domain/
      requester_trust/
      policy/
      capabilities/
      posture/
      routing/
      execution/
      status/
      guards/
      review/
      forensics/
      shared/
    app/
      intake_service.rs
      decision_service.rs
      execution_service.rs
      review_service.rs
      forensic_service.rs
    adapters/
      storage/
      schemas/
      clock/
      hashing/
      ids/
      exports/
    integrations/
      cortex/
      neuronforge_local/
      df_local/
    errors/
    config/
  tests/
    contracts/
    requester_trust/
    policy/
    capabilities/
    posture/
    routing/
    degraded/
    review/
    forensics/
    smoke/
    denial/
```

---

# 5. Build sequence

## Phase A — constitutional skeleton and hard fail-closed baseline

### Deliverables
- crate scaffold
- top-level docs placeholders
- baseline enums and IDs
- environment mode enum
- requester class enum
- approval posture enum
- degraded subtype enum
- side-effect class enum
- revocation state enum
- common error taxonomy
- fail-closed guard helpers

### Required enums
At minimum define:
- `EnvironmentMode = Dev | Test | Staging | Prod | Airgapped | TestHarness`
- `RequesterClass = TrustedAppSurface | TrustedInternalService | ReviewSurface | DevelopmentTestSurface | UntrustedUnknown`
- `ApprovalPosture = Allowed | AllowedWithAudit | ReviewRequired | ExplicitApprovalRequired | Denied`
- `DegradedSubtype = DegradedPreStart | DegradedInFlight | DegradedFallbackEquivalent | DegradedFallbackLimited | DegradedPartial | UnavailableDependencyBlock`
- `SideEffectClass = None | LocalFileWrite | LocalDbMutation | LocalProcessSpawn | ExternalNetworkDeniedByDefault | OtherGoverned`
- `RevocationState = Active | Disabled | Revoked`

### Acceptance gate
- project builds
- all enums serialize/deserialize deterministically
- unknown enum values fail load
- no execution path exists yet

---

## Phase B — schema contracts first

### Build these schemas first
1. requester trust
2. policy artifact
3. capability registry entry/set
4. execution request
5. route decision
6. bounded execution plan
7. execution status
8. denial guard
9. forensic event
10. review package
11. friction payload

### Requirements
- every contract has:
  - schema file
  - Rust domain type
  - example JSON
  - round-trip serialization test
  - invalid-case tests
- no free-form unbounded maps unless justified
- use explicit field presence rules

### Acceptance gate
- all schema examples validate
- invalid fixtures fail predictably
- artifact loading can reject malformed JSON before business logic runs

---

## Phase C — requester trust engine

### Build
Implement trusted request intake with a strict envelope.

### Requester trust contract fields
- requester_id
- requester_class
- app_context
- environment_mode
- trust_basis
- trust_basis_provenance
- user_intent_basis when required
- request_nonce_or_token
- issued_at
- expires_at

### Rules
- missing requester envelope => deny
- unknown requester class => deny
- missing trust basis => deny
- mismatched environment => deny
- expired request token/nonce => deny
- malformed app context => deny
- approval-worthy action without user-intent basis => review or deny

### Codex implementation notes
- keep trust evaluation as pure function over request + trust context
- return structured denial reasons, not booleans
- separate validation failure from authorization failure

### Acceptance gate
- one deny test per baseline denial rule
- no request reaches policy evaluation if trust fails

---

## Phase D — policy artifact loader and coupling validator

### Build
Create policy artifacts as first-class governed files.

### Policy fields
- policy_id
- policy_version
- scope
- capability_rules
- side_effect_rules
- approval_rules
- environment_conditions
- dependency_readiness_conditions
- failure_behavior
- policy_provenance

### Coupling validation
On load, verify:
- every referenced capability exists
- side-effect classes align with capability registry
- approval constraints are coherent
- no orphan capability references remain
- no revoked capability is treated as active

### Rules
- invalid policy => deny
- missing required policy => deny
- load failure => deny
- policy/capability mismatch => deny affected execution

### Acceptance gate
- smoke harness loads valid policy + registry
- broken references fail closed
- policy cannot silently ignore missing capabilities

---

## Phase E — capability admission registry

### Build
Capability registry with strict admission posture.

### Capability fields
- capability_id
- owner_service
- capability_type
- side_effect_class
- approval_posture
- allowed_requester_classes
- timeout_budget
- retry_budget
- max_duration_budget
- max_cpu_budget optional
- max_mem_budget optional
- enabled_state
- review_class
- provenance
- revocation_state
- version_range optional

### Rules
- unregistered => deny
- disabled => deny
- revoked => deny
- requester class not allowed => deny
- version range mismatch => deny
- policy mismatch => deny

### Acceptance gate
- registry load test
- revocation test
- disabled capability test
- requester mismatch test

---

## Phase F — approval posture resolver

### Build
Pure decision engine that resolves posture from:
- requester trust outcome
- policy rules
- capability properties
- side-effect class
- dependency readiness
- environment mode
- degraded state

### Required output
Structured `RouteDecision` containing:
- route_decision_id
- correlation_id
- resolved_posture
- reasons[]
- allowed_capabilities[]
- denied_capabilities[]
- review_required bool
- explicit_approval_required bool
- degraded_subtype optional
- policy_id / version
- execution_allowed bool

### Acceptance gate
- golden tests for each posture
- posture demotion tests on degraded readiness
- no ambiguous posture output

---

## Phase G — bounded execution plan validator

### Build
Multi-step plans must be accepted only if fully bounded.

### Execution plan fields
- execution_plan_id
- ordered_step_ids
- referenced_capabilities
- max_step_count
- fallback_refs optional
- cancellation_policy
- completion_policy
- max_duration_budget
- stable_plan_hash

### Rules
- no dynamic step invention
- no unregistered capability in any step
- no execution if step count exceeds declared max
- fallbacks must already be declared and policy-valid
- plan hash must be computed on acceptance

### Plan hashing
- define canonical serialization order
- compute stable SHA-256 hash of accepted plan envelope
- include hash in status, forensics, review package, and deviation reports

### Acceptance gate
- same plan input produces same hash
- changed step order changes hash
- undeclared fallback invalidates plan

---

## Phase H — execution coordinator

### Build
Coordinator that executes a prevalidated bounded plan across approved local routes.

### Responsibilities
- step sequencing
- timeout enforcement
- retry enforcement
- cancellation handling
- completion semantics
- degraded-state transition handling
- truthful partial/failure/cancel reporting
- correlation propagation

### Non-responsibilities
- planning
- semantic reinterpretation
- ad hoc fallback invention

### Acceptance gate
- single-step execution tests
- multi-step bounded execution tests
- timeout path tests
- cancel path tests
- partial completion tests

---

## Phase I — service integration adapters

### DF Local adapter
Use only for:
- readiness
- registration lookup
- bounded persistence
- forensic storage and retrieval where allowed

### Cortex adapter
Use only for:
- approved Cortex contracts
- dependency readiness
- correlation propagation

Do not absorb content semantics.

### NeuronForge Local adapter
Use only for:
- approved inferential contracts
- readiness and invocation
- truthful fallback/degraded handling

Do not absorb inference semantics.

### Acceptance gate
- fake adapter tests first
- then real adapter boundary tests
- every adapter call carries correlation ID
- unavailable dependency yields explicit degraded subtype or deny

---

## Phase J — execution status model and review handover

### Build execution status contract
Status must expose:
- request_id
- correlation_id
- execution_plan_id
- stable_plan_hash
- current_posture
- state
- degraded_subtype optional
- started_at / updated_at / completed_at
- current_step
- completion_summary
- failure_summary optional
- truthful_user_visible_summary

### Build review package contract
When posture is `ExplicitApprovalRequired`, emit:
- review_package_id
- originating_request_id
- correlation_id
- execution_plan_id
- stable_plan_hash
- intent_basis
- requester_summary
- proposed_execution_summary
- side_effect_assessment
- degraded_or_fallback_posture optional
- approval_options_allowed_by_policy
- denial_consequences_if_declined

### Acceptance gate
- review package generated for all explicit approval cases
- no silent waiting state exists
- status always includes degraded subtype when present

---

## Phase K — forensic event model

### Build
Minimal event system for execution truth, not surveillance.

### Mandatory fields
- forensic_event_id
- correlation_id
- event_type
- request_id
- execution_plan_id optional
- stable_plan_hash optional
- timestamp_utc
- posture
- degraded_subtype optional
- summary
- redaction_level
- payload_minimized

### Storage posture
- SQLite as primary local store
- JSONL export / inspect support
- no proprietary dependency to inspect records

### Acceptance gate
- events can be written and queried locally
- events can export to JSONL
- sensitive payloads obey minimization/redaction rules

---

## Phase L — denial harness and smoke suite

### Build
Dedicated fail-closed harness before any “happy path” confidence claims.

### Minimum deny suite
At least one test for each:
- unknown requester
- malformed requester envelope
- environment mismatch
- expired token/nonce
- missing trust basis
- missing required policy
- invalid policy
- policy/capability mismatch
- unregistered capability
- disabled capability
- revoked capability
- requester class not allowed
- undeclared fallback
- dependency unavailable where fallback is not allowed

### Acceptance gate
- all deny tests pass
- deny outputs are structured and explainable
- no deny path results in partial execution side effects

---

# 6. Recommended implementation order inside Rust

## Domain-first order
1. shared primitives
2. contract models
3. schema loader/validator
4. requester trust evaluator
5. capability registry loader
6. policy loader + coupling validator
7. posture resolver
8. execution plan validator + hasher
9. execution coordinator
10. adapters
11. review package emitter
12. forensic recorder
13. smoke harness

## Important rule
Do not start adapter-heavy work before the trust/policy/capability/posture core is passing tests.

---

# 7. Required test strategy

## Test layers

### Unit tests
- enum validity
- contract parsing
- plan hashing determinism
- posture resolution
- denial reason generation

### Contract tests
- schema validation
- example fixture validity
- invalid fixture rejection

### Property / determinism tests
- stable plan hash reproducibility
- correlation propagation integrity
- posture monotonicity under degraded demotion rules

### Integration tests
- valid request to admitted route
- multi-step bounded execution
- review package handoff
- forensic write/export

### Smoke / security tests
- all baseline deny cases
- no unauthorized capability execution
- no revoked capability use after registry reload

---

# 8. Done definition

FA Local is **not done** when it can merely run a route.

FA Local is done for MVP only when:
- trusted request intake exists
- invalid or untrusted requests deny fail-closed
- policy artifacts are required and enforced
- capability registry is required and enforced
- policy/capability coupling validation works
- approval posture resolution is deterministic
- bounded execution plans are required for multi-step work
- stable plan hashes are emitted and recorded
- truthful degraded subtypes are surfaced
- explicit approval cases emit review packages
- local forensic events are open-format and minimal
- denial smoke harness passes

---

# 9. Explicit non-goals for Codex

Tell Codex not to add any of the following unless separately planned:
- autonomous planning engine
- natural-language intent interpretation layer
- durable workflow memory
- semantic content analysis
- generic agent framework
- cloud dependency as baseline
- hidden telemetry or surveillance analytics
- unbounded plugin execution
- fallback invention at runtime

---

# 10. First concrete build tickets

## Ticket 1
Scaffold repo, crate modules, docs stubs, and core enums.

## Ticket 2
Author JSON schemas and Rust models for all core contracts.

## Ticket 3
Implement schema validation loader and example fixture validation tests.

## Ticket 4
Implement requester trust evaluator and structured denial reasons.

## Ticket 5
Implement capability registry loader, revocation handling, and admission checks.

## Ticket 6
Implement policy artifact loader and policy/capability coupling validation.

## Ticket 7
Implement approval posture resolver with deterministic golden tests.

## Ticket 8
Implement bounded execution plan validator and stable plan hash generation.

## Ticket 9
Implement execution coordinator with timeout/cancel/partial/degraded handling.

## Ticket 10
Implement review package emission and explicit-approval handoff flow.

## Ticket 11
Implement forensic event recorder with SQLite + JSONL export.

## Ticket 12
Implement denial smoke harness covering all baseline fail-closed rules.

---

# 11. Codex handoff prompt

Use this prompt in VS Code Codex:

```text
Build FA Local as a Rust local execution control service based on the attached FA Local v1.2 plan and this build plan.

Hard constraints:
- Fail closed by default.
- Unknown, malformed, expired, mismatched, revoked, disabled, unregistered, or policy-invalid requests must deny.
- Do not add autonomous planning, semantic authority, durable semantic memory, or generic agent behavior.
- Multi-step execution must require a bounded validated execution plan.
- Every admitted multi-step plan must receive a stable plan hash.
- Explicit approval cases must emit a structured review package.
- Execution status must be truthful about denied, canceled, failed, partial, and degraded states.
- Degraded must always carry an explicit subtype.
- Forensics must be minimal, locally auditable, and exportable in open format.
- Keep Cortex, NeuronForge Local, and DF Local behind explicit adapters and do not absorb their semantics.

Implementation order:
1. scaffold repo and core enums
2. define schema contracts and Rust domain models
3. implement schema loading and validation
4. implement requester trust evaluation
5. implement capability registry admission
6. implement policy artifact loading and coupling validation
7. implement approval posture resolution
8. implement bounded execution plan validation and stable hashing
9. implement execution coordination
10. implement review package emission
11. implement forensic recording/export
12. implement full denial smoke harness

Testing requirements:
- contract tests for all schemas
- deterministic tests for plan hashing
- golden tests for posture resolution
- deny tests for all baseline fail-closed cases
- integration tests for bounded execution and review handoff

Output expectations:
- production-structured Rust modules
- explicit enums and typed contracts
- test coverage for all guardrails
- no placeholder “TODO” logic in trust/policy/capability/denial paths
```

---

# 12. Final build judgment

This should be built **inside-out**:

- governance core first
- contracts second
- execution third
- integrations fourth
- UI-facing review and diagnostics fifth

That order keeps FA Local from drifting into “it runs, so ship it” territory before the authority model is actually real.

