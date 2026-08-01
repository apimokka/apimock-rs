//! Routing model for apimock.
//!
//! # Responsibilities
//!
//! This crate owns everything declarative about *how* requests match rules:
//!
//! - [`RuleSet`] — an ordered collection of rules, plus optional prefix /
//!   default / guard metadata. Loaded from one TOML file.
//! - [`Rule`] — a single `when { … } respond { … }` entry.
//! - [`Respond`] — the declarative description of a response (file /
//!   text / status). **Does not** build the `hyper::Response` — that's
//!   `apimock-server`'s job.
//! - [`Strategy`] — how multiple matching rules in a set are resolved.
//!   (First-match only, at present.)
//! - Matching primitives: glob, JSONPath, URL-path normalization.
//! - Read-only views for GUI tooling: [`RouteCatalogSnapshot`],
//!   [`RuleSetView`], [`RuleView`], [`RespondView`], [`RouteMatchView`],
//!   [`RouteValidation`].
//!
//! # What is deliberately *not* here
//!
//! - HTTP response construction — a matched [`Respond`] is interpreted
//!   by `apimock-server`.
//! - File I/O for response bodies — also server-side.
//! - TOML parsing orchestration (which files to load, in which order) —
//!   that's `apimock-config`. This crate only parses an *individual*
//!   rule-set TOML file when asked.
//!
//! # Stability
//!
//! The types in [`view`] are the stable surface a GUI can depend on.
//! The internal [`RuleSet`] struct and its methods can still churn as
//! apimock's rule language evolves — views act as the insulation layer.

pub mod error;
pub mod parsed_request;
pub mod rule_set;
pub mod strategy;
pub mod util;
pub mod view;

pub use error::{RoutingError, RoutingResult};
pub use parsed_request::ParsedRequest;
pub use rule_set::{RuleSet, rule::Rule, rule::respond::Respond};
pub use strategy::Strategy;
pub use view::build::{BUILTIN_EXCLUDES, FileTreeFilter};
pub use view::{
    BodyConditionView, FileNodeKind, FileNodeView, FileTreeView, HeaderConditionView,
    MatchConsidered, MatchedRule, RespondView, RouteCatalogSnapshot, RouteMatchView,
    RouteValidation, RouteValidationIssue, RuleSetView, RuleView, ScriptRouteView, UrlPathView,
    ValidationSeverity, WhenView,
};
