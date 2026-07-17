//! Codex App Server integration for MealZ.
//!
//! This module is intentionally the only AI runtime boundary in the app:
//! `codex app-server` is spawned as a long-lived child and spoken to over its
//! newline-delimited JSON protocol. Domain capabilities enter Codex through
//! [`DynamicToolSpec`] and are executed by a MealZ-owned [`ToolExecutor`].

mod agent;
pub mod host;
pub mod protocol;
mod types;

pub use agent::CodexAgent;
pub use host::{ProcessHost, ResumeError, resolve_codex_program};
pub use types::{
    AgentConfig, AgentEvent, AgentStatus, DynamicToolSpec, MemoryThreadStore, SessionInfo,
    ThreadStore, ToolCall, ToolExecutor, TurnHandle, UserInput,
};
