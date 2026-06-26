# 実施済み: Start/Stopがdaemon lifecycleではないことの文書化

---

**status**: implemented
**last_updated**: 2026-01-01

---

> **メモ**: この計画は実施済みです。`start()`と`stop()`メソッドがdaemon lifecycleではないことの文書化、および既存CIワークフローの成功確認が完了しています。

## 1. タスク概要

`rust/runtime`の`start()`と`stop()`メソッドが**daemon lifecycle（永続的なデーモンプロセスの起動/停止）ではない**ことを文書化し、既存のCIワークフロー（cargo fmt/check/clippy/test）が成功することを確認する。

## 2. 現状分析

### 2.1 docs/ディレクトリの現状

- [`docs/rust-runtime-design.md`](rust-runtime-design.md): Rust Runtime MVPの設計ドキュメント
  - `start()`/`stop()`の言及は155-156行目、164-174行目に限定的
  - 「daemon lifecycle」に関する言及は現状なし

### 2.2 Cargo.tomlの確認

- ルート`Cargo.toml`は存在せず、`rust/Cargo.toml`がワークスペースmanifest
- 5つのcrateで構成: `protocol`, `bus`, `runtime`, `cli`, `sdk`
- resolver = "2"（Rust 2021 edition）

### 2.3 .github/workflows/の現状

- [`rust.yml`](../.github/workflows/rust.yml): 4つのjob（fmt, check, clippy, test）
- 各jobで`cd rust && cargo ...`を実行
- `all-features`フラグを使用

### 2.4 Rustプロジェクト構造

| Crate | 種別 | 説明 |
|-------|------|------|
| `thalamus-protocol` | Library | イベントエンベロープ、ペイロード、サブジェクト定義 |
| `thalamus-bus` | Library | インメモリPub/Subバス |
| `thalamus-runtime` | Library | ランタイムライフサイクル、タスク追跡、プロバイダー/ツール媒介 |
| `thalamus-cli` | Binary | CLIコマンドサーフェス（clap使用） |
| `thalamus-sdk` | Library | FFI/SDKスケルトン |

### 2.5 start()/stop()実装の詳細

#### [`start()`](rust/runtime/src/runtime.rs:236)
- 内部ハンドラーを全canonical subjectに対して登録
- 状態を`Initialized` → `Starting` → `Running`へ遷移
- 既に`Running`状態の場合、`LifecycleError`を返す（単一起動保証）
- **デーモンプロセスはspawnしない**

#### [`stop()`](rust/runtime/src/runtime.rs:645)
- 内部ハンドラーをunsubscribeし、busをclose
- 状態を`Running` → `Stopping` → `Stopped`へ遷移
- 既に`Stopped`または`Stopping`の場合、`LifecycleError`を返す
- **デーモンプロセスは終了しない（busをcloseするのみ）**

#### CLIコマンドの扱い
- [`CLICommand::Start`](rust/cli/src/lib.rs:33): `BasicBus`を作成し`runtime.start()`を呼ぶのみ
- [`CLICommand::Stop`](rust/cli/src/lib.rs:84): 単に`"Runtime stopped"`をprintし、実際のstop処理は行わない（idempotent）
- [`CLICommand::Status`](rust/cli/src/lib.rs:93): 新規`BasicBus`を作成し`"Runtime status: initialized"`をprintするのみ

### 2.6 結論: Start/Stopはdaemon lifecycleではない

1. `start()`は内部ハンドラー登録と状態遷移のみ（デーモンspawnなし）
2. `stop()`はbus closeと状態遷移のみ（デーモン終了なし）
3. CLIコマンドは永続的なプロセスを管理しない（各コマンドが独立したスコープを持つ）
4. これは**イベント処理パイプラインの起動/停止**であり、デーモン lifecycleではない

## 3. 実装計画（3ステップ）

### ステップ1: 現状分析ドキュメントの更新

**ファイル**: [`docs/rust-runtime-design.md`](rust-runtime-design.md)

**変更内容**:
- 「Lifecycle」セクション（153-156行目）を拡張
- 「Start/Stopはdaemon lifecycleではない」ことを明示的に文書化
- CLIコマンドの非永続的な性質を記載

**追加セクションアウトライン**:
```markdown
## Lifecycle Clarification

### Start/Stop are NOT Daemon Lifecycle

The `start()` and `stop()` methods do NOT implement daemon lifecycle management:

- `start()` registers internal handlers and transitions state to Running. It does NOT spawn a daemon process.
- `stop()` unsubscribes internal handlers, closes the bus, and transitions state to Stopped. It does NOT terminate a daemon process.
- CLI commands (`start`, `stop`, `status`, `list-agents`) operate within isolated scopes and do NOT manage persistent processes.

This is an event pipeline start/stop, not a daemon lifecycle.
```

### ステップ2: 計画ドキュメントの作成

**新規ファイル**: `docs/plans/start-stop-not-daemon-lifecycle-plan.md`

**内容**:
- この計画書自体
- 現状分析、変更範囲、検証コマンド、完了条件

### ステップ3: 検証コマンドの実行と成功確認

**実行コマンド**（既存CIワークフローと同一）:
```bash
cd rust
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

**完了条件**:
- 4つのコマンド全てがexit code 0で完了
- 既存ワークフロー(`rust.yml`)と同一のコマンドを使用

## 4. 推奨されるファイルパス

| タイプ | パス | 説明 |
|--------|------|------|
| 更新 | `docs/rust-runtime-design.md` | Lifecycleセクションの拡張 |
| 新規 | `docs/plans/start-stop-not-daemon-lifecycle-plan.md` | 計画書 |
| 参照 | `.github/workflows/rust.yml` | 既存CIワークフロー |
| 参照 | `rust/runtime/src/runtime.rs:236` | start()実装 |
| 参照 | `rust/runtime/src/runtime.rs:645` | stop()実装 |
| 参照 | `rust/cli/src/lib.rs:68` | CLI run()実装 |

## 5. 制約事項

- 既存のワークフローを尊重し、破壊的変更は避ける
- `rust.yml`の4 job構成は維持
- 計画書のみを作成し、実際のファイル変更は別タスクで実行

## 6. 関連コード箇所

| 箇所 | ファイル | 行番号 |
|------|----------|--------|
| start()実装 | `rust/runtime/src/runtime.rs` | 236-640 |
| stop()実装 | `rust/runtime/src/runtime.rs` | 645-674 |
| CLICommand定義 | `rust/cli/src/lib.rs` | 31-46 |
| CLI run()実装 | `rust/cli/src/lib.rs` | 68-230 |
| RuntimeState列挙型 | `rust/runtime/src/state.rs` | 7-18 |
| lifecycleテスト | `rust/runtime/tests/lifecycle.rs` | 1-420 |
