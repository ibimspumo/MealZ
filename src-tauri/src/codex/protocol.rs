//! Pure wire framing for `codex app-server`.
//!
//! The protocol is JSON-RPC-like JSONL over stdio, but deliberately omits
//! the `"jsonrpc":"2.0"` member. Shapes here are frozen against the live
//! Codex 0.144.1 fixtures used by the proven SwarmZ integration.

use serde_json::{Value, json};

#[derive(Debug, PartialEq)]
pub enum Incoming {
    Response {
        id: u64,
        result: Result<Value, String>,
    },
    ServerRequest {
        id: Value,
        method: String,
        params: Value,
    },
    Notification {
        method: String,
        params: Value,
    },
}

pub fn parse_line(line: &str) -> Option<Incoming> {
    let message: Value = serde_json::from_str(line).ok()?;
    let object = message.as_object()?;
    let method = object.get("method").and_then(Value::as_str);
    let has_id = object.contains_key("id");

    match (method, has_id) {
        (Some(method), true) => Some(Incoming::ServerRequest {
            id: object.get("id").cloned().unwrap_or(Value::Null),
            method: method.to_string(),
            params: object.get("params").cloned().unwrap_or(Value::Null),
        }),
        (Some(method), false) => Some(Incoming::Notification {
            method: method.to_string(),
            params: object.get("params").cloned().unwrap_or(Value::Null),
        }),
        (None, true) => {
            let id = object.get("id").and_then(Value::as_u64)?;
            let result = match object.get("error") {
                Some(error) => {
                    let message = error
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown Codex App Server error");
                    let code = error.get("code").and_then(Value::as_i64);
                    Err(match code {
                        Some(code) => format!("{message} (code {code})"),
                        None => message.to_string(),
                    })
                }
                None => Ok(object.get("result").cloned().unwrap_or(Value::Null)),
            };
            Some(Incoming::Response { id, result })
        }
        _ => None,
    }
}

pub fn request_line(id: u64, method: &str, params: &Value) -> String {
    json!({ "id": id, "method": method, "params": params }).to_string()
}

pub fn notification_line(method: &str, params: Option<&Value>) -> String {
    match params {
        Some(params) => json!({ "method": method, "params": params }).to_string(),
        None => json!({ "method": method }).to_string(),
    }
}

pub fn response_line(id: &Value, result: &Value) -> String {
    json!({ "id": id, "result": result }).to_string()
}

pub fn error_response_line(id: &Value, code: i64, message: &str) -> String {
    json!({ "id": id, "error": { "code": code, "message": message } }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESPONSE: &str = r#"{"id":3,"result":{"turn":{"id":"turn-1","status":"inProgress"}}}"#;
    const TOOL_CALL: &str = r#"{"method":"item/tool/call","id":0,"params":{"threadId":"thread-1","turnId":"turn-1","callId":"call-1","namespace":null,"tool":"recipes_search","arguments":{"query":"Lasagne"}}}"#;
    const DELTA: &str = r#"{"method":"item/agentMessage/delta","params":{"threadId":"thread-1","turnId":"turn-1","itemId":"message-1","delta":"Hallo"}}"#;
    const ERROR: &str =
        r#"{"id":7,"error":{"code":-32600,"message":"no rollout found for thread id thread-1"}}"#;

    #[test]
    fn classifies_live_wire_shapes() {
        match parse_line(RESPONSE) {
            Some(Incoming::Response { id, result }) => {
                assert_eq!(id, 3);
                assert_eq!(result.unwrap()["turn"]["status"], "inProgress");
            }
            other => panic!("expected response, got {other:?}"),
        }

        match parse_line(TOOL_CALL) {
            Some(Incoming::ServerRequest { id, method, params }) => {
                assert_eq!(id, json!(0));
                assert_eq!(method, "item/tool/call");
                assert_eq!(params["arguments"]["query"], "Lasagne");
            }
            other => panic!("expected server request, got {other:?}"),
        }

        match parse_line(DELTA) {
            Some(Incoming::Notification { method, params }) => {
                assert_eq!(method, "item/agentMessage/delta");
                assert_eq!(params["delta"], "Hallo");
            }
            other => panic!("expected notification, got {other:?}"),
        }

        match parse_line(ERROR) {
            Some(Incoming::Response { result, .. }) => {
                let error = result.unwrap_err();
                assert!(error.contains("no rollout found"));
                assert!(error.contains("-32600"));
            }
            other => panic!("expected error response, got {other:?}"),
        }
    }

    #[test]
    fn outgoing_messages_omit_jsonrpc_and_newlines() {
        for line in [
            request_line(1, "initialize", &json!({ "x": 1 })),
            notification_line("initialized", None),
            response_line(&json!(0), &json!({ "ok": true })),
            error_response_line(&json!(1), -32601, "unsupported"),
        ] {
            let value: Value = serde_json::from_str(&line).unwrap();
            assert!(value.get("jsonrpc").is_none());
            assert!(!line.contains('\n'));
        }
    }

    #[test]
    fn malformed_or_foreign_messages_are_ignored() {
        assert_eq!(parse_line("not json"), None);
        assert_eq!(parse_line("42"), None);
        assert_eq!(parse_line(r#"{"id":"foreign","result":{}}"#), None);
    }
}
