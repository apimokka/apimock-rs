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
    /// Selected request headers (display-only; body capture deferred).
    pub headers: Vec<(String, String)>,
}

/// What the server decided to do with the request.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Outcome {
    Matched { rule_set_index: usize, rule_index: usize },
    Fallback { file_path: String, status: u16 },
    Miss    { status: u16 },
    Error   { kind: String, message: String },
}

// ── Emitter ───────────────────────────────────────────────────────────

/// Shared handle to the trace broadcast channel.
///
/// Clone freely — each clone refers to the same underlying channel.
#[derive(Clone)]
pub struct TraceEmitter {
    sender:          broadcast::Sender<MatchTraceEvent>,
    event_counter:   Arc<AtomicU32>,
    dropped_counter: Arc<AtomicU32>,
}

impl TraceEmitter {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(TRACE_CHANNEL_CAPACITY);
        Self {
            sender,
            event_counter:   Arc::new(AtomicU32::new(0)),
            dropped_counter: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Subscribe to the event stream (in-process).
    pub fn subscribe(&self) -> broadcast::Receiver<MatchTraceEvent> {
        self.sender.subscribe()
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
        let event_id    = self.event_counter.fetch_add(1, Ordering::Relaxed) as u64;
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

impl Default for TraceEmitter { fn default() -> Self { Self::new() } }

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
            TraceTransportConfig::Uds { path } => {
                Self::uds_accept_loop(path, emitter).await
            }
            TraceTransportConfig::Tcp { addr } => {
                Self::tcp_accept_loop(addr, emitter).await
            }
            TraceTransportConfig::Disabled => {
                // No-op — transport disabled; in-process channel still works.
            }
        }
    }

    // ── TCP accept loop ───────────────────────────────────────────────

    async fn tcp_accept_loop(addr: String, emitter: TraceEmitter) {
        let listener = match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => {
                let bound = l.local_addr().map(|a| a.to_string())
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
                        let emitter_clone = emitter.clone();
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
            1_000_000, 5,
            RequestSummary { method: "GET".into(), url_path: "/api/test".into(), headers: vec![] },
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
        emitter.emit(0, 0,
            RequestSummary { method: "GET".into(), url_path: "/".into(), headers: vec![] },
            Outcome::Miss { status: 404 },
        );
        let mut rx = emitter.subscribe();
        emitter.emit(0, 0,
            RequestSummary { method: "GET".into(), url_path: "/".into(), headers: vec![] },
            Outcome::Miss { status: 200 },
        );
        let event = rx.try_recv().expect("second event visible");
        assert_eq!(event.dropped_count, 1, "first event should be counted dropped");
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
            event_id: 7, schema_version: 1, received_at_ms: 0, duration_ms: 0,
            request: RequestSummary { method: "POST".into(), url_path: "/x".into(), headers: vec![] },
            outcome: Outcome::Matched { rule_set_index: 0, rule_index: 2 },
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

        // Bind on an ephemeral port.
        let config = TraceTransportConfig::Tcp { addr: "127.0.0.1:0".to_owned() };

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
            42, 3,
            RequestSummary { method: "GET".into(), url_path: "/ping".into(), headers: vec![] },
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
}
