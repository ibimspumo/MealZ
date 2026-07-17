//! High-level durable MealZ agent built on the generic process host.

use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::{Map, Value, json};
use tokio::sync::{Mutex, broadcast};

use super::host::{
    Connection, ProcessHost, RPC_TIMEOUT_MS, ResumeError, THREAD_TIMEOUT_MS, ThreadEvent,
    resume_thread, route_channel,
};
use super::types::{
    AgentConfig, AgentEvent, AgentStatus, SessionInfo, ThreadStore, ToolCall, ToolExecutor,
    TurnHandle, UserInput,
};

const EVENT_BUFFER: usize = 512;

#[derive(Default)]
struct SessionState {
    thread_id: Option<String>,
    generation: u64,
    active_turn_id: Option<String>,
}

struct Inner {
    host: ProcessHost,
    executor: Arc<dyn ToolExecutor>,
    thread_store: Arc<dyn ThreadStore>,
    config: RwLock<AgentConfig>,
    state: Mutex<SessionState>,
    operation: Mutex<()>,
    events: broadcast::Sender<AgentEvent>,
}

/// The personal MealZ Codex agent. Clones share one process host, durable
/// thread, active-turn state and event stream.
#[derive(Clone)]
pub struct CodexAgent {
    inner: Arc<Inner>,
}

