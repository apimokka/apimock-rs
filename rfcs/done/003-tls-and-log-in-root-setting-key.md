# RFC 003 — TLS and log settings in RootSettingKey

**Status.** Implemented (v5.8.0)
**Tracks.** Stage-2 GUI editing — extending the addressable root
settings to cover TLS configuration and log settings.
**Touches.** `apimock-config` (`RootSettingKey`, `UpdateRootSetting`
handling, `EditValue` shape for paths, validation, `toml_writer` for
root-level TLS / log sections), `apimock-server` (no internal change
but reload semantics are clarified).

## Summary

`RootSettingKey` currently exposes four variants:
`ListenerIpAddress`, `ListenerPort`, `ServiceFallbackRespondDir`,
`ServiceStrategy`. TLS and log configuration exist on disk and are
loaded at server startup, but they cannot be edited through the
GUI `UpdateRootSetting` command. This RFC adds variants for both
groups, with explicit notes on which fields are reloadable and which
require a server restart.

## Motivation

A user setting up an HTTPS mock server today must either:

1. hand-edit the TOML file, *then* restart the server, *then* reload
   the GUI's workspace; or
2. use a non-GUI configuration tool entirely.

Both contradict the stage-2 direction: the GUI should be the
authoritative editing surface. TLS and log are the two most-requested
root settings according to past discussion and the GUI architect
brief's §12 "Known gaps" list.

The reload semantics are a complication: changing the TLS certificate
path while the server is listening requires either a rebind or a
process restart. The GUI cannot make that decision blindly. This RFC
proposes that `UpdateRootSetting` returns a richer `ReloadHint` that
distinguishes "reload config" from "restart process".

## Guide-level explanation

The root settings panel in the GUI gains two new sections:

```
TLS
  [✓] Enable HTTPS
  Certificate file:   [ /etc/apimock/cert.pem            ]
  Key file:           [ /etc/apimock/key.pem             ]

Logging
  Level:              [ info ▼ ]
  File:               [ /var/log/apimock/server.log      ]
  Format:             [ json ▼ ]
```

When a user changes a TLS field, the save flow shows: "This change
requires the server process to be restarted." When a user changes
the log level only, it shows: "This change will take effect after
the next config reload." The hint comes from the server crate via
the existing `ReloadHint` enum (extended as needed).

## Reference-level explanation

### Variant additions

```rust
pub enum RootSettingKey {
    ListenerIpAddress,
    ListenerPort,
    ServiceFallbackRespondDir,
    ServiceStrategy,

    // TLS (NEW)
    TlsEnabled,
    TlsCertFile,
    TlsKeyFile,

    // Log (NEW)
    LogLevel,
    LogFile,
    LogFormat,
}
```

### Value typing

`UpdateRootSetting { key, value: EditValue }` uses `EditValue`
(currently `String` / `Integer` / `Boolean`). The new variants need:

| Key           | EditValue variant | Notes                                  |
|---------------|-------------------|----------------------------------------|
| `TlsEnabled`  | `Boolean`         |                                        |
| `TlsCertFile` | `String`          | Path; validation checks existence.     |
| `TlsKeyFile`  | `String`          | Path; validation checks existence.     |
| `LogLevel`    | `String`          | One of `trace/debug/info/warn/error`.  |
| `LogFile`     | `String`          | Path; validation checks parent dir.    |
| `LogFormat`   | `String`          | One of `text/json`.                    |

`EditValue` may need a new `Path` variant for type clarity, but the
RFC's recommendation is to defer that — `String` plus key-specific
validation is enough at stage-2 and keeps the variant set small.

### Reload semantics

The server's existing `ReloadHint` is currently a single advisory
value the workspace returns to the GUI. This RFC extends it to:

```rust
pub enum ReloadHint {
    None,                 // change has no runtime effect (e.g. comment edits)
    SoftReload,            // server can re-read config without rebinding
    HardRestart,           // process must restart (rebind socket, reload TLS)
}
```

Mapping per key:

| Key                          | Hint           |
|------------------------------|----------------|
| `ListenerIpAddress`          | `HardRestart`  |
| `ListenerPort`               | `HardRestart`  |
| `TlsEnabled` / `TlsCert*`    | `HardRestart`  |
| `ServiceFallbackRespondDir`  | `SoftReload`   |
| `ServiceStrategy`            | `SoftReload`   |
| `LogLevel` / `LogFormat`     | `SoftReload`   |
| `LogFile`                    | `HardRestart`  |

