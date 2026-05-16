//! Live match-trace channel — RFC 006.
//!
//! # What this module provides
//!
//! A structured event stream that lets a GUI subscriber observe every
//! incoming HTTP request alongside the rule that matched (or a "miss"
//! if no rule matched). Events are emitted in-process via a bounded
//! `tokio::sync::broadcast` channel.
//!
//! # What is deliberately stubbed / deferred
//!
//! The in-process channel is fully implemented. The *transport layer*
//! (Unix-domain socket, named pipe, or TCP loopback) that forwards
//! events to an out-of-process GUI subscriber is stubbed — the
//! `TraceTransport::accept_loop` method panics with an explicit
//! "not yet implemented" message. Implementing it requires
//! OS-specific IPC plumbing that is scoped to a future release.
//!
//! # Back-pressure
//!
//! The channel is bounded by `TRACE_CHANNEL_CAPACITY`. When the
//! channel is full (subscriber too slow), `broadcast::Sender::send`
//! returns `SendError`; the emitter drops the event and increments
//! `dropped_count` so the next event can report how many were lost.

use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::broadcast;

/// Capacity of the in-process broadcast channel.
pub const TRACE_CHANNEL_CAPACITY: usize = 1_024;

// ── Event types ───────────────────────────────────────────────────────

/// A single request/response trace event.
#[derive(Clone, Debug)]
pub struct MatchTraceEvent {
    /// Monotonically increasing event counter within this server run.
    pub event_id: u64,
    /// Unix timestamp (milliseconds) of when the request was received.
    pub received_at_ms: u64,
    /// Time taken to produce the response, in milliseconds.
    pub duration_ms: u32,
    /// Summary of the incoming request.
    pub request: RequestSummary,
    /// What the server did with the request.
    pub outcome: Outcome,
    /// Number of events dropped since the last successfully delivered
    /// event (0 when no events were dropped).
    pub dropped_count: u32,
}

/// Key fields from the incoming HTTP request.
#[derive(Clone, Debug)]
pub struct RequestSummary {
    pub method: String,
    pub url_path: String,
    /// Selected request headers (not all — see RFC 006 §drawbacks on
    /// body capture being deferred).
    pub headers: Vec<(String, String)>,
}

/// What the server decided to do with the request.
#[derive(Clone, Debug)]
pub enum Outcome {
    /// A rule in a rule set matched.
    Matched {
        rule_set_index: usize,
        rule_index: usize,
    },
    /// No rule matched; the dynamic-route fallback served the request.
    Fallback { file_path: String, status: u16 },
    /// No rule matched and the fallback produced no response.
    Miss { status: u16 },
    /// An error occurred while processing the request.
    Error { kind: String, message: String },
}

// ── Channel handle ────────────────────────────────────────────────────

/// Shared handle to the trace broadcast channel.
///
/// Clone freely — each clone refers to the same underlying channel.
#[derive(Clone)]
pub struct TraceEmitter {
    sender: broadcast::Sender<MatchTraceEvent>,
    event_counter: Arc<AtomicU32>,
    dropped_counter: Arc<AtomicU32>,
}

impl TraceEmitter {
    /// Create a new emitter with a fresh broadcast channel.
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(TRACE_CHANNEL_CAPACITY);
        Self {
            sender,
            event_counter: Arc::new(AtomicU32::new(0)),
            dropped_counter: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Subscribe to the event stream. Returns a receiver that will
    /// receive future events. Events sent before this call are not
    /// replayed (no backfill — deferred per RFC 006 unresolved
    /// questions §4).
    pub fn subscribe(&self) -> broadcast::Receiver<MatchTraceEvent> {
        self.sender.subscribe()
    }

    /// Emit one event. If the channel is full, the event is dropped
    /// and the internal drop counter incremented.
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
            received_at_ms,
            duration_ms,
            request,
            outcome,
            dropped_count,
        };

        if self.sender.send(event).is_err() {
            // No active receivers OR channel full — increment dropped.
            self.dropped_counter.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Return `true` iff at least one subscriber is currently active.
    pub fn has_subscribers(&self) -> bool {
        self.sender.receiver_count() > 0
    }
}

impl Default for TraceEmitter {
    fn default() -> Self {
        Self::new()
    }
}

// ── Transport stub (RFC 006 §deferred) ───────────────────────────────

/// Configuration for the out-of-process trace transport.
///
/// Currently only UDS is listed; TCP and named-pipe variants are
/// reserved for the future implementation.
#[derive(Clone, Debug, Default)]
pub enum TraceTransportConfig {
    /// Unix-domain socket at the given path.
    Uds { path: String },
    /// TCP loopback on the given address.
    Tcp { addr: String },
    /// Disabled — no out-of-process forwarding.
    #[default]
    Disabled,
}

/// Stub transport layer.
///
/// The `accept_loop` method is explicitly unimplemented — it is
/// intentionally left for a future session, as documented in the RFC 006
/// implementation notes and the session handoff document.
pub struct TraceTransport;

impl TraceTransport {
    /// Start accepting out-of-process subscribers and forwarding events.
    ///
    /// # Panics
    ///
    /// Always panics with "not yet implemented — RFC 006 socket I/O
    /// transport is a stub, deferred to a future release."
    pub async fn accept_loop(
        _config: TraceTransportConfig,
        _emitter: TraceEmitter,
    ) -> ! {
        unimplemented!(
            "RFC 006 socket I/O transport is a stub, deferred to a future release. \
             The in-process broadcast channel is fully functional."
        )
    }
}

// ── Timestamp helper ──────────────────────────────────────────────────

/// Current time as Unix milliseconds.
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
            },
            Outcome::Miss { status: 404 },
        );

        let event = rx.try_recv().expect("event should be in channel");
        assert_eq!(event.event_id, 0);
        assert_eq!(event.request.method, "GET");
        assert_eq!(event.request.url_path, "/api/test");
        assert_eq!(event.duration_ms, 5);
        assert_eq!(event.dropped_count, 0);
        assert!(matches!(event.outcome, Outcome::Miss { status: 404 }));
    }

    #[tokio::test]
    async fn emit_with_no_subscriber_increments_dropped() {
        let emitter = TraceEmitter::new();
        // No subscriber — send should fail and increment dropped.
        emitter.emit(
            0,
            0,
            RequestSummary { method: "GET".into(), url_path: "/".into(), headers: vec![] },
            Outcome::Miss { status: 404 },
        );

        // Now subscribe and emit again; dropped_count on the second
        // event should reflect the one we lost.
        let mut rx = emitter.subscribe();
        emitter.emit(
            0,
            0,
            RequestSummary { method: "GET".into(), url_path: "/".into(), headers: vec![] },
            Outcome::Miss { status: 404 },
        );

        let event = rx.try_recv().expect("second event visible to new subscriber");
        assert_eq!(event.dropped_count, 1, "first event should be counted as dropped");
    }

    #[test]
    fn has_subscribers_reflects_state() {
        let emitter = TraceEmitter::new();
        assert!(!emitter.has_subscribers());
        let _rx = emitter.subscribe();
        assert!(emitter.has_subscribers());
    }
}
