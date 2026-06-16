# Rust Runtime MVP Alignment Plan

## Purpose

Rust Runtime MVP の既存実装を、Protocol、Bus/Runtime、CLI、Docs の観測可能な契約へ整合させる。新機能拡張ではなく、現行 Rust workspace の MVP 契約を明確化し、後続を AI 軽量 TDD の小単位へ分割する。

## Source of Truth

- Raw prompt: Rust Runtime MVP整合。
- Index Probe: query `Rust Runtime MVP EventEnvelope payload BasicBus Runtime handlers TaskState WorkerRegistry CLI run-demo`, path `rust/`, candidates `rust/protocol/src/subject.rs`, `rust/protocol/src/payload.rs`, `rust/runtime/src/lib.rs`, `rust/cli/src/lib.rs`。
- Current design reference: `docs/rust-runtime-design.md`。
- GitHub Integration State: `unknown-skipped`; GitHub MCP、release、diagnostic Issue は対象外。

## Current Responsibilities

- `rust/protocol/src/subject.rs`: Runtime subject constants and `runtime_task_assign_for_agent(agent_id)` dynamic subject helper.
- `rust/protocol/src/payload.rs`: Runtime task, agent, tool, and LLM payload serde contracts. LLM request accepts `prompt` or last `messages[].content` through custom deserialize behavior.
- `rust/runtime/src/lib.rs`: Runtime lifecycle, handler registry, task handle tracking, `TaskState`, `WorkerRegistry`, deterministic `MockLlmProvider`, `EchoTool`, bus-backed publish/handle paths.
- `rust/cli/src/lib.rs`: CLI command surface and deterministic `RunDemo` path using `MockLlmProvider` and `EchoTool`.
- `docs/rust-runtime-design.md`: Existing MVP description to update only after implementation and verification.

## MVP Contract to Preserve or Align

### Protocol and Data Contracts

- Event envelopes must keep `scope` and `refs` as structured `serde_json::Value`-compatible data, not lossy string-only fields.
- Runtime payload structs must remain serde-compatible and preserve `Value` fields for `input`, `metadata`, `output`, and `result`.
- `runtime.llm.response` and `runtime.tool.result` events published from request handling must set `correlation_id` and `causation_id` to the source request event id.
- LLM response and tool result payloads must preserve request identity through `request_id`; optional `task_id` in requests may fall back to empty string only if no task id is supplied by the request.

### Bus and Runtime Contracts

- `BasicBus` publish semantics must allow accepted publishes even when a subject has no subscribers; runtime-level `publish` must not reject solely because no subscriber or registered handler exists.
- Runtime default handling for `runtime.llm.request` and `runtime.tool.request` must publish deterministic response/result events through the bus without external services.
- `handle_event` may keep explicit schedule errors for direct dispatch to unknown handlers, because this is separate from bus publish acceptance.
- `TaskState` must expose stateful task assignment behavior; `WorkerRegistry` must retain registered worker ids and capabilities and support lookup without external persistence.

### CLI Contract

- `RunDemo` remains a local deterministic demo and must not introduce Python rewrites, Cingulater HTTP, MCP Gateway, NATS production backend, Qdrant/Indexer, ZooCodeCustom integration, SDK, or FFI expansion.
- CLI output should continue to demonstrate mock LLM response and echo tool result using Rust runtime payloads.

### Documentation Contract

- `docs/rust-runtime-design.md` and `README.md` are forbidden for this planning task.
- Documentation updates, if needed after implementation and verification, must be delegated to Technical Writer and must describe only verified behavior.

## Lightweight TDD Work Breakdown

All implementation units start with Level 1 Contract Test or Level 2 Behavior Test. Initial tests per unit are capped at 3. Exploratory tests, if introduced, must be promoted to contract/behavior/regression or removed before completion. Coverage gate is 85% or higher after Green verification.

### 1. Protocol envelope structured fields

- Mode sequence: `test-writer` -> `tester` -> `consistency-checker` -> `code` -> `tester` -> `consistency-checker`.
- TDD Level: Level 1 Contract Test.
- Test Classification: contract.
- Edit candidates: test file under `rust/protocol` only for Red, then `rust/protocol/src/message.rs` or equivalent protocol implementation only for Green.
- Initial Test Count: up to 2.
- Expected Red Signature: structured `scope` and `refs` cannot round-trip as `serde_json::Value` structures, or constructor rejects structured values.
- Acceptance Criteria: `scope` accepts object/array/null-compatible `Value`; `refs` preserves structured references; existing envelope fields remain serde-compatible.

### 2. Payload identity and message fallback contracts

- Mode sequence: `test-writer` -> `tester` -> `consistency-checker` -> `code` -> `tester` -> `consistency-checker`.
- TDD Level: Level 1 Contract Test.
- Test Classification: contract.
- Edit candidates: test file under `rust/protocol`, then `rust/protocol/src/payload.rs` only if Red confirms a payload mismatch.
- Initial Test Count: up to 3.
- Expected Red Signature: LLM request `messages` fallback, request id preservation, or optional task id behavior diverges from MVP contract.
- Acceptance Criteria: `RuntimeLLMRequestPayload` preserves explicit prompt priority; falls back to last message content; tool/LLM result payloads preserve `request_id` and task identity behavior.

