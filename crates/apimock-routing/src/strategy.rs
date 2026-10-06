use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, Visitor, value::MapAccessDeserializer},
};

/// Rule-evaluation strategy: decides which rule wins when multiple
/// rules match the same request.
///
/// # RFC 007 — Strategy variants
///
/// The original `FirstMatch` strategy is unchanged and remains the
/// default. Three new strategies are added:
///
/// - [`UniformRandom`](Strategy::UniformRandom) — pick uniformly at
///   random from all matching rules.
/// - [`WeightedRandom`](Strategy::WeightedRandom) — pick randomly,
///   weighted by each rule's `weight`.
/// - [`Priority`](Strategy::Priority) — group by priority, apply a
///   tiebreaker within the group.
///
/// # How a strategy is spelled in a config file (RFC 085)
///
/// Name the strategy: `strategy = "weighted_random"`. That is every
/// strategy with its options at their defaults. To set an option, use the
/// table form, `strategy = { weighted_random = { seed = 7 } }`; the table
/// form also accepts an empty table, so `{ weighted_random = {} }` and the
/// bare name load to the same strategy. Both spellings work at both levels
/// (`[service] strategy` and a rule set's `strategy`), because both are this
/// type. Before RFC 085 only `first_match` and `round_robin` loaded as bare
/// names.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub enum Strategy {
    /// Walk rules in order; return the first that matches. Default.
    #[default]
    FirstMatch,

    /// Pick uniformly at random from all matching rules.
    /// `seed = Some(n)` for reproducible test runs.
    UniformRandom { seed: Option<u64> },

    /// Pick randomly, weighted by each rule's `weight` field (default 1).
    WeightedRandom { seed: Option<u64> },

    /// Group matching rules by `priority` (higher wins). Within the
    /// top-priority group, apply `tiebreaker` (default: `first_match`).
    Priority { tiebreaker: PriorityTiebreaker },
    /// Cycle through matching rules in order, one per request. Rotates
    /// independently per distinct set of matching rules (RFC 070) —
    /// state is kept in a per-match-group counter map on the parent
    /// `RuleSet`, not one counter for the whole rule set.
    RoundRobin,
}

/// Tiebreaker applied within a priority group by [`Strategy::Priority`].
#[derive(Clone, Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum PriorityTiebreaker {
    #[default]
    FirstMatch,
    UniformRandom,
}

/// The strategies a config file can name, in the order the refusal lists
/// them. The one list the bare-name spelling is checked against and the
/// refusal is generated from, so the two cannot drift (compare RFC 082
/// Amendment 1).
const NAMES: [&str; 5] = [
    "first_match",
    "round_robin",
    "uniform_random",
    "weighted_random",
    "priority",
];

/// The strategy a bare name stands for: that strategy, every option at its
/// default.
fn from_bare_name(name: &str) -> Option<Strategy> {
    Some(match name {
        "first_match" => Strategy::FirstMatch,
        "round_robin" => Strategy::RoundRobin,
        "uniform_random" => Strategy::UniformRandom { seed: None },
        "weighted_random" => Strategy::WeightedRandom { seed: None },
        "priority" => Strategy::Priority {
            tiebreaker: PriorityTiebreaker::default(),
        },
        _ => return None,
    })
}

/// The table spelling, `{ <name> = { …options } }`: the shape `Strategy`
/// loaded as before RFC 085, kept as a private mirror so that the table
/// form behaves exactly as it did (including how an unknown option key
/// inside it is treated, and serde's own refusal of an unknown name).
/// `Strategy`'s `Deserialize` delegates a table to this.
///
/// It lists the variants a second time; `tests::every_variant_has_a_bare_name`
/// and the exhaustive `From` below stop a new variant being added to one and
/// not the other.
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum StrategyTable {
    FirstMatch,
    UniformRandom {
        #[serde(default)]
        seed: Option<u64>,
    },
    WeightedRandom {
        #[serde(default)]
        seed: Option<u64>,
    },
    Priority {
        #[serde(default)]
        tiebreaker: PriorityTiebreaker,
    },
    RoundRobin,
}

impl From<StrategyTable> for Strategy {
    fn from(table: StrategyTable) -> Self {
        match table {
            StrategyTable::FirstMatch => Strategy::FirstMatch,
            StrategyTable::UniformRandom { seed } => Strategy::UniformRandom { seed },
            StrategyTable::WeightedRandom { seed } => Strategy::WeightedRandom { seed },
            StrategyTable::Priority { tiebreaker } => Strategy::Priority { tiebreaker },
            StrategyTable::RoundRobin => Strategy::RoundRobin,
        }
    }
}

impl<'de> Deserialize<'de> for Strategy {
    /// A bare name, or a table naming a strategy with its options. A name
    /// that is neither is refused with the names listed, not with serde's
    /// "data did not match any variant".
    fn deserialize<__D>(deserializer: __D) -> Result<Self, __D::Error>
    where
        __D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrategyVisitor)
    }
}

struct StrategyVisitor;

impl<'de> Visitor<'de> for StrategyVisitor {
    type Value = Strategy;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a strategy name, or a table naming one with its options")
    }

    fn visit_str<E: de::Error>(self, name: &str) -> Result<Strategy, E> {
        from_bare_name(name).ok_or_else(|| {
            let names = NAMES
                .iter()
                .map(|n| format!("`{n}`"))
                .collect::<Vec<_>>()
                .join(", ");
            E::custom(format!(
                "unknown strategy `{name}`, expected one of {names}"
            ))
        })
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Strategy, A::Error> {
        StrategyTable::deserialize(MapAccessDeserializer::new(map)).map(Strategy::from)
    }
}

