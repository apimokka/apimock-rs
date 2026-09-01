pub const LISTENER_DEFAULT_IP_ADDRESS: &str = "127.0.0.1";
pub const LISTENER_DEFAULT_PORT: u16 = 3001;

/// RFC 074 S-07: how long an incomplete TLS handshake may hold a
/// connection before it's dropped, when `[listener.tls]` doesn't set
/// `handshake_timeout_seconds`. A local handshake completes in
/// milliseconds; 10s is generous for a loaded CI runner while still
/// being a real bound on a client that opens a connection and sends
/// nothing.
pub const TLS_DEFAULT_HANDSHAKE_TIMEOUT_SECONDS: u64 = 10;

/// RFC 074 S-07: maximum concurrent HTTPS connections, when
/// `[listener.tls]` doesn't set `max_connections`. Far above what a
/// parallel test suite driving this project's own integration tests
/// needs (each test dials its own dedicated listener; the busiest
/// realistic case is a handful of sequential requests per test), far
/// below "no bound at all."
pub const TLS_DEFAULT_MAX_CONNECTIONS: usize = 256;

pub const SERVICE_DEFAULT_FALLBACK_RESPOND_DIR: &str = ".";

/// RFC 068 S-02: default cap on a single request body, when
/// `[service]` doesn't set `max_request_body_bytes`. Far above any
/// realistic mock request, far below the 462 MiB RSS the external
/// audit reached with no limit at all.
pub const SERVICE_DEFAULT_MAX_REQUEST_BODY_BYTES: u64 = 32 * 1024 * 1024;

/// RFC 068 S-03: default cap on Rhai operations per middleware
/// evaluation, when `[service]` doesn't set
/// `middleware_max_operations`. Generous enough that no reasonable
/// script hits it — Rhai's own default engine limit is unbounded, and
/// this project's own middleware examples do a handful of field
/// lookups and string operations, nowhere near this ceiling.
pub const SERVICE_DEFAULT_MIDDLEWARE_MAX_OPERATIONS: u64 = 10_000_000;

pub const PRINT_DELIMITER: &str = "------------------------------------";
