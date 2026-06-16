# Rust Runtime MVP 設計・軽量TDD実行計画

## 1. 目的とスコープ

- 目的: 既存Python実装を変更せず、[`rust/`](rust/) 側だけでRuntime MVPに到達する。
- 対象crate: [`rust/protocol`](rust/protocol/), [`rust/bus`](rust/bus/), [`rust/runtime`](rust/runtime/), [`rust/cli`](rust/cli/)。
- 明示的に対象外: Python実装、NATS production backend、Cingulater HTTP provider、MCP Gateway、Qdrant/Indexer、shell/file worker production実装。
- 未検証のcoverage値や性能値を [`README.md`](README.md) に記載しない。

Index Probe: query `Rust runtime MVP protocol envelope bus runtime CLI run-demo tests docs` / path [`rust/`](rust/) / candidates [`rust/protocol/src/message.rs`](rust/protocol/src/message.rs), [`rust/bus/src/lib.rs`](rust/bus/src/lib.rs), [`rust/runtime/src/lib.rs`](rust/runtime/src/lib.rs)。

## 2. 現状整理

- Workspaceは [`rust/Cargo.toml`](rust/Cargo.toml:1) で [`rust/protocol`](rust/protocol/)、[`rust/bus`](rust/bus/)、[`rust/runtime`](rust/runtime/)、[`rust/cli`](rust/cli/) を含む。
- [`EventEnvelope`](rust/protocol/src/message.rs:5) はcanonical envelope項目とcurrent extension項目を持つが、既存利用側にフィールド不整合があるため、serde互換の契約テストを先行する。
- [`RuntimeLLMRequestPayload`](rust/protocol/src/payload.rs:56) はprompt中心で、messages最後尾contentのMVP入力に対応する余地がある。
- [`BasicBus::publish`](rust/bus/src/lib.rs:135) はsubscriberなしをエラーにしているため、MVPではclosed時のみErr、subscriberなしOk、published event記録へ契約変更する。
- [`ThalamusRuntime`](rust/runtime/src/lib.rs:57) は汎用handler登録とpublishを持つが、agent/task/llm/tool subjectのdefault subscription、Mock provider、Echo toolが未定義。
- [`CLICommand`](rust/cli/src/lib.rs:19) は `run-demo` subcommandを未提供。

## 3. MVP設計

### 3.1 責任分担

| 領域 | 責任 | 変更方針 |
|---|---|---|
| [`rust/protocol`](rust/protocol/) | Event envelope、subject、payloadのserde契約 | [`EventEnvelope`](rust/protocol/src/message.rs:5) とMVP payloadを後方互換寄りに整備 |
| [`rust/bus`](rust/bus/) | in-memory publish/subscribe | [`BasicBus`](rust/bus/src/lib.rs:80) にpublished event記録とsubscriberなしOkを追加 |
| [`rust/runtime`](rust/runtime/) | default subscription、agent/task/llm/toolのMVP処理 | Runtime起動時にdefault subjectを購読し、Mock LLMとEcho toolで完結 |
| [`rust/cli`](rust/cli/) | 手動疎通用entrypoint | `run-demo` でRuntime MVPの一連イベントを実行 |
| docs | 利用者向け事実記述 | 実行確認済み事実だけ [`README.md`](README.md) またはdocsへ反映 |

### 3.2 API契約

- [`EventEnvelope`](rust/protocol/src/message.rs:5): canonical envelopeとcurrent extensionをserdeで読み書きできる。既存canonical JSONにextension欠落があってもdefaultで復元できることを目標にする。
- MVP payload: [`RuntimeTaskAssignPayload`](rust/protocol/src/payload.rs:5)、[`RuntimeTaskResultPayload`](rust/protocol/src/payload.rs:17)、[`RuntimeLLMRequestPayload`](rust/protocol/src/payload.rs:56)、[`RuntimeLLMResponsePayload`](rust/protocol/src/payload.rs:65)、[`RuntimeToolRequestPayload`](rust/protocol/src/payload.rs:38)、[`RuntimeToolResultPayload`](rust/protocol/src/payload.rs:47) をRuntime MVPの入出力境界にする。
- [`BasicBus::publish`](rust/bus/src/lib.rs:135): closed時のみErr。subscriberなしはOk。公開済みイベントは後続テストが検証できるよう読み取りAPIで参照可能にする。
- [`ThalamusRuntime::start`](rust/runtime/src/lib.rs:137): agent/task/llm/tool subjectのdefault subscriptionを登録する。
- Mock LLM provider: promptまたはmessages最後尾contentから deterministic mock response を生成する。外部HTTPを呼ばない。
- Echo tool: inputを変形せず返す。shell/file worker production実装はしない。
- [`CLICommand`](rust/cli/src/lib.rs:19): `run-demo` を追加し、MVPイベント処理の成功をCLIから確認可能にする。

