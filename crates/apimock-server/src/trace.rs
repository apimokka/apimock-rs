//! Live match-trace channel — RFC 006 (in-process) + RFC 009 (transport).
//!
//! # Architecture
//!
//! ```text
//!  HTTP handler ──► TraceEmitter::emit()
//!                         │
//!              tokio::sync::broadcast (bounded, 1024)
//!                         │
//!           ┌─────────────┴──────────────┐
//!      in-process                  TraceTransport::accept_loop
//!      subscriber                  (UDS on Unix, TCP fallback)
//!                                        │
//!                                  up to 4 GUI connections
//!                                  (newline-delimited JSON)
//! ```
//!
//! # Transport variants
//!
//! | `TraceTransportConfig` | Platform | Notes |
//! |---|---|---|
//! | `Uds { path }` | Unix/macOS | Default when available |
//! | `Tcp { addr }` | All | Portable fallback; `addr = "127.0.0.1:0"` assigns ephemeral port |
//! | `Disabled` | All | No out-of-process forwarding (default) |
//!
//! # Back-pressure
//!
//! The broadcast channel is bounded by [`TRACE_CHANNEL_CAPACITY`]. When
//! the channel is full, `emit` drops the event and increments an internal
//! counter; the count is reported as `dropped_count` on the next event.
//!
//! Slow out-of-process subscribers receive a `RecvError::Lagged` from the
//! broadcast channel; the gap is reported in the next JSON line via
//! `dropped_count`.
//!
//! # Subscriber cap
//!
//! At most [`MAX_SUBSCRIBERS`] out-of-process connections are accepted.
//! A fifth connection receives `{"error":"max_subscribers_reached"}` and
//! is then closed.

