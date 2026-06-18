/// RuntimeError: ランタイム操作時のエラー型
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("bus error: {0}")]
    BusError(String),
    #[error("schedule error: {0}")]
    ScheduleError(String),
    #[error("lifecycle error: {0}")]
    LifecycleError(String),
}
