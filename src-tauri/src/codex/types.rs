use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// One function that Codex may call through the experimental dynamic-tools
/// API. The schema must describe a JSON object.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DynamicToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

impl DynamicToolSpec {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
    ) -> Result<Self, String> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err("dynamic tool name must not be empty".into());
        }
        if input_schema.get("type").and_then(Value::as_str) != Some("object") {
            return Err(format!(
                "dynamic tool {name:?} needs an object input schema"
            ));
        }
        Ok(Self {
            name,
            description: description.into(),
            input_schema,
        })
    }

    pub(crate) fn to_wire_value(&self) -> Value {
        json!({
            "type": "function",
            "name": self.name,
            "description": self.description,
            "inputSchema": self.input_schema,
        })
    }
}

/// A validated callback from `item/tool/call`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub call_id: Option<String>,
    pub namespace: Option<String>,
    pub tool: String,
    pub arguments: Value,
}

/// MealZ implements this trait at the domain boundary. Implementations own
/// validation and SQLite transactions; the Codex host never parses prose to
/// produce writes.
#[async_trait]
pub trait ToolExecutor: Send + Sync + 'static {
    fn definitions(&self) -> Vec<DynamicToolSpec>;
    async fn execute(&self, call: ToolCall) -> Result<Value, String>;
}

/// Persistence boundary for the one durable MealZ agent thread id.
#[async_trait]
pub trait ThreadStore: Send + Sync + 'static {
    async fn load_thread_id(&self) -> Result<Option<String>, String>;
    async fn save_thread_id(&self, thread_id: &str) -> Result<(), String>;
    async fn clear_thread_id(&self) -> Result<(), String>;
}

/// Small in-memory implementation useful in previews and tests. Production
/// MealZ supplies a SQLite-backed implementation.
#[derive(Clone, Default)]
pub struct MemoryThreadStore {
    thread_id: Arc<RwLock<Option<String>>>,
}

#[async_trait]
impl ThreadStore for MemoryThreadStore {
    async fn load_thread_id(&self) -> Result<Option<String>, String> {
        Ok(self.thread_id.read().clone())
    }

    async fn save_thread_id(&self, thread_id: &str) -> Result<(), String> {
        *self.thread_id.write() = Some(thread_id.to_string());
        Ok(())
    }

    async fn clear_thread_id(&self) -> Result<(), String> {
        *self.thread_id.write() = None;
        Ok(())
    }
}

/// Runtime configuration applied when a thread is started or resumed.
#[derive(Clone, Debug)]
pub struct AgentConfig {
    pub cwd: PathBuf,
    pub developer_instructions: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub personality: Option<String>,
    pub service_tier: Option<String>,
    pub codex_binary: Option<PathBuf>,
}

impl AgentConfig {
    pub fn new(cwd: impl Into<PathBuf>, developer_instructions: impl Into<String>) -> Self {
        Self {
            cwd: cwd.into(),
            developer_instructions: developer_instructions.into(),
            model: None,
            effort: None,
            personality: Some("friendly".into()),
            service_tier: None,
            codex_binary: None,
        }
    }
}

/// Supported turn inputs. MealZ currently exposes text and local/remote
/// recipe images without leaking raw protocol objects through the API.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UserInput {
    Text { text: String },
    Image { url: String },
    LocalImage { path: String },
}

impl UserInput {
    pub(crate) fn to_wire_value(&self) -> Value {
        match self {
            UserInput::Text { text } => json!({ "type": "text", "text": text }),
            UserInput::Image { url } => json!({ "type": "image", "url": url }),
            UserInput::LocalImage { path } => {
                json!({ "type": "localImage", "path": path })
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub thread_id: String,
    pub resumed: bool,
    pub server_version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TurnHandle {
    pub thread_id: String,
    pub turn_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub running: bool,
    pub thread_id: Option<String>,
    pub active_turn_id: Option<String>,
    pub server_version: Option<String>,
}

/// Events are intentionally loss-tolerant: the raw notification is retained
/// for forward compatibility, while lifecycle/tool events provide a stable
/// UI contract.
#[derive(Clone, Debug, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AgentEvent {
    ThreadReady {
        thread_id: String,
        resumed: bool,
        server_version: String,
    },
    Notification {
        method: String,
        params: Value,
    },
    ToolStarted {
        call: ToolCall,
    },
    ToolCompleted {
        call: ToolCall,
        success: bool,
        result: Value,
    },
    ProcessExited,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_specs_validate_and_match_the_wire_shape() {
        assert!(DynamicToolSpec::new("bad", "", json!({ "type": "string" })).is_err());
        let spec = DynamicToolSpec::new(
            "recipes_search",
            "Search the local recipe catalog",
            json!({ "type": "object", "properties": {} }),
        )
        .unwrap();
        let wire = spec.to_wire_value();
        assert_eq!(wire["type"], "function");
        assert_eq!(wire["name"], "recipes_search");
        assert_eq!(wire["inputSchema"]["type"], "object");
        assert!(wire.get("input_schema").is_none());
    }

    #[tokio::test]
    async fn memory_thread_store_roundtrips() {
        let store = MemoryThreadStore::default();
        assert_eq!(store.load_thread_id().await.unwrap(), None);
        store.save_thread_id("thread-1").await.unwrap();
        assert_eq!(
            store.load_thread_id().await.unwrap().as_deref(),
            Some("thread-1")
        );
        store.clear_thread_id().await.unwrap();
        assert_eq!(store.load_thread_id().await.unwrap(), None);
    }

    #[test]
    fn agent_events_serialize_for_the_typescript_bridge_in_camel_case() {
        let value = serde_json::to_value(AgentEvent::ThreadReady {
            thread_id: "thread-1".into(),
            resumed: true,
            server_version: "MealZ/0.144.1".into(),
        })
        .unwrap();
        assert_eq!(value["type"], "threadReady");
        assert_eq!(value["threadId"], "thread-1");
        assert_eq!(value["serverVersion"], "MealZ/0.144.1");
        assert!(value.get("thread_id").is_none());
    }
}