### 3.3 データ構造

- Envelope: id、type、subject、source、timestamp、schema、payload、metadata、correlation/causationを維持し、extensionはserde defaultで互換性を守る。
- Runtime state: 既存 [`RuntimeState`](rust/runtime/src/lib.rs:22) を維持し、MVP処理結果はbusへpublishされたeventとして観測する。
- Published event log: [`BasicBus`](rust/bus/src/lib.rs:80) 内部に追加し、テスト向けにclone済みevent列を返す読み取りメソッドを用意する。

## 4. 軽量TDD実行計画

全unit共通: 初期テスト最大3個、Red-Green-Refactor、Coverage 85%以上、security-auditor gate、reviewer gateを維持する。exploratory testは作成しない。必要時は完了前にcontract / behavior / regressionへ昇格または削除する。

### Unit 1: Protocol contract

- Mode順: test-writer → tester Red → consistency-checker → code → tester Green/Coverage → consistency-checker。
- Read Files: [`rust/protocol/src/message.rs`](rust/protocol/src/message.rs), [`rust/protocol/src/payload.rs`](rust/protocol/src/payload.rs), [`rust/protocol/src/subject.rs`](rust/protocol/src/subject.rs)。
- Edit Files: [`rust/protocol/tests/protocol_contract.rs`](rust/protocol/tests/protocol_contract.rs) then [`rust/protocol/src/message.rs`](rust/protocol/src/message.rs), [`rust/protocol/src/payload.rs`](rust/protocol/src/payload.rs)。
- TDD Level / Classification: Level 1 Contract Test / contract。
- Initial Test Count: 最大3。
- Red: canonical envelopeまたはMVP payloadのserde契約不一致によるcompile/test failure。
- Acceptance Criteria: canonical/current extension JSON互換、MVP payload round-trip、subject constantsの期待値維持。
- Artifact Handoff: Red [`artifacts/test-results/rust-protocol-red.log`](artifacts/test-results/rust-protocol-red.log)、Green [`artifacts/test-results/rust-protocol-green.log`](artifacts/test-results/rust-protocol-green.log)、Coverage [`artifacts/coverage/rust-protocol-coverage.log`](artifacts/coverage/rust-protocol-coverage.log)。

### Unit 2: Bus behavior

- Mode順: test-writer → tester Red → consistency-checker → code → tester Green/Coverage → consistency-checker。
- Read Files: [`rust/bus/src/lib.rs`](rust/bus/src/lib.rs), [`rust/protocol/src/message.rs`](rust/protocol/src/message.rs)。
- Edit Files: [`rust/bus/tests/bus_behavior.rs`](rust/bus/tests/bus_behavior.rs) then [`rust/bus/src/lib.rs`](rust/bus/src/lib.rs)。
- TDD Level / Classification: Level 2 Behavior Test / behavior。
- Initial Test Count: 最大3。
- Red: [`BasicBus::publish`](rust/bus/src/lib.rs:135) がsubscriberなしをErrにする、またはpublished event記録APIが未実装。
- Acceptance Criteria: subscriberなしOk、closed時Err、publish済みeventを順序維持で取得可能。
- Artifact Handoff: Red [`artifacts/test-results/rust-bus-red.log`](artifacts/test-results/rust-bus-red.log)、Green [`artifacts/test-results/rust-bus-green.log`](artifacts/test-results/rust-bus-green.log)、Coverage [`artifacts/coverage/rust-bus-coverage.log`](artifacts/coverage/rust-bus-coverage.log)。

### Unit 3: Runtime default subscriptions and MVP handlers

- Mode順: analyzer必要時 → test-writer → tester Red → consistency-checker → code → tester Green/Coverage → consistency-checker。
- Read Files: [`rust/runtime/src/lib.rs`](rust/runtime/src/lib.rs), [`rust/bus/src/lib.rs`](rust/bus/src/lib.rs), [`rust/protocol/src/payload.rs`](rust/protocol/src/payload.rs)。
- Edit Files: [`rust/runtime/tests/runtime_basic.rs`](rust/runtime/tests/runtime_basic.rs) then [`rust/runtime/src/lib.rs`](rust/runtime/src/lib.rs)。
- TDD Level / Classification: Level 2 Behavior Test / behavior。
- Initial Test Count: 最大3。
- Red: default subscriptions、Mock LLM provider、Echo toolの未実装によるcompile/test failure。
- Acceptance Criteria: agent/task/llm/tool subjectを購読、Mock LLMがpromptまたはmessages最後尾contentから応答、Echo toolがinputをそのまま返す。
- Artifact Handoff: Red [`artifacts/test-results/rust-runtime-red.log`](artifacts/test-results/rust-runtime-red.log)、Green [`artifacts/test-results/rust-runtime-green.log`](artifacts/test-results/rust-runtime-green.log)、Coverage [`artifacts/coverage/rust-runtime-coverage.log`](artifacts/coverage/rust-runtime-coverage.log)。

