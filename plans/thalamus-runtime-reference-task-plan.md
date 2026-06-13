# Thalamus Runtime Reference Task Plan

## 前提

- Goal: Python版Thalamus Runtimeをprotocol準拠の最小reference implementationへ修正する。
- Design: [`plans/thalamus-runtime-reference-design.md`](thalamus-runtime-reference-design.md)
- GitHub Integration State: github、owner/repo: `globalpocket/thalamus`
- Read Scope: 現在のワークスペース配下のみ。
- 非対象: ZooCodeCustom、Cingulater本体、Rust実装、実MCP gateway、実LLM provider、NATS分散運用、永続化。
- 共通Quality Gate: 各実装単位でRed-Green-Refactorを維持し、最終的にCoverage 85%以上、security-auditor Pass、reviewer Passを必須にする。
- 初期テスト数: 各軽量TDD単位で最大3個。Level 4 Full TDDを初手にしない。
- Exploratory Handling: exploratory testを使う場合は完了時にcontract / behavior / regressionへ昇格、または削除する。

## Index Probe

- query: Python Thalamus Runtime protocol reference implementation canonical event envelope dict EventBus validator payload models agent registry flow tool mediation mock LLM task state examples README pydantic dependency
- path: workspace root
- 主要候補: [`runtime/events/types.py`](../runtime/events/types.py:11), [`runtime/runtime.py`](../runtime/runtime.py:6), [`README.md`](../README.md:378)
- 後続候補: [`runtime/events/publisher.py`](../runtime/events/publisher.py), [`runtime/bus/interface.py`](../runtime/bus/interface.py), [`runtime/events/validator.py`](../runtime/events/validator.py), [`runtime/registry/registry.py`](../runtime/registry/registry.py), [`examples/simple-task-flow/`](../examples/simple-task-flow/)

## 軽量TDDチェックリスト

- [x] **Unit 1: canonical event envelope contract** — TDD Level 1 / Test Classification: contract / Red: `RuntimeEvent` またはpublisher生成eventが `id`, `type`, `subject`, `source`, `timestamp`, `schema`, `payload`, `correlation_id`, `causation_id`, `metadata` を満たさず失敗するテストを最大3個作成 / Green: [`runtime/events/types.py`](../runtime/events/types.py:11) とpublisherを最小修正 / Refactor: envelope生成重複を整理 / Acceptance: event envelope contractが通り、Coverage 85%以上対象、security-auditorとreviewerで契約逸脱なし。
- [x] **Unit 2: subject-based payload validator** — TDD Level 1 / Test Classification: contract / Red: `runtime.tool.request`, `runtime.llm.request`, `runtime.agent.error` のpayload model不足で失敗するテストを最大3個作成 / Green: [`runtime/events/types.py`](../runtime/events/types.py:11) と [`runtime/events/validator.py`](../runtime/events/validator.py) にsubject別modelを追加 / Refactor: subject-to-model registryを単一表にする / Acceptance: 既存task/agent payloadも維持し、Coverage 85%以上対象、未知subjectの扱いが明示される。
- [x] **Unit 3: in-memory dict EventBus** — TDD Level 1 / Test Classification: contract / Red: NATSなしでpublish/subscribe履歴と完全一致subject dispatchができず失敗するテストを最大3個作成 / Green: [`runtime/bus/interface.py`](../runtime/bus/interface.py) 周辺にreference busを追加しRuntimeへ注入可能にする / Refactor: NATS adapterとreference busの共通interfaceを整理 / Acceptance: 外部NATS不要でunit test可能、Coverage 85%以上対象、NATS分散互換の過剰実装なし。
- [x] **Unit 4: agent.ready/exit/error registry flow** — TDD Level 2 / Test Classification: behavior / Red: [`ThalamusRuntime`](../runtime/runtime.py:6) が `runtime.agent.ready`, `runtime.agent.exit`, `runtime.agent.error` でregistry stateを更新できず失敗するテストを最大3個作成 / Green: register/unregister subjectをprotocol lifecycle subjectへ置換または互換化 / Refactor: registry state更新APIを `ready/exited/error` に整理 / Acceptance: agent lifecycleがprotocol subjectに準拠し、Coverage 85%以上対象、旧 `register/unregister` への依存が残る場合は互換理由を明記。
- [x] **Unit 5: task state transition** — TDD Level 2 / Test Classification: behavior / Red: `runtime.task.assign` から `assigned -> running -> completed/failed` を追跡できず失敗するテストを最大3個作成 / Green: Runtimeにtask state storeを追加しresult/errorで更新 / Refactor: task state enumまたはliteralを一箇所へ集約 / Acceptance: direct addressing `runtime.task.assign.<agent_id>` を壊さず、Coverage 85%以上対象、永続化は非対象。
- [x] **Unit 6: runtime-mediated tool request/result** — TDD Level 2 / Test Classification: behavior / Red: `runtime.tool.request` がToolRegistryを通らず `runtime.tool.result` をpublishできないテストを最大3個作成 / Green: dict-based ToolRegistryと未登録capability error resultを実装 / Refactor: request_id/correlation_id伝播を共通化 / Acceptance: workerが外部toolを直接呼ばない設計を満たし、Coverage 85%以上対象、実MCP gatewayは非対象。
- [x] **Unit 7: mock LLM request/response path** — TDD Level 2 / Test Classification: behavior / Red: `runtime.llm.request` がmock provider経由で `runtime.llm.response` をpublishできず失敗するテストを最大3個作成 / Green: deterministic MockLLMProviderをRuntimeへ注入 / Refactor: LLM provider interfaceを最小化 / Acceptance: 実Cingulaterや外部HTTPなしで動作し、Coverage 85%以上対象、responseはrequest_id/correlation_idを保持。
- [x] **Unit 8: simple-task-flow example** — TDD Level 2 / Test Classification: behavior / Red: [`examples/simple-task-flow/`](../examples/simple-task-flow/) がreference runtime flowを示せず失敗または実行不能になるテストを最大3個作成 / Green: exampleをin-memory runtime、agent.ready、task.assign、mock LLM/tool、task.resultの流れへ更新 / Refactor: example固有の重複fixtureを削減 / Acceptance: examplesは外部NATS/実LLM不要、Coverage 85%以上対象に含めるかsmokeとして扱う範囲を明記。
- [x] **Unit 9: README Status and dependency alignment** — TDD Level 1 / Test Classification: contract / Red: [`README.md`](../README.md:378) のStatusと [`pyproject.toml`](../pyproject.toml) の `pydantic` 依存がreference runtime状態を表さず失敗する文書/manifest確認を最大3個作成 / Green: README Statusをminimal reference implementedへ更新し、`pydantic` 依存を維持または不足時に追加 / Refactor: READMEの未実装一覧を実態に合わせて整理 / Acceptance: READMEがLLM/tool最小実装と非対象を明記し、Coverage 85%以上・security-auditor・reviewer後に文書不整合なし。

