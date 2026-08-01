use super::super::rule_op::RuleOp;

struct TestCase<'a> {
    text: &'a str,
    checker: &'a str,
    expect: bool,
}

#[test]
fn equal() {
    let cases = vec![
        TestCase {
            text: "a",
            checker: "a",
            expect: true,
        },
        TestCase {
            text: "a",
            checker: "b",
            expect: false,
        },
        TestCase {
            text: "a",
            checker: "",
            expect: false,
        },
        TestCase {
            text: "",
            checker: "a",
            expect: false,
        },
        TestCase {
            text: "",
            checker: "",
            expect: true,
        },
    ];
    for case in cases {
        assert_eq!(RuleOp::Equal.is_match(case.text, case.checker), case.expect);
    }
}

#[test]
fn not_equal() {
    let cases = vec![
        TestCase {
            text: "a",
            checker: "a",
            expect: false,
        },
        TestCase {
            text: "a",
            checker: "b",
            expect: true,
        },
        TestCase {
            text: "a",
            checker: "",
            expect: true,
        },
        TestCase {
            text: "",
            checker: "a",
            expect: true,
        },
        TestCase {
            text: "",
            checker: "",
            expect: false,
        },
    ];
    for case in cases {
        assert_eq!(
            RuleOp::NotEqual.is_match(case.text, case.checker),
            case.expect
        );
    }
}

#[test]
fn starts_with() {
    let cases = vec![
        TestCase {
            text: "ab",
            checker: "a",
            expect: true,
        },
        TestCase {
            text: "a",
            checker: "a",
            expect: true,
        },
        TestCase {
            text: "ba",
            checker: "a",
            expect: false,
        },
        TestCase {
            text: "b",
            checker: "a",
            expect: false,
        },
        TestCase {
            text: "",
            checker: "a",
            expect: false,
        },
        TestCase {
            text: "",
            checker: "",
            expect: true,
        },
    ];
    for case in cases {
        assert_eq!(
            RuleOp::StartsWith.is_match(case.text, case.checker),
            case.expect
        );
    }
}

#[test]
fn contains() {
    let cases = vec![
        TestCase {
            text: "abc",
            checker: "a",
            expect: true,
        },
        TestCase {
            text: "ab",
            checker: "a",
            expect: true,
        },
        TestCase {
            text: "ba",
            checker: "a",
            expect: true,
        },
        TestCase {
            text: "bc",
            checker: "a",
            expect: false,
        },
        TestCase {
            text: " a",
            checker: "a",
            expect: true,
        },
        TestCase {
            text: "a ",
            checker: "a",
            expect: true,
        },
        TestCase {
            text: " a ",
            checker: "a",
            expect: true,
        },
        TestCase {
            text: " ",
            checker: " ",
            expect: true,
        },
        TestCase {
            text: "",
            checker: " ",
            expect: false,
        },
        TestCase {
            text: " ",
            checker: "",
            expect: true,
        },
        TestCase {
            text: "",
            checker: "",
            expect: true,
        },
    ];
    for case in cases {
        assert_eq!(
            RuleOp::Contains.is_match(case.text, case.checker),
            case.expect
        );
    }
}

/// work with src/util/wild_card tests
#[test]
fn wild_card() {
    let cases = vec![
        TestCase {
            text: "a",
            checker: "?",
            expect: true,
        },
        TestCase {
            text: "a",
            checker: "a?",
            expect: false,
        },
        TestCase {
            text: "aa",
            checker: "a?",
            expect: true,
        },
        TestCase {
            text: "ba",
            checker: "?a",
            expect: true,
        },
        TestCase {
            text: "a",
            checker: "a*",
            expect: true,
        },
        TestCase {
            text: "a",
            checker: "*a",
            expect: true,
        },
        TestCase {
            text: "aa",
            checker: "a*",
            expect: true,
        },
        TestCase {
            text: "ba",
            checker: "*a",
            expect: true,
        },
        TestCase {
            text: "ba",
            checker: "a*",
            expect: false,
        },
    ];
    for case in cases {
        assert_eq!(
            RuleOp::WildCard.is_match(case.text, case.checker),
            case.expect
        );
    }
}


// RFC 017: EndsWith and Regex tests
#[test]
fn ends_with() {
    let cases = vec![
        ("foobar", "bar", true),
        ("foobar", "foo", false),
        ("foobar", "foobar", true),
        ("foobar", "", true),
        ("", "x", false),
    ];
    for (text, checker, expect) in cases {
        assert_eq!(RuleOp::EndsWith.is_match(text, checker), expect,
            "EndsWith({:?}, {:?})", text, checker);
    }
}

#[test]
fn regex_match() {
    assert!(RuleOp::Regex.is_match("application/json", r"^application/(json|xml)$"));
    assert!(RuleOp::Regex.is_match("application/xml", r"^application/(json|xml)$"));
    assert!(!RuleOp::Regex.is_match("text/plain", r"^application/(json|xml)$"));
}

#[test]
fn regex_invalid_pattern_returns_false() {
    // Invalid pattern must not panic — return false gracefully.
    assert!(!RuleOp::Regex.is_match("anything", r"[invalid"));
}
