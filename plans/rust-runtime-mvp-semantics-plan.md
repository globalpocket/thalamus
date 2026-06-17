# Rust Runtime MVP Semantics Plan

## Goal

Complete the Rust Runtime MVP semantic contract for protocol structured fields, task and worker state, default runtime handlers, response correlation/causation, and CLI demo event flow without expanding into non-MVP systems.

## Source of Truth

* Index Probe: query `Rust runtime EventEnvelope TaskState WorkerRegistry default handlers CLI RunDemo`, path `rust`, candidates [`message.rs`](../rust/protocol/src/message.rs:5), [`payload.rs`](../rust/protocol/src/payload.rs:10), [`lib.rs`](../rust/runtime/src/lib.rs:199), and [`lib.rs`](../rust/cli/src/lib.rs:37).
* Design reference: [`rust-runtime-design.md`](../docs/rust-runtime-design.md:1).
* Status reference: [`README.md`](../README.md:378).

## Scope Boundaries

In scope:

* [`EventEnvelope.scope`](../rust/protocol/src/message.rs:14) and [`EventEnvelope.refs`](../rust/protocol/src/message.rs:15) as optional structured values.
* [`TaskState`](../rust/runtime/src/lib.rs:66), task status transitions, [`WorkerRegistry`](../rust/runtime/src/lib.rs:113), default runtime handlers, and [`ThalamusRuntime::publish()`](../rust/runtime/src/lib.rs:530).
* LLM/tool result envelope `correlation_id` and `causation_id` semantics.
* [`RunDemo`](../rust/cli/src/lib.rs:37) through runtime event flow.

Out of scope and forbidden for this MVP unit:

* Python implementation changes, Cingulater HTTP integration, MCP Gateway, NATS production backend, Qdrant/indexer, ZooCodeCustom integration, SDK/FFI expansion, and new external provider mediation.

## Semantic Contracts

* Protocol: [`EventEnvelope`](../rust/protocol/src/message.rs:5) exposes canonical fields plus optional structured `scope` and `refs`; tests must cover object/array values and omitted-field defaults.
* Task state: [`TaskState`](../rust/runtime/src/lib.rs:66) must represent task id, optional assigned agent, and status transitions from assignment to result completion without unrelated states.
* Worker state: [`WorkerRegistry`](../rust/runtime/src/lib.rs:113) must preserve worker records, capabilities, latest state, and lookup by id across ready/exit/error events.
* Publish semantics: [`ThalamusRuntime::publish()`](../rust/runtime/src/lib.rs:530) succeeds for accepted events even with no external subscribers, records through the bus, then applies default behavior.
* Response semantics: LLM/tool result envelopes use the request event id for envelope-level correlation and causation; payload-level correlation preserves the request payload value.
* CLI semantics: [`RunDemo`](../rust/cli/src/lib.rs:37) must exercise runtime-mediated LLM/tool publication and keep deterministic output markers.

## Lightweight TDD Plan

Initial Red phase is capped at three tests and must not use exploratory tests.

1. Level 1 contract test in [`protocol_contract.rs`](../rust/protocol/tests/protocol_contract.rs:14): verify structured `scope`/`refs` values and omitted-field defaults for [`EventEnvelope`](../rust/protocol/src/message.rs:5).
2. Level 2 behavior test in [`runtime_basic.rs`](../rust/runtime/tests/runtime_basic.rs:31): verify task state, worker registry updates, default handler completion, and publish-without-external-handler success as one runtime event-flow unit if the current test file structure supports it.
3. Level 2 behavior test in [`runtime_basic.rs`](../rust/runtime/tests/runtime_basic.rs:31) or [`cli_contract.rs`](../rust/cli/tests/cli_contract.rs:5): verify LLM/tool response envelope correlation/causation and [`RunDemo`](../rust/cli/src/lib.rs:37) runtime event flow. Split only if a single test would exceed one observable behavior.

Additional user acceptance criteria may be promoted into follow-up Level 1/2 contract/behavior tests only after the first Red/Green cycle. Final tests must remain contract/behavior tests; exploratory tests are prohibited and must not be committed.

## Red-Green-Refactor Flow

1. test-writer adds only the initial three contract/behavior tests in the allowed Rust test files; it does not run tests or edit implementation.
2. tester runs targeted `cargo test` and saves stdout/stderr to `artifacts/test-results/rust-runtime-mvp-semantics-red.log`; consistency-checker verifies the expected Red signature and rejects syntax/import/mock-boundary failures.
3. code implements the minimal Rust changes in protocol/runtime/CLI files only after Red is confirmed; it does not run tests.
4. tester runs `cargo fmt`, `cargo clippy`, `cargo test`, and coverage if configured, writing logs under `artifacts/build/`, `artifacts/test-results/`, and `artifacts/coverage/`; consistency-checker verifies Green and Coverage 85%以上.
5. refactorer may run only after Green if readability cleanup is behavior-preserving; tester and consistency-checker re-run the relevant checks.

## Quality Gates

* Coverage gate: final Rust verification must demonstrate Coverage 85%以上 or return a dependency/environment handoff if the coverage provider is unavailable.
* security-auditor gate: run after Green to confirm no hardcoded secrets, unsafe external mediation expansion, or fabricated dependencies were introduced.
* reviewer gate: run after security-auditor to confirm design alignment, MVP scope boundaries, test inventory, and maintainability.
* GitHub completion gate: because GitHub Integration State is `github`, release-manager and diagnostic-reporter run only after tests, Coverage 85%以上, security-auditor, and reviewer pass. Issue-tracker intake is skipped because no Issue URL was supplied.

## Artifact Handoff

* Red test log: `artifacts/test-results/rust-runtime-mvp-semantics-red.log`.
* Green test log: `artifacts/test-results/rust-runtime-mvp-semantics-green.log`.
* Build/lint logs: `artifacts/build/rust-runtime-mvp-semantics-fmt.log` and `artifacts/build/rust-runtime-mvp-semantics-clippy.log`.
* Coverage log: `artifacts/coverage/rust-runtime-mvp-semantics-coverage.log`.
* Security/review logs, if longer than five lines, must be stored under `artifacts/security/` and `artifacts/handoff/` and passed by path only.

## Execution Checklist

- [ ] test-writer: add up to three Level 1/2 contract/behavior tests for protocol fields, runtime state/default handlers, and response/demo semantics.
- [ ] tester: run Red command and store the artifact path; no failure classification in tester output.
- [ ] consistency-checker: verify Red matches expected contract/behavior gaps and no syntax/import/mock-boundary error.
- [ ] code: implement only Rust MVP semantic changes in allowed protocol/runtime/CLI files.
- [ ] tester: run fmt, clippy, tests, and coverage with artifacts.
- [ ] consistency-checker: verify Green, Coverage 85%以上, implementation scope, and test inventory.
- [ ] security-auditor: audit implementation and dependency integrity.
- [ ] reviewer: confirm design, tests, MVP boundary, and maintainability.
- [ ] technical-writer: update README/docs only if implementation changes make current status text inconsistent.
- [ ] release-manager and diagnostic-reporter: run GitHub completion gates after all quality gates pass.