impl CodexAgent {
    pub fn new(
        config: AgentConfig,
        executor: Arc<dyn ToolExecutor>,
        thread_store: Arc<dyn ThreadStore>,
    ) -> Self {
        let (events, _) = broadcast::channel(EVENT_BUFFER);
        let host = ProcessHost::new(config.codex_binary.clone());
        Self {
            inner: Arc::new(Inner {
                host,
                executor,
                thread_store,
                config: RwLock::new(config),
                state: Mutex::new(SessionState::default()),
                operation: Mutex::new(()),
                events,
            }),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.inner.events.subscribe()
    }

    pub async fn status(&self) -> AgentStatus {
        let state = self.inner.state.lock().await;
        let thread_id = state.thread_id.clone();
        let active_turn_id = state.active_turn_id.clone();
        drop(state);
        AgentStatus {
            running: self.inner.host.alive().await.is_some(),
            thread_id,
            active_turn_id,
            server_version: self.inner.host.last_version().await,
        }
    }

    /// Spawn the server if needed, then resume the SQLite-persisted thread.
    /// A missing rollout automatically starts and persists a fresh thread.
    pub async fn start_or_resume(&self) -> Result<SessionInfo, String> {
        let _operation = self.inner.operation.lock().await;
        self.ensure_session_locked().await
    }

    /// Start one streamed turn. The acknowledgement is returned immediately;
    /// content and completion arrive through [`Self::subscribe`].
    pub async fn start_turn(&self, input: Vec<UserInput>) -> Result<TurnHandle, String> {
        if input.is_empty() {
            return Err("a Codex turn needs at least one input".into());
        }
        if input
            .iter()
            .any(|item| matches!(item, UserInput::Text { text } if text.trim().is_empty()))
        {
            return Err("text input must not be blank".into());
        }

        let _operation = self.inner.operation.lock().await;
        let session = self.ensure_session_locked().await?;
        if let Some(turn_id) = self.inner.state.lock().await.active_turn_id.clone() {
            return Err(format!(
                "turn {turn_id} is still active; steer it or interrupt it before starting another"
            ));
        }
        let (connection, _) = self.inner.host.ensure().await?;

        let config = self.inner.config.read().clone();
        let mut params = Map::new();
        params.insert("threadId".into(), json!(session.thread_id));
        params.insert(
            "input".into(),
            Value::Array(input.iter().map(UserInput::to_wire_value).collect()),
        );
        if let Some(effort) = config.effort.filter(|value| !value.trim().is_empty()) {
            params.insert("effort".into(), json!(effort));
        }

        let response = connection
            .request("turn/start", Value::Object(params), RPC_TIMEOUT_MS)
            .await
            .map_err(|error| format!("turn/start failed: {error}"))?;
        let turn_id = response
            .pointer("/turn/id")
            .or_else(|| response.get("turnId"))
            .and_then(Value::as_str)
            .ok_or_else(|| "turn/start returned no turn id".to_string())?
            .to_string();
        self.inner.state.lock().await.active_turn_id = Some(turn_id.clone());
        Ok(TurnHandle {
            thread_id: session.thread_id,
            turn_id,
        })
    }

    pub async fn send_message(&self, text: impl Into<String>) -> Result<TurnHandle, String> {
        self.start_turn(vec![UserInput::Text { text: text.into() }])
            .await
    }

    /// Inject follow-up text into the currently running turn.
    pub async fn steer(&self, text: impl Into<String>) -> Result<TurnHandle, String> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err("steering text must not be blank".into());
        }
        let _operation = self.inner.operation.lock().await;
        let session = self.ensure_session_locked().await?;
        let turn_id = self
            .inner
            .state
            .lock()
            .await
            .active_turn_id
            .clone()
            .ok_or_else(|| "there is no running turn to steer".to_string())?;
        let (connection, _) = self.inner.host.ensure().await?;
        connection
            .request(
                "turn/steer",
                json!({
                    "threadId": session.thread_id,
                    "expectedTurnId": turn_id,
                    "input": [{ "type": "text", "text": text }],
                }),
                RPC_TIMEOUT_MS,
            )
            .await
            .map_err(|error| format!("turn/steer failed: {error}"))?;
        Ok(TurnHandle {
            thread_id: session.thread_id,
            turn_id,
        })
    }

    pub async fn interrupt(&self) -> Result<(), String> {
        let _operation = self.inner.operation.lock().await;
        let session = self.ensure_session_locked().await?;
        let turn_id = self
            .inner
            .state
            .lock()
            .await
            .active_turn_id
            .clone()
            .ok_or_else(|| "there is no running turn to interrupt".to_string())?;
        let (connection, _) = self.inner.host.ensure().await?;
        connection
            .request(
                "turn/interrupt",
                json!({ "threadId": session.thread_id, "turnId": turn_id }),
                RPC_TIMEOUT_MS,
            )
            .await
            .map_err(|error| format!("turn/interrupt failed: {error}"))?;
        Ok(())
    }

    pub async fn compact(&self) -> Result<(), String> {
        let _operation = self.inner.operation.lock().await;
        let session = self.ensure_session_locked().await?;
        let (connection, _) = self.inner.host.ensure().await?;
        connection
            .request(
                "thread/compact/start",
                json!({ "threadId": session.thread_id }),
                THREAD_TIMEOUT_MS,
            )
            .await
            .map_err(|error| format!("thread/compact/start failed: {error}"))?;
        Ok(())
    }

    /// Forget the old rollout and immediately create a fresh MealZ thread.
    pub async fn reset_thread(&self) -> Result<SessionInfo, String> {
        let _operation = self.inner.operation.lock().await;
        if let Some(connection) = self.inner.host.alive().await {
            let state = self.inner.state.lock().await;
            let thread_id = state.thread_id.clone();
            let active_turn_id = state.active_turn_id.clone();
            drop(state);
            if let (Some(thread_id), Some(turn_id)) = (&thread_id, active_turn_id) {
                let _ = connection
                    .request(
                        "turn/interrupt",
                        json!({ "threadId": thread_id, "turnId": turn_id }),
                        RPC_TIMEOUT_MS,
                    )
                    .await;
            }
            if let Some(thread_id) = thread_id {
                connection.unregister_thread(&thread_id);
            }
        }
        self.inner.thread_store.clear_thread_id().await?;
        *self.inner.state.lock().await = SessionState::default();
        self.ensure_session_locked().await
    }

    /// Switch the running host to the thread of the conversation that the
    /// application's ThreadStore has already selected. Unlike `reset_thread`,
    /// this never clears a persisted thread id, so archived conversations can
    /// be resumed later without losing their Codex rollout.
    pub async fn switch_to_persisted_thread(
        &self,
        thread_id: Option<String>,
    ) -> Result<SessionInfo, String> {
        let _operation = self.inner.operation.lock().await;
        let mut state = self.inner.state.lock().await;
        if let Some(turn_id) = state.active_turn_id.as_deref() {
            return Err(format!(
                "Das Gespräch kann nicht gewechselt werden, solange Antwort {turn_id} läuft. Bitte die Antwort zuerst stoppen."
            ));
        }
        let previous_thread_id = state.thread_id.clone();
        *state = SessionState {
            thread_id,
            generation: 0,
            active_turn_id: None,
        };
        drop(state);
        if let Some(connection) = self.inner.host.alive().await
            && let Some(previous_thread_id) = previous_thread_id
        {
            connection.unregister_thread(&previous_thread_id);
        }
        self.ensure_session_locked().await
    }

    /// Apply personality/model/instruction changes by restarting only the
    /// App Server process and resuming the same durable thread.
    pub async fn reconfigure(&self, config: AgentConfig) -> Result<SessionInfo, String> {
        let _operation = self.inner.operation.lock().await;
        self.inner
            .host
            .set_binary_override(config.codex_binary.clone());
        *self.inner.config.write() = config;
        self.inner.host.shutdown().await;
        self.inner.state.lock().await.generation = 0;
        self.ensure_session_locked().await
    }

    pub async fn list_models(&self) -> Result<Value, String> {
        let _operation = self.inner.operation.lock().await;
        self.ensure_session_locked().await?;
        let (connection, _) = self.inner.host.ensure().await?;
        connection
            .request("model/list", json!({}), THREAD_TIMEOUT_MS)
            .await
    }

    pub async fn read_account(&self) -> Result<Value, String> {
        let _operation = self.inner.operation.lock().await;
        self.ensure_session_locked().await?;
        let (connection, _) = self.inner.host.ensure().await?;
        connection
            .request("account/read", json!({}), RPC_TIMEOUT_MS)
            .await
    }

    /// Reads the provider capabilities exposed by the exact running Codex App
    /// Server. The UI uses this rather than claiming web/image support based
    /// on a hard-coded assumption.
    pub async fn read_provider_capabilities(&self) -> Result<Value, String> {
        let _operation = self.inner.operation.lock().await;
        self.ensure_session_locked().await?;
        let (connection, _) = self.inner.host.ensure().await?;
        connection
            .request("modelProvider/capabilities/read", json!({}), RPC_TIMEOUT_MS)
            .await
    }

    /// Read the persisted message transcript after an app restart. Codex
    /// owns rollout history; MealZ can rebuild the chat UI without parsing
    /// files under `~/.codex/sessions`.
    pub async fn read_thread(&self) -> Result<Value, String> {
        let _operation = self.inner.operation.lock().await;
        let session = self.ensure_session_locked().await?;
        let (connection, _) = self.inner.host.ensure().await?;
        connection
            .request(
                "thread/read",
                json!({ "threadId": session.thread_id, "includeTurns": true }),
                THREAD_TIMEOUT_MS,
            )
            .await
    }

    /// Gracefully close the child while retaining the durable thread id for
    /// the next application launch.
    pub async fn shutdown(&self) {
        let _operation = self.inner.operation.lock().await;
        self.inner.host.shutdown().await;
        self.inner.state.lock().await.active_turn_id = None;
    }

    async fn ensure_session_locked(&self) -> Result<SessionInfo, String> {
        let (connection, generation) = self.inner.host.ensure().await?;
        let mut state = self.inner.state.lock().await;
        if state.thread_id.is_none() {
            state.thread_id = self.inner.thread_store.load_thread_id().await?;
        }

        if state.generation == generation
            && let Some(thread_id) = &state.thread_id
        {
            return Ok(SessionInfo {
                thread_id: thread_id.clone(),
                resumed: true,
                server_version: connection.version().to_string(),
            });
        }

        if let Some(thread_id) = state.thread_id.clone() {
            let (sink, receiver) = route_channel();
            connection.register_thread(&thread_id, sink);
            spawn_dispatcher(self.inner.clone(), generation, receiver);
            drop(state);

            let resume_params = {
                let config = self.inner.config.read();
                thread_resume_params(&thread_id, &config)
            };
            match resume_thread(&connection, resume_params).await {
                Ok(_) => {
                    let mut state = self.inner.state.lock().await;
                    state.thread_id = Some(thread_id.clone());
                    state.generation = generation;
                    state.active_turn_id = None;
                    let info = SessionInfo {
                        thread_id: thread_id.clone(),
                        resumed: true,
                        server_version: connection.version().to_string(),
                    };
                    let _ = self.inner.events.send(AgentEvent::ThreadReady {
                        thread_id,
                        resumed: true,
                        server_version: info.server_version.clone(),
                    });
                    return Ok(info);
                }
                Err(ResumeError::ThreadNotFound(_)) => {
                    connection.unregister_thread(&thread_id);
                    self.inner.thread_store.clear_thread_id().await?;
                    *self.inner.state.lock().await = SessionState::default();
                }
                Err(ResumeError::Other(error)) => {
                    connection.unregister_thread(&thread_id);
                    return Err(format!("thread/resume failed: {error}"));
                }
            }
        } else {
            drop(state);
        }

        self.start_fresh_thread(connection, generation).await
    }

    async fn start_fresh_thread(
        &self,
        connection: Arc<Connection>,
        generation: u64,
    ) -> Result<SessionInfo, String> {
        let params = thread_start_params(&self.inner.config.read(), self.inner.executor.as_ref());
        let response = connection
            .request("thread/start", params, THREAD_TIMEOUT_MS)
            .await
            .map_err(guard_dynamic_tools_error)?;
        let thread_id = response
            .pointer("/thread/id")
            .or_else(|| response.get("threadId"))
            .and_then(Value::as_str)
            .ok_or_else(|| "thread/start returned no thread id".to_string())?
            .to_string();

        let (sink, receiver) = route_channel();
        connection.register_thread(&thread_id, sink);
        spawn_dispatcher(self.inner.clone(), generation, receiver);
        self.inner.thread_store.save_thread_id(&thread_id).await?;
        *self.inner.state.lock().await = SessionState {
            thread_id: Some(thread_id.clone()),
            generation,
            active_turn_id: None,
        };

        let info = SessionInfo {
            thread_id: thread_id.clone(),
            resumed: false,
            server_version: connection.version().to_string(),
        };
        let _ = self.inner.events.send(AgentEvent::ThreadReady {
            thread_id,
            resumed: false,
            server_version: info.server_version.clone(),
        });
        Ok(info)
    }
}

