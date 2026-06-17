# Rust Runtime MVP 最終整合修正計画

## 背景

新機能追加ではなく、Protocol payload contract と実際の published event JSON を一致させる。

## 問題点

### 1. RuntimeAgentErrorPayload が agent_id/task_id を型として持たない

**現状** (payload.rs:98-101):
```rust
pub struct RuntimeAgentErrorPayload {
    #[serde(default)]
    pub error: Value,
}
```

**修正後**:
```rust
pub struct RuntimeAgentErrorPayload {
    pub agent_id: String,
    pub task_id: String,
    #[serde(default)]
    pub error: Value,
}
```

### 2. RuntimeLLMRequestPayload / RuntimeToolRequestPayload に request_id がない

**方針**: request_id を request/response payload 型に明示的に追加する。

**RuntimeLLMRequestPayload 修正後**:
```rust
pub struct RuntimeLLMRequestPayload {
    pub task_id: String,
    pub request_id: Option<String>,
    pub model: Option<String>,
    pub prompt: Option<String>,
    #[serde(default)]
    pub messages: Vec<Value>,
    #[serde(default)]
    pub options: Value,
    pub correlation_id: Option<String>,
}
```

**RuntimeToolRequestPayload 修正後**:
```rust
pub struct RuntimeToolRequestPayload {
    pub task_id: String,
    pub request_id: Option<String>,
    pub capability: String,
    pub input: Value,
    pub timeout_seconds: Option<u64>,
    pub correlation_id: Option<String>,
}
```

### 3. README.md の構成図に rust/ がない

**現状** (README.md:57-74):
```text
Human
 ↓
AI Agent
 ↓
Cingulater
(Shared Cognition Plane)
 ↓
Thalamus Runtime
(Event-driven Cognitive Coordination Layer)
 ↓
Sandboxed Runtime Workers
 ↓
mcp-routing-gateway
(Capability Plane)
 ↓
MCP Servers
```

**修正後**:
```text
Human
 ↓
AI Agent
 ↓
Cingulater
(Shared Cognition Plane)
 ↓
Thalamus Runtime
(Event-driven Cognitive Coordination Layer)
 ↓
Rust Runtime / Python Runtime
 ↓
Sandboxed Runtime Workers
 ↓
mcp-routing-gateway
(Capability Plane)
 ↓
MCP Servers
```

### 4. docs/rust-runtime-design.md のリンク確認

既存のリンクはすべて `rust/` 配下のファイルへの相対リンクであり、存在しない artifact リンクはない。

## 修正ファイル一覧

| ファイル | 変更内容 |
|----------|----------|
| `rust/protocol/src/payload.rs` | RuntimeAgentErrorPayload に agent_id/task_id 追加、RuntimeLLMRequestPayload/RuntimeToolRequestPayload に request_id 追加 |
| `rust/runtime/src/lib.rs` | publish_default_runtime_result() で request_id を使用 |
| `README.md` | 構成図に Rust Runtime を追加 |
| `docs/rust-runtime-design.md` | リンク確認後、必要に応じて修正 |
| `rust/protocol/tests/protocol_contract.rs` | payload 整合性テスト追加 |
| `rust/runtime/tests/runtime_basic.rs` | request_id テスト追加 |

## TDD 方針

- Level 1 Contract Test: payload 型の field 整合性
- Level 2 Behavior Test: publish_default_runtime_result() の request_id 動作

## 品質ゲート

- Coverage 85% 以上
- cargo fmt / clippy --all-targets -- -D warnings 通過
- security-auditor / reviewer 通過