use std::sync::{
    Arc,
    atomic::{AtomicU32, AtomicUsize, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast;

/// Capacity of the broadcast channel (events).
pub const TRACE_CHANNEL_CAPACITY: usize = 1_024;
/// Maximum concurrent out-of-process subscriber connections.
pub const MAX_SUBSCRIBERS: usize = 4;

// ── Event schema ──────────────────────────────────────────────────────

/// A single request/response trace event.
#[derive(Clone, Debug, Serialize)]
pub struct MatchTraceEvent {
    /// Monotonically increasing event counter within this server run.
    pub event_id: u64,
    /// Schema version — bumped on breaking changes.
    pub schema_version: u8,
    /// Unix timestamp (milliseconds) when the request was received.
    pub received_at_ms: u64,
    /// Processing time in milliseconds.
    pub duration_ms: u32,
    /// Key fields from the incoming request.
    pub request: RequestSummary,
    /// What the server decided to do with the request.
    pub outcome: Outcome,
    /// Events dropped since the last successfully delivered event.
    pub dropped_count: u32,
}

/// Key fields from the incoming HTTP request.
#[derive(Clone, Debug, Serialize)]
pub struct RequestSummary {
    pub method: String,
    pub url_path: String,
    /// Request headers, redacted per `TraceConfig`'s policy at capture
    /// (RFC 040) — not otherwise filtered by name. A redacted header
    /// keeps its name and carries [`REDACTED_HEADER_VALUE`] instead of
    /// its real value, so its presence stays visible.
    pub headers: Vec<(String, String)>,
    /// Captured JSON body (RFC 023). Present only when `TraceConfig::capture_body`
    /// is `true` and the request body is valid JSON within the size cap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_json: Option<serde_json::Value>,
    /// `true` when the body was omitted because it exceeded `max_body_bytes`.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub body_truncated: bool,
}

impl RequestSummary {
    /// Construct from a live request's headers, applying `config`'s
    /// redaction policy (RFC 040). This is the only place header
    /// redaction happens — do not build `RequestSummary` from live
    /// request headers any other way; a future formatter or display
    /// path must not need to know about redaction at all.
    pub fn new(
        method: String,
        url_path: String,
        headers: Vec<(String, String)>,
        config: &TraceConfig,
    ) -> Self {
        Self {
            method,
            url_path,
            headers: config.redact_headers(headers),
            body_json: None,
            body_truncated: false,
        }
    }
}

/// Placeholder value substituted for a redacted header (RFC 040 Goal 4).
/// The header name is kept so a consumer can tell "redacted" from
/// "the request never sent this header" — only the value differs.
pub const REDACTED_HEADER_VALUE: &str = "[redacted]";

/// Built-in denylist of well-known credential-bearing request headers,
/// applied by default (RFC 040 Q1). Compared case-insensitively.
///
/// `set-cookie` is a *response* header and can never appear on a
/// request, so it never matches here — kept anyway because RFC 040's
/// own example list included it; dropping it silently would read as
/// deliberately narrowing the list rather than the request-only scope
/// this RFC already states.
pub const DEFAULT_HEADER_DENYLIST: &[&str] = &[
    "authorization",
    "cookie",
    "set-cookie",
    "proxy-authorization",
    "x-api-key",
];

/// Which request headers get redacted before a trace event is built
/// (RFC 040 Q1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeaderRedactionMode {
    /// Redact headers named in `TraceConfig::header_denylist`; capture
    /// everything else. Fails open on an unanticipated header name —
    /// accepted deliberately, see RFC 040's Risks table.
    Denylist,
    /// Capture only headers named in `TraceConfig::header_allowlist`;
    /// redact everything else. Fails closed.
    Allowlist,
}

/// Trace-channel behaviour configuration (RFC 023, extended by RFC 040).
///
/// Configurable today only at this Rust level — the trace channel has
/// no config-file or CLI surface yet (RFC 040's own Motivation notes
/// this; RFC 023's `[trace]` TOML section was never wired to this
/// struct). `TraceEmitter::new()` always uses `TraceConfig::default()`.
#[derive(Clone, Debug)]
pub struct TraceConfig {
    /// Capture the JSON request body in each event. Default: `false`.
    pub capture_body: bool,
    /// Maximum serialised body size in bytes. Bodies larger than this
    /// are omitted and `body_truncated = true` is set. Default: 8 192.
    pub max_body_bytes: usize,
    /// Denylist or allowlist by default. Default: `Denylist`.
    pub header_redaction: HeaderRedactionMode,
    /// Header names redacted when `header_redaction` is `Denylist`.
    /// Compared case-insensitively. Default: [`DEFAULT_HEADER_DENYLIST`].
    pub header_denylist: Vec<String>,
    /// Header names captured when `header_redaction` is `Allowlist`;
    /// every other header is redacted. Compared case-insensitively.
    /// Default: empty, i.e. allowlist mode redacts everything until
    /// configured — the safe direction for a fail-closed mode.
    pub header_allowlist: Vec<String>,
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self {
            capture_body: false,
            max_body_bytes: 8_192,
            header_redaction: HeaderRedactionMode::Denylist,
            header_denylist: DEFAULT_HEADER_DENYLIST
                .iter()
                .map(|s| s.to_string())
                .collect(),
            header_allowlist: Vec::new(),
        }
    }
}

impl TraceConfig {
    /// Redact header values per this config's policy. Names and order
    /// are preserved; only a matched entry's value is replaced with
    /// [`REDACTED_HEADER_VALUE`] (RFC 040 Goal 4 — marked, not omitted).
    fn redact_headers(&self, headers: Vec<(String, String)>) -> Vec<(String, String)> {
        headers
            .into_iter()
            .map(|(name, value)| {
                let redact = match self.header_redaction {
                    HeaderRedactionMode::Denylist => self
                        .header_denylist
                        .iter()
                        .any(|denied| denied.eq_ignore_ascii_case(&name)),
                    HeaderRedactionMode::Allowlist => !self
                        .header_allowlist
                        .iter()
                        .any(|allowed| allowed.eq_ignore_ascii_case(&name)),
                };
                if redact {
                    (name, REDACTED_HEADER_VALUE.to_string())
                } else {
                    (name, value)
                }
            })
            .collect()
    }
}

