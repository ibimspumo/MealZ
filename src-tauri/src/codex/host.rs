//! Long-lived `codex app-server` process host.
//!
//! One process can multiplex many Codex threads. MealZ currently registers
//! its durable personal-agent thread while retaining a generic route table so
//! additional focused food conversations can be added without another host.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use parking_lot::{Mutex, RwLock};
use serde_json::{Value, json};
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot, watch};

use super::protocol::{self, Incoming};

pub const RPC_TIMEOUT_MS: u64 = 30_000;
pub const THREAD_TIMEOUT_MS: u64 = 120_000;
pub const TURN_TIMEOUT_MS: u64 = 30 * 60 * 1_000;

const SHUTDOWN_KILL_AFTER_MS: u64 = 5_000;
const MAX_PROTOCOL_LINE_BYTES: usize = 4 * 1024 * 1024;
const MAX_STDERR_LINE_BYTES: usize = 16 * 1024;
const EVENT_CHANNEL_CAPACITY: usize = 128;
const ROUTE_CHANNEL_CAPACITY: usize = 128;
const STDIN_CHANNEL_CAPACITY: usize = 256;

#[derive(Debug)]
pub enum ServerEvent {
    Request {
        id: Value,
        method: String,
        params: Value,
    },
    Notification {
        method: String,
        params: Value,
    },
    Exited,
}

#[derive(Default)]
struct PendingRpc {
    inner: Mutex<HashMap<u64, oneshot::Sender<Result<Value, String>>>>,
}

impl PendingRpc {
    fn register(&self, id: u64) -> oneshot::Receiver<Result<Value, String>> {
        let (sender, receiver) = oneshot::channel();
        self.inner.lock().insert(id, sender);
        receiver
    }

    fn remove(&self, id: u64) {
        self.inner.lock().remove(&id);
    }

    fn resolve(&self, id: u64, result: Result<Value, String>) -> bool {
        self.inner
            .lock()
            .remove(&id)
            .is_some_and(|sender| sender.send(result).is_ok())
    }

    fn fail_all(&self, reason: &str) {
        for (_, sender) in self.inner.lock().drain() {
            let _ = sender.send(Err(reason.to_string()));
        }
    }
}

enum BoundedLine {
    Line(String),
    Oversize,
    Eof,
}

async fn next_line_bounded<R>(reader: &mut R, max_bytes: usize) -> std::io::Result<BoundedLine>
where
    R: tokio::io::AsyncBufRead + Unpin,
{
    use tokio::io::AsyncBufReadExt;

    let mut buffer = Vec::new();
    let mut skipping = false;
    loop {
        let chunk = reader.fill_buf().await?;
        if chunk.is_empty() {
            return Ok(if skipping {
                BoundedLine::Oversize
            } else if buffer.is_empty() {
                BoundedLine::Eof
            } else {
                BoundedLine::Line(String::from_utf8_lossy(&buffer).into_owned())
            });
        }

        if let Some(newline) = chunk.iter().position(|byte| *byte == b'\n') {
            let oversize = !skipping && buffer.len() + newline > max_bytes;
            if !skipping && !oversize {
                buffer.extend_from_slice(&chunk[..newline]);
            }
            reader.consume(newline + 1);
            if skipping || oversize {
                return Ok(BoundedLine::Oversize);
            }
            if buffer.last() == Some(&b'\r') {
                buffer.pop();
            }
            return Ok(BoundedLine::Line(
                String::from_utf8_lossy(&buffer).into_owned(),
            ));
        }

        let len = chunk.len();
        if !skipping {
            if buffer.len() + len > max_bytes {
                buffer.clear();
                skipping = true;
            } else {
                buffer.extend_from_slice(chunk);
            }
        }
        reader.consume(len);
    }
}

/// Handle to one child process and its ordered stdio pumps.
pub struct Client {
    stdin_tx: Mutex<Option<mpsc::Sender<String>>>,
    pending: Arc<PendingRpc>,
    next_id: AtomicU64,
    alive: Arc<AtomicBool>,
    kill: watch::Sender<bool>,
}

