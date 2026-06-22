use async_trait::async_trait;
use serde_json::Value;

use crate::error::RuntimeError;

/// Tool: エージェントが呼び出す操作のトレイト
#[async_trait]
pub trait Tool: Send + Sync {
    /// ツール名を返す
    fn name(&self) -> &str;

    /// ツールの説明を返す
    fn description(&self) -> &str;

    /// ツールを実行する
    async fn invoke(&self, parameters: Value) -> Result<Value, RuntimeError>;
}

/// EchoTool: 入力そのままを返すデモ用ツール
#[derive(Default)]
pub struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "Echoes the input back"
    }

    async fn invoke(&self, parameters: Value) -> Result<Value, RuntimeError> {
        Ok(parameters)
    }
}

/// ToolRegistry: ツール登録・取得・一覧管理
#[derive(Default)]
pub struct ToolRegistry {
    tools: std::collections::HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// ツールを登録する
    pub fn register(&mut self, capability: String, tool: Box<dyn Tool>) {
        self.tools.insert(capability, tool);
    }

    /// ツール名でツールを取得する
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// 登録済みツール名のリストを返す
    pub fn list(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// 登録済みツール数を返す
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// 登録ツールが空か
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}