### 3. Runtime publish without subscribers

- Mode sequence: `test-writer` -> `tester` -> `consistency-checker` -> `code` -> `tester` -> `consistency-checker`.
- TDD Level: Level 2 Behavior Test.
- Test Classification: behavior.
- Edit candidates: runtime test file, then `rust/runtime/src/lib.rs` only.
- Initial Test Count: up to 2.
- Expected Red Signature: runtime `publish` returns bus error for subject with no subscribers/handlers.
- Acceptance Criteria: `publish` returns accepted `EventEnvelope`; bus records the event; direct `handle_event` unknown subject error remains unchanged.

### 4. Runtime response/result correlation

- Mode sequence: `test-writer` -> `tester` -> `consistency-checker` -> `code` -> `tester` -> `consistency-checker`.
- TDD Level: Level 2 Behavior Test.
- Test Classification: behavior.
- Edit candidates: runtime test file, then `rust/runtime/src/lib.rs` only.
- Initial Test Count: up to 2.
- Expected Red Signature: published `runtime.llm.response` or `runtime.tool.result` has missing `correlation_id`/`causation_id`.
- Acceptance Criteria: generated response/result events set both ids to request event id; payload remains deterministic; no external service or production backend is introduced.

### 5. TaskState and WorkerRegistry statefulness

- Mode sequence: `test-writer` -> `tester` -> `consistency-checker` -> `code` -> `tester` -> `consistency-checker`.
- TDD Level: Level 1 Contract Test.
- Test Classification: contract.
- Edit candidates: runtime test file, then `rust/runtime/src/lib.rs` only if required.
- Initial Test Count: up to 3.
- Expected Red Signature: task assignment or worker lookup does not preserve state/capabilities.
- Acceptance Criteria: assigned agent can be observed after assignment; registry lookup returns registered id and capabilities; absent worker returns none.

### 6. CLI deterministic run-demo

- Mode sequence: `test-writer` -> `tester` -> `consistency-checker` -> `code` -> `tester` -> `consistency-checker`.
- TDD Level: Level 2 Behavior Test.
- Test Classification: behavior.
- Edit candidates: CLI test file, then `rust/cli/src/lib.rs` only if required.
- Initial Test Count: up to 2.
- Expected Red Signature: `RunDemo` does not exercise both mock LLM and echo tool path, or output contract cannot be observed deterministically.
- Acceptance Criteria: run-demo uses Rust mock provider/tool only; output includes deterministic mock response and echo payload; no forbidden integration is introduced.

### 7. Documentation alignment after Green

- Mode sequence: `technical-writer` only after implementation, Green, and coverage gates pass.
- TDD Level: not applicable to docs; must reference verified artifacts.
- Test Classification: not applicable.
- Edit candidates: `docs/rust-runtime-design.md` and/or `README.md` only when Orchestrator explicitly delegates docs update.
- Acceptance Criteria: docs describe verified Rust MVP behavior, do not claim Python replacement, production NATS, MCP Gateway, real LLM, Qdrant/Indexer, SDK/FFI, or unmeasured coverage/security results.

## Quality Gates and Artifact Handoff

- Red verification: `tester` stores command output under `artifacts/test-results/`; `consistency-checker` confirms expected Red signature.
- Green verification: `tester` reruns targeted tests and Rust workspace tests; `consistency-checker` confirms pass.
- Coverage gate: `tester` stores coverage output under `artifacts/coverage/`; `consistency-checker` confirms 85% or higher. Missing coverage provider is a DevOps task, not Code.
- Security gate: `security-auditor` audits changed Rust code and dependency integrity after Green.
- Review gate: `reviewer` checks design alignment, test inventory, maintainability, and absence of forbidden integrations.
- GitHub gates: skipped because GitHub Integration State is `unknown-skipped`.

## Forbidden Scope

- Python大改修。
- Cingulater HTTP連携。
- MCP Gateway。
- NATS production backend。
- Qdrant/Indexer。
- ZooCodeCustom連携。
- SDK/FFI拡張。
- README/docs edits during implementation units unless delegated as the documentation unit after verification.

## Execution Checklist

- [ ] Protocol structured envelope contract: Level 1 contract, max 2 initial tests, Red-Green-Refactor, Coverage 85% gate.
- [ ] Payload identity/fallback contract: Level 1 contract, max 3 initial tests, Red-Green-Refactor, Coverage 85% gate.
- [ ] Runtime publish acceptance behavior: Level 2 behavior, max 2 initial tests, Red-Green-Refactor, Coverage 85% gate.
- [ ] Runtime correlation/causation behavior: Level 2 behavior, max 2 initial tests, Red-Green-Refactor, Coverage 85% gate.
- [ ] TaskState/WorkerRegistry statefulness: Level 1 contract, max 3 initial tests, Red-Green-Refactor, Coverage 85% gate.
- [ ] CLI run-demo behavior: Level 2 behavior, max 2 initial tests, Red-Green-Refactor, Coverage 85% gate.
- [ ] Test inventory cleanup: no exploratory tests remain; final tests are contract, regression, or necessary behavior only.
- [ ] Security Auditor gate passes.
- [ ] Reviewer gate passes.
- [ ] Documentation update is delegated only after verified behavior changes.
