use super::super::glob::glob_match;

#[test]
fn exact_match() {
    assert!(glob_match("hello", "hello"));
    assert!(!glob_match("hello", "hell"));
    assert!(!glob_match("hello", "ello"));
}

#[test]
fn star_wildcard() {
    assert!(glob_match("file*.txt", "file.txt"));
    assert!(glob_match("file*.txt", "file123.txt"));
    assert!(glob_match("file*.txt", "files/doc.txt"));
}

#[test]
fn question_wildcard() {
    assert!(glob_match("file?.txt", "file1.txt"));
    assert!(glob_match("file?.txt", "fileX.txt"));
    assert!(!glob_match("file?.txt", "file.txt"));
    assert!(!glob_match("file?.txt", "file12.txt"));
}

#[test]
fn star_and_question_combined() {
    assert!(glob_match("a*b?c", "axyzbxc"));
    assert!(glob_match("a*b?c", "ab1c"));
    assert!(!glob_match("a*b?c", "abc"));
}

#[test]
fn leading_and_trailing_star() {
    assert!(glob_match("*test*", "my_test_file"));
    assert!(glob_match("*test*", "test"));
    assert!(!glob_match("*test*", "tes"));
}

#[test]
fn only_star() {
    assert!(glob_match("*", ""));
    assert!(glob_match("*", "anything"));
}

#[test]
fn empty_cases() {
    assert!(glob_match("", ""));
    assert!(!glob_match("", "nonempty"));
}

#[test]
fn consecutive_stars() {
    assert!(glob_match("a**b", "acb"));
    assert!(glob_match("a***b", "ab"));
}

#[test]
fn trailing_question() {
    assert!(glob_match("file?", "file1"));
    assert!(!glob_match("file?", "file12"));
}

#[test]
fn unicode_characters() {
    assert!(glob_match("こんにちは*", "こんにちは世界"));
    assert!(glob_match("こんにち?世界", "こんにちは世界"));
}