impl Client {
    pub async fn spawn(program: &Path, events: mpsc::Sender<ServerEvent>) -> Result<Self, String> {
        let mut command = Command::new(program);
        command
            .arg("app-server")
            // MealZ exposes its own domain tools via dynamicTools. Disable the
            // unrelated built-in `codex_apps` MCP surface for this child;
            // provider-native web search and image generation stay available.
            .args(["--disable", "apps"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // A bundled macOS application inherits a tiny PATH. Keep the user's
        // existing path and prepend the resolved Codex directory so helpers
        // spawned by Codex can locate their sibling binaries.
        if let Some(directory) = program.parent() {
            let current = std::env::var("PATH").unwrap_or_default();
            command.env("PATH", format!("{}:{current}", directory.display()));
        }

        let mut child = command.spawn().map_err(|error| {
            format!(
                "failed to start `{} app-server`: {error}",
                program.display()
            )
        })?;
        let mut stdin = child.stdin.take().ok_or("Codex App Server has no stdin")?;
        let stdout = child
            .stdout
            .take()
            .ok_or("Codex App Server has no stdout")?;
        let stderr = child.stderr.take();

        let (stdin_tx, mut stdin_rx) = mpsc::channel::<String>(STDIN_CHANNEL_CAPACITY);
        let pending = Arc::new(PendingRpc::default());
        let alive = Arc::new(AtomicBool::new(true));

        tokio::spawn(async move {
            while let Some(line) = stdin_rx.recv().await {
                if stdin.write_all(line.as_bytes()).await.is_err()
                    || stdin.write_all(b"\n").await.is_err()
                    || stdin.flush().await.is_err()
                {
                    break;
                }
            }
        });

        if let Some(stderr) = stderr {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr);
                loop {
                    match next_line_bounded(&mut reader, MAX_STDERR_LINE_BYTES).await {
                        Ok(BoundedLine::Line(line)) => tracing::debug!(
                            target: "mealz::codex_app_server",
                            "{line}"
                        ),
                        Ok(BoundedLine::Oversize) => tracing::warn!(
                            target: "mealz::codex_app_server",
                            "oversized stderr line was dropped"
                        ),
                        Ok(BoundedLine::Eof) | Err(_) => break,
                    }
                }
            });
        }

        let (kill, mut kill_rx) = watch::channel(false);
        {
            let pending = pending.clone();
            let alive = alive.clone();
            tokio::spawn(async move {
                let mut stdout = BufReader::new(stdout);
                'read: loop {
                    tokio::select! {
                        biased;
                        changed = kill_rx.changed() => {
                            if changed.is_err() || *kill_rx.borrow() {
                                let _ = child.kill().await;
                                break 'read;
                            }
                        }
                        line = next_line_bounded(&mut stdout, MAX_PROTOCOL_LINE_BYTES) => {
                            let line = match line {
                                Ok(BoundedLine::Line(line)) => line,
                                Ok(BoundedLine::Oversize) => {
                                    tracing::warn!(target: "mealz::codex_app_server", "oversized protocol line was dropped");
                                    continue;
                                }
                                Ok(BoundedLine::Eof) | Err(_) => break,
                            };

                            match protocol::parse_line(&line) {
                                Some(Incoming::Response { id, result }) => {
                                    if !pending.resolve(id, result) {
                                        tracing::debug!(target: "mealz::codex_app_server", id, "response for expired request ignored");
                                    }
                                }
                                Some(Incoming::ServerRequest { id, method, params }) => {
                                    let delivered = tokio::select! {
                                        biased;
                                        _ = kill_rx.changed() => {
                                            let _ = child.kill().await;
                                            false
                                        }
                                        result = events.send(ServerEvent::Request { id, method, params }) => result.is_ok(),
                                    };
                                    if !delivered {
                                        break;
                                    }
                                }
                                Some(Incoming::Notification { method, params }) => {
                                    let delivered = tokio::select! {
                                        biased;
                                        _ = kill_rx.changed() => {
                                            let _ = child.kill().await;
                                            false
                                        }
                                        result = events.send(ServerEvent::Notification { method, params }) => result.is_ok(),
                                    };
                                    if !delivered {
                                        break;
                                    }
                                }
                                None => tracing::trace!(target: "mealz::codex_app_server", "unrecognized stdout line ignored"),
                            }
                        }
                    }
                }

                alive.store(false, Ordering::SeqCst);
                pending.fail_all("Codex App Server exited");
                let _ = child.wait().await;
                let _ = events.send(ServerEvent::Exited).await;
            });
        }

        Ok(Self {
            stdin_tx: Mutex::new(Some(stdin_tx)),
            pending,
            next_id: AtomicU64::new(1),
            alive,
            kill,
        })
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    fn send_line(&self, line: String) -> Result<(), String> {
        self.stdin_tx
            .lock()
            .as_ref()
            .ok_or_else(|| "Codex App Server stdin is closed".to_string())?
            .try_send(line)
            .map_err(|_| "Codex App Server stdin queue is full or closed".to_string())
    }

    pub async fn request(
        &self,
        method: &str,
        params: Value,
        timeout_ms: u64,
    ) -> Result<Value, String> {
        if !self.is_alive() {
            return Err("Codex App Server is not running".into());
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let receiver = self.pending.register(id);
        if let Err(error) = self.send_line(protocol::request_line(id, method, &params)) {
            self.pending.remove(id);
            return Err(error);
        }

        match tokio::time::timeout(Duration::from_millis(timeout_ms), receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(format!("{method}: response channel dropped")),
            Err(_) => {
                self.pending.remove(id);
                Err(format!("{method} timed out after {timeout_ms} ms"))
            }
        }
    }

    pub fn notify(&self, method: &str, params: Option<&Value>) -> Result<(), String> {
        self.send_line(protocol::notification_line(method, params))
    }

    pub fn respond(&self, id: &Value, result: &Value) {
        let _ = self.send_line(protocol::response_line(id, result));
    }

    pub fn respond_error(&self, id: &Value, code: i64, message: &str) {
        let _ = self.send_line(protocol::error_response_line(id, code, message));
    }

    pub fn shutdown(&self) {
        if self.stdin_tx.lock().take().is_none() {
            return;
        }
        let alive = self.alive.clone();
        let kill = self.kill.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(SHUTDOWN_KILL_AFTER_MS)).await;
            if alive.load(Ordering::SeqCst) {
                let _ = kill.send(true);
            }
        });
    }
}

