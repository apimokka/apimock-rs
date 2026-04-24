use console::style;

/// multiple condition string connector with style
pub fn fmt_condition_connector() -> String {
    style(" && ").dim().to_string()
}