impl std::fmt::Display for Strategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FirstMatch => write!(f, "first_match"),
            Self::UniformRandom { .. } => write!(f, "uniform_random"),
            Self::WeightedRandom { .. } => write!(f, "weighted_random"),
            Self::Priority { .. } => write!(f, "priority"),
            Self::RoundRobin => write!(f, "round_robin"),
        }
    }
}

// ── minimal PRNG (no external dep) ───────────────────────────────────

/// xorshift64 PRNG — fast, no-alloc, no external dependency.
pub struct Xorshift64(u64);

impl Xorshift64 {
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0xdeadbeef_cafebabe } else { seed })
    }

    // clippy: renaming `next` would change apimock_routing::strategy::
    // Xorshift64's public API surface; this PRNG helper intentionally does
    // not implement std::iter::Iterator.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform index in `0..len`.
    pub fn next_index(&mut self, len: usize) -> usize {
        (self.next() % len as u64) as usize
    }
}

pub fn make_rng(seed: Option<u64>) -> Xorshift64 {
    let s = seed.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xc0ffee)
    });
    Xorshift64::new(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Strategy` has no `PartialEq` (adding one would widen the public API);
    /// its derived `Debug` shows every option, so equal strings mean equal
    /// strategies.
    fn same(a: &Strategy, b: &Strategy) -> bool {
        format!("{a:?}") == format!("{b:?}")
    }

    #[derive(Deserialize)]
    struct Holder {
        strategy: Strategy,
    }

    fn load(spelling: &str) -> Result<Strategy, String> {
        toml::from_str::<Holder>(&format!("strategy = {spelling}"))
            .map(|h| h.strategy)
            .map_err(|e| e.to_string())
    }

    /// RFC 085 Option B: a bare name is that strategy with default options,
    /// for all five. Before, only `first_match` and `round_robin` loaded.
    #[test]
    fn every_bare_name_loads_with_default_options() {
        assert!(same(
            &load("\"first_match\"").unwrap(),
            &Strategy::FirstMatch
        ));
        assert!(same(
            &load("\"round_robin\"").unwrap(),
            &Strategy::RoundRobin
        ));
        assert!(same(
            &load("\"uniform_random\"").unwrap(),
            &Strategy::UniformRandom { seed: None }
        ));
        assert!(same(
            &load("\"weighted_random\"").unwrap(),
            &Strategy::WeightedRandom { seed: None }
        ));
        assert!(same(
            &load("\"priority\"").unwrap(),
            &Strategy::Priority {
                tiebreaker: PriorityTiebreaker::FirstMatch
            }
        ));
    }

    /// The bare name and an empty table are the same strategy, and a table
    /// with options still sets them.
    #[test]
    fn a_bare_name_and_an_empty_table_load_to_the_same_strategy() {
        for name in ["uniform_random", "weighted_random", "priority"] {
            let bare = load(&format!("\"{name}\"")).unwrap();
            let table = load(&format!("{{ {name} = {{}} }}")).unwrap();
            assert!(same(&bare, &table), "{name}: {bare:?} vs {table:?}");
        }
        assert!(same(
            &load("{ weighted_random = { seed = 7 } }").unwrap(),
            &Strategy::WeightedRandom { seed: Some(7) }
        ));
        assert!(same(
            &load("{ priority = { tiebreaker = \"uniform_random\" } }").unwrap(),
            &Strategy::Priority {
                tiebreaker: PriorityTiebreaker::UniformRandom
            }
        ));
    }

    /// The table form behaves as it did: an unknown option key inside it is
    /// ignored, as the derived loader always did.
    #[test]
    fn an_unknown_option_key_in_the_table_form_is_ignored_as_before() {
        assert!(same(
            &load("{ weighted_random = { seed = 7, future = 1 } }").unwrap(),
            &Strategy::WeightedRandom { seed: Some(7) }
        ));
    }

    /// A typo is refused with the five names, not serde's generic
    /// "data did not match any variant".
    #[test]
    fn a_typo_in_a_bare_name_is_refused_with_the_names_listed() {
        let err = load("\"weigthed_random\"").unwrap_err();
        assert!(
            err.contains(
                "unknown strategy `weigthed_random`, expected one of `first_match`, \
                 `round_robin`, `uniform_random`, `weighted_random`, `priority`"
            ),
            "{err}"
        );
        assert!(!err.contains("did not match any variant"), "{err}");
    }

    /// The names the refusal lists are exactly the ones that load.
    #[test]
    fn the_refusal_lists_exactly_the_names_that_load() {
        for name in NAMES {
            assert!(
                from_bare_name(name).is_some(),
                "{name} is listed but does not load"
            );
        }
        let err = load("\"nope\"").unwrap_err();
        for name in NAMES {
            assert!(
                err.contains(&format!("`{name}`")),
                "{name} missing from: {err}"
            );
        }
    }

    /// A new variant must get a bare name, a table spelling and a `Display`
    /// name. The match has no wildcard, so a variant added to `Strategy` and
    /// not listed here fails to compile (this is inside the defining crate,
    /// where `#[non_exhaustive]` does not apply).
    #[test]
    fn every_variant_has_a_bare_name() {
        fn name(s: &Strategy) -> &'static str {
            match s {
                Strategy::FirstMatch => "first_match",
                Strategy::UniformRandom { .. } => "uniform_random",
                Strategy::WeightedRandom { .. } => "weighted_random",
                Strategy::Priority { .. } => "priority",
                Strategy::RoundRobin => "round_robin",
            }
        }
        for listed in NAMES {
            let s = from_bare_name(listed).unwrap();
            assert_eq!(name(&s), listed);
            assert_eq!(s.to_string(), listed, "Display must agree with the name");
        }
        assert_eq!(NAMES.len(), 5);
    }
}
