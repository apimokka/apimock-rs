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
//! RFC 073 S-06/D-02: this section used to describe a mechanism
//! `tokio::sync::broadcast` does not have — `Sender::send` only fails
//! when there are **no receivers at all**, never because the channel is
//! "full" (a full channel instead evicts the oldest unread event for
//! whichever receiver is slowest, which is a *per-receiver* event, not
//! a send-time one). What's true now, and implemented to match:
//!
//! - [`TraceEmitter::emit`] increments a **shared** counter only when
//!   `send` fails outright (no receiver existed at that moment) — rare,
//!   and not what "back-pressure" usually means here.
//! - A slow **out-of-process** subscriber (UDS/TCP, via
//!   [`TraceTransport::accept_loop`]) gets `RecvError::Lagged(n)` on its
//!   own [`broadcast::Receiver`] when it falls behind by more than
//!   [`TRACE_CHANNEL_CAPACITY`] events; `n` is accumulated **per
//!   subscriber** and added to `dropped_count` on that subscriber's next
//!   forwarded event — see `forward_events`'s doc comment. Two
//!   independently-lagging subscribers each see their own true count,
//!   not each other's.
//! - A direct **in-process** subscriber (calling [`TraceEmitter::subscribe`]
//!   itself, bypassing the transport) gets the same `RecvError::Lagged`
//!   from its own receiver and is responsible for folding it into
//!   `dropped_count` the same way, since this crate has no way to patch
//!   an event already broadcast to that caller's receiver — see
//!   `subscribe`'s own doc comment.
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

use apimock_routing::util::http::percent_decode_url_path;
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
#[non_exhaustive]
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
    /// Byte length of the request body, if one arrived (RFC 050) —
    /// presence and size only, **never content**: no bytes, no snippet,
    /// no preview. Populated for every body, JSON included — required
    /// 2026-08-17 by review of this RFC, which found the original,
    /// JSON-excluding version left the *common* case (a JSON body with
    /// `capture_body` at its default `false`) still indistinguishable
    /// from no body at all, the exact ambiguity this RFC exists to
    /// close. So the three states this field distinguishes, together
    /// with `body_json`, are: both absent (no body); `body_len` present
    /// and `body_json` present (body present, JSON captured); `body_len`
    /// present and `body_json` absent (body present, not captured —
    /// non-JSON, capture disabled, or over `max_body_bytes`; the last of
    /// those is further distinguished by `body_truncated`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_len: Option<usize>,
}

impl RequestSummary {
    /// Construct from a live request's headers, applying `config`'s
    /// redaction policy (RFC 040). This is the only place header
    /// redaction happens — do not build `RequestSummary` from live
    /// request headers any other way; a future formatter or display
    /// path must not need to know about redaction at all.
    ///
    /// `body_len` should be the source request's own `ParsedRequest.body_len`
    /// (RFC 050) — pass it through unconditionally, JSON body or not;
    /// `enrich_with_body` populates `body_json` separately for the JSON
    /// case, and the two fields together carry the distinction.
    pub fn new(
        method: String,
        url_path: String,
        headers: Vec<(String, String)>,
        body_len: Option<usize>,
        config: &TraceConfig,
    ) -> Self {
        Self {
            method,
            url_path,
            headers: config.redact_headers(headers),
            body_json: None,
            body_truncated: false,
            body_len,
        }
    }
}

/// Placeholder value substituted for a redacted header (RFC 040 Goal 4).
/// The header name is kept so a consumer can tell "redacted" from
/// "the request never sent this header" — only the value differs.
pub const REDACTED_HEADER_VALUE: &str = "[redacted]";

/// Built-in denylist of well-known credential-bearing names, applied by
/// default (RFC 040 Q1). Compared case-insensitively.
///
/// # RFC 073 S-05: also the query-string and body-key denylist now
///
/// Originally header names only. `TraceConfig::header_denylist` (and
/// `is_redacted_key`, below) now gate query-string parameter names and
/// JSON request-body object keys too — one policy, one list, wherever
/// a name-value pair leaves the process (a header, a query parameter,
/// or a body field), rather than a second, parallel config surface for
/// the same idea. The field/const names stay header-branded (renaming
/// an already-public field is a bigger break than this RFC's fix
/// needs), but the *scope* is broader than the name suggests — this
/// comment, and `is_redacted_key`'s, are where that's said plainly.
/// Entries added for this: `token`, `access_token`, `refresh_token`,
/// `password`, `secret`, `client_secret`, `api_key` — none of which is
/// a header name a request would ever send, but all of which are
/// common query-parameter and body-field names for the same kind of
/// value `authorization`/`x-api-key` already cover as headers.
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
    "token",
    "access_token",
    "refresh_token",
    "password",
    "secret",
    "client_secret",
    "api_key",
];