The hint is advisory: the server does not auto-restart (per the
constitution in the architect brief §11). The GUI surfaces the hint
to the user and lets them choose.

### Validation

- `TlsCertFile` / `TlsKeyFile`: file exists, readable, parses as the
  appropriate PEM/DER format. The cert/key pair compatibility check
  is *not* required at validation — it's expensive and the server
  will fail loudly at bind time anyway. Validation should catch
  obvious mistakes (file missing, empty file) cheaply.
- `LogLevel`: one of the canonical levels.
- `LogFile`: parent directory exists and is writable.
- `LogFormat`: one of the canonical formats.

### TOML shape

The current `apimock.toml` already has these sections; this RFC does
not change their on-disk shape. The change is that `toml_writer` and
the `UpdateRootSetting` handler now cover the additional keys.

## Drawbacks

1. **`RootSettingKey` grows from 4 to 10 variants.** Once the trend
   continues (CORS, rate limits, custom middleware paths, etc.), the
   enum will accumulate variants. A future RFC may need to introduce
   key namespacing (`Tls(TlsKey)`, `Log(LogKey)`) to keep things
   readable.
2. **The TLS variants embed a usability cliff.** Authoring TLS
   correctly requires understanding cert chains, hostname matching,
   etc. The GUI can't fix that on its own; bad TLS config still
   fails at bind. The RFC accepts this — the GUI's job is to make
   editing possible, not to fix TLS UX.
3. **`HardRestart` shifts user burden.** A user changing the port
   currently sees a hint but the server keeps running on the old
   port until they restart. This is functionally identical to today
   but new keys make it more visible. We accept this as honest:
   the alternative (auto-restart) violates the no-self-restart
   constitution.

## Rationale and alternatives

**Alternative A: namespace into sub-enums now.** Defines
`RootSettingKey::Tls(TlsKey)` and `RootSettingKey::Log(LogKey)`
upfront. Cleaner long-term but creates a needless transitional shape
for stage-2. We can adopt it once the variant count justifies the
ergonomic cost.

**Alternative B (this RFC): flat enum, named with prefixes.** Maps
1:1 to the existing four variants' style. Easiest to slot into
existing code.

**Alternative C: a "free-form root setting" command that accepts a
path and a value.** Maximally flexible, minimally typed. Rejected
because GUI forms benefit from typed enumeration of keys.

We pick B. A is the right shape for ~15+ variants; not yet.

## Prior art

- nginx-mock-server uses a flat YAML root-config with explicit
  TLS / log sections — close to the current apimock shape.
- Mountebank exposes log settings through CLI flags rather than the
  imposter file; not directly applicable.
- The Caddy admin API treats every config field as a JSON-Pointer
  key into the live config; a more dynamic shape than this RFC, but
  worth mentioning as a stage-3 evolution target.

## Unresolved questions

1. **TLS hot-reload feasibility.** Some servers can reload TLS certs
   without rebinding. Whether the apimock-server crate's HTTP stack
   supports it depends on the underlying library version. If it
   does, `TlsCert*` could become `SoftReload`. To be confirmed by a
   server-crate audit; the conservative default in this RFC is
   `HardRestart`.
2. **Log file rotation interaction.** External rotation tools
   (`logrotate`, etc.) hold file handles; changing the log path
   in-place could conflict. Out of scope for this RFC but worth a
   note in operational docs.
3. **Encrypted-key passphrase.** If the TLS key is encrypted, the
   GUI needs a passphrase input. Defer to a follow-up RFC — current
   apimock-server may not support encrypted keys at all.

## Future possibilities

- Per-listener TLS (separate cert per virtual host) — needs
  significant server-side work; out of scope.
- Structured log filters (per-module level overrides). Today's
  `LogLevel` is global; if the server crate gains per-target
  filtering, this RFC's `LogLevel` variant can grow into a
  composite key.
- mTLS (client cert verification). Another `Tls*` variant once
  the server stack supports it.
- Namespacing refactor (alternative A above) once the variant
  count crosses ~15.
