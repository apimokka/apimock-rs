use super::*;
use std::fs;
use std::path::Path;

fn create_dir(path: &Path) {
    fs::create_dir_all(path).unwrap();
}

fn remove_dir(path: &Path) {
    if path.exists() {
        fs::remove_dir_all(path).unwrap();
    }
}

#[test]
fn test_same_path() {
    let base = std::env::temp_dir().join("test_same_path");
    create_dir(&base);

    let rel = relative_path(&base, &base).unwrap();
    assert_eq!(rel, PathBuf::from("."));

    remove_dir(&base);
}

#[test]
fn test_to_is_child() {
    let base = std::env::temp_dir().join("test_to_is_child");
    let child = base.join("child");
    create_dir(&child);

    let rel = relative_path(&base, &child).unwrap();
    assert_eq!(rel, PathBuf::from("child"));

    remove_dir(&base);
}

#[test]
fn test_to_is_parent() {
    let base = std::env::temp_dir().join("test_to_is_parent");
    let child = base.join("child");
    create_dir(&child);

    let rel = relative_path(&child, &base).unwrap();
    assert_eq!(rel, PathBuf::from(".."));

    remove_dir(&base);
}

#[test]
fn test_sibling_path() {
    let base = std::env::temp_dir().join("test_sibling_path");
    let a = base.join("a");
    let b = base.join("b");
    create_dir(&a);
    create_dir(&b);

    let rel = relative_path(&a, &b).unwrap();
    assert_eq!(rel, PathBuf::from("../b"));

    remove_dir(&base);
}

#[test]
fn test_completely_different() {
    let tmp = std::env::temp_dir();
    let a = tmp.join("completely_a");
    let b = tmp.join("completely_b");
    create_dir(&a);
    create_dir(&b);

    let rel = relative_path(&a, &b).unwrap();
    assert_eq!(rel, PathBuf::from("../completely_b"));

    remove_dir(&a);
    remove_dir(&b);
}

#[test]
fn test_nonexistent_paths_fail() {
    let base = std::env::temp_dir().join("this_path_should_not_exist_abc");
    let result = relative_path(&base, &base);
    assert!(result.is_err());
}
