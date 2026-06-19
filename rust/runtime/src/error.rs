/// RuntimeError: ランタイム操作時のエラー型
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("bus error: {0}")]
    BusError(String),
    #[error("schedule error: {0}")]
    ScheduleError(String),
    #[error("lifecycle error: {0}")]
    LifecycleError(String),
    #[error("invalid payload: {0}")]
    InvalidPayload(String),
    #[error("provider error: {0}")]
    ProviderError(String),
    #[error("tool error: {0}")]
    ToolError(String),
}
