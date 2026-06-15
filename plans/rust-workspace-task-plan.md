# Thalamus Rust Workspace Task Plan

## 前提

- Goal: Rust Cargoワークスペース（protocol、bus、runtime、cli、sdk）を作成する。
- Design: [`plans/rust-workspace-design.md`](rust-workspace-design.md)
- GitHub Integration State: non-github（GitHub操作はすべてskipped）
- Read Scope: 現在のワークスペース配下のみ。
- 共通Quality Gate: 各実装単位でRed-Green-Refactorを維持し、最終的にCoverage 85%以上、security-auditor Pass、reviewer Passを必須にする。
- 初期テスト数: 各軽量TDD単位で最大3個。Level 4 Full TDDを初手にしない。
- Exploratory Handling: exploratory testを使う場合は完了時にcontract / behavior / regressionへ昇格、または削除する。

## Index Probe

- query: Rust Thalamus workspace protocol bus runtime cli sdk cargo workspace serde tokio pubsub message envelope
- path: rust
- 主要候補: 新規作成（rust/ 配下未存在）

## 軽量TDDチェックリスト

- [x] **Unit 1: Cargo workspace root and protocol crate** — TDD Level 1 / Test Classification: contract / Red: `rust/Cargo.toml` と `rust/protocol/Cargo.toml` が存在せずworkspace member解決に失敗するテストを1個作成 / Green: workspaceルートとprotocolクレートのCargo.tomlを作成 / Refactor: 依存関係整理 / Acceptance: `cargo check` がworkspaceルートで成功し、Coverage 85%以上対象、security-auditorとreviewerで依存問題なし。Status: protocol contract test Green（`2 passed; 0 failed`）、Coverage 93.5%判定済み。
- [x] **Unit 2: EventEnvelope struct and serialization** — TDD Level 1 / Test Classification: contract / Red: `EventEnvelope` が `id`, `subject`, `source`, `timestamp`, `schema`, `payload`, `correlation_id`, `causation_id`, `metadata` を持たずシリアライゼーションに失敗するテストを最大3個作成 / Green: `rust/protocol/src/message.rs` にstruct定義とserde deriveを追加 / Refactor: envelope生成重複を整理 / Acceptance: serialize/deserializeがroundtripし、Coverage 85%以上対象。Status: protocol contract test Green（`2 passed; 0 failed`）、Coverage 93.5%判定済み。
- [x] **Unit 3: EventBus pub/sub contract** — TDD Level 1 / Test Classification: contract / Red: `EventBus` が `subscribe` / `publish` を持たずsubject dispatchに失敗するテストを最大3個作成 / Green: `rust/bus/src/pubsub.rs` にpub/sub実装を追加 / Refactor: handler登録APIを整理 / Acceptance: 完全一致subjectでhandlerが呼ばれ、Coverage 85%以上対象。Status: bus Green（`6 passed`）、追加behavior test（`3 passed`）、Coverage 96.33%、security-auditor Pass、reviewer Pass。
- [ ] **Unit 4: Subject-based routing** — TDD Level 2 / Test Classification: behavior / Red: `runtime.task.assign.<agent_id>` への特化ルーティングができず失敗するテストを最大3個作成 / Green: `rust/bus/src/router.rs` にワイルドカードマッチを追加 / Refactor: subject matchingロジックを単一関数に集約 / Acceptance: `runtime.task.assign.*` が特定agentへdispatchされ、Coverage 85%以上対象。
- [ ] **Unit 5: Runtime lifecycle start/stop** — TDD Level 2 / Test Classification: behavior / Red: `ThalamusRuntime` が `start` / `stop` でbus接続とhandler登録ができず失敗するテストを最大3個作成 / Green: `rust/runtime/src/lifecycle.rs` にstart/stop実装を追加 / Refactor: lifecycle state管理を整理 / Acceptance: start後にbusが接続し、stop後にhandlerが解除され、Coverage 85%以上対象。
- [ ] **Unit 6: Agent registry flow** — TDD Level 2 / Test Classification: behavior / Red: `runtime.agent.ready` / `runtime.agent.exit` / `runtime.agent.error` でregistry stateを更新できず失敗するテストを最大3個作成 / Green: Runtimeがagent state storeを保持しsubjectで更新 / Refactor: agent state enumを単一箇所へ集約 / Acceptance: agent lifecycleがprotocol subjectに準拠し、Coverage 85%以上対象。
- [ ] **Unit 7: CLI command parsing** — TDD Level 1 / Test Classification: contract / Red: `clap` 製CLIが `--subject` / `--source` / `--payload` をパースできず失敗するテストを最大3個作成 / Green: `rust/cli/src/commands.rs` にコマンド定義を追加 / Refactor: CLI引数構造を整理 / Acceptance: 主要コマンドが正しくパースされ、Coverage 85%以上対象。
- [ ] **Unit 8: SDK FFI bindings** — TDD Level 1 / Test Classification: contract / Red: `extern "C"` 関数が未定義で外部リンクに失敗するテストを最大3個作成 / Green: `rust/sdk/src/bindings.rs` に `thalamus_publish` / `thalamus_subscribe` / `thalamus_shutdown` を追加 / Refactor: FFIシグネチャを整理 / Acceptance: FFI関数がC互換シグネチャを持ち、Coverage 85%以上対象。

## GitHub終了ゲートチェックリスト

- [ ] **Release ManagerによるVersion Tag Push** — GitHub終了ゲートとしてversion/tag pushを実行する。non-githubのためskipped。
- [ ] **Diagnostic Issue登録** — GitHub終了ゲートとして診断Issueを日本語で登録する。non-githubのためskipped。

## 実行順序

1. code: Unit 1のworkspace rootとprotocolクレートを作成する。
2. test-writer: Unit 2のEventEnvelope contract Redテストを作成する。
3. tester: Unit 2 RedをArtifact Pathへ保存する。
4. consistency-checker: Expected Red Signature一致を判定する。
5. code: Unit 2をGreen化する。
6. tester + consistency-checker: GreenとCoverage 85%以上を確認する。
7. Unit 3からUnit 8まで同じ責務分離で反復し、bus、runtime、cli、sdkを個別にGreen化する。
8. security-auditor: 全クレート後にsecret、unsafe dependency、捏造依存を監査する。
9. reviewer: 設計書との整合、TDD inventory、exploratory test残存なしを再監査する。
10. technical-writer: README更新が実装差分とずれる場合だけ最小修正する。
11. release-manager: GitHub終了ゲートとしてversion/tag pushを実行する。non-githubのためskipped。
12. diagnostic-reporter: release-managerがVersion/Tag pushを完了した場合だけ、GitHub終了ゲートとして診断Issueを日本語で登録する。non-githubのためskipped。

## 後続モード委任の注意

- test-writerへは各UnitごとにInitial Test Count最大3、Exact Imports、Allowed Test Doubles、Forbidden Patterns、Expected Red Signatureを渡す。
- testerへは必ずArtifact Pathを準備してから実行コマンドだけを渡す。
- codeへは検証コマンド実行を含めず、Edit Filesを1から2件に制限する。
- dependency問題、coverage provider不足、`serde` / `tokio` 未導入が出た場合はcodeではなくsegregated-devopsへ分離する。
- GitHub関連操作はOrchestratorが直接行わず、release-manager、diagnostic-reporter、必要時issue-trackerに分離する。