/// Which names get redacted before a trace event is built, or before a
/// verbose console log line is printed (RFC 040 Q1, extended by RFC 073
/// S-05 to query-string parameters and JSON body keys — see
/// `DEFAULT_HEADER_DENYLIST`'s doc comment).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeaderRedactionMode {
    /// Redact names in `TraceConfig::header_denylist`; capture
    /// everything else. Fails open on an unanticipated name —
    /// accepted deliberately, see RFC 040's Risks table.
    Denylist,
    /// Capture only names in `TraceConfig::header_allowlist`; redact
    /// everything else. Fails closed.
    Allowlist,
}

/// Trace-channel behaviour configuration (RFC 023, extended by RFC 040).
///
/// Configurable today only at this Rust level — the trace channel has
/// no config-file or CLI surface yet (RFC 040's own Motivation notes
/// this; RFC 023's `[trace]` TOML section was never wired to this
/// struct). `TraceEmitter::new()` always uses `TraceConfig::default()`.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct TraceConfig {
    /// Capture the JSON request body in each event. Default: `false`.
    pub capture_body: bool,
    /// Maximum serialised body size in bytes. Bodies larger than this
    /// are omitted and `body_truncated = true` is set. Default: 8 192.
    pub max_body_bytes: usize,
    /// Denylist or allowlist by default. Default: `Denylist`. Governs
    /// headers, query-string parameters, and JSON body keys alike (RFC
    /// 073 S-05) — see `DEFAULT_HEADER_DENYLIST`'s doc comment.
    pub header_redaction: HeaderRedactionMode,
    /// Names redacted when `header_redaction` is `Denylist` — header
    /// names, query-string parameter names, and JSON body object keys
    /// alike (RFC 073 S-05). Compared case-insensitively. Default:
    /// [`DEFAULT_HEADER_DENYLIST`].
    pub header_denylist: Vec<String>,
    /// Names captured when `header_redaction` is `Allowlist`; every
    /// other name (header, query parameter, or body key) is redacted.
    /// Compared case-insensitively. Default: empty, i.e. allowlist mode
    /// redacts everything until configured — the safe direction for a
    /// fail-closed mode.
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
    /// Whether `name` is redacted under this config's policy
    /// (case-insensitive). The single definition behind every place a
    /// name-value pair can leave the process: a request header
    /// (`redact_headers`, below), a query-string parameter
    /// (`redact_query_string`), a JSON body key (`redact_json_value`),
    /// and verbose console logging (`render_request_log` in
    /// `parsed_request.rs`, RFC 051/073) — one policy, shared by
    /// reference, not copied or reimplemented per call site.
    ///
    /// Named for what it does, not for headers specifically (RFC 073
    /// S-05 extended this from a header-only check) — see
    /// `DEFAULT_HEADER_DENYLIST`'s doc comment for why the *fields*
    /// stay header-branded regardless.
    pub(crate) fn is_redacted_key(&self, name: &str) -> bool {
        match self.header_redaction {
            HeaderRedactionMode::Denylist => self
                .header_denylist
                .iter()
                .any(|denied| denied.eq_ignore_ascii_case(name)),
            HeaderRedactionMode::Allowlist => !self
                .header_allowlist
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(name)),
        }
    }

    /// Redact header values per this config's policy. Names and order
    /// are preserved; only a matched entry's value is replaced with
    /// [`REDACTED_HEADER_VALUE`] (RFC 040 Goal 4 — marked, not omitted).
    fn redact_headers(&self, headers: Vec<(String, String)>) -> Vec<(String, String)> {
        headers
            .into_iter()
            .map(|(name, value)| {
                if self.is_redacted_key(&name) {
                    (name, REDACTED_HEADER_VALUE.to_string())
                } else {
                    (name, value)
                }
            })
            .collect()
    }

    /// Redact a raw query string per this config's policy (RFC 073
    /// S-05) — `?token=secret&page=2` becomes `?token=[redacted]&page=2`.
    /// Parameter names and their order are preserved verbatim in the
    /// output, including any percent-encoding in the original string
    /// (this is a display-time transform, not a re-parse of the
    /// request — nothing here is used for matching); only a matched
    /// parameter's value is replaced with [`REDACTED_HEADER_VALUE`],
    /// the same marker header redaction uses. A key with no `=` (a bare
    /// flag parameter) is left alone — there is no value to redact and
    /// the key itself is never secret content.
    ///
    /// # REVIEW-001 F-01: the *key* is decoded before the denylist
    /// # check, even though the printed key stays as written
    ///
    /// A client can percent-encode ASCII in a parameter name
    /// (`%74oken` decodes to `token`) — unusual, but not invalid, and
    /// the whole point of this method is to not depend on an attacker
    /// (or just an unusual client) spelling the name the way the
    /// denylist expects. `percent_decode_url_path` (RFC 075,
    /// `apimock-routing`) decodes *only* the copy used for the
    /// `is_redacted_key` check; the key actually written to the output
    /// is the original, unmodified slice, so a legitimately
    /// percent-encoded name that happens to look like `token` still
    /// displays as it was sent — only the *value* changes when it
    /// matches. This mirrors why the ordering matters at all in RFC
    /// 075 F-03: checking the undecoded form is the same class of
    /// bypass as never decoding at all.
    ///
    /// JSON body keys (`redact_json_value`, below) don't need this:
    /// JSON keys aren't percent-encoded on the wire — a key is a JSON
    /// string, not a URI component — so there is no undecoded form to
    /// bypass through.
    pub(crate) fn redact_query_string(&self, query: &str) -> String {
        query
            .split('&')
            .map(|pair| match pair.split_once('=') {
                Some((key, _value)) if self.is_redacted_key(&percent_decode_url_path(key)) => {
                    format!("{key}={REDACTED_HEADER_VALUE}")
                }
                _ => pair.to_string(),
            })
            .collect::<Vec<String>>()
            .join("&")
    }

    /// Redact a JSON body per this config's policy (RFC 073 S-05),
    /// recursively — a secret nested inside an object is just as real a
    /// leak as one at the top level. For each object encountered, a key
    /// matching the denylist/allowlist has its **value** replaced with
    /// [`REDACTED_HEADER_VALUE`] (the key itself stays, same
    /// mark-don't-omit convention as header redaction); a key that
    /// isn't redacted is recursed into, so a secret nested under a
    /// non-secret-named parent is still caught. Arrays are walked
    /// element-wise. A scalar (string/number/bool/null) has nothing to
    /// redact by itself — only an object's *keys* name what a value is,
    /// which is what redaction here keys off of.
    pub(crate) fn redact_json_value(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => serde_json::Value::Object(
                map.iter()
                    .map(|(key, val)| {
                        let redacted_val = if self.is_redacted_key(key) {
                            serde_json::Value::String(REDACTED_HEADER_VALUE.to_string())
                        } else {
                            self.redact_json_value(val)
                        };
                        (key.clone(), redacted_val)
                    })
                    .collect(),
            ),
            serde_json::Value::Array(items) => {
                serde_json::Value::Array(items.iter().map(|v| self.redact_json_value(v)).collect())
            }
            scalar => scalar.clone(),
        }
    }
}

