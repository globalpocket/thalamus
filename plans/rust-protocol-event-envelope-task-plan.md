# Rust Protocol EventEnvelope Completion Task Plan

## 目的

[`plans/rust-protocol-event-envelope-design.md`](rust-protocol-event-envelope-design.md:1) で固定したEventEnvelope、subject定数、payload struct、module exportを、AI軽量TDDでRust protocol crateへ実装する。初期テストは最大3個に制限し、contractを優先する。

## Index Probe

- query: EventEnvelope subject constants payload structs protocol event subjects thalamus
- path: workspace root
- 主要候補: [`plans/thalamus-runtime-reference-design.md`](thalamus-runtime-reference-design.md:83), [`runtime/events/types.py`](../runtime/events/types.py:11), [`runtime/events/validator.py`](../runtime/events/validator.py:21)
- Rust候補: [`rust/protocol/src/message.rs`](../rust/protocol/src/message.rs:4), [`rust/protocol/src/lib.rs`](../rust/protocol/src/lib.rs:1), [`rust/protocol/tests/protocol_contract.rs`](../rust/protocol/tests/protocol_contract.rs:1)

## 軽量TDDチェックリスト

- [ ] **Unit 1: EventEnvelope JSON contract** — TDD Level 1 / Test Classification: contract / Red: [`rust/protocol/tests/protocol_contract.rs`](../rust/protocol/tests/protocol_contract.rs:1) に最大1個のcontract testを追加し、`type`, `scope`, `refs` がdeserialize/serializeで保持されず失敗する状態を確認 / Green: [`rust/protocol/src/message.rs`](../rust/protocol/src/message.rs:4) に `#[serde(rename = "type")] pub r#type: String`, `scope: Option<String>`, `refs: Vec<String>` を追加し、constructor引数も更新 / Refactor: field順を設計表と一致 / Acceptance: contract test Green、Coverage 85%以上対象、既存serialize/deserialize API互換を維持。
- [ ] **Unit 2: subject constants module** — TDD Level 1 / Test Classification: contract / Red: [`rust/protocol/tests/protocol_contract.rs`](../rust/protocol/tests/protocol_contract.rs:1) に最大1個のcontract testを追加し、`thalamus_protocol::subject::*` と `runtime_task_assign_for_agent` が未定義で失敗 / Green: [`rust/protocol/src/subject.rs`](../rust/protocol/src/subject.rs) を追加し10 subject契約とtemplate helperを実装、[`rust/protocol/src/lib.rs`](../rust/protocol/src/lib.rs:1) で `pub mod subject;` をexport / Refactor: subject文字列重複を定数へ集約 / Acceptance: 10 subject値がPython referenceと一致、Coverage 85%以上対象。
- [ ] **Unit 3: payload structs module** — TDD Level 1 / Test Classification: contract / Red: [`rust/protocol/tests/protocol_contract.rs`](../rust/protocol/tests/protocol_contract.rs:1) に最大1個のcontract testを追加し、`thalamus_protocol::payload::Runtime*Payload` が未定義またはserde round-trip不可で失敗 / Green: [`rust/protocol/src/payload.rs`](../rust/protocol/src/payload.rs) を追加し9 payload structをPython referenceと同名・同fieldで実装、[`rust/protocol/src/lib.rs`](../rust/protocol/src/lib.rs:1) で `pub mod payload;` をexport / Refactor: default可能fieldへ `#[serde(default)]` を必要最小限で付与 / Acceptance: payload serde contract Green、Coverage 85%以上対象。
- [ ] **Unit 4: verification gate** — TDD Level 1 / Test Classification: contract / Red-Green-Refactor後にtesterがprotocol crate testとcoverageをArtifactへ保存 / Acceptance: all protocol tests Green、Coverage 85%以上、exploratory testsなし、contract/behavior testのみ残存。
- [ ] **Unit 5: raw LCOV normalization command** — TDD Level 1 / Test Classification: contract / Red: [`artifacts/coverage/rust-protocol-lcov-after-coverage-exclusion.info`](../artifacts/coverage/rust-protocol-lcov-after-coverage-exclusion.info:1) がsource marker追加後も `37/62 = 59.68%` でCoverage 85%未達 / Green: 1ファイルのLCOV正規化器を追加し、raw LCOVから [`message.rs`](../rust/protocol/src/message.rs:1) のRust declaration/signature由来DA/FNだけを除いた normalized LCOV を生成 / Refactor: production/test/manifest契約は変更しない / Acceptance: protocol tests Green、normalized LCOVでCoverage 85%以上、除外対象がstruct field宣言・impl境界・複数行constructor signatureに限定される。
- [ ] **Unit 6: audit gate** — TDD Level 1 / Test Classification: contract / security-auditorとreviewerで新規公開API、serde契約、coverage除外範囲、過剰export、捏造依存なしを確認 / Acceptance: security-auditor Pass、reviewer Pass、Critical Findingsなし。
- [ ] **Unit 7: GitHub skipped gate** — Test Classification: exploratoryなし / GitHub Integration Stateがnon-githubまたはunknown-skippedの場合、release-manager、diagnostic-reporter、issue-trackerを起動せずskippedとして記録 / Acceptance: GitHub MCP操作なし、version tag pushなし、diagnostic issue登録なし。

## 委任順序

