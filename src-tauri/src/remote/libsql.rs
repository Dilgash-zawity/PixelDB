//! libSQL over HTTP (the Hrana pipeline API that sqld and Turso serve).

use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value as JsonValue, json};

use super::{CONNECT_TIMEOUT, QUERY_TIMEOUT, SAFE_INTEGER, binary_label, tables_from};
use crate::errors::{AppError, AppResult};
use crate::models::{Connection, RemoteResult, RemoteTable};

/// `libsql://host` is HTTPS; http(s) URLs are used as given.
fn pipeline_url(url: &str) -> String {
    let base = url.trim_end_matches('/');
    let base = match base.get(..9) {
        Some(s) if s.eq_ignore_ascii_case("libsql://") => format!("https://{}", &base[9..]),
        _ => base.to_string(),
    };
    format!("{base}/v2/pipeline")
}

#[derive(Deserialize)]
struct Pipeline {
    results: Vec<StreamResult>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamResult {
    Ok { response: JsonValue },
    Error { error: HranaError },
}

#[derive(Deserialize)]
struct HranaError {
    message: String,
}

#[derive(Deserialize)]
struct ExecuteResult {
    cols: Vec<Col>,
    rows: Vec<Vec<HranaValue>>,
}

#[derive(Deserialize)]
struct Col {
    name: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum HranaValue {
    Null,
    Integer { value: String },
    Float { value: f64 },
    Text { value: String },
    Blob { base64: String },
}

impl HranaValue {
    fn into_json(self) -> JsonValue {
        match self {
            HranaValue::Null => JsonValue::Null,
            HranaValue::Integer { value } => value
                .parse::<i64>()
                .ok()
                .filter(|n| n.abs() <= SAFE_INTEGER)
                .map_or(JsonValue::String(value), JsonValue::from),
            HranaValue::Float { value } => serde_json::Number::from_f64(value).map_or(JsonValue::Null, JsonValue::Number),
            HranaValue::Text { value } => JsonValue::String(value),
            // Base64 is 4 characters per 3 bytes.
            HranaValue::Blob { base64 } => binary_label(base64.trim_end_matches('=').len() * 3 / 4),
        }
    }
}

/// Runs `sql` between BEGIN and ROLLBACK in one pipeline request.
pub async fn run(c: &Connection, sql: &str, limit: usize) -> AppResult<RemoteResult> {
    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(CONNECT_TIMEOUT + QUERY_TIMEOUT - Duration::from_secs(1))
        .build()
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let body = json!({
        "requests": [
            { "type": "execute", "stmt": { "sql": "BEGIN" } },
            { "type": "execute", "stmt": { "sql": sql } },
            { "type": "execute", "stmt": { "sql": "ROLLBACK" } },
            { "type": "close" }
        ]
    });
    let mut request = client.post(pipeline_url(&c.url)).json(&body);
    if !c.token.is_empty() {
        request = request.bearer_auth(&c.token);
    }
    let response = request.send().await.map_err(|e| AppError::Remote(format!("couldn't reach the server: {}", without_url(&e))))?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(AppError::Remote("the server refused the access token".into()));
    }
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(AppError::Remote(format!("the server answered {status}{}", if text.is_empty() { String::new() } else { format!(": {}", text.trim()) })));
    }
    let pipeline: Pipeline = response.json().await.map_err(|e| AppError::Remote(format!("unexpected answer from the server: {}", without_url(&e))))?;
    let mut results = pipeline.results.into_iter();
    let (Some(begin), Some(main)) = (results.next(), results.next()) else {
        return Err(AppError::Remote("unexpected answer from the server".into()));
    };
    if let StreamResult::Error { error } = begin {
        return Err(AppError::Remote(error.message));
    }
    let response = match main {
        StreamResult::Error { error } => return Err(AppError::Remote(error.message)),
        StreamResult::Ok { response } => response,
    };
    let result: ExecuteResult = serde_json::from_value(response.get("result").cloned().unwrap_or(JsonValue::Null))
        .map_err(|e| AppError::Remote(format!("unexpected answer from the server: {e}")))?;
    let truncated = result.rows.len() > limit;
    Ok(RemoteResult {
        columns: result.cols.into_iter().map(|c| c.name.unwrap_or_default()).collect(),
        rows: result.rows.into_iter().take(limit).map(|r| r.into_iter().map(HranaValue::into_json).collect()).collect(),
        truncated,
        elapsed_ms: 0,
    })
}