/// What the server decided to do with the request.
///
/// # RFC 073 F-08: every variant here must actually be emitted
///
/// Before this RFC, `server.rs` emitted `Miss { status: 0 }` for
/// *every* request regardless of outcome — the correct index was
/// computed and discarded — and nothing was emitted at all for a
/// middleware match, a dyn-route fallback file, or a genuine 404.
/// `Fallback` and `Miss` already existed but were unused outside this
/// module's own tests; `Middleware` is new, added because no existing
/// shape fit a middleware match (its own script path, not a rule-set
/// index, is what identifies which one answered).
///
/// # `#[non_exhaustive]` — added in the same change that added `Middleware`
/// # (REVIEW-001 F-02)
///
/// Adding `Middleware` here breaks any consumer with an exhaustive
/// `match` on `Outcome`, regardless of what the public-API baseline
/// diff shows — a new-variant addition is "additive" in the sense that
/// tool tracks (a new pub item exists), not in the sense that matters
/// to semver (a consumer's exhaustive match stops compiling). `Outcome`
/// was the one public enum in this crate not already carrying this
/// attribute — `ReloadHint`, `ServerState`, `ServerError`,
/// `ServerErrorKind` and `TlsKind` all have it; RFC 052 (which added it
/// to five *other* types) never considered `Outcome`, so this gap
/// pre-dates this tranche and simply hadn't been triggered by an actual
/// variant addition yet. Since this change breaks exhaustive matchers
/// either way, marking it now means every *future* variant is free —
/// the same reasoning RFC 052 used for the five types it covered.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Outcome {
    Matched {
        rule_set_index: usize,
        rule_index: usize,
    },
    /// A Rhai middleware handled the request. `file_path` is the
    /// matched middleware script's own path (`MiddlewareHandler::file_path`),
    /// mirroring `Fallback`'s use of a path over an index for the same
    /// reason: it identifies *which* handler answered without requiring
    /// a consumer to also have the server's own middleware list on hand.
    Middleware {
        file_path: String,
        status: u16,
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
    ///
    /// # A direct subscriber owns its own lag accounting
    ///
    /// This is a plain `tokio::sync::broadcast::Receiver` — if this
    /// caller's own `recv()` loop falls behind by more than
    /// [`TRACE_CHANNEL_CAPACITY`] events, it gets `RecvError::Lagged(n)`
    /// the same way `TraceTransport::forward_events` does internally
    /// for the UDS/TCP transport. This crate cannot fold that `n` into
    /// `dropped_count` on this caller's behalf — an event is broadcast
    /// once and already in flight to every receiver by the time any one
    /// of them lags, so nothing can retroactively patch the copy this
    /// receiver eventually reads. A caller that wants an honest
    /// `dropped_count` (rather than a stale one from a rarer, shared
    /// counter — see this module's own doc comment on back-pressure)
    /// should accumulate `n` itself across `Lagged` and account for it
    /// however it reports events onward, the same way `forward_events`
    /// does for its own two transports.
    pub fn subscribe(&self) -> broadcast::Receiver<MatchTraceEvent> {
        self.sender.subscribe()
    }

    /// Attach body JSON to a `RequestSummary` according to this emitter's
    /// `TraceConfig`. Call before `emit` when the request body is available.
    ///
    /// # RFC 073 S-05: the captured body is redacted, not raw
    ///
    /// This is the trace *channel*'s own body capture — it reaches
    /// out-of-process subscribers over the UDS/TCP transport, not just
    /// a local terminal, so leaving it unredacted here would be at
    /// least as serious a leak as the verbose-console-log one this RFC
    /// also fixes. Redaction runs before the size check, so the size
    /// cap applies to what is actually stored (post-redaction), not to
    /// the original.
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
                let redacted = self.config.redact_json_value(v);
                // Check serialised size against the cap.
                match serde_json::to_string(&redacted) {
                    Ok(s) if s.len() <= self.config.max_body_bytes => {
                        summary.body_json = Some(redacted);
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

    /// # RFC 073: this transport has no authentication
    ///
    /// Anything that can open a TCP connection to `addr` receives the
    /// live request trace feed — there is no login, token, or
    /// allowlist. A non-loopback `addr` is a documentation ask this RFC
    /// cannot enforce (an operator may have a real reason this process
    /// doesn't know), so this only warns loudly rather than refusing to
    /// bind — see `docs/src/reference/threat-model.md`'s trace-transport
    /// section for the full statement, and prefer the Unix-socket
    /// transport (restrictive permissions, RFC 073) wherever the
    /// platform supports it.
    async fn tcp_accept_loop(addr: String, emitter: TraceEmitter) {
        let listener = match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => {
                let bound = l
                    .local_addr()
                    .map(|a| a.to_string())
                    .unwrap_or_else(|_| addr.clone());
                log::info!("trace transport: TCP listening on {}", bound);
                if !l.local_addr().map(|a| a.ip().is_loopback()).unwrap_or(true) {
                    log::warn!(
                        "trace transport: TCP listening on a non-loopback address ({}) — \
                         this transport has no authentication; anything that can reach it \
                         receives the live request trace feed",
                        bound
                    );
                }
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
                        tokio::spawn(async move {
                            let (_, mut writer) = tokio::io::split(stream);
                            let _ = writer
                                .write_all(b"{\"error\":\"max_subscribers_reached\"}\n")
                                .await;
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

    /// # RFC 073: restrictive permissions, owner-only
    ///
    /// `UnixListener::bind` creates the socket file with permissions
    /// governed by the process umask — often group/world-readable in a
    /// default shell configuration, which would let any other local
    /// user connect and receive the live request trace feed. Set to
    /// `0600` (owner read/write only) immediately after binding, before
    /// the accept loop starts, so there is no window where the socket
    /// exists at its umask-derived permissions. This has no Windows
    /// equivalent — the UDS transport is `#[cfg(unix)]` only; Windows
    /// always uses the TCP transport, which this crate cannot restrict
    /// the same way (see `tcp_accept_loop`'s own doc comment).
    #[cfg(unix)]
    async fn uds_accept_loop(path: String, emitter: TraceEmitter) {
        use std::os::unix::fs::PermissionsExt;

        // Remove stale socket file from a previous run.
        let _ = std::fs::remove_file(&path);

        let listener = match tokio::net::UnixListener::bind(&path) {
            Ok(l) => {
                if let Err(e) =
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
                {
                    log::error!(
                        "trace transport: failed to restrict UDS permissions on {}: {}",
                        path,
                        e
                    );
                }
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
    ///
    /// # RFC 073 S-06/D-02: `dropped_count` is patched per subscriber
    ///
    /// `event.dropped_count`, as built by `TraceEmitter::emit`, only
    /// ever reflects the rare shared no-receivers counter — it cannot
    /// know about *this* lag, since a lag is detected on this
    /// subscriber's own `Receiver`, after the event was already
    /// broadcast identically to everyone. `lagged_events` accumulates
    /// `n` from every `Lagged` this subscriber's own receiver reports,
    /// and is folded into the next event this loop actually forwards
    /// (each subscriber gets its own `Clone` of the event from
    /// `broadcast`, so mutating it here affects only this subscriber's
    /// own JSON line) — then reset, so a later event isn't charged for
    /// a gap already reported.
    async fn forward_events<W>(mut writer: W, mut rx: broadcast::Receiver<MatchTraceEvent>)
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        let mut lagged_events: u32 = 0;
        loop {
            let mut event = match rx.recv().await {
                Ok(e) => e,
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    lagged_events =
                        lagged_events.saturating_add(u32::try_from(n).unwrap_or(u32::MAX));
                    log::debug!("trace: subscriber lagged, {} events dropped", n);
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            };

            event.dropped_count = event.dropped_count.saturating_add(lagged_events);
            lagged_events = 0;

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
                body_len: None,
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
                body_len: None,
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
                body_len: None,
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
                body_len: None,
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
                body_len: None,
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

    fn dummy_summary() -> RequestSummary {
        RequestSummary {
            method: "GET".into(),
            url_path: "/".into(),
            headers: vec![],
            body_json: None,
            body_truncated: false,
            body_len: None,
        }
    }

    /// RFC 073 S-06/D-02: a subscriber that falls behind by more than
    /// the channel's capacity gets a nonzero `dropped_count` on the
    /// next event it actually receives — the documented back-pressure
    /// behaviour, implemented rather than only described. Overflowing
    /// the channel before this subscriber ever reads anything, then
    /// dropping the emitter (closing the channel) so `forward_events`
    /// drains the remaining buffered events and terminates on `Closed`
    /// rather than hanging forever waiting for one more.
    #[tokio::test]
    async fn a_lagging_subscriber_reports_dropped_count_on_its_next_event() {
        let emitter = TraceEmitter::new();
        let rx = emitter.subscribe();

        for _ in 0..(TRACE_CHANNEL_CAPACITY + 10) {
            emitter.emit(0, 0, dummy_summary(), Outcome::Miss { status: 404 });
        }
        drop(emitter);

        let mut buf: Vec<u8> = Vec::new();
        TraceTransport::forward_events(&mut buf, rx).await;

        let text = String::from_utf8(buf).expect("valid utf8");
        let first_line = text.lines().next().expect("at least one forwarded event");
        let event: serde_json::Value = serde_json::from_str(first_line).expect("valid JSON");
        assert!(
            event["dropped_count"].as_u64().unwrap_or(0) > 0,
            "the first event surviving a lag must report it: {first_line}"
        );
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
            body_len: None,
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
            body_len: None,
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
            body_len: None,
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
            body_len: None,
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
            None,
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
            None,
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
        let summary = RequestSummary::new("GET".into(), "/".into(), headers, None, &config);

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
        let summary = RequestSummary::new("GET".into(), "/".into(), headers, None, &config);

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
            None,
            &config,
        );
        assert_eq!(summary.headers[0].1, REDACTED_HEADER_VALUE);
    }

    // ── RFC 050: body presence (never content) ──────────────────────────

    /// The three states RFC 050 exists to distinguish, asserted on the
    /// *serialised* event — since that is what reaches a consumer.
    /// `body_len` is populated for every body (RFC 050 review, R-09-
    /// adjacent fix, 2026-08-17) — including the JSON-captured case,
    /// which the first version of this RFC omitted, leaving the common
    /// case (`capture_body`'s own default, `false`) still indistinguishable
    /// from no body at all.
    #[test]
    fn three_body_states_are_distinguishable_in_the_serialised_form() {
        let config = TraceConfig::default();

        let no_body = RequestSummary::new("GET".into(), "/".into(), vec![], None, &config);
        let no_body_json = serde_json::to_string(&no_body).unwrap();
        assert!(!no_body_json.contains("body_json"), "{no_body_json}");
        assert!(!no_body_json.contains("body_len"), "{no_body_json}");

        let mut json_captured =
            RequestSummary::new("POST".into(), "/".into(), vec![], Some(11), &config);
        let emitter = TraceEmitter::with_config(TraceConfig {
            capture_body: true,
            ..Default::default()
        });
        emitter.enrich_with_body(&mut json_captured, Some(&serde_json::json!({"a": 1})));
        let json_captured_str = serde_json::to_string(&json_captured).unwrap();
        assert!(
            json_captured_str.contains("\"body_json\""),
            "{json_captured_str}"
        );
        assert!(
            json_captured_str.contains("\"body_len\":11"),
            "a JSON-captured body must still report its length: {json_captured_str}"
        );

        let body_present_not_captured =
            RequestSummary::new("POST".into(), "/".into(), vec![], Some(27), &config);
        let not_captured_str = serde_json::to_string(&body_present_not_captured).unwrap();
        assert!(
            !not_captured_str.contains("body_json"),
            "{not_captured_str}"
        );
        assert!(
            not_captured_str.contains("\"body_len\":27"),
            "{not_captured_str}"
        );
    }

    /// No content, ever — a recognisable string from the original body
    /// must not appear anywhere in the serialised event, however it got
    /// there.
    #[test]
    fn non_json_body_reports_length_but_never_content() {
        let config = TraceConfig::default();
        let summary = RequestSummary::new("POST".into(), "/".into(), vec![], Some(32), &config);
        let json = serde_json::to_string(&summary).unwrap();

        assert!(json.contains("\"body_len\":32"), "json was: {json}");
        assert!(
            !json.contains("username") && !json.contains("hunter2"),
            "no fragment of a body — captured or not — should appear: {json}"
        );
    }

    // ── RFC 073 S-05: query-string and body redaction ────────────────

    /// The tranche 5 handoff's own acceptance example: a secret in a
    /// query parameter is redacted the same way a header is, under the
    /// broadened default denylist.
    #[test]
    fn a_query_string_token_is_redacted_by_default() {
        let config = TraceConfig::default();
        let redacted = config.redact_query_string("token=secret&page=2");
        assert_eq!(redacted, "token=[redacted]&page=2");
    }

    /// A parameter name not on the denylist survives untouched, and
    /// parameter order is preserved.
    #[test]
    fn a_non_denied_query_parameter_survives() {
        let config = TraceConfig::default();
        let redacted = config.redact_query_string("page=2&access_token=abc123&sort=asc");
        assert_eq!(redacted, "page=2&access_token=[redacted]&sort=asc");
    }

    /// A bare flag parameter (no `=`) has nothing to redact and is left
    /// alone, even if its name happens to match the denylist.
    #[test]
    fn a_bare_flag_parameter_is_left_alone() {
        let config = TraceConfig::default();
        let redacted = config.redact_query_string("verbose&token=secret");
        assert_eq!(redacted, "verbose&token=[redacted]");
    }

    /// A denylist match is case-insensitive regardless of how the key
    /// arrived.
    #[test]
    fn a_query_string_key_is_matched_case_insensitively() {
        let config = TraceConfig::default();
        let redacted = config.redact_query_string("TOKEN=secret");
        assert_eq!(redacted, "TOKEN=[redacted]");
    }

    /// REVIEW-001 F-01: a percent-encoded key must not bypass the
    /// denylist. `%74oken` decodes to `token` — the value still gets
    /// redacted, but the key is printed exactly as it arrived (not
    /// decoded), since only the *value* is ever supposed to change.
    #[test]
    fn a_percent_encoded_query_key_does_not_bypass_redaction() {
        let config = TraceConfig::default();
        let redacted = config.redact_query_string("%74oken=secret");
        assert_eq!(redacted, "%74oken=[redacted]");
    }

    /// The tranche 5 handoff's other acceptance example: a secret in a
    /// JSON body is redacted, by key, the same way a header is.
    #[test]
    fn a_top_level_body_secret_is_redacted_by_default() {
        let config = TraceConfig::default();
        let body = serde_json::json!({"username": "alice", "password": "hunter2"});
        let redacted = config.redact_json_value(&body);
        assert_eq!(redacted["username"], "alice");
        assert_eq!(redacted["password"], REDACTED_HEADER_VALUE);
    }

    /// A secret nested under a non-secret-named parent is still caught
    /// — redaction recurses into objects it doesn't itself redact.
    #[test]
    fn a_nested_body_secret_is_redacted_too() {
        let config = TraceConfig::default();
        let body = serde_json::json!({
            "user": {"name": "alice", "api_key": "sk-live-very-secret"},
            "items": [{"id": 1}, {"token": "should-not-appear"}],
        });
        let redacted = config.redact_json_value(&body);
        assert_eq!(redacted["user"]["name"], "alice");
        assert_eq!(redacted["user"]["api_key"], REDACTED_HEADER_VALUE);
        assert_eq!(redacted["items"][0]["id"], 1);
        assert_eq!(redacted["items"][1]["token"], REDACTED_HEADER_VALUE);

        let json = serde_json::to_string(&redacted).unwrap();
        assert!(
            !json.contains("sk-live-very-secret") && !json.contains("should-not-appear"),
            "no redacted value should survive serialisation: {json}"
        );
    }

    /// RFC 073 S-05's actual delivery mechanism for the trace channel:
    /// `enrich_with_body` — not just the standalone `redact_json_value`
    /// helper — redacts before storing, so a subscriber over the
    /// UDS/TCP transport never receives the raw secret either.
    #[test]
    fn enrich_with_body_redacts_a_captured_body() {
        let emitter = TraceEmitter::with_config(TraceConfig {
            capture_body: true,
            ..Default::default()
        });
        let mut summary = dummy_summary();
        let body = serde_json::json!({"action": "login", "password": "hunter2"});
        emitter.enrich_with_body(&mut summary, Some(&body));

        let captured = summary.body_json.expect("body should be captured");
        assert_eq!(captured["action"], "login");
        assert_eq!(captured["password"], REDACTED_HEADER_VALUE);
    }

    /// `Outcome::Middleware` (RFC 073 F-08) serialises with a
    /// discriminated `type` tag like every other variant, carrying the
    /// matched middleware script's own path.
    #[test]
    fn outcome_middleware_serialises_with_file_path_and_status() {
        let outcome = Outcome::Middleware {
            file_path: "middleware/auth.rhai".into(),
            status: 200,
        };
        let json = serde_json::to_string(&outcome).unwrap();
        assert!(json.contains("\"type\":\"middleware\""), "json was: {json}");
        assert!(
            json.contains("\"file_path\":\"middleware/auth.rhai\""),
            "json was: {json}"
        );
        assert!(json.contains("\"status\":200"), "json was: {json}");
    }

    /// RFC 073: the UDS socket file is created owner-only (`0600`), not
    /// left at whatever the process umask would otherwise produce.
    #[cfg(unix)]
    #[tokio::test]
    async fn uds_socket_is_created_with_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("trace.sock").to_str().unwrap().to_owned();
        let emitter = TraceEmitter::new();

        let accept_loop = tokio::spawn(TraceTransport::accept_loop(
            TraceTransportConfig::Uds { path: path.clone() },
            emitter,
        ));

        // The accept loop sets permissions synchronously, right after
        // bind, before its first `accept().await` — poll for the file
        // to exist rather than a fixed sleep, since scheduling order
        // between this test task and the spawned one isn't guaranteed.
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !std::path::Path::new(&path).exists() {
            assert!(
                std::time::Instant::now() < deadline,
                "socket never appeared"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        let mode = std::fs::metadata(&path)
            .expect("stat socket")
            .permissions()
            .mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "socket permissions should be owner-only, got {mode:o}"
        );

        accept_loop.abort();
    }
}