fn thread_start_params(config: &AgentConfig, executor: &dyn ToolExecutor) -> Value {
    let mut params = thread_base_params(config);
    params.insert(
        "dynamicTools".into(),
        Value::Array(
            executor
                .definitions()
                .iter()
                .map(|definition| definition.to_wire_value())
                .collect(),
        ),
    );
    Value::Object(params)
}

fn thread_resume_params(thread_id: &str, config: &AgentConfig) -> Value {
    let mut params = thread_base_params(config);
    params.insert("threadId".into(), json!(thread_id));
    // Dynamic tools are persisted in the rollout and cannot be re-declared
    // on resume (verified against Codex 0.144.1).
    Value::Object(params)
}

fn thread_base_params(config: &AgentConfig) -> Map<String, Value> {
    let cwd = if config.cwd.is_dir() {
        config.cwd.clone()
    } else {
        dirs::home_dir().unwrap_or_else(|| config.cwd.clone())
    };
    let mut params = Map::new();
    params.insert("cwd".into(), json!(cwd.to_string_lossy()));
    params.insert("sandbox".into(), json!("read-only"));
    params.insert("approvalPolicy".into(), json!("never"));
    params.insert(
        "developerInstructions".into(),
        json!(config.developer_instructions),
    );
    params.insert("ephemeral".into(), json!(false));
    params.insert("historyMode".into(), json!("legacy"));
    if let Some(model) = config
        .model
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        params.insert("model".into(), json!(model));
    }
    if let Some(personality) = config
        .personality
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        params.insert("personality".into(), json!(personality));
    }
    if let Some(service_tier) = config
        .service_tier
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        params.insert("serviceTier".into(), json!(service_tier));
    }
    params
}

