//! RFC 012 — config-driven FileTreeFilter.

use super::common::{make_workspace, write_apimock_toml};
use crate::{
    view::EditCommand,
    workspace::Workspace,
};

#[test]
fn snapshot_default_filter_hides_dotfiles_and_target() {
    let dir = tempfile::tempdir().unwrap();
    let fallback = dir.path().join("fallback");
    std::fs::create_dir_all(&fallback).unwrap();
    std::fs::write(fallback.join("users.json"), "{}").unwrap();
    std::fs::write(fallback.join(".env"), "SECRET=1").unwrap();
    std::fs::create_dir_all(fallback.join("target")).unwrap();

    let rs_toml = "[[rules]]\nwhen.request.url_path = \"/api\"\nrespond = { text = \"ok\" }\n";
    std::fs::write(dir.path().join("apimock-rule-set.toml"), rs_toml).unwrap();
    let cfg_path = write_apimock_toml(dir.path(), "fallback", "");

    let ws = Workspace::load(cfg_path).unwrap();
    let snap = ws.snapshot();
    let tree = snap.routes.file_tree.as_ref().expect("file tree present");

    let names: Vec<&str> = tree.entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"users.json"), "users.json must be visible");
    assert!(!names.contains(&".env"), ".env must be hidden by default filter");
    assert!(!names.contains(&"target"), "target/ must be hidden by default filter");
}

#[test]
fn snapshot_show_hidden_exposes_dotfiles() {
    let dir = tempfile::tempdir().unwrap();
    let fallback = dir.path().join("fallback");
    std::fs::create_dir_all(&fallback).unwrap();
    std::fs::write(fallback.join("users.json"), "{}").unwrap();
    std::fs::write(fallback.join(".env"), "SECRET=1").unwrap();

    let rs_toml = "[[rules]]\nwhen.request.url_path = \"/api\"\nrespond = { text = \"ok\" }\n";
    std::fs::write(dir.path().join("apimock-rule-set.toml"), rs_toml).unwrap();
    let extra_toml = "[file_tree_view]\nshow_hidden = true\n";
    let cfg_path = write_apimock_toml(dir.path(), "fallback", extra_toml);

    let ws = Workspace::load(cfg_path).unwrap();
    let snap = ws.snapshot();
    let tree = snap.routes.file_tree.as_ref().expect("file tree present");

    let names: Vec<&str> = tree.entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&".env"), ".env should be visible when show_hidden=true");
}

#[test]
fn snapshot_extra_excludes_hides_named_dir() {
    let dir = tempfile::tempdir().unwrap();
    let fallback = dir.path().join("fallback");
    std::fs::create_dir_all(&fallback).unwrap();
    std::fs::write(fallback.join("users.json"), "{}").unwrap();
    std::fs::create_dir_all(fallback.join("generated")).unwrap();

    let rs_toml = "[[rules]]\nwhen.request.url_path = \"/api\"\nrespond = { text = \"ok\" }\n";
    std::fs::write(dir.path().join("apimock-rule-set.toml"), rs_toml).unwrap();
    let extra_toml = "[file_tree_view]\nextra_excludes = [\"generated\"]\n";
    let cfg_path = write_apimock_toml(dir.path(), "fallback", extra_toml);

    let ws = Workspace::load(cfg_path).unwrap();
    let snap = ws.snapshot();
    let tree = snap.routes.file_tree.as_ref().expect("file tree present");

    let names: Vec<&str> = tree.entries.iter().map(|e| e.name.as_str()).collect();
    assert!(!names.contains(&"generated"), "generated/ should be excluded");
    assert!(names.contains(&"users.json"));
}

#[test]
fn snapshot_include_filter_shows_only_matching_files() {
    let dir = tempfile::tempdir().unwrap();
    let fallback = dir.path().join("fallback");
    std::fs::create_dir_all(&fallback).unwrap();
    std::fs::write(fallback.join("users.json"), "{}").unwrap();
    std::fs::write(fallback.join("schema.toml"), "").unwrap();

    let rs_toml = "[[rules]]\nwhen.request.url_path = \"/api\"\nrespond = { text = \"ok\" }\n";
    std::fs::write(dir.path().join("apimock-rule-set.toml"), rs_toml).unwrap();
    let extra_toml = "[file_tree_view]\ninclude = [\"*.json\"]\n";
    let cfg_path = write_apimock_toml(dir.path(), "fallback", extra_toml);

    let ws = Workspace::load(cfg_path).unwrap();
    let snap = ws.snapshot();
    let tree = snap.routes.file_tree.as_ref().expect("file tree present");

    let names: Vec<&str> = tree.entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"users.json"), "json file should be included");
    assert!(!names.contains(&"schema.toml"), "toml file should be excluded by include filter");
}

#[test]
fn update_root_setting_file_tree_show_hidden() {
    let (_dir, root) = make_workspace();
    let mut ws = Workspace::load(root).expect("load");

    let result = ws.apply(EditCommand::UpdateRootSetting {
        key: crate::view::RootSettingKey::FileTreeShowHidden,
        value: crate::view::EditValue::Boolean(true),
    });
    assert!(result.is_ok(), "FileTreeShowHidden update should succeed");
    assert!(
        ws.config().file_tree_view.as_ref().map(|c| c.show_hidden).unwrap_or(false),
        "show_hidden should be persisted on config"
    );
}