/// What the server decided to do with the request.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Outcome {
    Matched {
        rule_set_index: usize,
        rule_index: usize,
    },
    Fallback {
        file_path: String,
        status: u16,
    },
    Miss {
        status: u16,
    },
    Error {
        kind: String,
        message: String,
    },
}

// ── Emitter ───────────────────────────────────────────────────────────

/// Shared handle to the trace broadcast channel.
///
/// Clone freely — each clone refers to the same underlying channel.
#[derive(Clone)]
pub struct TraceEmitter {
    sender: broadcast::Sender<MatchTraceEvent>,
    event_counter: Arc<AtomicU32>,
    dropped_counter: Arc<AtomicU32>,
    /// Behaviour settings (body capture, etc.).
    pub config: Arc<TraceConfig>,
}

impl TraceEmitter {
    pub fn new() -> Self {
        Self::with_config(TraceConfig::default())
    }

    pub fn with_config(config: TraceConfig) -> Self {
        let (sender, _) = broadcast::channel(TRACE_CHANNEL_CAPACITY);
        Self {
            sender,
            event_counter: Arc::new(AtomicU32::new(0)),
            dropped_counter: Arc::new(AtomicU32::new(0)),
            config: Arc::new(config),
        }
    }

    /// Subscribe to the event stream (in-process).
    pub fn subscribe(&self) -> broadcast::Receiver<MatchTraceEvent> {
        self.sender.subscribe()
    }

    /// Attach body JSON to a `RequestSummary` according to this emitter's
    /// `TraceConfig`. Call before `emit` when the request body is available.
    pub fn enrich_with_body(
        &self,
        summary: &mut RequestSummary,
        body_json: Option<&serde_json::Value>,
    ) {
        if !self.config.capture_body {
            return;
        }
        match body_json {
            None => {} // non-JSON or empty body — leave body_json = None
            Some(v) => {
                // Check serialised size against the cap.
                match serde_json::to_string(v) {
                    Ok(s) if s.len() <= self.config.max_body_bytes => {
                        summary.body_json = Some(v.clone());
                    }
                    Ok(_) => {
                        summary.body_truncated = true;
                    }
                    Err(_) => {} // shouldn't happen for a valid Value
                }
            }
        }
    }