fn guard_dynamic_tools_error(error: String) -> String {
    let lower = error.to_lowercase();
    if lower.contains("dynamictools")
        || lower.contains("dynamic_tools")
        || lower.contains("experimental")
        || lower.contains("unknown field")
    {
        format!(
            "the installed Codex CLI does not support the experimental App Server dynamic-tools API; update Codex. Underlying error: {error}"
        )
    } else {
        error
    }
}

fn spawn_dispatcher(
    inner: Arc<Inner>,
    generation: u64,
    mut receiver: tokio::sync::mpsc::Receiver<ThreadEvent>,
) {
    tokio::spawn(async move {
        while let Some(event) = receiver.recv().await {
            match event {
                ThreadEvent::Request {
                    method,
                    params,
                    responder,
                } => {
                    if method == "item/tool/call" {
                        let executor = inner.executor.clone();
                        let events = inner.events.clone();
                        tokio::spawn(async move {
                            match parse_tool_call(&params) {
                                Ok(call) => {
                                    let _ =
                                        events.send(AgentEvent::ToolStarted { call: call.clone() });
                                    let result = executor.execute(call.clone()).await;
                                    let response = tool_call_response(&result);
                                    responder.ok(&response);
                                    let (success, result) = match result {
                                        Ok(value) => (true, value),
                                        Err(error) => (false, json!({ "error": error })),
                                    };
                                    let _ = events.send(AgentEvent::ToolCompleted {
                                        call,
                                        success,
                                        result,
                                    });
                                }
                                Err(error) => responder.error(-32602, &error),
                            }
                        });
                    } else if method == "currentTime/read" {
                        responder.ok(&json!({ "currentTimeAt": chrono::Utc::now().timestamp() }));
                    } else if method.ends_with("/requestApproval") {
                        // MealZ runs Codex read-only with approvalPolicy=never.
                        // If a version still asks, decline deterministically so
                        // the turn cannot deadlock.
                        responder.ok(&json!({ "decision": "decline" }));
                    } else {
                        responder.error(-32601, "unsupported Codex server request");
                    }
                }
                ThreadEvent::Notification { method, params } => {
                    update_lifecycle(&inner, generation, &method, &params).await;
                    let _ = inner
                        .events
                        .send(AgentEvent::Notification { method, params });
                }
                ThreadEvent::Exited => {
                    let mut state = inner.state.lock().await;
                    let is_current_generation = state.generation == generation;
                    if is_current_generation {
                        state.active_turn_id = None;
                    }
                    drop(state);
                    // Reconfiguration intentionally replaces the App Server process. Its old
                    // dispatcher may observe the expected exit after the new generation is
                    // already active; that must not surface as a crash/error in the UI.
                    if is_current_generation {
                        let _ = inner.events.send(AgentEvent::ProcessExited);
                    }
                    break;
                }
            }
        }
    });
}