/// reqwest errors print the URL; keep messages short.
fn without_url(e: &reqwest::Error) -> String {
    let mut msg = e.to_string();
    if let Some(url) = e.url() {
        msg = msg.replace(&format!(" for url ({url})"), "");
    }
    msg
}

pub async fn version(c: &Connection) -> AppResult<String> {
    let r = run(c, "SELECT sqlite_version()", 1).await?;
    let v = r.rows.first().and_then(|row| row.first()).and_then(|v| v.as_str()).unwrap_or("");
    Ok(format!("libSQL (SQLite {v})"))
}

pub async fn tables(c: &Connection) -> AppResult<Vec<RemoteTable>> {
    let r = run(
        c,
        "SELECT name, type FROM sqlite_schema WHERE type IN ('table', 'view') \
         AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' AND name NOT LIKE 'libsql\\_%' ESCAPE '\\' \
         AND name NOT LIKE '\\_litestream%' ESCAPE '\\' AND name NOT LIKE '\\_\\_app\\_%' ESCAPE '\\' ORDER BY name",
        100_000,
    )
    .await?;
    Ok(tables_from(r, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        assert_eq!(pipeline_url("libsql://shop-alex.turso.io"), "https://shop-alex.turso.io/v2/pipeline");
        assert_eq!(pipeline_url("http://localhost:8080/"), "http://localhost:8080/v2/pipeline");
    }

    /// Serves one canned pipeline answer and returns the request it got.
    fn fake_server(answer: &'static str) -> (String, std::thread::JoinHandle<String>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = socket.read(&mut buf).unwrap();
                request.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&request);
                if let Some(head) = text.find("\r\n\r\n") {
                    let len = text[..head]
                        .lines()
                        .find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap()))
                        .unwrap_or(0);
                    if request.len() >= head + 4 + len {
                        break;
                    }
                }
            }
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}", answer.len()).unwrap();
            String::from_utf8_lossy(&request).into_owned()
        });
        (url, handle)
    }

    #[test]
    fn runs_through_a_pipeline() {
        let (url, server) = fake_server(
            r#"{"baton":null,"results":[{"type":"ok","response":{"type":"execute","result":{"cols":[{"name":"n"}],"rows":[[{"type":"integer","value":"1"}],[{"type":"integer","value":"2"}]]}}},{"type":"ok","response":{"type":"execute","result":{"cols":[{"name":"n"}],"rows":[[{"type":"integer","value":"1"}],[{"type":"integer","value":"2"}]]}}},{"type":"ok","response":{"type":"execute","result":{"cols":[],"rows":[]}}},{"type":"ok","response":{"type":"close"}}]}"#,
        );
        let conn = Connection {
            name: "t".into(),
            provider: crate::models::Provider::Turso,
            host: String::new(),
            port: None,
            database: String::new(),
            user: String::new(),
            password: String::new(),
            url,
            token: "tok".into(),
            tls: crate::models::TlsMode::Require,
            trust_certificate: false,
        };
        let result = tauri::async_runtime::block_on(run(&conn, "SELECT n FROM t", 1)).unwrap();
        assert_eq!(result.columns, vec!["n"]);
        assert_eq!(result.rows, vec![vec![json!(1)]]);
        assert!(result.truncated);
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /v2/pipeline"));
        assert!(request.to_ascii_lowercase().contains("authorization: bearer tok"));
        assert!(request.contains(r#""sql":"BEGIN""#) && request.contains(r#""sql":"ROLLBACK""#));
    }

    #[test]
    fn values() {
        let v: Vec<HranaValue> = serde_json::from_value(json!([
            {"type": "integer", "value": "7"}, {"type": "float", "value": 1.5}, {"type": "null"},
            {"type": "text", "value": "a"}, {"type": "blob", "base64": "AAEC"}
        ]))
        .unwrap();
        let out: Vec<JsonValue> = v.into_iter().map(HranaValue::into_json).collect();
        assert_eq!(out, vec![json!(7), json!(1.5), JsonValue::Null, json!("a"), json!("<binary, 3 bytes>")]);
    }
}