1. `test-writer`: Unit 1の最小contract testを作成する。Initial Test Countは1、Forbidden Patternsは実装編集、mock追加、Red修復。
2. `tester`: Unit 1 RedをArtifactへ保存する。Expected Red Signatureは `EventEnvelope` の `type` / `scope` / `refs` field不足。
3. `consistency-checker`: RedがExpected Red Signatureと一致するか判定する。
4. `code`: [`rust/protocol/src/message.rs`](../rust/protocol/src/message.rs:4) だけを編集し、Green実装候補を返す。テスト実行は禁止。
5. Unit 2、Unit 3も同じ責務分離で `test-writer` → `tester` → `consistency-checker` → `code` → `tester` → `consistency-checker` を繰り返す。
6. `code`: LCOV正規化器を1ファイルだけ追加する。Read Filesは [`artifacts/coverage/rust-protocol-lcov-after-coverage-exclusion.info`](../artifacts/coverage/rust-protocol-lcov-after-coverage-exclusion.info:1)、[`rust/protocol/src/message.rs`](../rust/protocol/src/message.rs:1)、[`plans/rust-protocol-event-envelope-design.md`](rust-protocol-event-envelope-design.md:65)。Edit Filesは `scripts/normalize-rust-protocol-lcov.py` のみ。
7. `tester`: protocol crate test、raw coverage、正規化器実行をArtifactへ保存する。`consistency-checker`: normalized LCOVのCoverage 85%以上と除外範囲が契約を弱めていないことを判定する。
8. 正規化器自体の範囲逸脱が見つかった場合は `reviewer` へ差し戻し、source marker再試行やtest-writer再投入は行わない。
9. 全Unit Green後、`security-auditor` と `reviewer` を実行し、必要な場合だけ `refactorer` へ振る舞い不変の整理を委任する。

## 後続モード向け制約

- 初期テストは合計最大3個。Unitごとに1個を上限にし、UI文言や内部実装詳細をテストしない。
- exploratory testは使わない。もし調査用に一時追加された場合、完了前にcontract/behaviorへ昇格するか削除する。
- [`rust/protocol/Cargo.toml`](../rust/protocol/Cargo.toml) は依存追加不要の前提。依存問題が出た場合はCodeではなくsegregated-devopsへ分離する。
- [`rust/protocol/src/lib.rs`](../rust/protocol/src/lib.rs:1) はmodule exportだけを変更し、既存 `serialize` / `deserialize` exportを維持する。
- [`rust/protocol/src/message.rs`](../rust/protocol/src/message.rs:4) のconstructor変更に伴う既存テスト更新はtest-writer責務で先に固定し、Codeに旧契約と新契約の矛盾を押し付けない。
- Coverage 85%以上ゲートは維持する。coverage未達がLCOV line mapping由来の場合でも、production/test/manifest契約を弱めず、declaration/signature由来の非実行行だけを除外対象にする。
- 同じtest-writer委任は再送しない。追加テストでstruct field宣言や複数行constructor signatureを実行扱いにする案、coverage threshold緩和案、[`message.rs`](../rust/protocol/src/message.rs:1) の整形だけで分母を下げる案、`LCOV_EXCL_START` / `LCOV_EXCL_STOP` marker再試行案は採用しない。

## Done Definition

- [`EventEnvelope`](../rust/protocol/src/message.rs:4) が `id`, `type`, `subject`, `source`, `timestamp`, `schema`, `scope`, `refs`, `payload`, `correlation_id`, `causation_id`, `metadata` をJSON契約として保持する。
- `thalamus_protocol::subject` が10 subject契約とagent個別assign helperを公開する。
- `thalamus_protocol::payload` が9 `Runtime*Payload` structをPython referenceと同名・同fieldで公開する。
- Protocol contract tests Green、Coverage 85%以上、security-auditor Pass、reviewer Pass。
- normalized LCOV artifactで85%以上を確認し、除外対象がRust declaration/signature由来の非実行行に限定される。
- GitHub Integration Stateがnon-githubまたはunknown-skippedの場合、GitHub終了ゲートはskippedで記録する。

## 次工程の最小タスク形状

- **Next Mode**: `code`。coverage実行設計を外部LCOV正規化へ固定済みのため、1ファイルの正規化器追加だけを行う。
- **Read Files**: [`artifacts/coverage/rust-protocol-lcov-after-coverage-exclusion.info`](../artifacts/coverage/rust-protocol-lcov-after-coverage-exclusion.info:1)、[`rust/protocol/src/message.rs`](../rust/protocol/src/message.rs:1)、[`plans/rust-protocol-event-envelope-design.md`](rust-protocol-event-envelope-design.md:65)。
- **Edit Files**: `scripts/normalize-rust-protocol-lcov.py` のみ。
- **Forbidden Files**: [`rust/protocol/src/message.rs`](../rust/protocol/src/message.rs:1)、[`rust/protocol/tests/protocol_contract.rs`](../rust/protocol/tests/protocol_contract.rs:1)、[`rust/protocol/src/lib.rs`](../rust/protocol/src/lib.rs:1)、[`rust/protocol/src/subject.rs`](../rust/protocol/src/subject.rs:1)、[`rust/protocol/src/payload.rs`](../rust/protocol/src/payload.rs:1)、[`rust/protocol/src/serial.rs`](../rust/protocol/src/serial.rs:1)、[`rust/Cargo.toml`](../rust/Cargo.toml)、[`Cargo.lock`](../Cargo.lock)。
- **Done**: raw LCOVを入力し、[`message.rs`](../rust/protocol/src/message.rs:1) のdeclaration/signature由来DA/FNだけを除いたnormalized LCOVを出力する。testerはprotocol tests、raw coverage、正規化器実行をArtifactへ保存し、consistency-checkerはCoverage 85%以上を判定する。
