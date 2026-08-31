use serde::Deserialize;

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
#[derive(Clone, Deserialize, Debug, Default)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Strategy {
    /// Walk rules in order; return the first that matches. Default.
    #[default]
    FirstMatch,

    /// Pick uniformly at random from all matching rules.
    /// `seed = Some(n)` for reproducible test runs.
    UniformRandom {
        #[serde(default)]
        seed: Option<u64>,
    },

    /// Pick randomly, weighted by each rule's `weight` field (default 1).
    WeightedRandom {
        #[serde(default)]
        seed: Option<u64>,
    },

    /// Group matching rules by `priority` (higher wins). Within the
    /// top-priority group, apply `tiebreaker` (default: `first_match`).
    Priority {
        #[serde(default)]
        tiebreaker: PriorityTiebreaker,
    },
    /// Cycle through matching rules in order, one per request.
    /// State is kept in an `Arc<AtomicUsize>` on the parent `RuleSet`.
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