## Reviewer再監査後の残Critical修正フロー

- [x] **Critical Unit A: task assign canonical payload adoption** — TDD Level 2 / Test Classification: behavior / Red: `runtime.task.assign` のpayloadを `RuntimeTaskAssignPayload(task_id, agent_id, input, capabilities, metadata)` だけにして [`ThalamusRuntime.handle_task_assign()`](../runtime/runtime.py:82) がtop-level `objective` を要求しないことを検証する失敗テストを最大2個作成 / Green: handlerは `input` または `metadata` から任意目的を保存し、`RuntimeTaskAssignPayload` へ `objective` を戻さない / Refactor: raw task互換の目的取得を小関数へ隔離 / Acceptance: canonical payloadが一意、Coverage 85%以上対象、reviewerのCritical 1再発なし。
- [x] **Critical Unit B: task state assigned-first transition** — TDD Level 2 / Test Classification: behavior / Red: `runtime.task.assign` 直後は `assigned`、tool/LLM mediation開始で `running`、result/errorで `completed` または `failed` へ進むことを検証する失敗テストを最大3個作成 / Green: assign handlerの即時running化をやめ、mediation開始点でのみrunning化 / Refactor: state literalを単一箇所へ集約 / Acceptance: direct addressingを壊さず、Coverage 85%以上対象、reviewerのCritical 2再発なし。
- [x] **Critical Unit C: tool result canonical envelope** — TDD Level 1 / Test Classification: contract / Red: `runtime.tool.result` が `id`, `type`, `subject`, `source`, `timestamp`, `schema`, `payload`, `correlation_id`, `causation_id`, `metadata` を持つfull envelopeで発行されない失敗テストを最大2個作成 / Green: [`ThalamusRuntime.handle_tool_request()`](../runtime/runtime.py:187) は `EventPublisher.publish()` 経由でresultを発行 / Refactor: request-response envelope生成を共通化 / Acceptance: payloadだけでなくenvelope全体を検証し、Coverage 85%以上対象、reviewerのCritical 3のtool側再発なし。
- [x] **Critical Unit D: LLM response canonical envelope** — TDD Level 1 / Test Classification: contract / Red: `runtime.llm.response` がfull canonical envelopeで発行されない失敗テストを最大2個作成 / Green: [`ThalamusRuntime.handle_llm_request()`](../runtime/runtime.py:251) は `EventPublisher.publish()` 経由でresponseを発行 / Refactor: tool responseと同じenvelope helperを使う / Acceptance: `correlation_id` と `causation_id` を保持し、Coverage 85%以上対象、reviewerのCritical 3のLLM側再発なし。
- [x] **Critical Unit E: quality gates after Critical A-D** — TDD Level 1 / Test Classification: contract / Red-Green: 新規テスト作成なし。testerがscoped testsとCoverageをArtifactへ保存し、consistency-checkerがGreenとCoverage 85%以上を判定 / Refactor: exploratory testが残れば正式testへ昇格または削除 / Acceptance: Coverage 85%以上、security-auditor Pass、reviewer Pass、Critical 1から3が再発しない。
- [x] **Critical Unit F: README/design alignment after reviewer Pass** — TDD Level 1 / Test Classification: contract / Red: README Statusまたは設計書がcanonical payload、assigned-first state、Publisher経由response envelopeとずれる文書確認を最大2個に限定 / Green: technical-writerが [`README.md`](../README.md) だけを最小更新 / Refactor: 重複説明を削除 / Acceptance: READMEが実装済み契約と非対象を明記し、Coverage 85%以上・security-auditor・reviewer後に文書不整合なし。

