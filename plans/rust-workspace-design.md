# Thalamus Rust Workspace Design

## 目的

Thalamusのprotocol、bus、runtime、cli、sdkをRustで実装し、Cargoワークスペースとして統合する。Python版の設計（[`plans/thalamus-runtime-reference-design.md`](thalamus-runtime-reference-design.md:1)）を参照し、canonical event envelope、subject-based routing、pub/sub bus、agent lifecycle、tool mediation、mock LLM pathをRustで再実装する。

## Index Probe

- query: Rust Thalamus workspace protocol bus runtime cli sdk cargo workspace serde tokio pubsub message envelope
- path: rust
- 主要候補: 新規作成（rust/ 配下未存在）

## ワークスペース構造

```
rust/
├── Cargo.toml          # ワークスペースルート
├── protocol/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── message.rs  # メッセージ型定義
│       └── serial.rs   # シリアライゼーション
├── bus/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── pubsub.rs   # Pub/Sub実装
│       └── router.rs   # ルーティング
├── runtime/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── scheduler.rs # タスクスケジューリング
│       └── lifecycle.rs # ライフサイクル
├── cli/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── lib.rs       # CLI定義とコマンド集約
└── sdk/
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        └── bindings.rs # 外部言語用バインディングAPI
```

## クレート責任とインターフェース

### `rust/protocol`

- **責任**: プロトコル定義（メッセージ型、シリアライゼーション）。canonical event envelopeのRust型定義。
- **依存**: `serde`, `serde_json`
- **公開API**:
  - `pub struct EventEnvelope { id: String, subject: String, source: String, timestamp: String, schema: String, payload: serde_json::Value, correlation_id: Option<String>, causation_id: Option<String>, metadata: serde_json::Value }`
  - `pub fn serialize(envelope: &EventEnvelope) -> Result<String, ProtocolError>`
  - `pub fn deserialize(s: &str) -> Result<EventEnvelope, ProtocolError>`
  - `pub enum ProtocolError { SerializationError, DeserializationError, InvalidEnvelope }`

### `rust/bus`

- **責任**: メッセージバス（pub/sub、ルーティング）。subject-based routingとhandler登録。
- **依存**: `protocol`, `tokio`, `tokio-sync`
- **公開API**:
  - `pub struct EventBus { /* ... */ }`
  - `impl EventBus { pub fn new() -> Self, pub async fn subscribe(&mut self, subject: String, handler: Handler) -> Result<SubscriptionId, BusError>, pub async fn publish(&self, envelope: EventEnvelope) -> Result<(), BusError>, pub async fn close(&self) }`
  - `pub type Handler = Box<dyn Fn(EventEnvelope) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;`
  - `pub enum BusError { NotFound, AlreadySubscribed, Closed }`

### `rust/runtime`

- **責任**: ランタイム（タスクスケジューリング、ライフサイクル）。`MessageBus` 上のhandler登録、runtime state遷移、publish/handle_event dispatch、spawn済みtaskの追跡を担当する。agent lifecycle、tool mediation、mock LLM pathは後続Unitで拡張する。
- **依存**: `protocol`, `bus`, `tokio`
- **公開API**:
  - `pub struct ThalamusRuntime { bus: EventBus, /* ... */ }`
  - `impl ThalamusRuntime { pub fn new(bus: EventBus) -> Self, pub async fn start(&mut self) -> Result<(), RuntimeError>, pub async fn stop(&mut self) -> Result<(), RuntimeError>, pub async fn spawn<F>(&self, future: F) -> TaskHandle, pub async fn publish(&self, subject: String, source: String, payload: serde_json::Value) -> Result<EventEnvelope, RuntimeError>, pub async fn handle_event(&self, subject: String, event: EventEnvelope) -> Result<(), RuntimeError>, pub async fn active_task_count(&self) -> usize }`
  - `pub enum RuntimeError { BusError, ScheduleError, LifecycleError }`
- **Unit 4完了状態**: `start()` は登録済みhandlerをbusへsubscribeして `RuntimeState::Running` へ遷移し、二重起動を `LifecycleError` として拒否する。`stop()` はrunning状態だけを停止対象にし、busをcloseして `RuntimeState::Stopped` へ遷移する。`spawn()` は `TaskHandle` を返して実行中taskだけを追跡し、完了済みtaskは `active_task_count()` から除外する。`publish()` はsubject/source/payloadから `EventEnvelope` を生成してbusへpublishし、`handle_event()` は登録済みhandlerへdispatchし、未登録subjectは `ScheduleError` を返す。