async fn update_lifecycle(inner: &Inner, generation: u64, method: &str, params: &Value) {
    let mut state = inner.state.lock().await;
    if state.generation != generation {
        return;
    }
    match method {
        "turn/started" => {
            state.active_turn_id = params
                .pointer("/turn/id")
                .or_else(|| params.get("turnId"))
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        "turn/completed" => {
            let completed = params
                .pointer("/turn/id")
                .or_else(|| params.get("turnId"))
                .and_then(Value::as_str);
            if completed.is_none() || completed == state.active_turn_id.as_deref() {
                state.active_turn_id = None;
            }
        }
        _ => {}
    }
}

fn parse_tool_call(params: &Value) -> Result<ToolCall, String> {
    let thread_id = required_string(params, "threadId")?;
    let tool = required_string(params, "tool")?;
    let arguments = match params.get("arguments") {
        None | Some(Value::Null) => json!({}),
        Some(Value::String(encoded)) => serde_json::from_str(encoded)
            .map_err(|error| format!("invalid stringified tool arguments: {error}"))?,
        Some(arguments) => arguments.clone(),
    };
    if !arguments.is_object() {
        return Err(format!("tool {tool:?} arguments must be a JSON object"));
    }
    Ok(ToolCall {
        thread_id,
        turn_id: optional_string(params, "turnId"),
        call_id: optional_string(params, "callId"),
        namespace: optional_string(params, "namespace"),
        tool,
        arguments,
    })
}

fn required_string(params: &Value, key: &str) -> Result<String, String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("item/tool/call is missing {key}"))
}

fn optional_string(params: &Value, key: &str) -> Option<String> {
    params.get(key).and_then(Value::as_str).map(str::to_string)
}