    /// Emit one event.  If the channel is full, the event is dropped and
    /// the internal drop counter incremented.
    pub fn emit(
        &self,
        received_at_ms: u64,
        duration_ms: u32,
        request: RequestSummary,
        outcome: Outcome,
    ) {
        let event_id = self.event_counter.fetch_add(1, Ordering::Relaxed) as u64;
        let dropped_count = self.dropped_counter.swap(0, Ordering::Relaxed);

        let event = MatchTraceEvent {
            event_id,
            schema_version: 1,
            received_at_ms,
            duration_ms,
            request,
            outcome,
            dropped_count,
        };

        if self.sender.send(event).is_err() {
            self.dropped_counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Returns `true` iff at least one receiver is currently active.
    pub fn has_subscribers(&self) -> bool {
        self.sender.receiver_count() > 0
    }
}

impl Default for TraceEmitter {
    fn default() -> Self {
        Self::new()
    }
}

// ── Transport configuration ───────────────────────────────────────────

/// Configuration for the out-of-process transport layer.
#[derive(Clone, Debug, Default)]
pub enum TraceTransportConfig {
    /// Unix-domain socket at the given path (Unix/macOS only).
    #[cfg(unix)]
    Uds { path: String },
    /// TCP loopback socket (portable fallback).
    Tcp { addr: String },
    /// No out-of-process forwarding.
    #[default]
    Disabled,
}

// ── Transport implementation ──────────────────────────────────────────

pub struct TraceTransport;

impl TraceTransport {
    /// Start accepting out-of-process subscriber connections and forwarding
    /// events as newline-delimited JSON.
    ///
    /// This future runs forever (until the process exits or the socket
    /// errors fatally). Spawn it with `tokio::spawn`.
    ///
    /// # Subscriber cap
    ///
    /// At most [`MAX_SUBSCRIBERS`] connections are served simultaneously.
    /// Connection #`MAX_SUBSCRIBERS + 1` receives a JSON error line and
    /// is closed.
    pub async fn accept_loop(config: TraceTransportConfig, emitter: TraceEmitter) {
        match config {
            #[cfg(unix)]
            TraceTransportConfig::Uds { path } => Self::uds_accept_loop(path, emitter).await,
            TraceTransportConfig::Tcp { addr } => Self::tcp_accept_loop(addr, emitter).await,
            TraceTransportConfig::Disabled => {
                // No-op — transport disabled; in-process channel still works.
            }
        }
    }

    // ── TCP accept loop ───────────────────────────────────────────────

    async fn tcp_accept_loop(addr: String, emitter: TraceEmitter) {
        let listener = match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => {
                let bound = l
                    .local_addr()
                    .map(|a| a.to_string())
                    .unwrap_or_else(|_| addr.clone());
                log::info!("trace transport: TCP listening on {}", bound);
                l
            }
            Err(e) => {
                log::error!("trace transport: failed to bind TCP {}: {}", addr, e);
                return;
            }
        };

        let active = Arc::new(AtomicUsize::new(0));
        loop {
            match listener.accept().await {
                Ok((stream, peer)) => {
                    let count = active.fetch_add(1, Ordering::Relaxed) + 1;
                    if count > MAX_SUBSCRIBERS {
                        active.fetch_sub(1, Ordering::Relaxed);
                        let active_clone = active.clone();
                        tokio::spawn(async move {
                            let (_, mut writer) = tokio::io::split(stream);
                            let _ = writer
                                .write_all(b"{\"error\":\"max_subscribers_reached\"}\n")
                                .await;
                            drop(active_clone);
                        });
                        continue;
                    }
                    log::debug!("trace: TCP subscriber connected from {}", peer);
                    let rx = emitter.subscribe();
                    let active_clone = active.clone();
                    tokio::spawn(async move {
                        let (_, writer) = tokio::io::split(stream);
                        Self::forward_events(writer, rx).await;
                        active_clone.fetch_sub(1, Ordering::Relaxed);
                        log::debug!("trace: TCP subscriber {} disconnected", peer);
                    });
                }
                Err(e) => {
                    log::error!("trace: TCP accept error: {}", e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    // ── UDS accept loop (Unix only) ───────────────────────────────────

    #[cfg(unix)]
    async fn uds_accept_loop(path: String, emitter: TraceEmitter) {
        // Remove stale socket file from a previous run.
        let _ = std::fs::remove_file(&path);

        let listener = match tokio::net::UnixListener::bind(&path) {
            Ok(l) => {
                log::info!("trace transport: UDS listening at {}", path);
                l
            }
            Err(e) => {
                log::error!("trace transport: failed to bind UDS {}: {}", path, e);
                return;
            }
        };

        let active = Arc::new(AtomicUsize::new(0));
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let count = active.fetch_add(1, Ordering::Relaxed) + 1;
                    if count > MAX_SUBSCRIBERS {
                        active.fetch_sub(1, Ordering::Relaxed);
                        tokio::spawn(async move {
                            let (_, mut writer) = tokio::io::split(stream);
                            let _ = writer
                                .write_all(b"{\"error\":\"max_subscribers_reached\"}\n")
                                .await;
                        });
                        continue;
                    }
                    log::debug!("trace: UDS subscriber connected");
                    let rx = emitter.subscribe();
                    let active_clone = active.clone();
                    tokio::spawn(async move {
                        let (_, writer) = tokio::io::split(stream);
                        Self::forward_events(writer, rx).await;
                        active_clone.fetch_sub(1, Ordering::Relaxed);
                        log::debug!("trace: UDS subscriber disconnected");
                    });
                }
                Err(e) => {
                    log::error!("trace: UDS accept error: {}", e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    // ── Event forwarder (shared by UDS and TCP) ───────────────────────

    /// Read events from `rx` and write each as a JSON line to `writer`
    /// until the connection closes or the channel is closed.
    async fn forward_events<W>(mut writer: W, mut rx: broadcast::Receiver<MatchTraceEvent>)
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        loop {
            let event = match rx.recv().await {
                Ok(e) => e,
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    // Receiver was too slow; `n` events were dropped.
                    // The next event will carry `dropped_count` so the
                    // subscriber can detect the gap. Continue.
                    log::debug!("trace: subscriber lagged, {} events dropped", n);
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            };

            let mut line = match serde_json::to_string(&event) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("trace: serialise error: {}", e);
                    continue;
                }
            };
            line.push('\n');

            if writer.write_all(line.as_bytes()).await.is_err() {
                break; // subscriber disconnected
            }
        }
    }
}

// ── Timestamp helper ──────────────────────────────────────────────────

/// Current Unix time in milliseconds.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn emit_received_by_subscriber() {
        let emitter = TraceEmitter::new();
        let mut rx = emitter.subscribe();

        emitter.emit(
            1_000_000,
            5,
            RequestSummary {
                method: "GET".into(),
                url_path: "/api/test".into(),
                headers: vec![],
                body_json: None,
                body_truncated: false,
            },
            Outcome::Miss { status: 404 },
        );

        let event = rx.try_recv().expect("event in channel");
        assert_eq!(event.event_id, 0);
        assert_eq!(event.schema_version, 1);
        assert_eq!(event.request.method, "GET");
        assert_eq!(event.duration_ms, 5);
        assert_eq!(event.dropped_count, 0);
        assert!(matches!(event.outcome, Outcome::Miss { status: 404 }));
    }

    #[tokio::test]
    async fn emit_no_subscriber_increments_dropped() {
        let emitter = TraceEmitter::new();
        emitter.emit(
            0,
            0,
            RequestSummary {
                method: "GET".into(),
                url_path: "/".into(),
                headers: vec![],
                body_json: None,
                body_truncated: false,
            },
            Outcome::Miss { status: 404 },
        );
        let mut rx = emitter.subscribe();
        emitter.emit(
            0,
            0,
            RequestSummary {
                method: "GET".into(),
                url_path: "/".into(),
                headers: vec![],
                body_json: None,
                body_truncated: false,
            },
            Outcome::Miss { status: 200 },
        );
        let event = rx.try_recv().expect("second event visible");
        assert_eq!(
            event.dropped_count, 1,
            "first event should be counted dropped"
        );
    }

    #[test]
    fn has_subscribers_reflects_state() {
        let emitter = TraceEmitter::new();
        assert!(!emitter.has_subscribers());
        let _rx = emitter.subscribe();
        assert!(emitter.has_subscribers());
    }

    #[tokio::test]
    async fn outcome_serialises_correctly() {
        let event = MatchTraceEvent {
            event_id: 7,
            schema_version: 1,
            received_at_ms: 0,
            duration_ms: 0,
            request: RequestSummary {
                method: "POST".into(),
                url_path: "/x".into(),
                headers: vec![],
                body_json: None,
                body_truncated: false,
            },
            outcome: Outcome::Matched {
                rule_set_index: 0,
                rule_index: 2,
            },
            dropped_count: 0,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"type\":\"matched\""));
        assert!(json.contains("\"rule_index\":2"));
        assert!(json.contains("\"schema_version\":1"));
    }

    #[tokio::test]
    async fn tcp_transport_delivers_events() {
        let emitter = TraceEmitter::new();
        let emitter_clone = emitter.clone();

        // We need to know the actual bound port before connecting.
        // Bind the listener ourselves to capture the address, then hand
        // the address to the transport accept loop via a channel.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bound_addr = listener.local_addr().unwrap();

        // Spawn a simplified accept loop that uses our pre-bound listener.
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let rx = emitter_clone.subscribe();
            let (_, writer) = tokio::io::split(stream);
            TraceTransport::forward_events(writer, rx).await;
        });

        // Connect a subscriber.
        let mut client = tokio::net::TcpStream::connect(bound_addr).await.unwrap();

        // Give the subscriber task a moment to subscribe before emitting.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        emitter.emit(
            42,
            3,
            RequestSummary {
                method: "GET".into(),
                url_path: "/ping".into(),
                headers: vec![],
                body_json: None,
                body_truncated: false,
            },
            Outcome::Miss { status: 404 },
        );

