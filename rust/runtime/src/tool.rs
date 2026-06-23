use async_trait::async_trait;
use serde_json::Value;
use std::fmt;

use crate::error::RuntimeError;

/// Tool: エージェントが呼び出す操作のトレイト
///
/// `dyn Tool` does not require `Debug` to avoid forcing tool implementations
/// to expose their internal state.  ToolRegistry's manual Debug prints only
/// capability names.
#[async_trait]
pub trait Tool: Send + Sync + Clone + fmt::Debug {
    /// ツール名を返す
    fn name(&self) -> &str;

    /// ツールの説明を返す
    fn description(&self) -> &str;

    /// ツールを実行する
    async fn invoke(&self, parameters: Value) -> Result<Value, RuntimeError>;
}

/// EchoTool: 入力そのままを返すデモ用ツール
///
/// Canonical capability name is `"tool.echo"`.  For backwards compatibility
/// it is also registered under the `"echo"` alias when added via
/// `ToolRegistry::register_alias`.
#[derive(Default, Clone)]
pub struct EchoTool;

impl fmt::Debug for EchoTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EchoTool").finish()
    }
}

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

    /// ツールにエイリアスを登録する
    pub fn register_alias(&mut self, alias: String, target: String) {
        if let Some(tool) = self.tools.get(&target) {
            let alias_tool: Box<dyn Tool> = tool.clone();
            self.tools.insert(alias, alias_tool);
        }
    }

    /// ツールを削除する
    pub fn unregister(&mut self, capability: &str) -> Option<Box<dyn Tool>> {
        self.tools.remove(capability)
    }

    /// ツールを取得する
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// 登録済みツールの一覧を返す（ソート済み）
    pub fn list_capabilities(&self) -> Vec<String> {
        let mut caps: Vec<String> = self.tools.keys().cloned().collect();
        caps.sort();
        caps
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

impl fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("capabilities", &self.list_capabilities())
            .finish_non_exhaustive()
    }
}