fn tool_call_response(result: &Result<Value, String>) -> Value {
    let (success, text) = match result {
        Ok(Value::String(text)) => (true, text.clone()),
        Ok(value) => (true, value.to_string()),
        Err(error) => (false, format!("ERROR: {error}")),
    };
    json!({
        "success": success,
        "contentItems": [{ "type": "inputText", "text": text }],
    })
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;
    use crate::codex::DynamicToolSpec;

    struct TestTools;

    #[async_trait]
    impl ToolExecutor for TestTools {
        fn definitions(&self) -> Vec<DynamicToolSpec> {
            vec![
                DynamicToolSpec::new(
                    "recipes_search",
                    "Search recipes",
                    json!({ "type": "object", "properties": {} }),
                )
                .unwrap(),
            ]
        }

        async fn execute(&self, _: ToolCall) -> Result<Value, String> {
            Ok(json!({ "ok": true }))
        }
    }

    fn config() -> AgentConfig {
        AgentConfig::new(
            std::env::temp_dir(),
            "You are the personal MealZ food agent.",
        )
    }

    #[test]
    fn start_declares_tools_but_resume_does_not() {
        let start = thread_start_params(&config(), &TestTools);
        assert_eq!(start["sandbox"], "read-only");
        assert_eq!(start["approvalPolicy"], "never");
        assert_eq!(start["dynamicTools"][0]["name"], "recipes_search");
        assert_eq!(start["dynamicTools"][0]["inputSchema"]["type"], "object");

        let resume = thread_resume_params("thread-1", &config());
        assert_eq!(resume["threadId"], "thread-1");
        assert!(resume.get("dynamicTools").is_none());
        assert!(
            resume["developerInstructions"]
                .as_str()
                .unwrap()
                .contains("MealZ")
        );
    }

    #[test]
    fn tool_calls_accept_object_null_and_stringified_arguments() {
        let base = json!({
            "threadId": "thread-1",
            "turnId": "turn-1",
            "callId": "call-1",
            "tool": "recipes_search",
        });
        let call = parse_tool_call(&base).unwrap();
        assert_eq!(call.arguments, json!({}));

        let mut stringified = base.clone();
        stringified["arguments"] = json!("{\"query\":\"Lasagne\"}");
        assert_eq!(
            parse_tool_call(&stringified).unwrap().arguments["query"],
            "Lasagne"
        );

        let mut invalid = base;
        invalid["arguments"] = json!([1, 2]);
        assert!(parse_tool_call(&invalid).is_err());
    }

    #[test]
    fn tool_results_match_dynamic_tool_response_shape() {
        let ok = tool_call_response(&Ok(json!({ "recipes": 3 })));
        assert_eq!(ok["success"], true);
        assert_eq!(ok["contentItems"][0]["type"], "inputText");
        assert!(
            ok["contentItems"][0]["text"]
                .as_str()
                .unwrap()
                .contains("recipes")
        );

        let error = tool_call_response(&Err("database busy".into()));
        assert_eq!(error["success"], false);
        assert!(
            error["contentItems"][0]["text"]
                .as_str()
                .unwrap()
                .starts_with("ERROR:")
        );
    }

    #[test]
    fn dynamic_tool_errors_are_actionable() {
        let message = guard_dynamic_tools_error("unknown field `dynamicTools`".into());
        assert!(message.contains("update Codex"));
    }

    /// Manual acceptance probe: starts a persistent thread, runs a real model
    /// turn, receives `item/tool/call`, answers it through ToolExecutor and
    /// observes turn completion over the streaming notification route.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "requires an installed/authenticated Codex CLI and consumes one model turn"]
    async fn real_dynamic_tool_turn_roundtrip() {
        let mut config = config();
        config.effort = Some("low".into());
        let agent = CodexAgent::new(
            config,
            Arc::new(TestTools),
            Arc::new(crate::codex::MemoryThreadStore::default()),
        );
        let mut events = agent.subscribe();
        agent.start_or_resume().await.expect("start thread");
        agent
            .send_message(
                "Call the recipes_search tool exactly once with an empty object, then reply with TOOL_OK.",
            )
            .await
            .expect("start turn");

        let (saw_tool, saw_completed) =
            tokio::time::timeout(std::time::Duration::from_secs(180), async {
                let mut saw_tool = false;
                loop {
                    match events.recv().await.expect("event stream") {
                        AgentEvent::ToolCompleted { success, call, .. } => {
                            assert!(success);
                            assert_eq!(call.tool, "recipes_search");
                            saw_tool = true;
                        }
                        AgentEvent::Notification { method, .. } if method == "turn/completed" => {
                            break (saw_tool, true);
                        }
                        _ => {}
                    }
                }
            })
            .await
            .expect("live turn timed out");
        assert!(saw_tool, "model did not call the declared dynamic tool");
        assert!(saw_completed);
        agent.shutdown().await;
    }
}
