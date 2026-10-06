//! RFC 085 — the editor writes a strategy the loader accepts.
//!
//! `UpdateRootSetting { ServiceStrategy }` and `UpdateRuleSetStrategy`
//! build the right model (`WeightedRandom { seed: None }`) and hand it to
//! the writer. The writer used to emit the bare name, which three of the
//! five strategies do not load as, so a GUI choosing a strategy wrote a file
//! its own loader refused. These tests run the real loader on what each
//! name writes.

use super::common::make_workspace;
use crate::{
    Config,
    view::{ConfigFileKind, EditCommand, EditValue, NodeKind, RootSettingKey},
    workspace::Workspace,
};
use apimock_routing::{Strategy, strategy::PriorityTiebreaker};

const NAMES: [&str; 5] = [
    "first_match",
    "round_robin",
    "uniform_random",
    "weighted_random",
    "priority",
];

fn load_config(root: &std::path::Path) -> Config {
    let path = root.to_string_lossy().into_owned();
    Config::new(Some(&path), None).unwrap_or_else(|e| {
        panic!(
            "the saved config does not load: {e}\n--- apimock.toml ---\n{}",
            std::fs::read_to_string(root).unwrap_or_default()
        )
    })
}

fn debug(s: &Strategy) -> String {
    format!("{s:?}")
}

fn default_model(name: &str) -> Strategy {
    match name {
        "first_match" => Strategy::FirstMatch,
        "round_robin" => Strategy::RoundRobin,
        "uniform_random" => Strategy::UniformRandom { seed: None },
        "weighted_random" => Strategy::WeightedRandom { seed: None },
        "priority" => Strategy::Priority {
            tiebreaker: PriorityTiebreaker::FirstMatch,
        },
        other => panic!("unknown strategy {other}"),
    }
}

/// § 4: each of the five names, chosen through the editor at `[service]`
/// level, produces a file that loads, with that strategy.
#[test]
fn every_service_strategy_the_editor_can_choose_loads() {
    for name in NAMES {
        let (_dir, root) = make_workspace();
        let mut ws = Workspace::load(root.clone()).unwrap();
        ws.apply(EditCommand::UpdateRootSetting {
            key: RootSettingKey::ServiceStrategy,
            value: EditValue::String(name.to_owned()),
        })
        .unwrap();
        ws.save().unwrap();

        let config = load_config(&root);
        assert_eq!(
            debug(config.service.strategy.as_ref().unwrap()),
            debug(&default_model(name)),
            "{name}"
        );
    }
}

/// The same for a rule set's own strategy.
#[test]
fn every_rule_set_strategy_the_editor_can_choose_loads() {
    for name in NAMES {
        let (_dir, root) = make_workspace();
        let mut ws = Workspace::load(root.clone()).unwrap();
        let rule_set = ws
            .snapshot()
            .files
            .iter()
            .find(|f| matches!(f.kind, ConfigFileKind::RuleSet))
            .unwrap()
            .nodes
            .iter()
            .find(|n| matches!(n.kind, NodeKind::RuleSet))
            .unwrap()
            .id;
        ws.apply(EditCommand::UpdateRuleSetStrategy {
            id: rule_set,
            strategy: Some(name.to_owned()),
        })
        .unwrap();
        ws.save().unwrap();

        let config = load_config(&root);
        assert_eq!(
            debug(config.service.rule_sets[0].strategy.as_ref().unwrap()),
            debug(&default_model(name)),
            "{name}"
        );
    }
}

/// § 4: a changed option is written, not dropped. No editor command sets a
/// seed, so the model is changed directly: unset, then 7, then 8. Each save
/// is read back with the real loader.
#[test]
fn a_changed_seed_is_written_and_loads() {
    let (_dir, root) = make_workspace();
    let rules = root.parent().unwrap().join("apimock-rule-set.toml");
    let mut ws = Workspace::load(root.clone()).unwrap();

    for seed in [Some(7), Some(8), None] {
        ws.config.service.rule_sets[0].strategy = Some(Strategy::WeightedRandom { seed });
        ws.save().unwrap();

        let text = std::fs::read_to_string(&rules).unwrap();
        match seed {
            Some(n) => assert!(text.contains(&format!("seed = {n}")), "{text}"),
            None => assert!(!text.contains("seed"), "{text}"),
        }
        let config = load_config(&root);
        assert_eq!(
            debug(config.service.rule_sets[0].strategy.as_ref().unwrap()),
            debug(&Strategy::WeightedRandom { seed }),
            "seed {seed:?}\n{text}"
        );
    }
}
