use std::collections::HashMap;

/// WorkerState: 登録済みワーカーの状態
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerState {
    Ready,
    Exited,
    Error,
}

impl WorkerState {
    fn from_runtime_state(state: &str) -> Self {
        match state {
            "exited" => Self::Exited,
            "error" => Self::Error,
            _ => Self::Ready,
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Exited => "exited",
            Self::Error => "error",
        }
    }
}

impl PartialEq<&str> for WorkerState {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<WorkerState> for &str {
    fn eq(&self, other: &WorkerState) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<String> for WorkerState {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<WorkerState> for String {
    fn eq(&self, other: &WorkerState) -> bool {
        self == other.as_str()
    }
}

/// WorkerRecord: 登録済みワーカーの公開情報
#[derive(Debug, Clone, PartialEq)]
pub struct WorkerRecord {
    pub id: String,
    pub agent_id: String,
    pub capabilities: Vec<String>,
    pub state: WorkerState,
    pub last_error: serde_json::Value,
    pub metadata: serde_json::Value,
}

pub type WorkerInfo = WorkerRecord;

/// WorkerRegistry: ワーカーIDから能力を検索する最小レジストリ
#[derive(Debug, Default, Clone)]
pub struct WorkerRegistry {
    workers: HashMap<String, WorkerInfo>,
}

impl WorkerRegistry {
    pub fn register(&mut self, agent_id: String, capabilities: Vec<String>) {
        let worker = WorkerRecord {
            id: agent_id.clone(),
            agent_id: agent_id.clone(),
            capabilities,
            state: WorkerState::Ready,
            last_error: serde_json::Value::Null,
            metadata: serde_json::json!({}),
        };
        self.workers.insert(agent_id, worker);
    }

    pub fn mark_ready(&mut self, agent_id: String, capabilities: Vec<String>) {
        let worker = WorkerRecord {
            id: agent_id.clone(),
            agent_id: agent_id.clone(),
            capabilities,
            state: WorkerState::Ready,
            last_error: serde_json::Value::Null,
            metadata: serde_json::json!({}),
        };
        self.workers.insert(agent_id, worker);
    }

    pub fn mark_exited(&mut self, agent_id: String, reason: Option<String>) {
        let metadata = reason
            .map(|reason| serde_json::json!({ "reason": reason }))
            .unwrap_or_else(|| serde_json::json!({}));
        self.workers
            .entry(agent_id.clone())
            .and_modify(|worker| {
                worker.state = WorkerState::Exited;
                worker.metadata = metadata.clone();
            })
            .or_insert_with(|| WorkerRecord {
                id: agent_id.clone(),
                agent_id,
                capabilities: Vec::new(),
                state: WorkerState::Exited,
                last_error: serde_json::Value::Null,
                metadata,
            });
    }

    pub fn mark_error(
        &mut self,
        agent_id: String,
        task_id: Option<String>,
        error: serde_json::Value,
    ) {
        let metadata = task_id
            .map(|task_id| serde_json::json!({ "task_id": task_id }))
            .unwrap_or_else(|| serde_json::json!({}));
        let agent_id_clone = agent_id.clone();
        let error_clone = error.clone();
        let metadata_clone = metadata.clone();
        self.workers
            .entry(agent_id_clone)
            .and_modify(|worker| {
                worker.state = WorkerState::Error;
                worker.last_error = error_clone.clone();
                worker.metadata = metadata_clone.clone();
            })
            .or_insert_with(|| WorkerRecord {
                id: String::new(),
                agent_id,
                capabilities: Vec::new(),
                state: WorkerState::Error,
                last_error: error,
                metadata,
            });
    }

    pub fn find_by_capability(&self, capability: &str) -> Vec<String> {
        self.workers
            .iter()
            .filter(|(_, worker)| {
                worker.state == WorkerState::Ready
                    && worker.capabilities.iter().any(|c| c == capability)
            })
            .map(|(_, worker)| worker.agent_id.clone())
            .collect()
    }

    pub fn lookup(&self, agent_id: &str) -> Option<&WorkerInfo> {
        self.workers.get(agent_id)
    }

    pub fn get_worker(&self, agent_id: &str) -> Option<&WorkerInfo> {
        self.workers.get(agent_id)
    }

    pub fn set_state(&mut self, id: String, state: String) {
        self.workers
            .entry(id)
            .and_modify(|worker| worker.state = WorkerState::from_runtime_state(&state));
    }
}
