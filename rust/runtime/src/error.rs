/// RuntimeError: ランタイム操作時のエラー型
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("bus error: {0}")]
    BusError(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("response error: {0}")]
    ResponseError(String),

    #[error("response timeout: {0}")]
    ResponseTimeout(String),

    #[error("lifecycle error: {0}")]
    LifecycleError(String),

    #[error("invalid payload: {0}")]
    InvalidPayload(String),

    #[error("schedule error: {0}")]
    ScheduleError(String),

    #[error("provider error: {0}")]
    ProviderError(String),

    #[error("tool error: {0}")]
    ToolError(String),
}
