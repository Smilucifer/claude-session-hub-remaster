use crate::models::{RunMeta, RunStatus, TaskRun};
use crate::storage;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

pub type AdapterResult<T> = Result<T, AgentAdapterError>;
pub type AdapterFuture<'a, T> = Pin<Box<dyn Future<Output = AdapterResult<T>> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq)]
pub struct AgentAdapterError {
    pub message: String,
}

impl AgentAdapterError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for AgentAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AgentAdapterError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    Claude,
    Codex,
    Gemini,
    Unknown,
}

impl AgentKind {
    pub fn from_agent(agent: &str) -> Self {
        match agent.to_ascii_lowercase().as_str() {
            "claude" => Self::Claude,
            "codex" => Self::Codex,
            "gemini" => Self::Gemini,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptScope {
    Room,
    Participant,
    Turn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCapabilities {
    pub kind: AgentKind,
    pub can_stream_message: bool,
    pub can_inject_prompt: bool,
    pub can_wait_turn_complete: bool,
}

impl AgentCapabilities {
    pub fn for_kind(kind: AgentKind) -> Self {
        match kind {
            AgentKind::Claude => Self {
                kind,
                can_stream_message: false,
                can_inject_prompt: false,
                can_wait_turn_complete: true,
            },
            AgentKind::Codex | AgentKind::Gemini | AgentKind::Unknown => Self {
                kind,
                can_stream_message: false,
                can_inject_prompt: false,
                can_wait_turn_complete: false,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnOutcomeStatus {
    Pending,
    Running,
    Complete,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnOutcome {
    pub run_id: String,
    pub status: TurnOutcomeStatus,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
}

pub trait AgentAdapter: Send {
    fn wait_turn_complete<'a>(&'a mut self) -> AdapterFuture<'a, TurnOutcome>;
    fn stream_message<'a>(&'a mut self, msg: &'a str) -> AdapterFuture<'a, ()>;
    fn inject_prompt(&mut self, scope: PromptScope, body: &str) -> AdapterResult<()>;
    fn capabilities(&self) -> AgentCapabilities;
}

#[derive(Debug, Clone)]
pub struct RunBackedAgentAdapter {
    run_id: String,
    kind: AgentKind,
    poll_interval: Duration,
    max_polls: usize,
}

impl RunBackedAgentAdapter {
    pub fn new(run: &RunMeta) -> Self {
        Self {
            run_id: run.id.clone(),
            kind: AgentKind::from_agent(&run.agent),
            poll_interval: Duration::from_millis(250),
            max_polls: 1200,
        }
    }

    #[cfg(test)]
    fn with_polling(mut self, poll_interval: Duration, max_polls: usize) -> Self {
        self.poll_interval = poll_interval;
        self.max_polls = max_polls;
        self
    }

    fn read_outcome(&self) -> AdapterResult<TurnOutcome> {
        let run = storage::runs::get_run(&self.run_id)
            .ok_or_else(|| AgentAdapterError::new(format!("Run {} not found", self.run_id)))?;
        Ok(outcome_from_run(&run))
    }
}

impl AgentAdapter for RunBackedAgentAdapter {
    fn wait_turn_complete<'a>(&'a mut self) -> AdapterFuture<'a, TurnOutcome> {
        Box::pin(async move {
            for _ in 0..=self.max_polls {
                let outcome = self.read_outcome()?;
                if matches!(
                    outcome.status,
                    TurnOutcomeStatus::Complete
                        | TurnOutcomeStatus::Failed
                        | TurnOutcomeStatus::Stopped
                ) {
                    return Ok(outcome);
                }
                tokio::time::sleep(self.poll_interval).await;
            }
            Err(AgentAdapterError::new(format!(
                "Timed out waiting for run {} to complete a turn",
                self.run_id
            )))
        })
    }

    fn stream_message<'a>(&'a mut self, _msg: &'a str) -> AdapterFuture<'a, ()> {
        Box::pin(async move {
            Err(AgentAdapterError::new(
                "Room participant streaming is not wired in Phase 2",
            ))
        })
    }

    fn inject_prompt(&mut self, _scope: PromptScope, _body: &str) -> AdapterResult<()> {
        Err(AgentAdapterError::new(
            "Room prompt injection is not wired in Phase 2",
        ))
    }

    fn capabilities(&self) -> AgentCapabilities {
        AgentCapabilities::for_kind(self.kind)
    }
}

pub fn adapter_for_run(run: &RunMeta) -> RunBackedAgentAdapter {
    RunBackedAgentAdapter::new(run)
}

pub fn adapter_for_task_run(run: &TaskRun) -> RunBackedAgentAdapter {
    RunBackedAgentAdapter {
        run_id: run.id.clone(),
        kind: AgentKind::from_agent(&run.agent),
        poll_interval: Duration::from_millis(250),
        max_polls: 1200,
    }
}

fn outcome_from_run(run: &RunMeta) -> TurnOutcome {
    TurnOutcome {
        run_id: run.id.clone(),
        status: match run.status {
            RunStatus::Pending => TurnOutcomeStatus::Pending,
            RunStatus::Running => TurnOutcomeStatus::Running,
            RunStatus::Idle | RunStatus::Completed => TurnOutcomeStatus::Complete,
            RunStatus::Failed => TurnOutcomeStatus::Failed,
            RunStatus::Stopped => TurnOutcomeStatus::Stopped,
        },
        exit_code: run.exit_code,
        error: run.error_message.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::RunStatus;

    fn with_temp_data_dir<T>(f: impl FnOnce() -> T) -> T {
        let _guard = crate::storage::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let previous = std::env::var_os("OPENCOVIBE_DATA_DIR");
        std::env::set_var("OPENCOVIBE_DATA_DIR", tmp.path());
        let result = f();
        match previous {
            Some(value) => std::env::set_var("OPENCOVIBE_DATA_DIR", value),
            None => std::env::remove_var("OPENCOVIBE_DATA_DIR"),
        }
        result
    }

    #[test]
    fn maps_known_agent_capabilities() {
        let claude = AgentCapabilities::for_kind(AgentKind::Claude);
        assert!(!claude.can_stream_message);
        assert!(!claude.can_inject_prompt);
        assert!(claude.can_wait_turn_complete);

        let codex = AgentCapabilities::for_kind(AgentKind::Codex);
        assert!(!codex.can_stream_message);
        assert!(!codex.can_inject_prompt);
        assert!(!codex.can_wait_turn_complete);
    }

    #[test]
    fn waits_until_run_is_turn_complete() {
        let outcome = with_temp_data_dir(|| {
            crate::storage::runs::create_run(
                "run-1",
                "hello",
                "D:/work/app",
                "claude",
                RunStatus::Idle,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap();
            let run = crate::storage::runs::get_run("run-1").unwrap();
            let mut adapter = adapter_for_run(&run).with_polling(Duration::from_millis(1), 0);

            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(adapter.wait_turn_complete())
        });

        let outcome = outcome.unwrap();
        assert_eq!(outcome.status, TurnOutcomeStatus::Complete);
        assert_eq!(outcome.run_id, "run-1");
    }
}