### Unit 4: CLI run-demo contract

- Mode順: test-writer → tester Red → consistency-checker → code → tester Green/Coverage → consistency-checker。
- Read Files: [`rust/cli/src/lib.rs`](rust/cli/src/lib.rs), [`rust/runtime/src/lib.rs`](rust/runtime/src/lib.rs)。
- Edit Files: [`rust/cli/tests/cli_contract.rs`](rust/cli/tests/cli_contract.rs) then [`rust/cli/src/lib.rs`](rust/cli/src/lib.rs)。
- TDD Level / Classification: Level 1 Contract Test / contract。
- Initial Test Count: 最大3。
- Red: `run-demo` subcommand未定義または実行契約不一致。
- Acceptance Criteria: `run-demo` をparse可能、verbose有無に依存せずMVP demoを実行可能、失敗時は [`CliError`](rust/cli/src/lib.rs:36) へ集約。
- Artifact Handoff: Red [`artifacts/test-results/rust-cli-red.log`](artifacts/test-results/rust-cli-red.log)、Green [`artifacts/test-results/rust-cli-green.log`](artifacts/test-results/rust-cli-green.log)、Coverage [`artifacts/coverage/rust-cli-coverage.log`](artifacts/coverage/rust-cli-coverage.log)。

### Unit 5: Documentation update

- Mode順: technical-writer → reviewer。
- Read Files: [`README.md`](README.md), [`plans/rust-runtime-mvp-plan.md`](plans/rust-runtime-mvp-plan.md)。
- Edit Files: [`README.md`](README.md) または [`docs/rust-runtime-design.md`](docs/rust-runtime-design.md) のうちOrchestratorが許可したMarkdownのみ。
- TDD Level / Classification: Level 0 Smoke documentation check / contract相当の事実確認。
- Acceptance Criteria: 実装済みRuntime MVPの使い方だけ記載、未検証coverage値を記載しない、Python側非破壊を明記。
- Artifact Handoff: docs差分要約 [`artifacts/handoff/rust-docs-handoff.md`](artifacts/handoff/rust-docs-handoff.md)。

### Unit 6: Verification, audit, release gates

- Mode順: tester → consistency-checker → security-auditor → reviewer → release-manager → diagnostic-reporter。
- Commands候補: `cargo test --workspace`、coverageは既存導入済み手段がある場合のみ実行し、不足依存はsegregated-devopsへ分離。
- Coverage Gate: Rust対象で85%以上をArtifactから確認する。未計測のままGreen完了にしない。
- Security Gate: hardcoded secret、外部HTTP導入、捏造crate、shell/file worker production実装混入をRejectする。
- Reviewer Gate: 設計スコープ、TDD inventory、Python非破壊、README未検証指標禁止を確認する。
- GitHub Gate: GitHub Integration StateはgithubだがIssue URL起点ではない。品質gate後のみrelease-managerとdiagnostic-reporterへ進む。

## 5. 後続委任テンプレート要点

- test-writer: テストファイルのみ編集。TDD Level、Test Classification、Initial Test Count最大3、Expected Red Signature、Forbidden Patternsを必ず渡す。
- tester: 指定commandをArtifact Pathへ保存するだけ。Red/Green/Coverage判定はしない。
- consistency-checker: Artifact PathまたはChanged Filesだけを読み、test-red / test-green / coverage / implementation-scope / test-inventoryを判定する。
- code: 実装ファイルだけ編集。テスト・coverage・buildは実行しない。
- security-auditor / reviewer: 実装後に必須。長い監査結果は [`artifacts/`](artifacts/) 配下へ保存する。

## 6. リスクと停止条件

- [`EventEnvelope`](rust/protocol/src/message.rs:5) の必須field追加が既存テストと衝突する場合は、Protocol Unitを優先して契約を確定し、codeへ無理にGreen化させない。
- coverage provider不足、lockfile更新要求、peer conflictは実装失敗ではなくsegregated-devopsへ分離する。
- test-writerのmock境界誤認、codeの同型失敗、testerの同型失敗が2回連続した場合はrecovery-supervisorへ切り替える。
- Python実装、NATS production backend、Cingulater HTTP provider、MCP Gateway、Qdrant/Indexer、shell/file worker production実装へ範囲拡大しない。