async fn handshake(client: &Client) -> Result<String, String> {
    let response = client
        .request(
            "initialize",
            json!({
                "clientInfo": {
                    "name": "MealZ",
                    "title": "MealZ",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "capabilities": {
                    "experimentalApi": true,
                    "optOutNotificationMethods": ["mcpServer/startupStatus/updated"],
                },
            }),
            RPC_TIMEOUT_MS,
        )
        .await?;
    client.notify("initialized", None)?;
    Ok(response
        .get("userAgent")
        .and_then(Value::as_str)
        .unwrap_or("unknown Codex version")
        .to_string())
}

fn first_executable(mut paths: impl Iterator<Item = PathBuf>) -> Option<PathBuf> {
    paths.find(|path| path.is_file())
}

fn normalize_override(path: &Path) -> Result<PathBuf, String> {
    if !path.is_file() {
        return Err(format!(
            "configured Codex binary does not exist: {}",
            path.display()
        ));
    }
    path.canonicalize()
        .map_err(|error| format!("cannot resolve Codex binary {}: {error}", path.display()))
}

/// Resolve `codex` to an absolute path for a packaged macOS application.
/// Search order: explicit override, inherited PATH, common package-manager
/// locations, and finally a login-shell probe for nvm/asdf/mise setups.
pub fn resolve_codex_program(override_path: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(path) = override_path {
        return normalize_override(path);
    }

    if let Some(path) = std::env::var_os("PATH")
        && let Some(found) =
            first_executable(std::env::split_paths(&path).map(|dir| dir.join("codex")))
    {
        return normalize_override(&found);
    }

    let mut known = vec![
        PathBuf::from("/opt/homebrew/bin/codex"),
        PathBuf::from("/usr/local/bin/codex"),
    ];
    if let Some(home) = dirs::home_dir() {
        for relative in [
            ".local/bin/codex",
            ".bun/bin/codex",
            ".volta/bin/codex",
            ".cargo/bin/codex",
            ".npm-global/bin/codex",
        ] {
            known.push(home.join(relative));
        }
    }
    if let Some(found) = first_executable(known.into_iter()) {
        return normalize_override(&found);
    }

    let output = std::process::Command::new("/bin/zsh")
        .args(["-lc", "command -v codex"])
        .output();
    if let Ok(output) = output
        && output.status.success()
    {
        let path = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
        if path.is_file() {
            return normalize_override(&path);
        }
    }

    Err("Codex CLI was not found; install it or choose its binary in MealZ settings".into())
}

#[derive(Clone)]
pub struct Responder {
    client: Arc<Client>,
    id: Value,
}

impl Responder {
    pub fn ok(&self, result: &Value) {
        self.client.respond(&self.id, result);
    }

    pub fn error(&self, code: i64, message: &str) {
        self.client.respond_error(&self.id, code, message);
    }
}

impl std::fmt::Debug for Responder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Responder")
            .field("id", &self.id)
            .finish()
    }
}

#[derive(Debug)]
pub enum ThreadEvent {
    Request {
        method: String,
        params: Value,
        responder: Responder,
    },
    Notification {
        method: String,
        params: Value,
    },
    Exited,
}

pub type EventSink = mpsc::Sender<ThreadEvent>;

#[derive(Default)]
struct RouteTable {
    inner: Mutex<HashMap<String, EventSink>>,
}

