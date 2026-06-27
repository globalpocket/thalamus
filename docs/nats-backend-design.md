# NATS Backend Design

## 目的

NATS backendは`MessageBus`の分散実装である。

- **BasicBus**: local deterministic in-memory bus（デフォルト）
- **NatsBus**: optional distributed transport backend behind `nats` feature
- NatsBusはruntime間event propagation用

## 非目標

- JetStreamなし
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
    pub url: String,
}

#[derive(Clone)]
pub struct NatsBus {
    // async_nats client
    // subscription task handles
    // closed flag
}
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
- client自体のclose APIがある場合は呼ぶ
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

- **subscribe**: subscribeでNATS subscription taskをspawn
- **unsubscribe**: unsubscribeでtask abort
- **close**: closeで全task abort

## Runtime integration

- `ThalamusRuntime<B: MessageBus>`はgenericなのでNatsBusをそのまま使える
- Runtime core semanticsは変えない
- `Runtime::publish()`のvalidation/normalizationは既存通り
- NatsBusはvalidated envelopeをtransportするだけ

## NATS未接続時のtest方針

- CIにNATS serverを必須にしない
- `THALAMUS_NATS_TEST_URL`が未設定ならskipして成功扱い
- 設定されている場合のみconnectしてround-tripを実行

## Future work

- JetStream backend
- durable consumers
- delivery ack
- replay
- cluster topology
- routing policy
- observability/tracing
