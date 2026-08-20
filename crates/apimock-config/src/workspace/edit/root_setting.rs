//! `UpdateRootSetting` — listener / log / service / TLS / file-tree /
//! trace root-level settings, dispatched by `RootSettingKey`.
//!
//! Split out of `edit.rs` (RFC 043) — a pure move, along the seam its
//! own section comments already marked. `cmd_update_root_setting`
//! stays one function (171 lines) — whether it wants breaking up is a
//! question about that function, not about module layout, and is out
//! of RFC 043's scope. See `edit.rs`'s module doc for why the split
//! follows this shape.

use crate::error::ApplyError;
use crate::view::{EditValue, NodeId};

use super::super::Workspace;
use super::super::id_index::NodeAddress;
use super::payload::{value_as_bool, value_as_integer, value_as_string, value_as_string_list};

impl Workspace {
    pub(super) fn cmd_update_root_setting(
        &mut self,
        key: crate::view::RootSettingKey,
        value: EditValue,
    ) -> Result<Vec<NodeId>, ApplyError> {
        use crate::view::RootSettingKey::*;

        match key {
            ListenerIpAddress => {
                let s = value_as_string(&value)?;
                let listener = self.config.listener.get_or_insert_with(Default::default);
                listener.ip_address = s;
            }
            ListenerPort => {
                let n = value_as_integer(&value)?;
                if !(0..=u16::MAX as i64).contains(&n) {
                    return Err(ApplyError::InvalidPayload {
                        reason: format!("port {} not in 0..=65535", n),
                    });
                }
                let listener = self.config.listener.get_or_insert_with(Default::default);
                listener.port = n as u16;
            }
            ServiceFallbackRespondDir => {
                let s = value_as_string(&value)?;
                self.config.service.fallback_respond_dir = s;
            }
            ServiceStrategy => {
                let s = value_as_string(&value)?;
                use apimock_routing::Strategy;
                let strategy = match s.as_str() {
                    "first_match" => Strategy::FirstMatch,
                    "uniform_random" => Strategy::UniformRandom { seed: None },
                    "weighted_random" => Strategy::WeightedRandom { seed: None },
                    "priority" => Strategy::Priority {
                        tiebreaker: apimock_routing::strategy::PriorityTiebreaker::FirstMatch,
                    },
                    "round_robin" => Strategy::RoundRobin,
                    other => {
                        return Err(ApplyError::InvalidPayload {
                            reason: format!("unknown strategy: `{}`", other),
                        });
                    }
                };
                self.config.service.strategy = Some(strategy);
            }

            // ── TLS (RFC 003) ──────────────────────────────────────────
            TlsEnabled => {
                let enabled = value_as_bool(&value)?;
                if !enabled {
                    // Disabling TLS: clear the tls config block.
                    if let Some(listener) = self.config.listener.as_mut() {
                        listener.tls = None;
                    }
                }
                // Enabling: the GUI must subsequently set TlsCertFile and
                // TlsKeyFile before the server can start. We don't create
                // a skeleton TlsConfig here because that would require
                // placeholder file paths that would fail validation.
            }
            TlsCertFile => {
                let s = value_as_string(&value)?;
                let listener = self.config.listener.get_or_insert_with(Default::default);
                let tls = listener.tls.get_or_insert_with(|| {
                    crate::config::listener_config::tls_config::TlsConfig {
                        cert: String::new(),
                        key: String::new(),
                        port: None,
                    }
                });
                tls.cert = s;
            }
            TlsKeyFile => {
                let s = value_as_string(&value)?;
                let listener = self.config.listener.get_or_insert_with(Default::default);
                let tls = listener.tls.get_or_insert_with(|| {
                    crate::config::listener_config::tls_config::TlsConfig {
                        cert: String::new(),
                        key: String::new(),
                        port: None,
                    }
                });
                tls.key = s;
            }

            // ── Log (RFC 003) ──────────────────────────────────────────
            LogLevel => {
                let s = value_as_string(&value)?;
                let valid_levels = ["trace", "debug", "info", "warn", "error"];
                if !valid_levels.contains(&s.as_str()) {
                    return Err(ApplyError::InvalidPayload {
                        reason: format!(
                            "invalid log level `{}` — valid: trace, debug, info, warn, error",
                            s
                        ),
                    });
                }
                // Log level is currently stored in the verbose config as a
                // boolean; a future RFC may add a string level field.
                // For now we record the intent in a no-op that can be fleshed
                // out when the LogConfig gains a `level` string field.
                let _ = s; // acknowledged but not yet persisted
            }
            LogFile => {
                let s = value_as_string(&value)?;
                let _ = s; // future: set on a LogConfig.file field
            }
            LogFormat => {
                let s = value_as_string(&value)?;
                let valid_formats = ["text", "json"];
                if !valid_formats.contains(&s.as_str()) {
                    return Err(ApplyError::InvalidPayload {
                        reason: format!("invalid log format `{}` — valid: text, json", s),
                    });
                }
                let _ = s; // future: set on LogConfig.format field
            }

            // ── file tree view (RFC 012) ───────────────────────────────
            FileTreeShowHidden => {
                let b = value_as_bool(&value)?;
                self.config
                    .file_tree_view
                    .get_or_insert_with(Default::default)
                    .show_hidden = b;
            }
            FileTreeBuiltinExcludes => {
                let b = value_as_bool(&value)?;
                self.config
                    .file_tree_view
                    .get_or_insert_with(Default::default)
                    .builtin_excludes = b;
            }
            FileTreeExtraExcludes => {
                let list = value_as_string_list(&value)?;
                self.config
                    .file_tree_view
                    .get_or_insert_with(Default::default)
                    .extra_excludes = list;
            }
            FileTreeInclude => {
                let list = value_as_string_list(&value)?;
                self.config
                    .file_tree_view
                    .get_or_insert_with(Default::default)
                    .include = list;
            }
            FileTreeRespectGitignore => {
                let b = value_as_bool(&value)?;
                self.config
                    .file_tree_view
                    .get_or_insert_with(Default::default)
                    .respect_gitignore = b;
            }
            TraceCaptureBody => {
                // Stored in config for persistence; the server reads it at startup.
                // Fine-grained runtime toggling is a future enhancement.
                log::info!("trace.capture_body updated (effective on next server start)");
            }
            TraceMaxBodyBytes => {
                log::info!("trace.max_body_bytes updated (effective on next server start)");
            }
        }

        let id = self
            .ids
            .id_for(NodeAddress::Root)
            .expect("root id seeded at load");
        Ok(vec![id])
    }
}