impl RouteTable {
    fn insert(&self, thread_id: &str, sink: EventSink) {
        self.inner.lock().insert(thread_id.to_string(), sink);
    }

    fn remove(&self, thread_id: &str) {
        self.inner.lock().remove(thread_id);
    }

    fn get(&self, thread_id: &str) -> Option<EventSink> {
        self.inner.lock().get(thread_id).cloned()
    }

    fn distinct(&self) -> Vec<EventSink> {
        let mut distinct = Vec::new();
        for sink in self.inner.lock().values() {
            if !distinct
                .iter()
                .any(|candidate: &EventSink| candidate.same_channel(sink))
            {
                distinct.push(sink.clone());
            }
        }
        distinct
    }

    fn drain_distinct(&self) -> Vec<EventSink> {
        let sinks: Vec<_> = self.inner.lock().drain().map(|(_, sink)| sink).collect();
        let mut distinct = Vec::new();
        for sink in sinks {
            if !distinct
                .iter()
                .any(|candidate: &EventSink| candidate.same_channel(&sink))
            {
                distinct.push(sink);
            }
        }
        distinct
    }
}

pub struct Connection {
    client: Arc<Client>,
    routes: Arc<RouteTable>,
    version: String,
}

impl Connection {
    async fn open(program: &Path) -> Result<Arc<Self>, String> {
        let (event_tx, event_rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
        let client = Arc::new(Client::spawn(program, event_tx).await?);
        let routes = Arc::new(RouteTable::default());
        spawn_router(client.clone(), routes.clone(), event_rx);
        let version = match handshake(&client).await {
            Ok(version) => version,
            Err(error) => {
                client.shutdown();
                return Err(format!("Codex App Server initialize failed: {error}"));
            }
        };
        Ok(Arc::new(Self {
            client,
            routes,
            version,
        }))
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn is_alive(&self) -> bool {
        self.client.is_alive()
    }

    pub async fn request(
        &self,
        method: &str,
        params: Value,
        timeout_ms: u64,
    ) -> Result<Value, String> {
        self.client.request(method, params, timeout_ms).await
    }

    pub fn register_thread(&self, thread_id: &str, sink: EventSink) {
        self.routes.insert(thread_id, sink);
    }

    pub fn unregister_thread(&self, thread_id: &str) {
        self.routes.remove(thread_id);
    }

    pub fn shutdown(&self) {
        self.client.shutdown();
    }
}

fn spawn_router(
    client: Arc<Client>,
    routes: Arc<RouteTable>,
    mut events: mpsc::Receiver<ServerEvent>,
) {
    tokio::spawn(async move {
        while let Some(event) = events.recv().await {
            match event {
                ServerEvent::Request { id, method, params } => {
                    let responder = Responder {
                        client: client.clone(),
                        id,
                    };
                    let sink = params
                        .get("threadId")
                        .and_then(Value::as_str)
                        .and_then(|thread_id| routes.get(thread_id));
                    match sink {
                        Some(sink) => {
                            match sink
                                .send(ThreadEvent::Request {
                                    method,
                                    params,
                                    responder,
                                })
                                .await
                            {
                                Ok(()) => {}
                                Err(error) => {
                                    let ThreadEvent::Request { responder, .. } = error.0 else {
                                        continue;
                                    };
                                    responder.error(-32601, "MealZ thread consumer is unavailable");
                                }
                            }
                        }
                        None => {
                            responder.error(-32601, "no MealZ consumer registered for this thread")
                        }
                    }
                }
                ServerEvent::Notification { method, params } => {
                    if let Some(thread_id) = params.get("threadId").and_then(Value::as_str) {
                        if let Some(sink) = routes.get(thread_id) {
                            let _ = sink
                                .send(ThreadEvent::Notification { method, params })
                                .await;
                        }
                    } else {
                        // Account/rate-limit notifications are process-global.
                        for sink in routes.distinct() {
                            let _ = sink
                                .send(ThreadEvent::Notification {
                                    method: method.clone(),
                                    params: params.clone(),
                                })
                                .await;
                        }
                    }
                }
                ServerEvent::Exited => {
                    for sink in routes.drain_distinct() {
                        let _ = sink.send(ThreadEvent::Exited).await;
                    }
                    break;
                }
            }
        }
    });
}

#[derive(Default)]
struct ProcessHostState {
    connection: Option<Arc<Connection>>,
    generation: u64,
    last_version: Option<String>,
}

/// A lazily spawned, transparently replaceable Codex App Server slot.
pub struct ProcessHost {
    state: tokio::sync::Mutex<ProcessHostState>,
    binary_override: RwLock<Option<PathBuf>>,
}

impl ProcessHost {
    pub fn new(binary_override: Option<PathBuf>) -> Self {
        Self {
            state: tokio::sync::Mutex::new(ProcessHostState::default()),
            binary_override: RwLock::new(binary_override),
        }
    }

    pub fn set_binary_override(&self, binary_override: Option<PathBuf>) {
        *self.binary_override.write() = binary_override;
    }

    pub async fn ensure(&self) -> Result<(Arc<Connection>, u64), String> {
        let mut state = self.state.lock().await;
        if let Some(connection) = &state.connection
            && connection.is_alive()
        {
            return Ok((connection.clone(), state.generation));
        }

        let binary_override = self.binary_override.read().clone();
        let program =
            tokio::task::spawn_blocking(move || resolve_codex_program(binary_override.as_deref()))
                .await
                .map_err(|error| error.to_string())??;
        let connection = Connection::open(&program).await?;
        state.generation += 1;
        state.last_version = Some(connection.version().to_string());
        state.connection = Some(connection.clone());
        Ok((connection, state.generation))
    }

    pub async fn alive(&self) -> Option<Arc<Connection>> {
        self.state
            .lock()
            .await
            .connection
            .as_ref()
            .filter(|connection| connection.is_alive())
            .cloned()
    }

    pub async fn last_version(&self) -> Option<String> {
        self.state.lock().await.last_version.clone()
    }

    pub async fn shutdown(&self) {
        if let Some(connection) = self.state.lock().await.connection.take() {
            connection.shutdown();
        }
    }
}

impl Default for ProcessHost {
    fn default() -> Self {
        Self::new(None)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ResumeError {
    ThreadNotFound(String),
    Other(String),
}

impl ResumeError {
    pub fn message(&self) -> &str {
        match self {
            ResumeError::ThreadNotFound(message) | ResumeError::Other(message) => message,
        }
    }
}

pub fn is_unknown_thread_error(error: &str) -> bool {
    let error = error.to_lowercase();
    [
        "no rollout found",
        "unknown thread",
        "no such thread",
        "does not exist",
        "not found",
    ]
    .iter()
    .any(|needle| error.contains(needle))
}

pub async fn resume_thread(connection: &Connection, params: Value) -> Result<Value, ResumeError> {
    connection
        .request("thread/resume", params, THREAD_TIMEOUT_MS)
        .await
        .map_err(|error| {
            if is_unknown_thread_error(&error) {
                ResumeError::ThreadNotFound(error)
            } else {
                ResumeError::Other(error)
            }
        })
}

pub fn route_channel() -> (EventSink, mpsc::Receiver<ThreadEvent>) {
    mpsc::channel(ROUTE_CHANNEL_CAPACITY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_thread_errors_are_classified_defensively() {
        for message in [
            "no rollout found for thread id x (code -32600)",
            "unknown thread x",
            "thread does not exist",
        ] {
            assert!(is_unknown_thread_error(message), "{message}");
        }
        assert!(!is_unknown_thread_error("authentication expired"));
    }

    #[test]
    fn explicit_binary_is_absolute_and_validated() {
        let shell = Path::new("/bin/sh");
        let resolved = resolve_codex_program(Some(shell)).unwrap();
        assert!(resolved.is_absolute());
        assert!(resolved.is_file());
        assert!(resolve_codex_program(Some(Path::new("/definitely/missing/codex"))).is_err());
    }

    #[tokio::test]
    async fn bounded_reader_drops_oversized_records_and_keeps_framing() {
        let bytes = b"0123456789\nOK\n";
        let mut reader = BufReader::new(&bytes[..]);
        assert!(matches!(
            next_line_bounded(&mut reader, 4).await.unwrap(),
            BoundedLine::Oversize
        ));
        match next_line_bounded(&mut reader, 4).await.unwrap() {
            BoundedLine::Line(line) => assert_eq!(line, "OK"),
            _ => panic!("expected second line"),
        }
    }

    /// Manual acceptance probe: exercises binary resolution, process spawn,
    /// JSONL framing, initialize/initialized and a real authenticated RPC.
    #[tokio::test]
    #[ignore = "requires an installed and authenticated Codex CLI"]
    async fn real_app_server_handshake_and_account_read() {
        let host = ProcessHost::default();
        let (connection, generation) = host.ensure().await.expect("live handshake");
        assert_eq!(generation, 1);
        assert!(!connection.version().is_empty());
        let account = connection
            .request("account/read", json!({}), RPC_TIMEOUT_MS)
            .await
            .expect("account/read");
        assert!(account.get("account").is_some());
        host.shutdown().await;
    }
}