## GitHub終了ゲートチェックリスト

- [ ] **Release ManagerによるVersion Tag Push** — GitHub終了ゲートとしてversion/tag pushを実行する。Python projectではstaging対象を[`pyproject.toml`](../pyproject.toml)のみに限定し、Node.js manifest（`package.json`, `package-lock.json`, `npm-shrinkwrap.json`）を必須引数にする固定許可コマンドしか使えない場合はcommand-policy conflictとしてblock停止し、Diagnostic Issue登録へ進めない。
- [ ] **Diagnostic Issue登録** — GitHub終了ゲートとして診断Issueを日本語で登録する。

## 実行順序

1. test-writer: Critical Unit Aのbehavior Redテストを作成する。
2. tester: Critical Unit A RedをArtifact Pathへ保存する。
3. consistency-checker: Expected Red Signature一致を判定する。
4. code: Critical Unit AをGreen化する。
5. tester + consistency-checker: GreenとCoverage 85%以上を確認する。
6. Critical Unit BからUnit Dまで同じ責務分離で反復し、payload、state、tool envelope、LLM envelopeを個別にGreen化する。
7. Critical Unit Eとしてtester + consistency-checkerでscoped tests、Coverage 85%以上、test inventoryを確認する。
8. security-auditor: Critical修正後にsecret、unsafe external execution、捏造依存を再監査する。
9. reviewer: 設計書との整合、TDD inventory、exploratory test残存なし、Critical 1から3の解消を再監査する。
10. technical-writer: reviewer Pass後、README更新が実装差分とずれる場合だけCritical Unit Fとして最小修正する。
11. release-manager: GitHub終了ゲートとしてversion/tag pushを実行する。Python project stagingは[`pyproject.toml`](../pyproject.toml)のみを許可し、固定許可コマンドがNode.js manifest pathspecを要求してPython単独stagingできない場合はblockとして停止する。
12. diagnostic-reporter: release-managerがVersion/Tag pushを完了した場合だけ、GitHub終了ゲートとして診断Issueを日本語で登録する。

## 後続モード委任の注意

- test-writerへは各UnitごとにInitial Test Count最大3、Exact Imports、Allowed Test Doubles、Forbidden Patterns、Expected Red Signatureを渡す。
- testerへは必ずArtifact Pathを準備してから実行コマンドだけを渡す。
- codeへは検証コマンド実行を含めず、Edit Filesを1から2件に制限する。
- dependency問題、coverage provider不足、`pydantic` 未導入が出た場合はcodeではなくsegregated-devopsへ分離する。
- GitHub関連操作はOrchestratorが直接行わず、release-manager、diagnostic-reporter、必要時issue-trackerに分離する。