### `rust/cli`

- **責任**: CLIツール（コマンドラインインターフェース）。コマンドパースとruntime操作。
- **依存**: `clap`, `thiserror`, `tokio`
- **公開API**:
  - `pub struct ThalamusCLI { verbose: bool, command: CLICommand }`
  - `pub enum CLICommand { Start { config: String }, Stop, Status, ListAgents }`
  - `pub enum CliError { ParseError(String), RuntimeError(String), IoError(std::io::Error) }`
  - `impl ThalamusCLI { pub fn new() -> Self, pub async fn run(&self) -> Result<(), CliError> }`
- **Entrypoint**: `rust/cli/src/main.rs` の `#[tokio::main] async fn main()` が `ThalamusCLI::new()` で引数をparseし、`cli.run().await` の失敗時にstderr出力と終了コード1を返す。
- **Unit 5完了状態**: CLIの型定義、subcommand、error型、実行分岐は `rust/cli/src/lib.rs` に集約済み。`commands.rs` 分割は現行実装では採用していない。

### `rust/sdk`

- **責任**: SDK（外部言語用バインディング用API）。FFIまたはgRPC経由の外部連携。
- **依存**: `protocol`, `tokio`
- **公開API**:
  - `pub extern "C" fn thalamus_publish(subject: *const c_char, source: *const c_char, payload: *const c_char) -> c_int`
  - `pub extern "C" fn thalamus_subscribe(subject: *const c_char, handler: extern "C" fn(*const c_char)) -> c_int`
  - `pub extern "C" fn thalamus_shutdown()`
- **FFI callback payload契約**: `thalamus_subscribe()` の `handler` に渡すpayload pointerはNUL終端された非nullのC文字列であり、callback呼出中だけ有効。callback後に呼び出し側がpointerを保持・再利用してはいけない。

## データ構造

### EventEnvelope（canonical event envelope）

```rust
pub struct EventEnvelope {
    pub id: String,
    pub subject: String,
    pub source: String,
    pub timestamp: String,
    pub schema: String,
    pub payload: serde_json::Value,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub metadata: serde_json::Value,
}
```

### Subject Patterns

- `runtime.agent.ready`
- `runtime.agent.exit`
- `runtime.agent.error`
- `runtime.task.assign`
- `runtime.task.assign.<agent_id>`
- `runtime.tool.request`
- `runtime.tool.result`
- `runtime.llm.request`
- `runtime.llm.response`

## 設計原則

1. canonical event envelopeを全eventの唯一の外部契約にする。
2. busはin-memory pub/subを基本とし、外部NATSは非対象（将来的な拡張用インターフェースのみ）。
3. payload modelはserdeで検証し、validatorはsubject別schemaを選択する。
4. runtimeはeventbus上のsubjectとevent envelopeでagent、task、tool、LLMを接続する。
5. 最小実装では実LLM、実MCP、分散NATS、永続化は行わない。

## 品質ゲート結果

- **Unit 4 runtime**: runtime basic tests Green（`9 passed; 0 failed`）。
- **Unit 5 CLI**: cli contract tests Green。`ThalamusCLI`、`CLICommand`、`CliError`、`#[tokio::main] async fn main()` の現行構成と一致。
- **Coverage**: Unit 5 CLIは `33/36`、91.67%。Coverage 85%以上を満たす。
- **Security / Review**: Unit 5 CLIはsecurity-auditor Pass、reviewer Pass。
- **Unit 6 SDK**: unit 5 testsとSDK contract tests Green（`unit 5 tests + contract 3 tests passed`）。FFI callback payload契約テストもpass。
- **Coverage**: Unit 6 SDKは `107/122`、87.7%。Coverage 85%以上を満たす。
- **Security / Review**: Unit 6 SDKはsecurity-auditor Pass、reviewer Pass。Critical Findingsなし。
- **FFI callback payload契約**: `thalamus_subscribe()` のcallback payload pointerはNUL終端・非nullで、callback呼出中だけ有効。
- **GitHub Integration State**: non-github。version tag pushとdiagnostic issue登録はskipped。

## GitHub終了ゲート

- **GitHub Integration State**: non-github
- **release-manager**: skipped（version tag pushなし）
- **diagnostic-reporter**: skipped（diagnostic issue登録なし）
