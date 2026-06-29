# NATS Backend Design

## 目的

NATS backendは`MessageBus`の分散実装である。

- **BasicBus**: local deterministic in-memory bus（デフォルト）
- **NatsBus**: optional distributed transport backend behind `nats` feature
- NatsBusはruntime間event propagation用

## 非目標

- durable consumerなし
- ack/retry/replayなし
- persistenceなし
- exactly-once deliveryなし
- workflow engine化なし

## フィーチャ設定

`thalamus-bus` クレートで、`nats` フィーチャを有効化すると `NatsBus` が利用可能になる。

```toml
[features]
default = []
nats = ["dep:async-nats", "dep:futures"]

[dependencies]
async-nats = { version = "0.37", optional = true }
futures = { version = "0.3", optional = true }
```

使用例:

```toml
[dependencies]
thalamus-bus = { version = "0.1", features = ["nats"] }
```

## NatsBus 構造体

```rust
#[derive(Clone, Debug)]
pub struct NatsBusConfig {
    /// NATS server URL (default: "nats://127.0.0.1:4222")
    pub url: String,
}

/// Internal state for NatsBus
struct NatsBusState {
    client: Option<async_nats::Client>,
    sub_handles: HashMap<String, tokio::task::JoinHandle<()>>,
    subject_handler_counts: HashMap<String, usize>,
    closed: bool,
}

#[derive(Clone)]
pub struct NatsBus {
    state: Arc<RwLock<NatsBusState>>,
    url: String,
}
```

### Re-exports
`NatsBus` and `NatsBusConfig` are re-exported from the crate root when the `nats` feature is enabled.
```

### NatsBusConfig

```rust
impl Default for NatsBusConfig {
    fn default() -> Self {
        Self {
            url: "nats://127.0.0.1:4222".to_string(),
        }
    }
}
```

### NatsBus API

```rust
impl NatsBus {
    pub async fn connect(config: NatsBusConfig) -> Result<Self, BusError>;
    pub async fn connect_url(url: impl Into<String>) -> Result<Self, BusError>;
    pub async fn is_closed(&self) -> bool;
}
```

## MessageBus トレイト実装

`NatsBus` は `MessageBus` トレイトを実装する。

### subscribe

```rust
async fn subscribe(
    &self,
    subject: String,
    handler: Handler,
) -> Result<SubscriptionId, BusError>;
```

- busがclosedなら`BusError::Closed`を返す
- NATS subjectへsubscribeする
- subscriptionごとに受信loopをTokio taskとしてspawnする
- NATS message payloadを`EventEnvelope`へdeserializeする
- deserializeに失敗したmessageはdropする
- handlerを`handler(envelope).await`で実行する
- `SubscriptionId`を返す
- subscription idとtask handleを内部mapに保存する

### publish

```rust
async fn publish(&self, envelope: EventEnvelope) -> Result<(), BusError>;
```

- busがclosedなら`BusError::Closed`
- `EventEnvelope`をJSON serializeする
- NATS subjectは`envelope.subject`
- `client.publish(subject, payload.into()).await`
- publish成功で`Ok(())`

### unsubscribe

```rust
async fn unsubscribe(&self, id: SubscriptionId) -> Result<(), BusError>;
```

- 該当subscription taskをabortする
- mapから削除する
- なければ`BusError::NotFound`

### close / is_closed

```rust
async fn close(&self);
async fn is_closed(&self) -> bool;
```

- `close()` は closed flagをtrueにし、全subscription taskをabortし、subscription mapをclearする
- `client.close().await` を呼び出してNATSクライアントもcloseする（ロック解放後）
- `is_closed()` は現在のclosed状態を返す

### handler_count

```rust
async fn handler_count(&self, subject: &str) -> usize;
```

- NATS server側の購読数ではなく、この`NatsBus`インスタンスが保持するlocal subscription mapから数える
- BasicBusと同じテスト用途の観測APIとして扱う

## Delivery semantics

- **at-most-once**: 配信保証はat-most-once
- **subject**: subject = envelope.subject
- **payload**: payload = serialized EventEnvelope JSON（全体をシリアライズ）
- **handler execution**: handler executionはlocal taskでawait
- **deserialize failure**: deserialize failureはdrop

## Subscription lifecycle

### Ownership model

- `NatsBus` は subscriber stream を受信 task が所有する
- subscribe 時に NATS subscription を作成し、Tokio task を spawn する
- task は `subscriber.next().await` でメッセージを受信し続ける

### State structure

- `NatsBusState` は subscription metadata と `JoinHandle<()` を保持する
- `sub_handles: HashMap<String, tokio::task::JoinHandle<()>>` で subscription id から handle を参照する
- `subject_handler_counts: HashMap<String, usize>` で subject ごとの handler 数を local metadata として管理する

### Unsubscribe and close

- `unsubscribe()`: 該当 subscription の `JoinHandle` に対して `handle.abort()` を呼び出し、task を中止する
- `close()`: 全 subscription の `JoinHandle` に対して `handle.abort()` を呼び出し、subscription map を drain する
- `close()` は `client.close().await` を呼び出してNATSクライアントもcloseする（ロック解放後）
- unsubscribe/close は task abort で subscription lifecycle を終了する

### handler_count

- `handler_count()`: `subject_handler_counts` から local metadata で数える
- NATS server 側の購読数ではなく、この `NatsBus` インスタンスが保持する local subscription map から数える
- BasicBus と同じテスト用途の観測 API として扱う

## Runtime integration

- `ThalamusRuntime<B: MessageBus>`はgenericなのでNatsBusをそのまま使える
- Runtime core semanticsは変えない
- `Runtime::publish()`のvalidation/normalizationは既存通り
- NatsBusはvalidated envelopeをtransportするだけ

## NATS未接続時のtest方針

- CIにNATS serverを必須にしない
- `THALAMUS_NATS_TEST_URL`が未設定ならskipして成功扱い
- 設定されている場合のみconnectしてround-tripを実行
- 複数メッセージの round-trip テストと subscribe/unsubscribe サイクルテストを追加

## Future work

- durable consumers
- delivery ack
- replay
- cluster topology
- routing policy
- observability/tracing