        // Read one JSON line from the TCP connection.
        use tokio::io::AsyncBufReadExt;
        let mut reader = tokio::io::BufReader::new(&mut client);
        let mut line = String::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            reader.read_line(&mut line),
        )
        .await
        .expect("timeout")
        .expect("read ok");

        let value: serde_json::Value = serde_json::from_str(line.trim()).expect("valid JSON");
        assert_eq!(value["request"]["url_path"], "/ping");
        assert_eq!(value["outcome"]["type"], "miss");
        assert_eq!(value["schema_version"], 1);
    }

    // ── RFC 023: body capture tests ───────────────────────────────────

    #[test]
    fn enrich_with_body_disabled_by_default() {
        let emitter = TraceEmitter::new(); // capture_body = false by default
        let mut summary = RequestSummary {
            method: "POST".into(),
            url_path: "/".into(),
            headers: vec![],
            body_json: None,
            body_truncated: false,
        };
        let body = serde_json::json!({"action": "create"});
        emitter.enrich_with_body(&mut summary, Some(&body));
        assert!(
            summary.body_json.is_none(),
            "body should not be captured when disabled"
        );
        assert!(!summary.body_truncated);
    }

    #[test]
    fn enrich_with_body_enabled_captures_small_body() {
        let emitter = TraceEmitter::with_config(TraceConfig {
            capture_body: true,
            max_body_bytes: 8_192,
            ..Default::default()
        });
        let mut summary = RequestSummary {
            method: "POST".into(),
            url_path: "/".into(),
            headers: vec![],
            body_json: None,
            body_truncated: false,
        };
        let body = serde_json::json!({"action": "create", "user_id": 42});
        emitter.enrich_with_body(&mut summary, Some(&body));
        assert!(
            summary.body_json.is_some(),
            "body should be captured when enabled"
        );
        assert_eq!(summary.body_json.unwrap()["action"], "create");
        assert!(!summary.body_truncated);
    }

    #[test]
    fn enrich_with_body_truncates_oversized_body() {
        let emitter = TraceEmitter::with_config(TraceConfig {
            capture_body: true,
            max_body_bytes: 10,
            ..Default::default()
        });
        let mut summary = RequestSummary {
            method: "POST".into(),
            url_path: "/".into(),
            headers: vec![],
            body_json: None,
            body_truncated: false,
        };
        let body = serde_json::json!({"data": "this is longer than 10 bytes"});
        emitter.enrich_with_body(&mut summary, Some(&body));
        assert!(
            summary.body_json.is_none(),
            "oversized body should be omitted"
        );
        assert!(summary.body_truncated, "body_truncated flag should be set");
    }

    #[test]
    fn request_summary_body_json_not_in_serialised_output_when_none() {
        let summary = RequestSummary {
            method: "GET".into(),
            url_path: "/api".into(),
            headers: vec![],
            body_json: None,
            body_truncated: false,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(
            !json.contains("body_json"),
            "absent body_json must be skipped"
        );
        assert!(
            !json.contains("body_truncated"),
            "false body_truncated must be skipped"
        );
    }

    // ── RFC 040: header redaction ──────────────────────────────────────

    fn headers_with_credentials() -> Vec<(String, String)> {
        vec![
            ("authorization".into(), "Bearer secret-token".into()),
            ("cookie".into(), "session=abc123".into()),
            ("x-api-key".into(), "sk-live-very-secret".into()),
            ("content-type".into(), "application/json".into()),
        ]
    }

    /// RFC 040 evidence requirement: with no trace configuration at all —
    /// `TraceConfig::default()` — none of the three credential values
    /// appear in the *serialised* event.
    #[test]
    fn default_config_redacts_credential_headers_in_serialised_output() {
        let config = TraceConfig::default();
        let summary = RequestSummary::new(
            "POST".into(),
            "/login".into(),
            headers_with_credentials(),
            &config,
        );

        let json = serde_json::to_string(&summary).unwrap();
        assert!(!json.contains("Bearer secret-token"), "json was: {json}");
        assert!(!json.contains("session=abc123"), "json was: {json}");
        assert!(!json.contains("sk-live-very-secret"), "json was: {json}");
        assert!(
            json.contains("application/json"),
            "a non-credential header must survive: {json}"
        );
    }

    /// Redacted headers stay present, marked with the placeholder — not
    /// silently dropped from the list (RFC 040 Goal 4).
    #[test]
    fn redacted_headers_are_present_and_marked_not_absent() {
        let config = TraceConfig::default();
        let summary = RequestSummary::new(
            "POST".into(),
            "/login".into(),
            headers_with_credentials(),
            &config,
        );

        assert_eq!(summary.headers.len(), 4, "no header should be dropped");
        let authorization = summary
            .headers
            .iter()
            .find(|(name, _)| name == "authorization")
            .expect("authorization header must still be present");
        assert_eq!(authorization.1, REDACTED_HEADER_VALUE);

        let json = serde_json::to_string(&summary).unwrap();
        assert!(
            json.contains("\"authorization\""),
            "redacted header name must still appear: {json}"
        );
        assert!(json.contains(REDACTED_HEADER_VALUE), "json was: {json}");
    }

    /// Header names are case-insensitive; a denylist compared
    /// case-sensitively would let a non-lowercase spelling through.
    #[test]
    fn denylist_matches_case_insensitively() {
        let config = TraceConfig::default();
        let headers = vec![
            ("Authorization".into(), "Bearer secret-token".into()),
            ("COOKIE".into(), "session=abc123".into()),
        ];
        let summary = RequestSummary::new("GET".into(), "/".into(), headers, &config);

        let json = serde_json::to_string(&summary).unwrap();
        assert!(!json.contains("Bearer secret-token"), "json was: {json}");
        assert!(!json.contains("session=abc123"), "json was: {json}");
        assert!(json.contains(REDACTED_HEADER_VALUE), "json was: {json}");
    }

    /// Allowlist mode fails closed: only the named header survives, and
    /// an ordinary, non-credential header not on the list is redacted
    /// too.
    #[test]
    fn allowlist_mode_redacts_everything_not_listed() {
        let config = TraceConfig {
            header_redaction: HeaderRedactionMode::Allowlist,
            header_allowlist: vec!["content-type".into()],
            ..Default::default()
        };
        let headers = vec![
            ("content-type".into(), "application/json".into()),
            ("authorization".into(), "Bearer secret-token".into()),
            ("x-request-id".into(), "not-a-credential".into()),
        ];
        let summary = RequestSummary::new("GET".into(), "/".into(), headers, &config);

        let by_name = |name: &str| {
            summary
                .headers
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.as_str())
        };
        assert_eq!(by_name("content-type"), Some("application/json"));
        assert_eq!(by_name("authorization"), Some(REDACTED_HEADER_VALUE));
        assert_eq!(
            by_name("x-request-id"),
            Some(REDACTED_HEADER_VALUE),
            "an unlisted, non-credential header must still be redacted in allowlist mode"
        );
    }

    /// An empty allowlist — the state before anyone configures one —
    /// redacts every header. That is the safe direction for a
    /// fail-closed mode, not an oversight.
    #[test]
    fn allowlist_mode_with_no_entries_redacts_everything() {
        let config = TraceConfig {
            header_redaction: HeaderRedactionMode::Allowlist,
            ..Default::default()
        };
        let summary = RequestSummary::new(
            "GET".into(),
            "/".into(),
            vec![("content-type".into(), "application/json".into())],
            &config,
        );
        assert_eq!(summary.headers[0].1, REDACTED_HEADER_VALUE);
    }
}
